import { spawnSync } from 'node:child_process'
import { existsSync, readdirSync, realpathSync, statSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const [tool, ...args] = process.argv.slice(2)
if (tool !== 'tsc' && tool !== 'tsdown') {
  throw new Error(`run-tool: unsupported tool ${JSON.stringify(tool)}`)
}

const toolEntrypoints = {
  tsc: ['typescript', 'bin', 'tsc'],
  tsdown: ['tsdown', 'dist', 'run.mjs'],
}
const moduleRoots = [join(root, 'node_modules')]

const explicitSource = process.env.DSH_SOURCE_ROOT?.trim()
if (explicitSource) moduleRoots.push(join(explicitSource, 'node_modules'))

// A linked DSH peer package gives us the active source checkout without a
// machine-specific path. Walk upward through its possible workspace roots.
const peer = join(root, 'node_modules', '@deepseek-ai', 'dsh-tools')
if (existsSync(peer)) {
  let current = realpathSync(peer)
  for (;;) {
    moduleRoots.push(join(current, 'node_modules'))
    const parent = dirname(current)
    if (parent === current) break
    current = parent
  }
}

// Installed DSH source bundles are another supported build environment.
const sourceStore = join(homedir(), '.dsh', 'source')
if (existsSync(sourceStore)) {
  const sources = readdirSync(sourceStore)
    .map(name => join(sourceStore, name))
    .filter(path => statSync(path).isDirectory())
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)
  for (const source of sources) {
    moduleRoots.push(join(source, 'node_modules'))
  }
}

const uniqueModuleRoots = [...new Set(moduleRoots)]
const entrypoint = uniqueModuleRoots
  .map(moduleRoot => join(moduleRoot, ...toolEntrypoints[tool]))
  .find(candidate => existsSync(candidate))
if (entrypoint === undefined) {
  throw new Error(`run-tool: ${tool} is unavailable; install dev dependencies or set DSH_SOURCE_ROOT`)
}
const typeRoots = uniqueModuleRoots.map(moduleRoot => join(moduleRoot, '@types'))
const discoveredTypeRoots = [...new Set(typeRoots.filter(path => existsSync(path)))]
const forwardedArgs = [...args]
if (tool === 'tsc' && discoveredTypeRoots.length > 0) {
  forwardedArgs.push('--typeRoots', discoveredTypeRoots.join(','))
}
const env = {
  ...process.env,
}
// Execute the JavaScript entrypoint through Node instead of a package-manager
// .bin shim. Windows shims require cmd.exe while POSIX shims require a shell;
// the underlying JavaScript CLIs are portable and preserve arguments exactly.
const result = spawnSync(process.execPath, [entrypoint, ...forwardedArgs], {
  cwd: root,
  env,
  stdio: 'inherit',
})
if (result.error) throw result.error
process.exitCode = result.status ?? 1
