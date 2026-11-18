#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { createServer } from 'node:http'
import { copyFile, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { basename, join } from 'node:path'

const fixture = process.argv[2]
if (!fixture) throw new Error('usage: node scripts/test-host.mjs <design.op> [expected-width expected-height]')
const expectedWidth = process.argv[3] === undefined ? undefined : Number(process.argv[3])
const expectedHeight = process.argv[4] === undefined ? undefined : Number(process.argv[4])

// Consume ignored bodies before daemon/server disposal. Leaving a response
// paused can race socket shutdown in the Desktop runtime's Undici client.
async function drainedFetch(url, options) {
  const response = await fetch(url, options)
  await response.arrayBuffer()
  return response
}

const root = await mkdtemp(join(tmpdir(), 'dsh-openpencil-host-'))
process.env.DSH_HOME = join(root, 'dsh-home')

const {
  RENDER_ROUTE_PREFIX,
  RenderAccessController,
  createDocumentSnapshot,
  findOpenPencilBinary,
  projectRenderGrant,
  runOpenPencilRender,
  verifyRenderOutput,
} = await import('../lib/renderer.js')
const {
  VIEWER_ASSET_ROUTE_PREFIX,
  prepareViewerAssets,
} = await import('../lib/viewer-assets.js')
const {
  EDITOR_ROUTE_PREFIX,
  EditorHostController,
} = await import('../lib/editor-host.js')
const { OPENPENCIL_RENDER_TOOL_NAME } = await import('../lib/tool-names.js')

let server
let editorHost
try {
  const mutableSource = join(root, basename(fixture))
  await copyFile(fixture, mutableSource)
  const sourceBytes = await readFile(mutableSource)
  const sourceHash = createHash('sha256').update(sourceBytes).digest('hex')
  const sourceDocument = JSON.parse(sourceBytes.toString('utf8'))
  const pageIndex = sourceDocument.editorMeta?.activePageIndex ?? sourceDocument.editorMeta?.active_page_index ?? 0
  const pages = Array.isArray(sourceDocument.pages) ? sourceDocument.pages : undefined
  const expectedFrames = pages?.[Math.min(pageIndex, pages.length - 1)]?.children ?? sourceDocument.children ?? []
  const snapshot = await createDocumentSnapshot(mutableSource)
  assert.equal(snapshot.sha256, sourceHash)
  assert.equal(snapshot.filename, `${sourceHash}.op`)

  // The immutable browser/render artifact must not follow later source edits.
  await writeFile(mutableSource, '{"version":"mutated-after-snapshot"}\n')
  assert.deepEqual(await readFile(snapshot.path), sourceBytes)
  await writeFile(mutableSource, sourceBytes)

  const binary = findOpenPencilBinary()
  assert.ok(binary, 'OpenPencil exact renderer should be installed for this smoke test')
  const exact = await runOpenPencilRender({
    binary,
    input: snapshot.path,
    scale: 1,
    signal: new AbortController().signal,
  })
  assert.equal(exact.frames.length, expectedFrames.length)
  assert.deepEqual(exact.frames.map(frame => frame.id), expectedFrames.map(frame => frame.id))
  assert.deepEqual(exact.frames.map(frame => frame.name), expectedFrames.map(frame => frame.name))
  assert.deepEqual(exact.frames.map(frame => frame.index), expectedFrames.map((_, index) => index))
  const verifiedFrames = await Promise.all(exact.frames.map(async frame => ({
    path: frame.png,
    filename: basename(frame.png),
    mimeType: 'image/png',
    ...await verifyRenderOutput(frame.png),
    id: frame.id,
    name: frame.name,
    index: frame.index,
  })))
  const image = verifiedFrames[0]
  if (expectedWidth !== undefined) assert.equal(image.width, expectedWidth)
  if (expectedHeight !== undefined) assert.equal(image.height, expectedHeight)

  const access = new RenderAccessController(Buffer.alloc(32, 7))
  const viewerAssets = await prepareViewerAssets()
  assert.equal(viewerAssets.available, true)
  const detachRender = access.attachRoute()
  const detachViewer = viewerAssets.attachRoute()
  const editorMasterKey = Buffer.alloc(32, 11)
  editorHost = new EditorHostController(editorMasterKey)
  assert.equal(editorHost.available, true)
  const detachEditor = editorHost.attachRoute()
  const editorGrant = editorHost.grantFor(mutableSource, sourceHash)
  assert.ok(editorGrant)

  const value = {
    path: exact.png,
    filename: basename(exact.png),
    mimeType: 'image/png',
    kind: 'image',
    description: 'host smoke',
    sourceTool: OPENPENCIL_RENDER_TOOL_NAME,
    previewIntent: 'image',
    bytes: image.bytes,
    width: image.width,
    height: image.height,
    sha256: image.sha256,
    sourcePath: mutableSource,
    renderer: 'openpencil',
    rendererBinary: binary,
    fidelity: 'exact',
    warnings: exact.warnings,
    frames: verifiedFrames,
    frameCount: verifiedFrames.length,
    document: snapshot,
  }
  const projected = projectRenderGrant(value, access, viewerAssets.viewerGrant, editorGrant)
  const envelope = projected.$dshOpenPencil
  assert.equal(envelope.schemaVersion, 2)
  assert.equal(envelope.image.width, image.width)
  assert.equal(envelope.frames.length, expectedFrames.length)
  assert.deepEqual(envelope.frames.map(frame => frame.id), expectedFrames.map(frame => frame.id))
  assert.deepEqual(envelope.frames.map(frame => frame.name), expectedFrames.map(frame => frame.name))
  assert.deepEqual(envelope.frames.map(frame => frame.index), expectedFrames.map((_, index) => index))
  assert.equal(envelope.document.path, mutableSource)
  assert.equal(envelope.document.sha256, sourceHash)
  assert.equal(envelope.rendererBinary, binary)
  assert.ok(envelope.viewer.sdkUrl.includes('/viewer-assets/'))
  assert.equal(envelope.editor.enabled, true)
  assert.match(envelope.editor.refreshUrl, /\/refresh$/)
  assert.equal(envelope.editor.launchUrl.includes(mutableSource), false)

  const decodedToken = JSON.parse(Buffer.from(envelope.image.previewUrl.split('/').at(-1).split('.')[0], 'base64url').toString())
  assert.equal(decodedToken.v, 2)
  assert.equal('path' in decodedToken, false, 'capability must not expose an absolute local path')

  server = createServer((req, res) => {
    if ((req.url ?? '').startsWith(VIEWER_ASSET_ROUTE_PREFIX)) void viewerAssets.handle(req, res)
    else if ((req.url ?? '').startsWith(EDITOR_ROUTE_PREFIX)) void editorHost.handle(req, res)
    else void access.handle(req, res)
  })
  await new Promise((resolve, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', resolve)
  })
  const address = server.address()
  assert.ok(address && typeof address !== 'string')
  const origin = `http://127.0.0.1:${address.port}`

  const imageResponse = await fetch(`${origin}${envelope.image.previewUrl}`)
  assert.equal(imageResponse.status, 200)
  assert.equal(imageResponse.headers.get('content-type'), 'image/png')
  assert.equal((await imageResponse.arrayBuffer()).byteLength, image.bytes)
  for (const [index, frame] of envelope.frames.entries()) {
    const response = await fetch(`${origin}${frame.previewUrl}`)
    assert.equal(response.status, 200)
    assert.equal(response.headers.get('content-type'), 'image/png')
    assert.equal((await response.arrayBuffer()).byteLength, verifiedFrames[index].bytes)
  }

  const documentResponse = await fetch(`${origin}${envelope.document.url}`)
  assert.equal(documentResponse.status, 200)
  assert.equal(documentResponse.headers.get('content-type'), 'application/json')
  assert.deepEqual(Buffer.from(await documentResponse.arrayBuffer()), sourceBytes)

  const sdkResponse = await fetch(`${origin}${envelope.viewer.sdkUrl}`, { method: 'HEAD' })
  assert.equal(sdkResponse.status, 200)
  assert.match(sdkResponse.headers.get('content-type') ?? '', /^text\/javascript/)
  const wasmResponse = await fetch(`${origin}${envelope.viewer.wasmUrl}`, { method: 'HEAD' })
  assert.equal(wasmResponse.headers.get('content-type'), 'application/wasm')

  const launchResponse = await fetch(`${origin}${envelope.editor.launchUrl}`, {
    method: 'POST',
    headers: { origin },
  })
  const launchText = await launchResponse.text()
  assert.equal(launchResponse.status, 200, launchText)
  const launch = JSON.parse(launchText)
  assert.match(launch.iframeUrl, /^http:\/\/127\.0\.0\.1:\d+\/\?embed=vscode$/)
  assert.ok(typeof launch.token === 'string' && launch.token.length > 16)
  assert.equal(launch.docJson, sourceBytes.toString('utf8'))
  const iframeResponse = await fetch(launch.iframeUrl)
  assert.equal(iframeResponse.status, 200)
  await iframeResponse.arrayBuffer()

  // The managed editor is also the direct-drive target. Mirror one browser
  // selection push, read it through the DSH proxy, then patch that selected
  // node through first-party MCP and prove the live document changed.
  const initialSelection = await editorHost.getActiveSelection()
  assert.ok(typeof initialSelection.activePageId === 'string')
  const selectedId = expectedFrames[0]?.id
  assert.ok(typeof selectedId === 'string' && selectedId.length > 0)
  const daemonOrigin = new URL(launch.iframeUrl).origin
  const selectResponse = await fetch(`${daemonOrigin}/api/mcp/selection`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
    },
    body: JSON.stringify({ selectedIds: [selectedId], activePageId: initialSelection.activePageId }),
  })
  assert.equal(selectResponse.status, 200, await selectResponse.text())
  const selected = await editorHost.getActiveSelection()
  assert.deepEqual(selected.selectedIds, [selectedId])
  assert.equal(selected.nodes[0]?.id, selectedId)
  const selectionResponse = await fetch(`${origin}${launch.selectionUrl}`)
  assert.equal(selectionResponse.status, 200)
  assert.deepEqual((await selectionResponse.json()).selection.selectedIds, [selectedId])
  await editorHost.callActiveMcp('update_node', {
    nodeId: selectedId,
    data: { name: 'DSH direct-drive smoke' },
    ...(initialSelection.activePageId === '' ? {} : { pageId: initialSelection.activePageId }),
  })
  const changedSelection = await editorHost.getActiveSelection()
  assert.equal(changedSelection.nodes[0]?.name, 'DSH direct-drive smoke')

  const saveBody = {
    sessionId: launch.sessionId,
    docJson: `${JSON.stringify({ ...sourceDocument, dshEditorSmoke: true })}\n`,
    generation: 1,
    revision: 1,
  }
  const saveResponse = await fetch(`${origin}${launch.saveUrl}`, {
    method: 'POST',
    headers: { origin, 'content-type': 'application/json' },
    body: JSON.stringify(saveBody),
  })
  const saveText = await saveResponse.text()
  assert.equal(saveResponse.status, 200, saveText)
  const saved = JSON.parse(saveText)
  assert.equal(saved.editor.enabled, true)
  const changedBytes = Buffer.from(saveBody.docJson)
  assert.deepEqual(await readFile(mutableSource), changedBytes)

  // A changed save returns a successor grant; after closing and recreating
  // the controller it must reopen the latest bytes, never the old snapshot.
  const savedCloseResponse = await fetch(`${origin}${launch.closeUrl}`, {
    method: 'DELETE', headers: { origin, 'content-type': 'application/json' },
    body: JSON.stringify({ sessionId: launch.sessionId, dirty: false }),
  })
  assert.equal(savedCloseResponse.status, 200)
  await savedCloseResponse.arrayBuffer()
  await editorHost.dispose()
  editorHost = new EditorHostController(editorMasterKey)
  editorHost.attachRoute()
  const savedReplayResponse = await fetch(`${origin}${saved.editor.launchUrl}`, {
    method: 'POST', headers: { origin },
  })
  const savedReplayText = await savedReplayResponse.text()
  assert.equal(savedReplayResponse.status, 200, savedReplayText)
  const savedReplay = JSON.parse(savedReplayText)
  assert.equal(savedReplay.docJson, changedBytes.toString('utf8'))
  await drainedFetch(`${origin}${savedReplay.closeUrl}`, {
    method: 'DELETE', headers: { origin, 'content-type': 'application/json' },
    body: JSON.stringify({ sessionId: savedReplay.sessionId, dirty: false }),
  })

  const conflictLaunchResponse = await fetch(`${origin}${saved.editor.launchUrl}`, {
    method: 'POST', headers: { origin },
  })
  const conflictLaunch = await conflictLaunchResponse.json()
  assert.equal(conflictLaunchResponse.status, 200, JSON.stringify(conflictLaunch))
  await writeFile(mutableSource, '{"version":"external-change"}\n')
  const conflictResponse = await fetch(`${origin}${conflictLaunch.saveUrl}`, {
    method: 'POST',
    headers: { origin, 'content-type': 'application/json' },
    body: JSON.stringify({ ...saveBody, sessionId: conflictLaunch.sessionId, revision: 2 }),
  })
  assert.equal(conflictResponse.status, 409)
  await conflictResponse.arrayBuffer()
  const closeResponse = await fetch(`${origin}${conflictLaunch.closeUrl}`, {
    method: 'DELETE',
    headers: { origin, 'content-type': 'application/json' },
    body: JSON.stringify({ sessionId: conflictLaunch.sessionId, dirty: true }),
  })
  assert.equal(closeResponse.status, 200)
  await closeResponse.arrayBuffer()

  // Editor launch capabilities are self-contained and survive a plugin
  // controller recreation when the persistent DSH access key is unchanged.
  await editorHost.dispose()
  editorHost = new EditorHostController(editorMasterKey)
  editorHost.attachRoute()
  await writeFile(mutableSource, sourceBytes)
  const replayResponse = await fetch(`${origin}${envelope.editor.launchUrl}`, {
    method: 'POST', headers: { origin },
  })
  const replayText = await replayResponse.text()
  assert.equal(replayResponse.status, 200, replayText)
  const replay = JSON.parse(replayText)
  assert.equal(replay.docJson, sourceBytes.toString('utf8'))
  await drainedFetch(`${origin}${replay.closeUrl}`, {
    method: 'DELETE', headers: { origin, 'content-type': 'application/json' },
    body: JSON.stringify({ sessionId: replay.sessionId, dirty: false }),
  })

  detachEditor()
  detachViewer()
  detachRender()
  console.log(JSON.stringify({
    renderer: 'openpencil',
    fidelity: 'exact',
    width: image.width,
    height: image.height,
    bytes: image.bytes,
    frameCount: verifiedFrames.length,
    frameIds: verifiedFrames.map(frame => frame.id),
    sourceSha256: sourceHash,
    imageSha256: image.sha256,
    viewerAssets: true,
    editor: true,
  }, null, 2))
} finally {
  await editorHost?.dispose()
  if (server !== undefined) await new Promise(resolve => server.close(resolve))
  await rm(root, { recursive: true, force: true })
}
