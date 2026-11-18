import { readFile, mkdir, rename, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')

export const DEFAULT_SOURCE_PATHS = [
  resolve(root, '../../crates/op-editor-ui/assets/iconify-catalog-core.json'),
  resolve(root, '../../crates/op-editor-ui/assets/iconify-catalog-brands.json'),
]
export const DEFAULT_OUTPUT_PATH = resolve(root, 'lib/assets/openpencil-design/icon-catalog.json')

const ICON_NAME = /^[a-z0-9][a-z0-9-]{0,63}$/
const COLLECTION_NAME = /^[a-z0-9][a-z0-9-]{0,63}$/

function fail(message) {
  throw new Error(`build-icon-catalog: ${message}`)
}

/** Fold the upstream editor icon catalogs into `{collection: [names…]}`. */
export function createIconCatalog(sources) {
  const families = new Map()
  for (const source of sources) {
    if (typeof source !== 'object' || source === null || !Array.isArray(source.icons)) {
      fail('upstream catalog must be an object with an icons array')
    }
    for (const icon of source.icons) {
      if (typeof icon !== 'object' || icon === null) fail('catalog entry is not an object')
      const { collection, name } = icon
      if (typeof collection !== 'string' || !COLLECTION_NAME.test(collection)) {
        fail(`catalog entry has an invalid collection: ${JSON.stringify(collection)}`)
      }
      if (typeof name !== 'string' || !ICON_NAME.test(name)) {
        fail(`catalog entry has an invalid icon name: ${JSON.stringify(name)}`)
      }
      let names = families.get(collection)
      if (names === undefined) families.set(collection, names = new Set())
      names.add(name)
    }
  }
  const lucide = families.get('lucide')
  if (lucide === undefined || lucide.size < 1000) {
    fail(`lucide collection is missing or unexpectedly small (${lucide?.size ?? 0})`)
  }
  for (const required of ['search', 'shopping-cart', 'sofa', 'bed', 'menu']) {
    if (!lucide.has(required)) fail(`lucide collection is missing sentinel glyph ${required}`)
  }
  const catalog = {}
  for (const collection of [...families.keys()].sort()) {
    catalog[collection] = [...families.get(collection)].sort()
  }
  return catalog
}

export async function buildIconCatalog(options = {}) {
  const sourcePaths = (options.sourcePaths ?? DEFAULT_SOURCE_PATHS).map(path => resolve(path))
  const outputPath = resolve(options.outputPath ?? DEFAULT_OUTPUT_PATH)
  const sources = []
  for (const path of sourcePaths) {
    try {
      sources.push(JSON.parse(await readFile(path, 'utf8')))
    } catch (error) {
      fail(`cannot read upstream catalog ${path}: ${error instanceof Error ? error.message : String(error)}`)
    }
  }
  const catalog = createIconCatalog(sources)
  const content = `${JSON.stringify(catalog)}\n`
  await mkdir(dirname(outputPath), { recursive: true })
  const temporary = `${outputPath}.tmp-${process.pid}`
  await writeFile(temporary, content, 'utf8')
  await rename(temporary, outputPath)
  return { outputPath, bytes: Buffer.byteLength(content), collections: Object.keys(catalog).length }
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : ''
if (import.meta.url === invokedPath) {
  const result = await buildIconCatalog()
  console.log(`built ${result.outputPath} (${result.collections} collections, ${result.bytes} bytes)`)
}
