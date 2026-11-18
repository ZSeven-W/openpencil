// Wrap the compiled client bundle into the loader-compatible artifact shape
// dsh-web requires: `window.__ModuleLoader__.load({ id, factory })`. The id
// must equal the plugin entry name (package name).
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = dirname(fileURLToPath(new URL('../package.json', import.meta.url)))
const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
const pluginId = manifest.name
if (typeof pluginId !== 'string' || pluginId.length === 0) {
  throw new TypeError('package.json must declare a non-empty package name')
}
const compiledPath = join(root, '.client-build', 'index.cjs')
const outputPath = join(root, 'lib', 'client.js')
const source = await readFile(compiledPath, 'utf8')
const wrapped = [
  `window.__ModuleLoader__.load({ id: ${JSON.stringify(pluginId)}, factory: (require) => {`,
  'var module = { exports: {} }; var exports = module.exports;',
  source.replace(/\n?\/\/# sourceMappingURL=.*$/u, ''),
  'return module.exports; } });',
  '',
].join('\n')

await mkdir(dirname(outputPath), { recursive: true })
await writeFile(outputPath, wrapped)
await rm(join(root, '.client-build'), { recursive: true, force: true })
