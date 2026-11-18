/**
 * Pin the package.json shape that keeps dsh-openpencil loadable on every DSH
 * runtime from 0.1.5 through 0.2.x.
 *
 * DSH 0.2.0 added a compatibility gate: the host semver-checks every
 * `@deepseek-ai/dsh` / `@deepseek-ai/dsh-*` range in the plugin's
 * peerDependencies against the running host version (includePrerelease) and
 * refuses to install or boot the bundle when any range fails. This plugin
 * therefore declares no host peer at all — host packages are pinned in
 * devDependencies for the build, and the inert `dshHostRuntime` field
 * documents the runtime contract.
 */

import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'
import semver from 'semver'

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const manifest = JSON.parse(await readFile(resolve(projectRoot, 'package.json'), 'utf8'))
const manifestText = await readFile(resolve(projectRoot, 'package.json'), 'utf8')

const HOST_PACKAGES = [
  '@deepseek-ai/dsh-client-locale',
  '@deepseek-ai/dsh-client-ui-renderer',
  '@deepseek-ai/dsh-client-ui-session',
  '@deepseek-ai/dsh-client-ui-conversation',
  '@deepseek-ai/dsh-client-ui-slots',
  '@deepseek-ai/dsh-client-ui-theme',
  '@deepseek-ai/dsh-client-ui-tool',
  '@deepseek-ai/dsh-fs',
  '@deepseek-ai/dsh-host-webserver',
  '@deepseek-ai/dsh-sandbox-policy',
  '@deepseek-ai/dsh-session',
  '@deepseek-ai/dsh-tools',
]

/**
 * Prefer the shipping host evaluator when the DSH checkout is reachable from
 * this worktree; the fallback re-implements the exact gate rule with the same
 * semver semantics so the assertion still works in a CI clone.
 */
let evaluatePluginCompatibility
const HARNESS_GATE = resolve(process.env.DSH_SOURCE_ROOT || resolve(projectRoot, '../../deepseek-harness'), 'packages/boot/app-boot/src/plugin-compatibility.ts')
try {
  ;({ evaluatePluginCompatibility } = await import(HARNESS_GATE))
} catch {
  evaluatePluginCompatibility = function minimalGate(manifestObject, _exemptions, runtimeVersion) {
    const fields = manifestObject ?? {}
    if (!Object.hasOwn(fields, 'peerDependencies')) return undefined
    const dependencies = fields.peerDependencies ?? {}
    const peers = {}
    for (const [name, range] of Object.entries(dependencies)) {
      if (typeof range !== 'string') {
        throw new Error(`Plugin manifest peerDependencies[${JSON.stringify(name)}] must be a string`)
      }
      if (name !== '@deepseek-ai/dsh' && !name.startsWith('@deepseek-ai/dsh-')) continue
      if (range.trim() === '' || !semver.satisfies(runtimeVersion, range, { includePrerelease: true })) {
        peers[name] = range
      }
    }
    if (Object.keys(peers).length === 0) return undefined
    return {
      name: fields.name,
      version: fields.version,
      runtimeVersion,
      peers,
      exempted: false,
    }
  }
}

test('package.json declares no @deepseek-ai host peers', () => {
  assert.equal(typeof manifest.peerDependencies, 'object')
  for (const name of Object.keys(manifest.peerDependencies)) {
    assert.equal(
      name === '@deepseek-ai/dsh' || name.startsWith('@deepseek-ai/dsh-'),
      false,
      `host package ${name} must not be a peerDependency`,
    )
  }
  assert.deepEqual(Object.keys(manifest.peerDependencies).sort(), ['react', 'react-dom'])
})

test('host packages are pinned to 0.2.0-rc.2 as devDependencies', () => {
  for (const name of HOST_PACKAGES) {
    assert.equal(manifest.devDependencies?.[name], '0.2.0-rc.2', `${name} devDependency pin`)
    assert.equal(manifest.dependencies?.[name], undefined, `${name} must not be a runtime dependency`)
    assert.equal(manifest.peerDependencies?.[name], undefined, `${name} must not be a peerDependency`)
  }
  assert.equal(manifest.devDependencies?.['@deepseek-ai/cordis'], '~4.0.4')
})

test('the injected client list names no removed runtime package', () => {
  const inject = manifest.dsh?.client?.inject
  // Spell the removed package name in two halves so this very test does not
  // become the one remaining occurrence of the literal in the repository.
  const removedRuntimePackage = '@deepseek-ai/dsh-client-' + 'runtime'
  assert.equal(Array.isArray(inject), true)
  assert.equal(inject.includes(removedRuntimePackage), false)
  // The removed package must not appear in the manifest at all, under any
  // spelling (full name or bare suffix).
  assert.equal(manifestText.includes(removedRuntimePackage), false)
  assert.equal(manifestText.includes(removedRuntimePackage.slice('@deepseek-ai/'.length)), false)
})

test('the DSH 0.2.0 compatibility gate admits this package', () => {
  assert.equal(evaluatePluginCompatibility(manifest, {}, '0.2.0-rc.2'), undefined)
})

test('the same manifest stays admitted on the 0.1.5 line', () => {
  assert.equal(evaluatePluginCompatibility(manifest, {}, '0.1.5'), undefined)
  assert.equal(evaluatePluginCompatibility(manifest, {}, '0.1.5-rc.1'), undefined)
})

test('the evaluator still rejects stale host peers (negative control)', () => {
  const stale = {
    ...manifest,
    peerDependencies: {
      ...manifest.peerDependencies,
      '@deepseek-ai/dsh-tools': '0.1.5-rc.1',
    },
  }
  assert.deepEqual(evaluatePluginCompatibility(stale, {}, '0.2.0-rc.2'), {
    name: manifest.name,
    version: manifest.version,
    runtimeVersion: '0.2.0-rc.2',
    peers: { '@deepseek-ai/dsh-tools': '0.1.5-rc.1' },
    exempted: false,
  })
  assert.equal(evaluatePluginCompatibility(stale, {}, '0.1.5-rc.1'), undefined)
})
