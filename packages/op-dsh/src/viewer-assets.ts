/**
 * Staged OpenPencil Web SDK assets + same-origin HTTP delivery.
 *
 * DSH only publishes the plugin's `client.js` automatically. The OpenPencil
 * viewer additionally needs its browser ESM bundle, the renderer WASM, and
 * CanvasKit's JS/WASM pair. `scripts/sync-viewer-assets.mjs` copies/builds
 * those files into `lib/viewer-assets`; this module validates that immutable
 * staging manifest and exposes a fixed, traversal-proof route.
 *
 * The controller deliberately degrades to `available === false` when the
 * default staging directory is absent. A headless or source-only install can
 * therefore keep using the PNG presentation path. An explicitly configured
 * invalid directory is treated as a configuration error and throws.
 * @module dsh-openpencil/viewer-assets
 */

import { createHash } from 'node:crypto'
import { constants as fsConstants } from 'node:fs'
import type { Stats } from 'node:fs'
import type { FileHandle } from 'node:fs/promises'
import { lstat, open, readFile, realpath } from 'node:fs/promises'
import type { IncomingMessage, ServerResponse } from 'node:http'
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import { pluginEnv, pluginEnvName } from './plugin-env.js'

/** HTTP namespace owned by the read-only OpenPencil viewer assets. */
export const VIEWER_ASSET_ROUTE_PREFIX = '/_dsh/dsh-openpencil/viewer-assets'

/** Suffix of the environment override for an externally staged viewer-asset directory. */
export const VIEWER_ASSET_DIR_ENV_SUFFIX = 'OPENPENCIL_VIEWER_ASSET_DIR'

/** Current name of that override (the legacy DSH_ name is still read). */
export const VIEWER_ASSET_DIR_ENV = pluginEnvName(VIEWER_ASSET_DIR_ENV_SUFFIX)

const MANIFEST_FILENAME = 'manifest.json'
const REVISION_PATTERN = /^[a-f0-9]{16,64}$/
const SHA256_PATTERN = /^[a-f0-9]{64}$/

/** The browser assets expected by the DSH read-only viewer. */
export const VIEWER_ASSET_FILES = [
  'sdk.js',
  'op_web_sdk_bg.wasm',
  'canvaskit/canvaskit.js',
  'canvaskit/canvaskit.wasm',
] as const

export type ViewerAssetName = typeof VIEWER_ASSET_FILES[number]

/** URLs handed to the browser half of the plugin. */
export interface ViewerGrant {
  sdkUrl: string
  wasmUrl: string
  canvasKitBaseUrl: string
}

interface ManifestFile {
  bytes: number
  sha256: string
}

interface ViewerAssetManifest {
  version: 1
  revision: string
  files: Record<ViewerAssetName, ManifestFile>
}

interface VerifiedAsset extends ManifestFile {
  name: ViewerAssetName
  path: string
  mimeType: string
  dev: number
  ino: number
}

/** Optional discovery controls, mainly useful to hosts and tests. */
export interface ViewerAssetOptions {
  assetDir?: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function mimeType(name: ViewerAssetName): string {
  if (name.endsWith('.wasm')) return 'application/wasm'
  return 'text/javascript; charset=utf-8'
}

function sha256(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex')
}

function parseManifest(value: unknown): ViewerAssetManifest {
  if (!isRecord(value) || value.version !== 1 || typeof value.revision !== 'string' || !REVISION_PATTERN.test(value.revision)) {
    throw new Error('viewer asset manifest has an invalid version or revision')
  }
  if (!isRecord(value.files)) throw new Error('viewer asset manifest has no files table')
  const files = {} as Record<ViewerAssetName, ManifestFile>
  for (const name of VIEWER_ASSET_FILES) {
    const descriptor = value.files[name]
    if (
      !isRecord(descriptor)
      || typeof descriptor.bytes !== 'number'
      || !Number.isSafeInteger(descriptor.bytes)
      || descriptor.bytes <= 0
      || typeof descriptor.sha256 !== 'string'
      || !SHA256_PATTERN.test(descriptor.sha256)
    ) {
      throw new Error(`viewer asset manifest entry is invalid: ${name}`)
    }
    files[name] = { bytes: descriptor.bytes, sha256: descriptor.sha256 }
  }
  return { version: 1, revision: value.revision, files }
}

function isContained(root: string, path: string): boolean {
  const rel = relative(root, path)
  return rel !== '' && rel !== '..' && !rel.startsWith(`..${sep}`) && !isAbsolute(rel)
}

async function verifyAssetDirectory(assetDir: string): Promise<{
  directory: string
  manifest: ViewerAssetManifest
  assets: ReadonlyMap<ViewerAssetName, VerifiedAsset>
}> {
  const directory = await realpath(assetDir)
  const rootInfo = await lstat(directory)
  if (!rootInfo.isDirectory()) throw new Error('viewer asset root is not a directory')
  const manifestPath = join(directory, MANIFEST_FILENAME)
  const manifestInfo = await lstat(manifestPath)
  if (manifestInfo.isSymbolicLink() || !manifestInfo.isFile()) {
    throw new Error('viewer asset manifest is not a regular file')
  }
  const manifest = parseManifest(JSON.parse(await readFile(manifestPath, 'utf8')))
  const assets = new Map<ViewerAssetName, VerifiedAsset>()
  for (const name of VIEWER_ASSET_FILES) {
    const unresolved = join(directory, ...name.split('/'))
    const pathInfo = await lstat(unresolved)
    if (pathInfo.isSymbolicLink() || !pathInfo.isFile()) {
      throw new Error(`viewer asset is not a regular file: ${name}`)
    }
    const path = await realpath(unresolved)
    if (!isContained(directory, path)) throw new Error(`viewer asset escaped its root: ${name}`)
    const bytes = await readFile(path)
    const descriptor = manifest.files[name]
    if (bytes.byteLength !== descriptor.bytes || sha256(bytes) !== descriptor.sha256) {
      throw new Error(`viewer asset does not match its manifest: ${name}`)
    }
    assets.set(name, {
      ...descriptor,
      name,
      path,
      mimeType: mimeType(name),
      dev: pathInfo.dev,
      ino: pathInfo.ino,
    })
  }
  return { directory, manifest, assets }
}

function candidateDirectories(explicit?: string): { explicit: boolean; paths: string[] } {
  const configured = explicit?.trim() || pluginEnv(VIEWER_ASSET_DIR_ENV_SUFFIX)?.trim()
  if (configured) return { explicit: true, paths: [resolve(configured)] }
  const moduleDir = dirname(fileURLToPath(import.meta.url))
  return {
    explicit: false,
    paths: [...new Set([
      join(moduleDir, 'viewer-assets'),
      join(moduleDir, '..', 'lib', 'viewer-assets'),
      join(process.cwd(), 'lib', 'viewer-assets'),
    ].map(path => resolve(path)))],
  }
}

function baseSecurityHeaders(res: ServerResponse): void {
  res.setHeader('X-Content-Type-Options', 'nosniff')
  res.setHeader('Referrer-Policy', 'no-referrer')
  res.setHeader('Cross-Origin-Resource-Policy', 'same-origin')
  res.setHeader('Cache-Control', 'private, max-age=31536000, immutable')
}

function sameFile(info: Stats, asset: VerifiedAsset): boolean {
  if (info.dev === 0 || info.ino === 0 || asset.dev === 0 || asset.ino === 0) return true
  return info.dev === asset.dev && info.ino === asset.ino
}

async function openVerifiedAsset(asset: VerifiedAsset): Promise<FileHandle> {
  const noFollow = typeof fsConstants.O_NOFOLLOW === 'number' ? fsConstants.O_NOFOLLOW : 0
  const handle = await open(asset.path, fsConstants.O_RDONLY | noFollow)
  try {
    const info = await handle.stat()
    if (!info.isFile() || info.size !== asset.bytes || !sameFile(info, asset)) {
      throw new Error('viewer asset changed after startup validation')
    }
    return handle
  } catch (error) {
    await handle.close().catch(() => {})
    throw error
  }
}

/**
 * Fixed-file controller for the staged SDK bundle.
 *
 * Use `prepareViewerAssets()` instead of constructing this directly. The
 * route attachment count mirrors `RenderAccessController`: browser grants
 * only appear while an HTTP route is actually mounted.
 */
export class ViewerAssetController {
  private routeCount = 0

  private constructor(
    readonly assetDirectory: string | undefined,
    readonly unavailableReason: string | undefined,
    private readonly revision: string | undefined,
    private readonly assets: ReadonlyMap<ViewerAssetName, VerifiedAsset>,
  ) {}

  static unavailable(reason: string): ViewerAssetController {
    return new ViewerAssetController(undefined, reason, undefined, new Map())
  }

  static ready(
    directory: string,
    revision: string,
    assets: ReadonlyMap<ViewerAssetName, VerifiedAsset>,
  ): ViewerAssetController {
    return new ViewerAssetController(directory, undefined, revision, assets)
  }

  /** Whether a complete, hash-verified SDK asset set was discovered. */
  get available(): boolean {
    return this.revision !== undefined && this.assets.size === VIEWER_ASSET_FILES.length
  }

  /** Whether at least one HTTP carrier currently owns the asset route. */
  get routeAvailable(): boolean {
    return this.available && this.routeCount > 0
  }

  /** Browser URLs, omitted unless both staging and the DSH route are ready. */
  get viewerGrant(): ViewerGrant | undefined {
    if (!this.routeAvailable || this.revision === undefined) return undefined
    const base = `${VIEWER_ASSET_ROUTE_PREFIX}/${this.revision}`
    return {
      sdkUrl: `${base}/sdk.js`,
      wasmUrl: `${base}/op_web_sdk_bg.wasm`,
      canvasKitBaseUrl: `${base}/canvaskit/`,
    }
  }

  /** Mark one registered prefix route; dispose it when the host route unloads. */
  attachRoute(): () => void {
    if (!this.available) throw new Error(this.unavailableReason ?? 'OpenPencil viewer assets are unavailable')
    this.routeCount += 1
    let active = true
    return () => {
      if (!active) return
      active = false
      this.routeCount -= 1
    }
  }

  /** Serve one allow-listed immutable asset over GET or HEAD. */
  async handle(req: IncomingMessage, res: ServerResponse): Promise<void> {
    baseSecurityHeaders(res)
    if (req.method !== 'GET' && req.method !== 'HEAD') {
      res.setHeader('Allow', 'GET, HEAD')
      res.writeHead(405)
      res.end()
      return
    }
    if (!this.available || this.revision === undefined) {
      res.writeHead(404)
      res.end()
      return
    }
    let url: URL
    try {
      url = new URL(req.url ?? '/', 'http://dsh.local')
    } catch {
      res.writeHead(400)
      res.end()
      return
    }
    if (url.search.length > 0) {
      res.writeHead(400)
      res.end()
      return
    }
    const routeBase = `${VIEWER_ASSET_ROUTE_PREFIX}/${this.revision}/`
    if (!url.pathname.startsWith(routeBase)) {
      res.writeHead(404)
      res.end()
      return
    }
    let name: string
    try {
      name = decodeURIComponent(url.pathname.slice(routeBase.length))
    } catch {
      res.writeHead(404)
      res.end()
      return
    }
    const asset = this.assets.get(name as ViewerAssetName)
    if (asset === undefined || asset.name !== name) {
      res.writeHead(404)
      res.end()
      return
    }
    let handle: FileHandle
    try {
      handle = await openVerifiedAsset(asset)
    } catch {
      res.writeHead(404)
      res.end()
      return
    }
    res.setHeader('Content-Type', asset.mimeType)
    res.setHeader('Content-Length', String(asset.bytes))
    res.setHeader('Content-Disposition', 'inline')
    res.writeHead(200)
    if (req.method === 'HEAD') {
      await handle.close().catch(() => {})
      res.end()
      return
    }
    const stream = handle.createReadStream({ autoClose: true })
    stream.on('error', () => {
      if (!res.headersSent) res.writeHead(500)
      res.destroy()
    })
    stream.pipe(res)
  }
}

/** Discover and hash-verify the staged Web SDK assets. */
export async function prepareViewerAssets(options: ViewerAssetOptions = {}): Promise<ViewerAssetController> {
  const candidates = candidateDirectories(options.assetDir)
  const failures: string[] = []
  for (const path of candidates.paths) {
    try {
      const verified = await verifyAssetDirectory(path)
      return ViewerAssetController.ready(verified.directory, verified.manifest.revision, verified.assets)
    } catch (error) {
      const code = (error as NodeJS.ErrnoException).code
      if (!candidates.explicit && (code === 'ENOENT' || code === 'ENOTDIR')) continue
      failures.push(`${path}: ${error instanceof Error ? error.message : String(error)}`)
    }
  }
  const reason = failures.length > 0
    ? failures.join('; ')
    : `run scripts/sync-viewer-assets.mjs to stage OpenPencil Web SDK assets under lib/viewer-assets`
  if (candidates.explicit) throw new Error(reason)
  return ViewerAssetController.unavailable(reason)
}
