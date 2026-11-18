import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { test } from 'node:test'

import {
  EditorRuntimeUnavailableError,
  editorRuntimeRequiredFiles,
  parseEditorRuntimeCatalog,
  parseEditorRuntimeManifest,
  resolveEditorRuntime,
  tryResolveEditorRuntime,
} from '../lib/editor-runtime.js'
import {
  collectRuntimePayloadPaths,
  editorBinarySourceCandidates,
  resolveWebBundleSource,
} from '../scripts/stage-editor-runtime.mjs'

const OPENPENCIL_VERSION = '0.8.5'
const OPENPENCIL_REVISION = '57f390cad94b84778527b3ef7415f63e1f46bd8b'
const PLATFORM = Object.freeze({
  id: 'darwin-arm64',
  os: 'darwin',
  cpu: 'arm64',
  rustTarget: 'aarch64-apple-darwin',
  runner: 'macos-15',
  packageName: '@zseven-w/dsh-openpencil-darwin-arm64',
  binaryName: 'op-host-web-server',
})

function catalog(platforms = [PLATFORM]) {
  return {
    schemaVersion: 1,
    openPencil: {
      version: OPENPENCIL_VERSION,
      revision: OPENPENCIL_REVISION,
    },
    platforms,
  }
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex')
}

async function stageRuntime(root, options = {}) {
  const binaryName = options.binaryName ?? PLATFORM.binaryName
  const layout = options.layout ?? 'canonical'
  const webRoot = layout === 'canonical' ? 'bin/web-bundle' : 'web/pkg'
  const canvasKitRoot = layout === 'canonical' ? `${webRoot}/canvaskit` : 'web/canvaskit'
  const contents = {
    [`bin/${binaryName}`]: Buffer.from('fake OpenPencil daemon'),
    [`${webRoot}/op_host_web.js`]: Buffer.from('export default "OpenPencil"'),
    [`${webRoot}/op_host_web_bg.wasm`]: Buffer.from([0, 97, 115, 109, 1]),
    [`${canvasKitRoot}/canvaskit.js`]: Buffer.from('globalThis.CanvasKitInit = () => {}'),
    [`${canvasKitRoot}/canvaskit.wasm`]: Buffer.from([0, 97, 115, 109, 2]),
    ...(options.contents ?? {}),
  }
  await Promise.all(Object.entries(contents).map(async ([relativePath, bytes]) => {
    const path = join(root, ...relativePath.split('/'))
    await mkdir(dirname(path), { recursive: true })
    await writeFile(path, bytes)
  }))
  await writeFile(join(root, 'package.json'), JSON.stringify({ name: PLATFORM.packageName }))
  const manifest = {
    schemaVersion: 1,
    platform: PLATFORM.id,
    packageName: PLATFORM.packageName,
    openPencilVersion: OPENPENCIL_VERSION,
    openPencilRevision: OPENPENCIL_REVISION,
    files: Object.fromEntries(Object.entries(contents).map(([path, bytes]) => [path, sha256(bytes)])),
    ...(options.manifest ?? {}),
  }
  await writeFile(join(root, 'openpencil-runtime.json'), JSON.stringify(manifest))
  return { contents, manifest }
}

async function tempRoot(label) {
  return mkdtemp(join(tmpdir(), `dsh-openpencil-${label}-`))
}

function resolutionOptions(projectRoot, overrides = {}) {
  return {
    platform: 'darwin',
    arch: 'arm64',
    env: {},
    projectRoot,
    catalog: catalog(),
    chmod: () => {},
    ...overrides,
  }
}

test('local staging prefers the fresh host build over a stale cross-target artifact', () => {
  const vendorRoot = join('/workspace', 'vendor', 'openpencil')
  const host = join(vendorRoot, 'target', 'release', PLATFORM.binaryName)
  const targeted = join(vendorRoot, 'target', PLATFORM.rustTarget, 'release', PLATFORM.binaryName)
  assert.deepEqual(editorBinarySourceCandidates(vendorRoot, PLATFORM, true), [host, targeted])
  assert.deepEqual(editorBinarySourceCandidates(vendorRoot, PLATFORM, false), [targeted, host])
})

test('fresh clone staging fails closed instead of relabeling the legacy pkg-ck bundle', async () => {
  const root = await tempRoot('runtime-fresh-clone')
  const vendorRoot = join(root, 'vendor', 'openpencil')
  const currentPkg = join(vendorRoot, 'crates', 'op-host-web', 'pkg')
  const legacyPkg = join(vendorRoot, 'crates', 'op-host-web', 'pkg-ck')
  const releasePkg = join(root, 'release-artifact', 'pkg')
  try {
    await mkdir(legacyPkg, { recursive: true })
    await Promise.all([
      writeFile(join(legacyPkg, 'op_host_web.js'), 'legacy bundle'),
      writeFile(join(legacyPkg, 'op_host_web_bg.wasm'), 'legacy wasm'),
    ])

    await assert.rejects(
      resolveWebBundleSource(vendorRoot),
      error => (
        /no default current web bundle found/.test(error.message)
        && error.message.includes(currentPkg)
        && !error.message.includes(legacyPkg)
      ),
    )

    await mkdir(currentPkg, { recursive: true })
    assert.equal(await resolveWebBundleSource(vendorRoot), currentPkg)

    await mkdir(releasePkg, { recursive: true })
    assert.equal(await resolveWebBundleSource(vendorRoot, releasePkg), releasePkg)
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('staging manifests every native and web payload file, including executable snippets', async () => {
  const root = await tempRoot('runtime-manifest-coverage')
  try {
    await Promise.all([
      'bin/op-host-web-server',
      'bin/web-bundle/op_host_web_bg.wasm',
      'bin/web-bundle/op_host_web.js',
      'bin/web-bundle/snippets/bindings/bridge.js',
      'bin/web-bundle/assets/templates/example.op',
      'bin/web-bundle/canvaskit/canvaskit.js',
      'bin/web-bundle/canvaskit/canvaskit.wasm',
    ].map(async relativePath => {
      const path = join(root, ...relativePath.split('/'))
      await mkdir(dirname(path), { recursive: true })
      await writeFile(path, relativePath)
    }))
    assert.deepEqual(await collectRuntimePayloadPaths(root), [
      'bin/op-host-web-server',
      'bin/web-bundle/assets/templates/example.op',
      'bin/web-bundle/canvaskit/canvaskit.js',
      'bin/web-bundle/canvaskit/canvaskit.wasm',
      'bin/web-bundle/op_host_web_bg.wasm',
      'bin/web-bundle/op_host_web.js',
      'bin/web-bundle/snippets/bindings/bridge.js',
    ])
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('pure catalog and manifest parsers bind package identity to one OpenPencil release', () => {
  const parsedCatalog = parseEditorRuntimeCatalog(catalog())
  assert.equal(parsedCatalog.platforms[0].packageName, PLATFORM.packageName)
  assert.equal(parsedCatalog.openPencil.version, OPENPENCIL_VERSION)

  const files = Object.fromEntries(editorRuntimeRequiredFiles(PLATFORM.binaryName).map(path => [path, 'a'.repeat(64)]))
  const parsedManifest = parseEditorRuntimeManifest({
    schemaVersion: 1,
    platform: PLATFORM.id,
    packageName: PLATFORM.packageName,
    openPencilVersion: OPENPENCIL_VERSION,
    openPencilRevision: OPENPENCIL_REVISION,
    files,
  }, {
    platform: PLATFORM.id,
    packageName: PLATFORM.packageName,
    openPencilVersion: OPENPENCIL_VERSION,
    openPencilRevision: OPENPENCIL_REVISION,
  })
  assert.deepEqual(parsedManifest.files, files)

  assert.throws(
    () => parseEditorRuntimeCatalog({ ...catalog(), schemaVersion: 2 }),
    error => error instanceof EditorRuntimeUnavailableError && error.code === 'invalid-catalog',
  )
  assert.throws(
    () => parseEditorRuntimeManifest({ ...parsedManifest, openPencilVersion: '0.8.4' }, {
      platform: PLATFORM.id,
      packageName: PLATFORM.packageName,
      openPencilVersion: OPENPENCIL_VERSION,
      openPencilRevision: OPENPENCIL_REVISION,
    }),
    /openPencilVersion must equal 0\.8\.5/,
  )
})

test('resolves and verifies the canonical optional platform package before development staging', async () => {
  const root = await tempRoot('runtime-optional')
  const packageRoot = join(root, 'optional-runtime')
  const chmodCalls = []
  try {
    await stageRuntime(packageRoot)
    const runtime = resolveEditorRuntime(resolutionOptions(root, {
      resolvePackageJson(specifier) {
        assert.equal(specifier, `${PLATFORM.packageName}/package.json`)
        return join(packageRoot, 'package.json')
      },
      chmod(path, mode) { chmodCalls.push([path, mode]) },
    }))
    assert.deepEqual(runtime, {
      binary: join(packageRoot, 'bin', PLATFORM.binaryName),
      webBundleDir: join(packageRoot, 'bin', 'web-bundle'),
      canvasKitDir: join(packageRoot, 'bin', 'web-bundle', 'canvaskit'),
      openPencilVersion: OPENPENCIL_VERSION,
      revision: OPENPENCIL_REVISION,
      source: 'optional-package',
    })
    assert.deepEqual(chmodCalls, [[runtime.binary, 0o755]])
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('continues to resolve and fully verify the published legacy platform layout', async () => {
  const root = await tempRoot('runtime-legacy')
  const packageRoot = join(root, 'legacy-runtime')
  try {
    await stageRuntime(packageRoot, { layout: 'legacy' })
    const runtime = resolveEditorRuntime(resolutionOptions(root, {
      resolvePackageJson: () => join(packageRoot, 'package.json'),
    }))
    assert.equal(runtime.webBundleDir, join(packageRoot, 'web', 'pkg'))
    assert.equal(runtime.canvasKitDir, join(packageRoot, 'web', 'canvaskit'))
    assert.equal(runtime.source, 'optional-package')

    await writeFile(join(packageRoot, 'web', 'pkg', 'op_host_web.js'), 'tampered legacy bundle')
    assert.throws(
      () => resolveEditorRuntime(resolutionOptions(root, {
        resolvePackageJson: () => join(packageRoot, 'package.json'),
      })),
      /SHA-256 mismatch/,
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('fails closed when canonical and legacy web payloads coexist', async () => {
  const root = await tempRoot('runtime-layout-conflict')
  const packageRoot = join(root, 'optional-runtime')
  try {
    await stageRuntime(packageRoot, {
      contents: {
        'web/pkg/op_host_web.js': Buffer.from('legacy JavaScript'),
        'web/pkg/op_host_web_bg.wasm': Buffer.from('legacy Wasm'),
        'web/canvaskit/canvaskit.js': Buffer.from('legacy CanvasKit JavaScript'),
        'web/canvaskit/canvaskit.wasm': Buffer.from('legacy CanvasKit Wasm'),
      },
    })
    assert.throws(
      () => resolveEditorRuntime(resolutionOptions(root, {
        resolvePackageJson: () => join(packageRoot, 'package.json'),
      })),
      error => (
        error instanceof EditorRuntimeUnavailableError
        && error.code === 'invalid-runtime'
        && /contains both canonical and legacy web layouts/.test(error.message)
      ),
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('falls back only to the atomic npm/<platform> development package', async () => {
  const root = await tempRoot('runtime-development')
  const developmentRoot = join(root, 'npm', PLATFORM.id)
  try {
    await stageRuntime(developmentRoot)
    const runtime = resolveEditorRuntime(resolutionOptions(root, {
      resolvePackageJson() { throw new Error('optional dependency absent') },
    }))
    assert.equal(runtime.source, 'development-package')
    assert.equal(runtime.binary, join(developmentRoot, 'bin', PLATFORM.binaryName))
    assert.equal(runtime.webBundleDir, join(developmentRoot, 'bin', 'web-bundle'))
    assert.equal(runtime.canvasKitDir, join(developmentRoot, 'bin', 'web-bundle', 'canvaskit'))
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('accepts complete canonical and legacy overrides, but rejects mixed layouts', async () => {
  const root = await tempRoot('runtime-override')
  const runtimeRoot = join(root, 'explicit-runtime')
  try {
    await stageRuntime(runtimeRoot)
    const env = {
      DSHPLUGIN_OPENPENCIL_EDITOR_BINARY: join(runtimeRoot, 'bin', PLATFORM.binaryName),
      DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR: join(runtimeRoot, 'bin', 'web-bundle'),
      DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR: join(runtimeRoot, 'bin', 'web-bundle', 'canvaskit'),
    }
    const runtime = resolveEditorRuntime(resolutionOptions(root, {
      env,
      resolvePackageJson() { throw new Error('override must win') },
    }))
    assert.equal(runtime.source, 'override')
    assert.equal(runtime.binary, env.DSHPLUGIN_OPENPENCIL_EDITOR_BINARY)

    assert.throws(
      () => resolveEditorRuntime(resolutionOptions(root, {
        env: { DSHPLUGIN_OPENPENCIL_EDITOR_BINARY: env.DSHPLUGIN_OPENPENCIL_EDITOR_BINARY },
      })),
      error => error instanceof EditorRuntimeUnavailableError && error.code === 'partial-override',
    )

    const legacyRoot = join(root, 'legacy-runtime')
    await stageRuntime(legacyRoot, { layout: 'legacy' })
    const legacyEnv = {
      DSHPLUGIN_OPENPENCIL_EDITOR_BINARY: join(legacyRoot, 'bin', PLATFORM.binaryName),
      DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR: join(legacyRoot, 'web', 'pkg'),
      DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR: join(legacyRoot, 'web', 'canvaskit'),
    }
    const legacyRuntime = resolveEditorRuntime(resolutionOptions(root, {
      env: legacyEnv,
      resolvePackageJson() { throw new Error('override must win') },
    }))
    assert.equal(legacyRuntime.webBundleDir, legacyEnv.DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR)
    assert.equal(legacyRuntime.canvasKitDir, legacyEnv.DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR)

    assert.throws(
      () => resolveEditorRuntime(resolutionOptions(root, {
        env: {
          ...env,
          DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR: join(runtimeRoot, 'web', 'canvaskit'),
        },
      })),
      error => error instanceof EditorRuntimeUnavailableError && error.code === 'invalid-override',
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('rejects a corrupt optional package instead of hiding it behind a development fallback', async () => {
  const root = await tempRoot('runtime-corrupt')
  const packageRoot = join(root, 'optional-runtime')
  const developmentRoot = join(root, 'npm', PLATFORM.id)
  try {
    await Promise.all([stageRuntime(packageRoot), stageRuntime(developmentRoot)])
    await writeFile(join(packageRoot, 'bin', 'web-bundle', 'op_host_web.js'), 'tampered')
    assert.throws(
      () => resolveEditorRuntime(resolutionOptions(root, {
        resolvePackageJson: () => join(packageRoot, 'package.json'),
      })),
      error => (
        error instanceof EditorRuntimeUnavailableError
        && error.code === 'invalid-runtime'
        && /SHA-256 mismatch/.test(error.message)
        && /expected [a-f0-9]{64}/.test(error.diagnostics[0])
        && /actual [a-f0-9]{64}/.test(error.diagnostics[1])
      ),
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('rejects manifest platform, version, revision, and required-file drift', async t => {
  const cases = [
    ['platform', 'darwin-x64', /platform must equal darwin-arm64/],
    ['openPencilVersion', '0.8.4', /openPencilVersion must equal 0\.8\.5/],
    ['openPencilRevision', 'a'.repeat(40), /openPencilRevision must equal/],
  ]
  for (const [field, value, pattern] of cases) {
    await t.test(`rejects ${field}`, async () => {
      const root = await tempRoot(`runtime-${field}`)
      const packageRoot = join(root, 'optional-runtime')
      try {
        await stageRuntime(packageRoot, { manifest: { [field]: value } })
        assert.throws(
          () => resolveEditorRuntime(resolutionOptions(root, {
            resolvePackageJson: () => join(packageRoot, 'package.json'),
          })),
          pattern,
        )
      } finally {
        await rm(root, { recursive: true, force: true })
      }
    })
  }

  await t.test('rejects a missing critical SHA-256 entry', async () => {
    const root = await tempRoot('runtime-missing-digest')
    const packageRoot = join(root, 'optional-runtime')
    try {
      const { manifest } = await stageRuntime(packageRoot)
      delete manifest.files['bin/web-bundle/canvaskit/canvaskit.wasm']
      await writeFile(join(packageRoot, 'openpencil-runtime.json'), JSON.stringify(manifest))
      assert.throws(
        () => resolveEditorRuntime(resolutionOptions(root, {
          resolvePackageJson: () => join(packageRoot, 'package.json'),
        })),
        /missing SHA-256 for bin\/web-bundle\/canvaskit\/canvaskit\.wasm/,
      )
    } finally {
      await rm(root, { recursive: true, force: true })
    }
  })
})

test('optional resolution returns undefined only for an absent or unsupported runtime', async () => {
  const root = await tempRoot('runtime-absent')
  try {
    const options = resolutionOptions(root, {
      resolvePackageJson() { throw new Error('MODULE_NOT_FOUND') },
    })
    assert.equal(tryResolveEditorRuntime(options), undefined)
    assert.equal(tryResolveEditorRuntime(resolutionOptions(root, {
      resolvePackageJson: () => join(root, 'npm', PLATFORM.id, 'package.json'),
    })), undefined)
    assert.throws(
      () => resolveEditorRuntime(options),
      error => (
        error instanceof EditorRuntimeUnavailableError
        && error.code === 'runtime-not-found'
        && error.diagnostics.some(line => line.includes(PLATFORM.packageName))
        && error.diagnostics.some(line => line.includes(join(root, 'npm', PLATFORM.id)))
      ),
    )

    assert.equal(tryResolveEditorRuntime({
      ...options,
      platform: 'freebsd',
      arch: 'x64',
    }), undefined)

    assert.throws(
      () => tryResolveEditorRuntime({
        ...options,
        env: { DSHPLUGIN_OPENPENCIL_EDITOR_BINARY: '/tmp/partial' },
      }),
      error => error instanceof EditorRuntimeUnavailableError && error.code === 'partial-override',
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('declared extra files are also integrity checked and unsafe paths are refused', async () => {
  const root = await tempRoot('runtime-extra')
  const packageRoot = join(root, 'optional-runtime')
  try {
    await stageRuntime(packageRoot, {
      contents: { 'bin/web-bundle/assets/catalog.json': Buffer.from('{"ok":true}') },
    })
    await writeFile(join(packageRoot, 'bin', 'web-bundle', 'assets', 'catalog.json'), '{"ok":false}')
    assert.throws(
      () => resolveEditorRuntime(resolutionOptions(root, {
        resolvePackageJson: () => join(packageRoot, 'package.json'),
      })),
      /SHA-256 mismatch/,
    )

    const raw = JSON.parse(await readFile(join(packageRoot, 'openpencil-runtime.json'), 'utf8'))
    raw.files['../escape'] = 'a'.repeat(64)
    assert.throws(
      () => parseEditorRuntimeManifest(raw, {
        platform: PLATFORM.id,
        packageName: PLATFORM.packageName,
        openPencilVersion: OPENPENCIL_VERSION,
        openPencilRevision: OPENPENCIL_REVISION,
      }),
      /unsafe path/,
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})
