#!/usr/bin/env node
/**
 * Stage the OpenPencil read-only Web SDK for DSH.
 *
 * Sources, in priority order:
 *   1. --source / DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE: a complete prebuilt directory
 *   2. --openpencil-root / OPENPENCIL_ROOT: an OpenPencil checkout
 *   3. the enclosing OpenPencil monorepo root (`../..`) or a sibling `../openpencil` checkout
 *
 * A checkout build uses its own esbuild installation, copies no files back to
 * OpenPencil, and patches CanvasKit URL resolution only in the temporary DSH
 * bundle. Output is an immutable, checksummed `lib/viewer-assets` directory.
 */

import { createHash, randomUUID } from 'node:crypto'
import { spawn } from 'node:child_process'
import { cp, mkdir, readFile, realpath, rename, rm, stat, writeFile } from 'node:fs/promises'
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const projectRoot = resolve(here, '..')
const REQUIRED_FILES = [
  'sdk.js',
  'op_web_sdk_bg.wasm',
  'canvaskit/canvaskit.js',
  'canvaskit/canvaskit.wasm',
]

function parseArgs(argv) {
  const options = {}
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index]
    if (!['--source', '--openpencil-root', '--out-dir', '--esbuild'].includes(key)) {
      throw new Error(`unknown argument: ${key}`)
    }
    const value = argv[index + 1]
    if (!value || value.startsWith('--')) throw new Error(`missing value for ${key}`)
    options[key.slice(2).replace(/-([a-z])/g, (_match, letter) => letter.toUpperCase())] = value
    index += 1
  }
  return options
}

function isContained(root, path) {
  const rel = relative(root, path)
  return rel !== '' && rel !== '..' && !rel.startsWith(`..${sep}`) && !isAbsolute(rel)
}

async function exists(path) {
  try {
    return (await stat(path)).isFile() || (await stat(path)).isDirectory()
  } catch (error) {
    if (error?.code === 'ENOENT' || error?.code === 'ENOTDIR') return false
    throw error
  }
}

async function requireFile(path, label) {
  const info = await stat(path).catch(() => undefined)
  if (!info?.isFile()) throw new Error(`${label} not found: ${path}`)
  return path
}

async function copyFileTree(source, destination) {
  await mkdir(dirname(destination), { recursive: true })
  await cp(source, destination, { force: true })
}

async function locateCheckout(configured) {
  const candidates = [
    configured,
    process.env.OPENPENCIL_ROOT,
    // Monorepo root wins automatically when packages/op-dsh is inside openpencil;
    // keep a sibling checkout fallback for detached test copies.
    resolve(projectRoot, '..', '..'),
    resolve(projectRoot, '..', 'openpencil'),
  ].filter(Boolean).map(value => resolve(value))
  for (const candidate of [...new Set(candidates)]) {
    if (
      await exists(join(candidate, 'packages', 'op-web-sdk', 'src', 'index.ts'))
      && await exists(join(candidate, 'crates', 'op-web-sdk', 'pkg', 'op_web_sdk.js'))
    ) return realpath(candidate)
  }
  throw new Error(`OpenPencil checkout not found (pass --openpencil-root or OPENPENCIL_ROOT)`)
}

async function locateEsbuild(checkout, configured) {
  const candidates = [
    configured,
    process.env.ESBUILD_BIN,
    join(projectRoot, 'node_modules', '.bin', 'esbuild'),
    join(checkout, 'node_modules', '.bin', 'esbuild'),
  ].filter(Boolean).map(value => resolve(value))
  for (const candidate of [...new Set(candidates)]) {
    const info = await stat(candidate).catch(() => undefined)
    if (info?.isFile()) return candidate
  }
  throw new Error(`esbuild not found (pass --esbuild or build OpenPencil's JS dependencies)`)
}

function run(command, args, cwd) {
  return new Promise((resolveRun, reject) => {
    const child = spawn(command, args, { cwd, stdio: 'inherit' })
    child.once('error', reject)
    child.once('exit', (code, signal) => {
      if (code === 0) resolveRun()
      else reject(new Error(`${command} exited with ${code ?? signal}`))
    })
  })
}

async function patchCheckoutSources(tempRoot) {
  const wasmSource = join(tempRoot, 'src', 'wasm.ts')
  const wasmText = await readFile(wasmSource, 'utf8')
  if (!wasmText.includes(`'virtual:op_web_sdk_wasm'`)) {
    throw new Error('OpenPencil SDK wasm seam changed; update the DSH sync script')
  }
  await writeFile(wasmSource, wasmText.replace(
    `'virtual:op_web_sdk_wasm'`,
    `'../wasm/op_web_sdk.js'`,
  ))

  const typesSource = join(tempRoot, 'src', 'types.ts')
  const typesText = await readFile(typesSource, 'utf8')
  if (!typesText.includes('  wasmUrl?: string;')) {
    throw new Error('OpenPencil SDK CreateViewerOptions changed; update the DSH sync script')
  }
  await writeFile(typesSource, typesText.replace(
    '  wasmUrl?: string;',
    '  wasmUrl?: string;\n  /** DSH extension: namespaced CanvasKit asset directory. */\n  canvasKitBaseUrl?: string;',
  ))

  const viewerSource = join(tempRoot, 'src', 'viewer.ts')
  const viewerText = await readFile(viewerSource, 'utf8')
  const createMarker = 'export async function createViewer(opts: CreateViewerOptions): Promise<OpViewer> {\n  await ensureWasm(opts.wasmUrl);'
  if (!viewerText.includes(createMarker)) {
    throw new Error('OpenPencil SDK createViewer seam changed; update the DSH sync script')
  }
  await writeFile(viewerSource, viewerText.replace(createMarker, [
    'export async function createViewer(opts: CreateViewerOptions): Promise<OpViewer> {',
    '  if (opts.canvasKitBaseUrl !== undefined) {',
    `    if (!/^\\/?[^?#]*\\/$/.test(opts.canvasKitBaseUrl)) throw new Error('op-web-sdk: canvasKitBaseUrl must end with /');`,
    `    globalThis.__OPENPENCIL_CANVASKIT_BASE_URL__ = opts.canvasKitBaseUrl;`,
    '  }',
    '  await ensureWasm(opts.wasmUrl);',
  ].join('\n')))

  const wasmEntry = join(tempRoot, 'wasm', 'op_web_sdk.js')
  const wasmEntryText = await readFile(wasmEntry, 'utf8')
  const bridgeImport = /from ['"]\.\/snippets\/([^/'"]+)\/src\/op_ck_bridge\.js['"]/.exec(wasmEntryText)
  if (bridgeImport?.[1] === undefined) {
    throw new Error('OpenPencil wasm-bindgen bridge import changed; update the DSH sync script')
  }
  const bridgeSource = join(tempRoot, 'wasm', 'snippets', bridgeImport[1], 'src', 'op_ck_bridge.js')
  const bridgeText = await readFile(bridgeSource, 'utf8')
  const scriptMarker = `  await loadScript('/canvaskit/canvaskit.js');`
  const locateMarker = `  const CK = await CanvasKitInit({ locateFile: (f) => '/canvaskit/' + f });`
  if (!bridgeText.includes(scriptMarker) || !bridgeText.includes(locateMarker)) {
    throw new Error('OpenPencil CanvasKit URL seam changed; update the DSH sync script')
  }
  const assetHelper = [
    '',
    'function openPencilCanvasKitAsset(filename) {',
    `  const configured = globalThis.__OPENPENCIL_CANVASKIT_BASE_URL__;`,
    `  const base = typeof configured === 'string' && configured.length > 0 ? configured : '/canvaskit/';`,
    '  return new URL(filename, new URL(base, globalThis.location.href)).href;',
    '}',
  ].join('\n')
  await writeFile(bridgeSource, bridgeText
    .replace('function copyBytes(u8) {', `${assetHelper}\n\nfunction copyBytes(u8) {`)
    .replace(scriptMarker, `  await loadScript(openPencilCanvasKitAsset('canvaskit.js'));`)
    .replace(locateMarker, `  const CK = await CanvasKitInit({ locateFile: (f) => openPencilCanvasKitAsset(f) });`))

  const globalTypes = [
    'export {};',
    'declare global {',
    '  var __OPENPENCIL_CANVASKIT_BASE_URL__: string | undefined;',
    '}',
    '',
  ].join('\n')
  await writeFile(join(tempRoot, 'src', 'dsh-viewer-assets.d.ts'), globalTypes)
}

async function stageFromCheckout(checkout, destination, configuredEsbuild) {
  const tempRoot = join(destination, '.sdk-build')
  await mkdir(tempRoot, { recursive: true })
  await cp(join(checkout, 'packages', 'op-web-sdk', 'src'), join(tempRoot, 'src'), { recursive: true })
  await cp(join(checkout, 'crates', 'op-web-sdk', 'pkg'), join(tempRoot, 'wasm'), { recursive: true })
  await patchCheckoutSources(tempRoot)
  const esbuild = await locateEsbuild(checkout, configuredEsbuild)
  await run(esbuild, [
    join(tempRoot, 'src', 'index.ts'),
    '--bundle',
    '--format=esm',
    '--platform=browser',
    '--target=es2022',
    // Strip esbuild's absolute/temporary source-path comments so identical
    // inputs produce the same immutable asset revision on every sync.
    '--minify-whitespace',
    `--outfile=${join(destination, 'sdk.js')}`,
    '--log-level=warning',
  ], projectRoot)
  const wasmRoot = join(checkout, 'crates', 'op-web-sdk', 'pkg')
  const rawWasm = join(wasmRoot, 'op_web_sdk_bg.wasm')
  const optimizedWasm = join(wasmRoot, 'op_web_sdk_bg.opt.wasm')
  let wasm = rawWasm
  if (await exists(optimizedWasm)) {
    const [rawInfo, optimizedInfo] = await Promise.all([stat(rawWasm), stat(optimizedWasm)])
    if (optimizedInfo.mtimeMs >= rawInfo.mtimeMs) {
      wasm = optimizedWasm
    } else {
      const [rawBytes, optimizedBytes] = await Promise.all([readFile(rawWasm), readFile(optimizedWasm)])
      const rawHash = createHash('sha256').update(rawBytes).digest('hex')
      const optimizedHash = createHash('sha256').update(optimizedBytes).digest('hex')
      if (rawHash === optimizedHash) {
        wasm = optimizedWasm
      } else {
        console.warn(`Ignoring stale optimized Web SDK WASM: ${optimizedWasm}`)
      }
    }
  }
  await copyFileTree(await requireFile(wasm, 'OpenPencil viewer WASM'), join(destination, 'op_web_sdk_bg.wasm'))
  const canvasKitRoot = join(checkout, 'crates', 'op-host-web', 'assets', 'canvaskit')
  await copyFileTree(await requireFile(join(canvasKitRoot, 'canvaskit.js'), 'CanvasKit JS'), join(destination, 'canvaskit', 'canvaskit.js'))
  await copyFileTree(await requireFile(join(canvasKitRoot, 'canvaskit.wasm'), 'CanvasKit WASM'), join(destination, 'canvaskit', 'canvaskit.wasm'))
  await rm(tempRoot, { recursive: true, force: true })
}

async function verifyStagedSdk(destination) {
  const sdk = await readFile(join(destination, 'sdk.js'), 'utf8')
  if (
    !sdk.includes('__OPENPENCIL_CANVASKIT_BASE_URL__')
    || !sdk.includes('openPencilCanvasKitAsset("canvaskit.js")')
  ) {
    throw new Error('staged SDK did not retain the namespaced CanvasKit URL patch')
  }
  const wasm = await readFile(join(destination, 'op_web_sdk_bg.wasm'))
  const module = await WebAssembly.compile(wasm)
  const envImports = WebAssembly.Module.imports(module).filter(entry => entry.module === 'env')
  if (envImports.length > 0) {
    throw new Error(`staged SDK WASM has ${envImports.length} unsupported env imports`)
  }
}

async function stageFromPrebuilt(source, destination) {
  const root = await realpath(source)
  for (const name of REQUIRED_FILES) {
    await copyFileTree(await requireFile(join(root, ...name.split('/')), `prebuilt viewer asset ${name}`), join(destination, ...name.split('/')))
  }
}

async function writeManifest(destination) {
  const files = {}
  for (const name of REQUIRED_FILES) {
    const bytes = await readFile(join(destination, ...name.split('/')))
    files[name] = {
      bytes: bytes.byteLength,
      sha256: createHash('sha256').update(bytes).digest('hex'),
    }
  }
  const revisionInput = REQUIRED_FILES.map(name => `${name}\0${files[name].bytes}\0${files[name].sha256}`).join('\n')
  const revision = createHash('sha256').update(revisionInput).digest('hex').slice(0, 20)
  await writeFile(join(destination, 'manifest.json'), `${JSON.stringify({ version: 1, revision, files }, null, 2)}\n`)
  return { revision, files }
}

async function main() {
  const options = parseArgs(process.argv.slice(2))
  const outDir = resolve(options.outDir ?? join(projectRoot, 'lib', 'viewer-assets'))
  if (!isContained(projectRoot, outDir) || outDir === projectRoot) {
    throw new Error(`refusing to write viewer assets outside the plugin workspace: ${outDir}`)
  }
  const tempDir = `${outDir}.tmp-${randomUUID()}`
  await mkdir(tempDir, { recursive: true })
  try {
    // Dual-read during the DSH_ -> DSHPLUGIN_ migration (see src/plugin-env.ts).
    const source = options.source
      ?? process.env.DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE
      ?? process.env.DSH_OPENPENCIL_VIEWER_SOURCE
    if (source) {
      await stageFromPrebuilt(resolve(source), tempDir)
    } else {
      const checkout = await locateCheckout(options.openpencilRoot)
      await stageFromCheckout(checkout, tempDir, options.esbuild)
    }
    await verifyStagedSdk(tempDir)
    const manifest = await writeManifest(tempDir)
    const previous = `${outDir}.previous-${randomUUID()}`
    if (await exists(outDir)) {
      await rename(outDir, previous)
      await rename(tempDir, outDir).catch(async error => {
        await rename(previous, outDir)
        throw error
      })
      await rm(previous, { recursive: true, force: true })
    } else {
      await mkdir(dirname(outDir), { recursive: true })
      await rename(tempDir, outDir)
    }
    const total = REQUIRED_FILES.reduce((sum, name) => sum + manifest.files[name].bytes, 0)
    console.log(`OpenPencil viewer assets staged: ${outDir}`)
    console.log(`revision=${manifest.revision} files=${REQUIRED_FILES.length} bytes=${total}`)
  } catch (error) {
    await rm(tempDir, { recursive: true, force: true })
    throw error
  }
}

await main()
