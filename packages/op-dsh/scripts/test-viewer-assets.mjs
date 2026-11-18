#!/usr/bin/env node
import { createHash } from 'node:crypto'
import { createServer } from 'node:http'
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  VIEWER_ASSET_FILES,
  VIEWER_ASSET_ROUTE_PREFIX,
  prepareViewerAssets,
} from '../lib/viewer-assets.js'

const root = await mkdtemp(join(tmpdir(), 'dsh-openpencil-viewer-assets-'))
try {
  const files = {}
  for (const name of VIEWER_ASSET_FILES) {
    const path = join(root, ...name.split('/'))
    await mkdir(join(path, '..'), { recursive: true })
    const bytes = Buffer.from(`fixture:${name}`)
    await writeFile(path, bytes)
    files[name] = { bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }
  }
  const revision = createHash('sha256').update('fixture').digest('hex').slice(0, 20)
  await writeFile(join(root, 'manifest.json'), JSON.stringify({ version: 1, revision, files }))

  const controller = await prepareViewerAssets({ assetDir: root })
  if (!controller.available || controller.viewerGrant !== undefined) throw new Error('controller availability lifecycle failed')
  const detach = controller.attachRoute()
  const grant = controller.viewerGrant
  if (!grant || !grant.canvasKitBaseUrl.endsWith('/canvaskit/')) throw new Error('viewer grant is invalid')

  const server = createServer((req, res) => void controller.handle(req, res))
  await new Promise((resolveListen, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', resolveListen)
  })
  const address = server.address()
  if (!address || typeof address === 'string') throw new Error('test server has no TCP address')
  const origin = `http://127.0.0.1:${address.port}`
  const js = await fetch(`${origin}${grant.sdkUrl}`)
  if (js.status !== 200 || !js.headers.get('content-type')?.startsWith('text/javascript')) throw new Error('SDK GET failed')
  const wasm = await fetch(`${origin}${grant.wasmUrl}`, { method: 'HEAD' })
  if (wasm.status !== 200 || wasm.headers.get('content-type') !== 'application/wasm') throw new Error('WASM HEAD failed')
  const traversal = await fetch(`${origin}${VIEWER_ASSET_ROUTE_PREFIX}/${revision}/missing.js`)
  if (traversal.status !== 404) throw new Error('unknown asset was not refused')
  const post = await fetch(`${origin}${grant.sdkUrl}`, { method: 'POST' })
  if (post.status !== 405 || post.headers.get('allow') !== 'GET, HEAD') throw new Error('method gate failed')
  await new Promise(resolveClose => server.close(resolveClose))
  detach()
  if (controller.viewerGrant !== undefined) throw new Error('route disposer failed')
  console.log('viewer asset controller smoke test passed')
} finally {
  await rm(root, { recursive: true, force: true })
}
