/**
 * Offscreen rendering + signed HTTP delivery for `.op` documents.
 *
 * `openpencil_render` snapshots the source document, uses OpenPencil's own
 * headless exporter for design-fidelity PNG output, retaining every
 * top-level frame for the conversation gallery, and only invokes Jian
 * as an explicitly disclosed runtime-preview fallback when the exact binary
 * is unavailable. Content-addressed image/document capabilities bind name,
 * byte length, and SHA-256 without exposing arbitrary host paths. Serving
 * refuses symbolic links so a delivered artifact cannot be redirected.
 *
 * Model-visible result values stay plain JSON; the browser-only envelope
 * rides `presentationMeta` (see `tool.ts`). Never return an ImageBlock.
 * @module dsh-openpencil/renderer
 */

import { createHash, createHmac, randomBytes, timingSafeEqual } from 'node:crypto'
import { constants as fsConstants, statSync } from 'node:fs'
import type { Stats } from 'node:fs'
import type { FileHandle } from 'node:fs/promises'
import { lstat, mkdir, mkdtemp, open, readFile, readdir, realpath, rename, rm, stat, writeFile } from 'node:fs/promises'
import type { IncomingMessage, ServerResponse } from 'node:http'
import { spawn } from 'node:child_process'
import { homedir } from 'node:os'
import { basename, delimiter, isAbsolute, join, relative, resolve, sep } from 'node:path'
import { randomUUID } from 'node:crypto'
import type { JsonValue } from './json-value.js'
import type { ViewerGrant } from './viewer-assets.js'
import type { EditorGrant } from './editor-host.js'
import { tryResolveEditorRuntime } from './editor-runtime.js'
import { parseOpenPencilMcpResponse } from './mcp-client.js'
import { OPENPENCIL_RENDER_TOOL_NAME } from './tool-names.js'
import { pluginEnv } from './plugin-env.js'

/** HTTP prefix owned by the render capability route. */
export const RENDER_ROUTE_PREFIX = '/_dsh/dsh-openpencil/render'

/** Presentation metadata key reserved by the browser half of this package. */
export const PRESENTATION_META_KEY = '$dshOpenPencil'

/** Refuse to deliver anything above this size (also enforced at render time). */
export const MAX_RENDER_BYTES = 32 * 1024 * 1024

/** Refuse unusually large source documents before copying them into managed state. */
export const MAX_DOCUMENT_BYTES = 64 * 1024 * 1024

/** Bound decoded image geometry as well as compressed bytes. */
const MAX_RENDER_DIMENSION = 32_768
const MAX_RENDER_PIXELS = 128 * 1024 * 1024

/** Hard per-render wall clock for either renderer. */
const RENDER_TIMEOUT_MS = 60_000

const KEY_BYTES = 32
const TOKEN_PATTERN = /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/
const MAX_TOKEN_LENGTH = 16 * 1024

/** One render delivery capability. */
interface LegacyRenderTokenPayload {
  v: 1
  path: string
  filename: string
  bytes: number
}

interface ArtifactTokenPayload {
  v: 2
  kind: 'image' | 'document'
  filename: string
  bytes: number
  sha256: string
}

type RenderTokenPayload = LegacyRenderTokenPayload | ArtifactTokenPayload

/** Browser grant paired to one rendered PNG. */
export interface RenderGrant {
  path: string
  previewUrl: string
  downloadUrl: string
  width?: number
  height?: number
  id?: string
  name?: string
  index?: number
}

/** Immutable image fields needed to mint a browser-only preview grant. */
export interface ImageArtifactPresentation {
  filename: string
  bytes: number
  sha256: string
  width?: number
  height?: number
  id?: string
  name?: string
  index?: number
}

/** One immutable top-level frame retained from an exact render. */
export interface RenderFrame {
  path: string
  filename: string
  mimeType: 'image/png'
  bytes: number
  width: number
  height: number
  sha256: string
  /** Canonical top-level node identity and user-facing name. */
  id?: string
  name?: string
  index?: number
}

export interface DocumentSnapshot {
  path: string
  filename: string
  mimeType: 'application/json'
  bytes: number
  sha256: string
}

export interface DocumentGrant extends DocumentSnapshot {
  url: string
  previewUrl: string
  downloadUrl: string
}

/** Minimal document-bearing result accepted by the document-only projector. */
export interface DocumentPresentationResult {
  path: string
  document: DocumentSnapshot
  /** Optional exact final preview paired with this published document. */
  preview?: RenderFrame
  autoOpenEditor?: boolean
  /** Pipeline identity used for the controlled live-draft -> published handoff. */
  draftId?: string
  liveCanvas?: boolean
  published?: boolean
}

/** Canonical result shape the tool returns and the envelope enriches. */
export interface RenderResult {
  path: string
  filename: string
  mimeType: 'image/png'
  kind: 'image'
  description: string
  sourceTool: typeof OPENPENCIL_RENDER_TOOL_NAME
  previewIntent: 'image'
  bytes: number
  width?: number
  height?: number
  sha256?: string
  sourcePath?: string
  renderer?: 'openpencil' | 'jian'
  rendererBinary?: string
  fidelity?: 'exact' | 'runtime-preview'
  warnings?: string[]
  /** Ordered top-level frames. The first entry is also exposed by legacy image fields. */
  frames?: RenderFrame[]
  frameCount?: number
  editable?: boolean
  /** Live-card intent: expand the editor once after a newly-created design renders. */
  autoOpenEditor?: boolean
  document?: DocumentSnapshot
  note?: string
}

function dshHome(): string {
  const env = process.env.DSH_HOME?.trim()
  return env === undefined || env.length === 0 ? join(homedir(), '.dsh') : resolve(env)
}

/** Plugin-managed state root (mirrors the dsh-vision-toolkit convention). */
export function stateRoot(): string {
  return join(dshHome(), 'cache', 'dsh-openpencil')
}

/** Plugin-managed render output directory. */
export function renderDir(): string {
  return join(stateRoot(), 'renders')
}

/** Content-addressed immutable `.op` snapshots served to the web viewer. */
export function snapshotDir(): string {
  return join(stateRoot(), 'snapshots')
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function mac(key: Buffer, payload: string): Buffer {
  return createHmac('sha256', key).update(payload).digest()
}

function safeEqual(left: Buffer, right: Buffer): boolean {
  return left.length === right.length && timingSafeEqual(left, right)
}

async function readKey(path: string): Promise<Buffer> {
  const info = await lstat(path)
  if (info.isSymbolicLink() || !info.isFile()) throw new Error('render access key is not a regular file')
  const key = await readFile(path)
  if (key.length !== KEY_BYTES) throw new Error('render access key has an invalid length')
  return key
}

/** Load or atomically create the per-DSH-home signing key. */
export async function prepareRenderAccessKey(): Promise<Buffer> {
  await mkdir(stateRoot(), { recursive: true, mode: 0o700 })
  const path = join(stateRoot(), 'render-access.key')
  try {
    return await readKey(path)
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error
  }
  const candidate = randomBytes(KEY_BYTES)
  try {
    await writeFile(path, candidate, { flag: 'wx', mode: 0o600 })
    return candidate
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'EEXIST') throw error
    return readKey(path)
  }
}

function parsePayload(value: unknown): RenderTokenPayload | undefined {
  if (!isRecord(value)) return undefined
  if (value.v === 2) {
    if (
      (value.kind !== 'image' && value.kind !== 'document')
      || typeof value.filename !== 'string'
      || basename(value.filename) !== value.filename
      || typeof value.bytes !== 'number'
      || !Number.isSafeInteger(value.bytes)
      || value.bytes <= 0
      || value.bytes > (value.kind === 'image' ? MAX_RENDER_BYTES : MAX_DOCUMENT_BYTES)
      || typeof value.sha256 !== 'string'
      || !/^[a-f0-9]{64}$/.test(value.sha256)
    ) return undefined
    if (value.kind === 'image' && !/^render-[A-Za-z0-9-]+\.png$/.test(value.filename)) return undefined
    if (value.kind === 'document' && value.filename !== `${value.sha256}.op`) return undefined
    return value as unknown as ArtifactTokenPayload
  }
  if (
    value.v !== 1
    || typeof value.path !== 'string'
    || !isAbsolute(value.path)
    || typeof value.filename !== 'string'
    || basename(value.path) !== value.filename
    || typeof value.bytes !== 'number'
    || !Number.isSafeInteger(value.bytes)
    || value.bytes < 0
    || value.bytes > MAX_RENDER_BYTES
  ) return undefined
  return value as unknown as RenderTokenPayload
}

function payloadPath(payload: RenderTokenPayload): string {
  if (payload.v === 1) return payload.path
  return join(payload.kind === 'image' ? renderDir() : snapshotDir(), payload.filename)
}

function payloadMimeType(payload: RenderTokenPayload): 'image/png' | 'application/json' {
  return payload.v === 1 || payload.kind === 'image' ? 'image/png' : 'application/json'
}

/**
 * Walk every path component from the managed root with `lstat`, refusing
 * any symbolic link — even one whose target stays inside the root — and
 * finish with a `realpath` containment check.
 */
async function assertNoSymlinkPath(root: string, path: string): Promise<void> {
  const rootInfo = await lstat(root)
  if (rootInfo.isSymbolicLink() || !rootInfo.isDirectory()) throw new Error('render root is not a real directory')
  const rel = relative(root, path)
  if (rel === '' || rel === '..' || rel.startsWith(`..${sep}`) || isAbsolute(rel)) {
    throw new Error('render path escaped its managed root')
  }
  let current = root
  const parts = rel.split(sep)
  for (let index = 0; index < parts.length; index += 1) {
    const part = parts[index]
    if (part === undefined || part.length === 0 || part === '.' || part === '..') throw new Error('render path is malformed')
    current = join(current, part)
    const info = await lstat(current)
    if (info.isSymbolicLink()) throw new Error('render path contains a symbolic link')
    const final = index === parts.length - 1
    if (final ? !info.isFile() : !info.isDirectory()) throw new Error('render path contains an unexpected entry type')
  }
  const [realRoot, realFile] = await Promise.all([realpath(root), realpath(path)])
  const relReal = relative(realRoot, realFile)
  if (relReal === '..' || relReal.startsWith(`..${sep}`) || isAbsolute(relReal)) {
    throw new Error('render path escaped its managed root')
  }
}

function sameFile(opened: Stats, current: Stats): boolean {
  if (opened.dev === 0 || current.dev === 0 || opened.ino === 0 || current.ino === 0) return true
  return opened.dev === current.dev && opened.ino === current.ino
}

async function openVerifiedRender(payload: RenderTokenPayload): Promise<{ handle: FileHandle; info: Stats }> {
  const root = payload.v === 1 || payload.kind === 'image' ? renderDir() : snapshotDir()
  const path = payloadPath(payload)
  await assertNoSymlinkPath(root, path)
  const noFollow = typeof fsConstants.O_NOFOLLOW === 'number' ? fsConstants.O_NOFOLLOW : 0
  const handle = await open(path, fsConstants.O_RDONLY | noFollow)
  try {
    const info = await handle.stat()
    if (!info.isFile() || info.size !== payload.bytes) throw new Error('render no longer matches its delivered descriptor')
    const current = await lstat(path)
    if (current.isSymbolicLink() || !current.isFile() || !sameFile(info, current)) {
      throw new Error('render changed while it was being opened')
    }
    if (payload.v === 2) {
      // Hash the same opened inode that will be served. Reading the pathname
      // here would leave a rename race between verification and streaming.
      const bytes = await handle.readFile()
      const digest = createHash('sha256').update(bytes).digest('hex')
      if (digest !== payload.sha256) throw new Error('artifact content hash changed')
    }
    await assertNoSymlinkPath(root, path)
    return { handle, info }
  } catch (error) {
    await handle.close().catch(() => {})
    throw error
  }
}

function asciiFilename(filename: string): string {
  const fallback = filename.replace(/[^A-Za-z0-9._-]/g, '_').slice(0, 160)
  return fallback.length === 0 ? 'render.png' : fallback
}

function securityHeaders(res: ServerResponse, payload: RenderTokenPayload, download: boolean): void {
  res.setHeader('Content-Type', payloadMimeType(payload))
  res.setHeader('Content-Length', String(payload.bytes))
  res.setHeader('Content-Disposition', `${download ? 'attachment' : 'inline'}; filename="${asciiFilename(payload.filename)}"`)
  res.setHeader('X-Content-Type-Options', 'nosniff')
  res.setHeader('Cache-Control', 'private, no-store, max-age=0')
  res.setHeader('Referrer-Policy', 'no-referrer')
  res.setHeader('Cross-Origin-Resource-Policy', 'same-origin')
  res.setHeader('Content-Security-Policy', "sandbox; default-src 'none'")
}

/** Signed render-capability encoder and safe route handler. */
export class RenderAccessController {
  private routeCount = 0

  constructor(private readonly key: Buffer) {
    if (key.length !== KEY_BYTES) throw new TypeError(`render access key must be ${KEY_BYTES} bytes`)
  }

  /** Whether at least one HTTP carrier currently owns the route. */
  get routeAvailable(): boolean {
    return this.routeCount > 0
  }

  /** Mark one route attachment; the returned disposer removes that attachment. */
  attachRoute(): () => void {
    this.routeCount += 1
    let active = true
    return () => {
      if (!active) return
      active = false
      this.routeCount -= 1
    }
  }

  /** Mint a deterministic, tamper-evident capability for one render result. */
  sign(result: RenderResult): string {
    if (result.sha256 !== undefined) {
      return this.signArtifact({
        kind: 'image',
        filename: result.filename,
        bytes: result.bytes,
        sha256: result.sha256,
      })
    }
    const payload: RenderTokenPayload = {
      v: 1,
      path: result.path,
      filename: result.filename,
      bytes: result.bytes,
    }
    const encoded = Buffer.from(JSON.stringify(payload)).toString('base64url')
    return `${encoded}.${mac(this.key, encoded).toString('base64url')}`
  }

  /** Mint an immutable capability without embedding an absolute local path. */
  signArtifact(artifact: Omit<ArtifactTokenPayload, 'v'>): string {
    const payload: ArtifactTokenPayload = { v: 2, ...artifact }
    const parsed = parsePayload(payload)
    if (parsed === undefined || parsed.v !== 2) throw new Error('invalid artifact descriptor')
    const encoded = Buffer.from(JSON.stringify(payload)).toString('base64url')
    return `${encoded}.${mac(this.key, encoded).toString('base64url')}`
  }

  /** Verify and decode one capability without touching the filesystem. */
  verify(token: string): RenderTokenPayload | undefined {
    if (token.length === 0 || token.length > MAX_TOKEN_LENGTH || !TOKEN_PATTERN.test(token)) return undefined
    const [encoded, signature] = token.split('.')
    if (encoded === undefined || signature === undefined) return undefined
    let supplied: Buffer
    try {
      supplied = Buffer.from(signature, 'base64url')
    } catch {
      return undefined
    }
    if (!safeEqual(mac(this.key, encoded), supplied)) return undefined
    try {
      return parsePayload(JSON.parse(Buffer.from(encoded, 'base64url').toString('utf8')))
    } catch {
      return undefined
    }
  }

  /** Serve one GET/HEAD capability request. */
  async handle(req: IncomingMessage, res: ServerResponse): Promise<void> {
    if (req.method !== 'GET' && req.method !== 'HEAD') {
      res.setHeader('Allow', 'GET, HEAD')
      res.writeHead(405)
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
    const prefix = `${RENDER_ROUTE_PREFIX}/`
    if (!url.pathname.startsWith(prefix) || url.pathname.slice(prefix.length).includes('/')) {
      res.writeHead(404)
      res.end()
      return
    }
    let token: string
    try {
      token = decodeURIComponent(url.pathname.slice(prefix.length))
    } catch {
      res.writeHead(404)
      res.end()
      return
    }
    const payload = this.verify(token)
    if (payload === undefined) {
      res.writeHead(404)
      res.end()
      return
    }
    const downloadValue = url.searchParams.get('download')
    if ([...url.searchParams.keys()].some(key => key !== 'download') || (downloadValue !== null && downloadValue !== '1')) {
      res.writeHead(400)
      res.end()
      return
    }
    let opened: { handle: FileHandle; info: Stats }
    try {
      opened = await openVerifiedRender(payload)
    } catch {
      res.writeHead(404)
      res.end()
      return
    }
    securityHeaders(res, payload, downloadValue === '1')
    res.writeHead(200)
    if (req.method === 'HEAD') {
      await opened.handle.close().catch(() => {})
      res.end()
      return
    }
    const stream = opened.handle.createReadStream({ autoClose: true, start: 0, end: opened.info.size - 1 })
    stream.on('error', () => {
      if (!res.headersSent) res.writeHead(500)
      res.destroy()
    })
    stream.pipe(res)
  }
}

/** Expand `~` / `~/` prefixes (the model frequently writes home-relative paths). */
export function expandUserHome(raw: string): string {
  if (raw === '~') return homedir()
  if (raw.startsWith('~/') || raw.startsWith(`~${sep}`)) return join(homedir(), raw.slice(2))
  return raw
}

/**
 * Validate the `.op` input path: expand `~`, resolve against the session
 * workspace, realpath, and require a regular `.op` file.
 */
export async function resolveInputFile(raw: string, cwd: string): Promise<string> {
  if (typeof raw !== 'string' || raw.trim().length === 0) {
    throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: path is required`)
  }
  const expanded = expandUserHome(raw.trim())
  const target = isAbsolute(expanded) ? expanded : resolve(cwd, expanded)
  let real: string
  try {
    real = await realpath(target)
  } catch {
    throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: .op file not found: ${raw}`)
  }
  const info = await stat(real)
  if (!info.isFile()) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: not a regular file: ${raw}`)
  const extension = real.slice(real.lastIndexOf('.')).toLowerCase()
  if (extension !== '.op') {
    throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: expected a .op file (got "${extension || '(none)'}")`)
  }
  return real
}

async function persistDocumentSnapshot(bytes: Buffer): Promise<DocumentSnapshot> {
  if (bytes.length === 0) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: source document is empty`)
  if (bytes.length > MAX_DOCUMENT_BYTES) {
    throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: source document exceeds ${MAX_DOCUMENT_BYTES} bytes`)
  }
  const sha256 = createHash('sha256').update(bytes).digest('hex')
  const filename = `${sha256}.op`
  await mkdir(snapshotDir(), { recursive: true, mode: 0o700 })
  const path = join(snapshotDir(), filename)
  try {
    await writeFile(path, bytes, { flag: 'wx', mode: 0o600 })
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'EEXIST') throw error
    const existing = await readFile(path)
    if (existing.length !== bytes.length || createHash('sha256').update(existing).digest('hex') !== sha256) {
      throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: content-addressed snapshot was modified`)
    }
  }
  return { path, filename, mimeType: 'application/json', bytes: bytes.length, sha256 }
}

/** Freeze source bytes before rendering so preview and web viewer cannot diverge. */
export async function createDocumentSnapshot(input: string): Promise<DocumentSnapshot> {
  return persistDocumentSnapshot(await readFile(input))
}

/**
 * Freeze an authoritative document string without reopening its published
 * source path. `openpencil_new` uses the exact `writeText().after` value so a
 * path replacement cannot create a TOCTOU split between the saved result and
 * the document capability handed to the editor.
 */
export async function createDocumentSnapshotFromText(documentJson: string): Promise<DocumentSnapshot> {
  return persistDocumentSnapshot(Buffer.from(documentJson, 'utf8'))
}

const WINDOWS_EXECUTABLE_SUFFIXES = ['.exe', '.com']

function platformExecutableCandidates(basePath: string): string[] {
  if (process.platform !== 'win32') return [basePath]
  const lower = basePath.toLowerCase()
  if (WINDOWS_EXECUTABLE_SUFFIXES.some(suffix => lower.endsWith(suffix))) return [basePath]
  return [basePath, ...WINDOWS_EXECUTABLE_SUFFIXES.map(suffix => `${basePath}${suffix}`)]
}

function firstExistingFile(candidates: readonly string[]): string | undefined {
  for (const candidate of candidates) {
    try {
      if (statSync(candidate).isFile()) return candidate
    } catch {
      // keep looking
    }
  }
  return undefined
}

/** Locate the exact OpenPencil renderer, preferring an explicit override. */
export function findOpenPencilBinary(): string | undefined {
  const override = pluginEnv('OPENPENCIL_BINARY')?.trim()
    || pluginEnv('OPENPENCIL_DESKTOP')?.trim()
  const candidates = [
    ...(override === undefined || override.length === 0 ? [] : platformExecutableCandidates(expandUserHome(override))),
    '/Applications/OpenPencil.app/Contents/MacOS/openpencil-desktop',
    join(homedir(), 'Applications', 'OpenPencil.app', 'Contents', 'MacOS', 'openpencil-desktop'),
  ]
  const pathEnv = process.env.PATH ?? ''
  for (const dir of pathEnv.split(delimiter)) {
    if (dir.length > 0) candidates.push(...platformExecutableCandidates(join(dir, 'openpencil-desktop')))
  }
  const desktop = firstExistingFile(candidates)
  if (desktop !== undefined) return desktop
  try {
    const runtimeBinary = tryResolveEditorRuntime()?.binary
    if (runtimeBinary !== undefined) return runtimeBinary
  } catch {
    // Fall back to PATH lookup if runtime configuration is invalid.
  }
  const hostCandidates: string[] = []
  for (const dir of pathEnv.split(delimiter)) {
    if (dir.length > 0) hostCandidates.push(...platformExecutableCandidates(join(dir, 'op-host-web-server')))
  }
  return firstExistingFile(hostCandidates)
}

/** Locate the `jian` binary: env override, known build location, then PATH. */
export function findJianBinary(): string {
  const override = pluginEnv('OPENPENCIL_JIAN')?.trim()
  if (override !== undefined && override.length > 0) {
    const expanded = expandUserHome(override)
    return firstExistingFile(platformExecutableCandidates(expanded)) ?? expanded
  }
  const candidates = [...platformExecutableCandidates(join(homedir(), 'workspace', 'jian', 'target', 'release', 'jian'))]
  const pathEnv = process.env.PATH ?? ''
  for (const dir of pathEnv.split(delimiter)) {
    if (dir.length > 0) candidates.push(...platformExecutableCandidates(join(dir, 'jian')))
  }
  return firstExistingFile(candidates) ?? 'jian'
}

/** Parse the physical size from `jian render`'s success line. */
export function parseRenderSize(stdout: string): { width?: number; height?: number } {
  const match = /\((\d+)x(\d+) physical/.exec(stdout)
  if (match === null) return {}
  return { width: Number(match[1]), height: Number(match[2]) }
}

/**
 * Run `jian render` for one input and wait for the PNG on disk.
 * Observes `signal` so caller cancellation stops the child promptly.
 */
export async function runJianRender(options: {
  binary: string
  input: string
  out: string
  width?: number
  height?: number
  scale?: number
  signal: AbortSignal
}): Promise<{ stdout: string; stderr: string }> {
  const args = ['render', options.input, '--out', options.out]
  if (options.width !== undefined) args.push('--width', String(options.width))
  if (options.height !== undefined) args.push('--height', String(options.height))
  if (options.scale !== undefined && options.scale !== 1) args.push('--scale', String(options.scale))

  const child = spawn(options.binary, args, {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: process.env,
  })
  const stdoutChunks: Buffer[] = []
  const stderrChunks: Buffer[] = []
  child.stdout.on('data', (chunk: Buffer) => stdoutChunks.push(chunk))
  child.stderr.on('data', (chunk: Buffer) => stderrChunks.push(chunk))

  const timeout = setTimeout(() => {
    child.kill('SIGKILL')
  }, RENDER_TIMEOUT_MS)

  const onAbort = (): void => {
    child.kill('SIGKILL')
  }
  options.signal.addEventListener('abort', onAbort, { once: true })

  try {
    const code = await new Promise<number | null>((resolveCode, reject) => {
      child.on('error', reject)
      child.on('close', (closeCode) => resolveCode(closeCode))
    })
    clearTimeout(timeout)
    const stdout = Buffer.concat(stdoutChunks).toString('utf8')
    const stderr = Buffer.concat(stderrChunks).toString('utf8')
    if (code !== 0) {
      const detail = stderr.trim() !== '' ? stderr.trim() : stdout.trim()
      throw new Error(`jian render exited with ${String(code)}${detail !== '' ? `: ${detail}` : ''}`)
    }
    return { stdout, stderr }
  } finally {
    clearTimeout(timeout)
    options.signal.removeEventListener('abort', onAbort)
  }
}

export class RendererBinaryMissingError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options)
    this.name = 'RendererBinaryMissingError'
  }
}

interface SourceFrameDescriptor {
  id: string
  name?: string
  index: number
}

function rendererFilename(id: string): string {
  return `${id.replace(/[^A-Za-z0-9_-]/g, '_')}.png`
}

/** Read active-page top-level metadata without changing renderer semantics. */
async function sourceFrameDescriptors(path: string): Promise<SourceFrameDescriptor[]> {
  try {
    const value: unknown = JSON.parse(await readFile(path, 'utf8'))
    if (!isRecord(value)) return []
    const editorMeta = isRecord(value.editorMeta) ? value.editorMeta : undefined
    const rawPageIndex = editorMeta?.activePageIndex ?? editorMeta?.active_page_index
    const pageIndex = typeof rawPageIndex === 'number' && Number.isInteger(rawPageIndex) && rawPageIndex >= 0
      ? rawPageIndex
      : 0
    const pages = Array.isArray(value.pages) ? value.pages.filter(isRecord) : undefined
    const selectedPage = pages === undefined || pages.length === 0
      ? undefined
      : pages[Math.min(pageIndex, pages.length - 1)]
    const children = Array.isArray(selectedPage?.children)
      ? selectedPage.children
      : Array.isArray(value.children) ? value.children : []
    return children.flatMap((child, index) => {
      if (!isRecord(child) || typeof child.id !== 'string' || child.id.length === 0) return []
      return [{
        id: child.id,
        ...(typeof child.name === 'string' && child.name.length > 0 ? { name: child.name } : {}),
        index,
      }]
    })
  } catch {
    // The exact renderer remains authoritative; metadata is presentation-only.
    return []
  }
}

function unwrapMcpTextResult(tool: string, rawLine: string): unknown {
  const envelope: unknown = JSON.parse(rawLine)
  if (!isRecord(envelope)) throw new Error(`OpenPencil MCP ${tool} returned an invalid JSON-RPC response`)
  if (isRecord(envelope.error)) {
    const message = typeof envelope.error.message === 'string'
      ? envelope.error.message
      : JSON.stringify(envelope.error)
    throw new Error(`OpenPencil MCP ${tool} failed: ${message}`)
  }
  if (!isRecord(envelope.result) || !Array.isArray(envelope.result.content)) {
    throw new Error(`OpenPencil MCP ${tool} omitted its result`)
  }
  const textItem = envelope.result.content.find(
    (item): item is Record<string, unknown> => isRecord(item) && item.type === 'text' && typeof item.text === 'string',
  )
  const rawText = typeof textItem?.text === 'string' ? textItem.text : ''
  if (envelope.result.isError === true) {
    throw new Error(`OpenPencil MCP ${tool} failed${rawText === '' ? '' : `: ${rawText}`}`)
  }
  return rawText === '' ? undefined : JSON.parse(rawText)
}

async function runOpenPencilMcpRender(options: {
  binary: string
  input: string
  scale: number
  signal: AbortSignal
}): Promise<{
  png: string
  frames: Array<{ png: string; id?: string; name?: string; index: number }>
  warnings: string[]
  stdout: string
  stderr: string
}> {
  const sourceFrames = await sourceFrameDescriptors(options.input)
  const child = spawn(options.binary, ['--mcp', options.input], {
    stdio: ['pipe', 'pipe', 'pipe'],
    env: process.env,
  })
  const stdoutChunks: Buffer[] = []
  const stderrChunks: Buffer[] = []
  child.stdout.on('data', (chunk: Buffer) => stdoutChunks.push(chunk))
  child.stderr.on('data', (chunk: Buffer) => stderrChunks.push(chunk))
  let timedOut = false
  const timeout = setTimeout(() => {
    timedOut = true
    child.kill('SIGKILL')
  }, RENDER_TIMEOUT_MS)
  const onAbort = (): void => { child.kill('SIGKILL') }
  options.signal.addEventListener('abort', onAbort, { once: true })
  try {
    child.stdin.write(`${JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'tools/call',
      params: { name: 'get_screenshot', arguments: { nodeId: 'root' } },
    })}\n`)
    if (sourceFrames.length > 0) {
      child.stdin.write(`${JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: 'export_nodes',
          arguments: {
            nodeIds: JSON.stringify(sourceFrames.map(frame => frame.id)),
            format: 'png',
            scale: String(options.scale),
          },
        },
      })}\n`)
    }
    child.stdin.end()

    const code = await new Promise<number | null>((resolveCode, reject) => {
      child.on('error', (error: NodeJS.ErrnoException) => {
        if (error.code === 'ENOENT') reject(new RendererBinaryMissingError(`OpenPencil binary not found: ${options.binary}`, { cause: error }))
        else reject(error)
      })
      child.on('close', resolveCode)
    })
    const stdout = Buffer.concat(stdoutChunks).toString('utf8')
    const stderr = Buffer.concat(stderrChunks).toString('utf8')
    if (options.signal.aborted) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: OpenPencil render was cancelled`)
    if (timedOut) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: OpenPencil render timed out after ${RENDER_TIMEOUT_MS} ms`)
    if (code !== 0) throw new Error(`OpenPencil render exited with ${String(code)}${stderr.trim() === '' ? '' : `: ${stderr.trim()}`}`)

    const lines = stdout
      .split(/\r?\n/)
      .map(line => line.trim())
      .filter(line => line.length > 0)
    const rootParsed = lines[0] === undefined
      ? undefined
      : parseOpenPencilMcpResponse('get_screenshot', JSON.parse(lines[0]))
    const frames: Array<{ png: string; id?: string; name?: string; index: number }> = []
    try {
      if (sourceFrames.length > 0 && lines[1] !== undefined) {
        const exported = unwrapMcpTextResult('export_nodes', lines[1])
        const files = isRecord(exported) && Array.isArray(exported.files)
          ? exported.files.filter(isRecord)
          : []
        const sourceById = new Map(sourceFrames.map(frame => [frame.id, frame]))
        for (const [index, entry] of files.entries()) {
          if (typeof entry.bytes_base64 !== 'string' || entry.bytes_base64.length === 0) {
            const detail = typeof entry.error === 'string' ? entry.error : 'missing PNG bytes'
            throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: OpenPencil failed to render node ${String(entry.nodeId ?? index)}: ${detail}`)
          }
          const out = await createRenderOutput()
          await writeFile(out, Buffer.from(entry.bytes_base64, 'base64'), { mode: 0o600 })
          const nodeId = typeof entry.nodeId === 'string' && entry.nodeId.length > 0 ? entry.nodeId : undefined
          const sourceFrame = nodeId === undefined ? undefined : sourceById.get(nodeId)
          frames.push({
            png: out,
            ...(sourceFrame?.id !== undefined ? { id: sourceFrame.id } : nodeId !== undefined ? { id: nodeId } : {}),
            ...(sourceFrame?.name === undefined ? {} : { name: sourceFrame.name }),
            index,
          })
        }
      } else if (rootParsed !== undefined && rootParsed.images.length > 0) {
        const out = await createRenderOutput()
        await writeFile(out, rootParsed.images[0]!.bytes, { mode: 0o600 })
        const rootId = isRecord(rootParsed.value) && typeof rootParsed.value.nodeId === 'string'
          ? rootParsed.value.nodeId
          : undefined
        frames.push({
          png: out,
          ...(rootId === undefined ? {} : { id: rootId }),
          index: 0,
        })
      }
      if (frames.length === 0) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: OpenPencil produced no PNG`)
    } catch (error) {
      await Promise.all(frames.map(frame => rm(frame.png, { force: true }).catch(() => {})))
      throw error
    }
    return { png: frames[0]!.png, frames, warnings: [], stdout, stderr }
  } finally {
    clearTimeout(timeout)
    options.signal.removeEventListener('abort', onAbort)
  }
}

/** Render all top-level active-page nodes through OpenPencil's exact scene exporter. */
export async function runOpenPencilRender(options: {
  binary: string
  input: string
  scale?: number
  signal: AbortSignal
}): Promise<{
  png: string
  frames: Array<{ png: string; id?: string; name?: string; index: number }>
  warnings: string[]
  stdout: string
  stderr: string
}> {
  const scale = options.scale ?? 1
  if (!Number.isFinite(scale) || scale <= 0 || scale > 8) {
    throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: scale must be a finite number greater than 0 and at most 8`)
  }
  if (/^op-host-web-server(?:\.exe|\.com)?$/i.test(basename(options.binary))) {
    return runOpenPencilMcpRender({ binary: options.binary, input: options.input, scale, signal: options.signal })
  }
  await mkdir(stateRoot(), { recursive: true, mode: 0o700 })
  const tempRoot = await mkdtemp(join(stateRoot(), 'exact-'))
  const outDir = join(tempRoot, 'shots')
  await mkdir(outDir, { mode: 0o700 })
  const env: NodeJS.ProcessEnv = { ...process.env, OPENPENCIL_RENDER_MARGIN: '0' }
  delete env.OPENPENCIL_DUMP_LAYOUT
  const child = spawn(options.binary, ['--render-shots', options.input, outDir, String(scale)], {
    stdio: ['ignore', 'pipe', 'pipe'],
    env,
  })
  const stdoutChunks: Buffer[] = []
  const stderrChunks: Buffer[] = []
  child.stdout.on('data', (chunk: Buffer) => stdoutChunks.push(chunk))
  child.stderr.on('data', (chunk: Buffer) => stderrChunks.push(chunk))
  let timedOut = false
  const timeout = setTimeout(() => {
    timedOut = true
    child.kill('SIGKILL')
  }, RENDER_TIMEOUT_MS)
  const onAbort = (): void => { child.kill('SIGKILL') }
  options.signal.addEventListener('abort', onAbort, { once: true })
  try {
    const code = await new Promise<number | null>((resolveCode, reject) => {
      child.on('error', (error: NodeJS.ErrnoException) => {
        if (error.code === 'ENOENT') reject(new RendererBinaryMissingError(`OpenPencil binary not found: ${options.binary}`, { cause: error }))
        else reject(error)
      })
      child.on('close', resolveCode)
    })
    const stdout = Buffer.concat(stdoutChunks).toString('utf8')
    const stderr = Buffer.concat(stderrChunks).toString('utf8')
    if (options.signal.aborted) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: OpenPencil render was cancelled`)
    if (timedOut) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: OpenPencil render timed out after ${RENDER_TIMEOUT_MS} ms`)
    if (code !== 0 && /unknown mode ["']?--render-shots/i.test(stderr)) {
      return await runOpenPencilMcpRender({ binary: options.binary, input: options.input, scale, signal: options.signal })
    }
    if (code !== 0) throw new Error(`OpenPencil render exited with ${String(code)}${stderr.trim() === '' ? '' : `: ${stderr.trim()}`}`)
    const discoveredPngs = (await readdir(outDir)).filter(name => name.toLowerCase().endsWith('.png'))
    const sourceFrames = await sourceFrameDescriptors(options.input)
    const writtenPrefix = 'render-shots: wrote '
    const writtenPngs = stderr
      .split(/\r?\n/)
      .filter(line => line.startsWith(writtenPrefix))
      .map(line => basename(line.slice(writtenPrefix.length).trim()))
      .filter(name => discoveredPngs.includes(name))
    const sourceOrderedPngs = sourceFrames
      .map(frame => rendererFilename(frame.id))
      .filter(name => discoveredPngs.includes(name))
    // Prefer canonical active-page child order. The CLI also reports files in
    // that order, which covers document variants our lightweight JSON metadata
    // reader does not understand. Unknown files are appended stably.
    const pngs = [...new Set([
      ...sourceOrderedPngs,
      ...writtenPngs,
      ...discoveredPngs
        .filter(name => !sourceOrderedPngs.includes(name) && !writtenPngs.includes(name))
        .sort(),
    ])]
    if (pngs.length === 0) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: OpenPencil produced no PNG`)
    const sourceFrameByPng = new Map(sourceFrames.map(frame => [rendererFilename(frame.id), frame]))
    const frames: Array<{ png: string; id?: string; name?: string; index: number }> = []
    try {
      for (const [index, filename] of pngs.entries()) {
        const out = await createRenderOutput()
        await rename(join(outDir, filename), out)
        const sourceFrame = sourceFrameByPng.get(filename)
        frames.push({
          png: out,
          ...(sourceFrame?.id === undefined ? {} : { id: sourceFrame.id }),
          ...(sourceFrame?.name === undefined ? {} : { name: sourceFrame.name }),
          index,
        })
      }
    } catch (error) {
      await Promise.all(frames.map(frame => rm(frame.png, { force: true }).catch(() => {})))
      throw error
    }
    return { png: frames[0]!.png, frames, warnings: [], stdout, stderr }
  } finally {
    clearTimeout(timeout)
    options.signal.removeEventListener('abort', onAbort)
    await rm(tempRoot, { recursive: true, force: true }).catch(() => {})
  }
}

/** Mint one immutable document capability while retaining its workspace path. */
function projectDocumentCapability(
  document: DocumentSnapshot,
  sourcePath: string,
  controller: RenderAccessController,
): DocumentGrant {
  const token = controller.signArtifact({
    kind: 'document',
    filename: document.filename,
    bytes: document.bytes,
    sha256: document.sha256,
  })
  const url = `${RENDER_ROUTE_PREFIX}/${token}`
  return {
    ...document,
    path: sourcePath,
    url,
    previewUrl: url,
    downloadUrl: `${url}?download=1`,
  }
}

/**
 * Project a newly-created document directly into the existing editor
 * workbench contract. No image grant is minted and no renderer is involved.
 */
export function projectDocumentGrant(
  value: JsonValue,
  controller: RenderAccessController,
  editor?: EditorGrant,
): JsonValue {
  if (!controller.routeAvailable || !isRecord(value)) return value
  const result = value as unknown as DocumentPresentationResult
  if (
    typeof result.path !== 'string'
    || !isAbsolute(result.path)
    || result.document === undefined
    || typeof result.document.filename !== 'string'
    || typeof result.document.bytes !== 'number'
    || typeof result.document.sha256 !== 'string'
  ) return value
  const document = projectDocumentCapability(result.document, result.path, controller)
  const draftId = typeof result.draftId === 'string' && /^[A-Za-z0-9_-]{32}$/.test(result.draftId)
    ? result.draftId
    : undefined
  const liveDraft = draftId !== undefined && result.liveCanvas === true && result.published === false
    ? true
    : draftId !== undefined && result.published === true
      ? false
      : undefined
  const preview = result.preview
  const image = preview === undefined
    ? undefined
    : {
        path: preview.path,
        previewUrl: `${RENDER_ROUTE_PREFIX}/${controller.signArtifact({
          kind: 'image',
          filename: preview.filename,
          bytes: preview.bytes,
          sha256: preview.sha256,
        })}`,
        downloadUrl: `${RENDER_ROUTE_PREFIX}/${controller.signArtifact({
          kind: 'image',
          filename: preview.filename,
          bytes: preview.bytes,
          sha256: preview.sha256,
        })}?download=1`,
        width: preview.width,
        height: preview.height,
        ...(preview.id === undefined ? {} : { id: preview.id }),
        ...(preview.name === undefined ? {} : { name: preview.name }),
        index: preview.index ?? 0,
      }
  const envelope = {
    schemaVersion: 2,
    ...(image === undefined ? {} : { image, frames: [image] }),
    document,
    sourcePath: result.path,
    ...(editor === undefined ? {} : { editor }),
    ...(editor === undefined || result.autoOpenEditor !== true ? {} : { autoOpenEditor: true }),
    ...(draftId === undefined ? {} : { draftId }),
    ...(liveDraft === undefined ? {} : { liveDraft }),
  }
  return { ...value, [PRESENTATION_META_KEY]: envelope } as unknown as JsonValue
}

/**
 * Purely enrich a canonical tool-result value with a browser render grant.
 * Returns the value unchanged when no route/artifact exists.
 */
export function projectRenderGrant(
  value: JsonValue,
  controller: RenderAccessController,
  viewer?: ViewerGrant,
  editor?: EditorGrant,
): JsonValue {
  if (!controller.routeAvailable || !isRecord(value)) return value
  if (
    typeof value.path !== 'string'
    || typeof value.filename !== 'string'
    || typeof value.bytes !== 'number'
  ) return value
  const result = value as unknown as RenderResult
  const grantFor = (frame: RenderFrame, index?: number): RenderGrant => {
    const token = controller.signArtifact({
      kind: 'image',
      filename: frame.filename,
      bytes: frame.bytes,
      sha256: frame.sha256,
    })
    return {
      path: frame.path,
      previewUrl: `${RENDER_ROUTE_PREFIX}/${token}`,
      downloadUrl: `${RENDER_ROUTE_PREFIX}/${token}?download=1`,
      width: frame.width,
      height: frame.height,
      ...(frame.id === undefined ? {} : { id: frame.id }),
      ...(frame.name === undefined ? {} : { name: frame.name }),
      ...(index === undefined ? {} : { index }),
    }
  }
  const projectedFrames = result.frames?.map((frame, index) => grantFor(frame, index))
  const primaryToken = controller.sign(result)
  const grant: RenderGrant = projectedFrames?.[0] ?? {
    path: result.path,
    previewUrl: `${RENDER_ROUTE_PREFIX}/${primaryToken}`,
    downloadUrl: `${RENDER_ROUTE_PREFIX}/${primaryToken}?download=1`,
    ...(result.width === undefined ? {} : { width: result.width }),
    ...(result.height === undefined ? {} : { height: result.height }),
  }
  let document: DocumentGrant | undefined
  if (result.document !== undefined) {
    document = projectDocumentCapability(
      result.document,
      result.sourcePath ?? result.document.path,
      controller,
    )
  }
  const envelope = {
    schemaVersion: projectedFrames === undefined ? 1 : 2,
    image: grant,
    ...(projectedFrames === undefined ? {} : { frames: projectedFrames }),
    ...(document === undefined ? {} : { document }),
    ...(result.sourcePath === undefined ? {} : { sourcePath: result.sourcePath }),
    ...(result.renderer === undefined ? {} : { renderer: result.renderer }),
    ...(result.rendererBinary === undefined ? {} : { rendererBinary: result.rendererBinary }),
    ...(result.fidelity === undefined ? {} : { fidelity: result.fidelity }),
    ...(result.warnings === undefined ? {} : { warnings: result.warnings }),
    ...(viewer === undefined ? {} : { viewer }),
    ...(editor === undefined ? {} : { editor }),
    ...(editor === undefined || result.autoOpenEditor !== true ? {} : { autoOpenEditor: true }),
  }
  return { ...value, [PRESENTATION_META_KEY]: envelope } as unknown as JsonValue
}

/**
 * Add one immutable image preview without disclosing its managed host path.
 *
 * The artifact must already live in `renderDir()` under `filename`. The v2
 * capability binds only filename, byte length, and digest; the presentation
 * envelope deliberately uses that basename as its display path so a browser
 * never receives the private cache directory.
 */
export function projectImageArtifactGrant(
  value: JsonValue,
  controller: RenderAccessController,
  artifact: ImageArtifactPresentation,
): JsonValue {
  if (!controller.routeAvailable || !isRecord(value)) return value
  if (
    typeof artifact.filename !== 'string'
    || basename(artifact.filename) !== artifact.filename
    || typeof artifact.bytes !== 'number'
    || !Number.isSafeInteger(artifact.bytes)
    || artifact.bytes <= 0
    || typeof artifact.sha256 !== 'string'
  ) return value
  const token = controller.signArtifact({
    kind: 'image',
    filename: artifact.filename,
    bytes: artifact.bytes,
    sha256: artifact.sha256,
  })
  const url = `${RENDER_ROUTE_PREFIX}/${token}`
  const image: RenderGrant = {
    path: artifact.filename,
    previewUrl: url,
    downloadUrl: `${url}?download=1`,
    ...(artifact.width === undefined ? {} : { width: artifact.width }),
    ...(artifact.height === undefined ? {} : { height: artifact.height }),
    ...(artifact.id === undefined ? {} : { id: artifact.id }),
    ...(artifact.name === undefined ? {} : { name: artifact.name }),
    ...(artifact.index === undefined ? {} : { index: artifact.index }),
  }
  return {
    ...value,
    [PRESENTATION_META_KEY]: {
      schemaVersion: 2,
      image,
      frames: [image],
    },
  } as unknown as JsonValue
}

/** Reserve a fresh output path inside the managed render directory. */
export async function createRenderOutput(): Promise<string> {
  await mkdir(renderDir(), { recursive: true, mode: 0o700 })
  return join(renderDir(), `render-${randomUUID()}.png`)
}

/** Stat + cap-check a freshly rendered PNG. */
export async function verifyRenderOutput(out: string): Promise<{
  bytes: number
  width: number
  height: number
  sha256: string
}> {
  const info = await lstat(out)
  if (info.isSymbolicLink() || !info.isFile()) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: renderer did not produce a regular file`)
  if (info.size > MAX_RENDER_BYTES) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: rendered PNG exceeds ${MAX_RENDER_BYTES} bytes`)
  if (info.size < 33) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: rendered PNG is truncated`)
  const bytes = await readFile(out)
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  if (!bytes.subarray(0, 8).equals(signature)) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: renderer output is not a PNG`)
  if (bytes.readUInt32BE(8) !== 13 || bytes.toString('ascii', 12, 16) !== 'IHDR') {
    throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: PNG has no valid IHDR`)
  }
  const width = bytes.readUInt32BE(16)
  const height = bytes.readUInt32BE(20)
  if (width === 0 || height === 0 || width > MAX_RENDER_DIMENSION || height > MAX_RENDER_DIMENSION) {
    throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: rendered PNG dimensions are invalid`)
  }
  if (width * height > MAX_RENDER_PIXELS) throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: rendered PNG has too many pixels`)
  return {
    bytes: info.size,
    width,
    height,
    sha256: createHash('sha256').update(bytes).digest('hex'),
  }
}
