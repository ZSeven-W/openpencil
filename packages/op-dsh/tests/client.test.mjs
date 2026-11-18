import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { test } from 'node:test'

const require = createRequire(import.meta.url)
const manifest = require('../package.json')
let client
let loadedPluginId
globalThis.window = {
  location: { href: 'http://127.0.0.1:3080/' },
  __ModuleLoader__: {
    load(definition) {
      loadedPluginId = definition.id
      client = definition.factory(require)
    },
  },
}
await import(`../lib/client.js?test=${Date.now()}`)

test('registers the client bundle under the published package name', () => {
  assert.equal(loadedPluginId, manifest.name)
})

function settled(meta) {
  return {
    kind: 'result',
    isError: false,
    content: [],
    meta,
  }
}

const DOCUMENT_SHA256 = 'a'.repeat(64)
const SCREENSHOT_SHA256 = 'b'.repeat(64)

function canonicalRenderResult(documentSha256 = DOCUMENT_SHA256, extraContent = []) {
  return {
    kind: 'tool-result',
    isError: false,
    content: [{
      type: 'text',
      text: JSON.stringify({
        path: '/private/render.png',
        sourcePath: '/private/design.op',
        document: { path: '/private/snapshot.op', sha256: documentSha256 },
      }),
    }, ...extraContent],
  }
}

function canonicalTextResult(text) {
  return {
    kind: 'tool-result',
    isError: false,
    content: [{ type: 'text', text }],
  }
}

function hydratedEnvelope() {
  return {
    $dshOpenPencil: {
      schemaVersion: 2,
      document: { url: '/_dsh/dsh-openpencil/document/signed' },
    },
  }
}

function memoryStorage() {
  const values = new Map()
  return {
    values,
    getItem(key) { return values.get(key) ?? null },
    setItem(key, value) { values.set(key, value) },
    removeItem(key) { values.delete(key) },
  }
}

function flushAsync() {
  return new Promise(resolve => setImmediate(resolve))
}

function controlledPollTimer() {
  const scheduled = []
  const cancelled = []
  return {
    scheduled,
    cancelled,
    timer: {
      schedule(callback, delayMs) {
        const handle = { callback, delayMs }
        scheduled.push(handle)
        return handle
      },
      cancel(handle) { cancelled.push(handle) },
    },
  }
}

function controlledHydrationRetryTimer() {
  const scheduled = []
  const cancelled = []
  return {
    scheduled,
    cancelled,
    timer: {
      schedule(callback, delayMs) {
        const handle = { callback, delayMs, cancelled: false }
        scheduled.push(handle)
        return handle
      },
      cancel(handle) {
        handle.cancelled = true
        cancelled.push(handle)
      },
    },
    runNext() {
      const handle = scheduled.shift()
      assert.notEqual(handle, undefined, 'expected a scheduled hydration retry')
      if (!handle.cancelled) handle.callback()
      return handle
    },
  }
}

test('keeps the established v1 PNG envelope replayable', () => {
  const grant = client.grantOf(settled({
    $dshOpenPencil: {
      schemaVersion: 1,
      image: {
        path: '/cache/render.png',
        previewUrl: '/preview-token',
        downloadUrl: '/preview-token?download=1',
      },
    },
  }))
  assert.deepEqual(grant.image, {
    path: '/cache/render.png',
    previewUrl: '/preview-token',
    downloadUrl: '/preview-token?download=1',
    width: undefined,
    height: undefined,
  })
  assert.equal(grant.document, undefined)
  assert.equal(grant.viewer, undefined)
})

test('accepts the additive document and viewer grants', () => {
  const grant = client.grantOf(settled({
    $dshOpenPencil: {
      schemaVersion: 1,
      image: {
        path: '/cache/render.png',
        previewUrl: '/preview-token',
        downloadUrl: '/preview-token?download=1',
        width: 375,
        height: 1091,
      },
      document: {
        path: '/designs/home.op',
        url: '/document-token',
        downloadUrl: '/document-token?download=1',
        bytes: 2048,
        sha256: 'abc123',
        mimeType: 'application/json',
      },
      viewer: {
        sdkUrl: '/viewer/rev/sdk.js',
        wasmUrl: '/viewer/rev/sdk.wasm',
        canvasKitBaseUrl: '/viewer/rev/canvaskit/',
      },
      editor: {
        enabled: true,
        launchUrl: '/editor/launch-token',
        refreshUrl: '/editor/refresh',
      },
      renderer: 'openpencil',
      rendererBinary: '/Applications/OpenPencil.app/Contents/MacOS/openpencil-desktop',
      fidelity: 'exact',
      warnings: ['one warning'],
    },
  }))
  assert.equal(grant.document.url, '/document-token')
  assert.equal(grant.document.path, '/designs/home.op')
  assert.equal(grant.viewer.sdkUrl, '/viewer/rev/sdk.js')
  assert.equal(grant.viewer.canvasKitBaseUrl, '/viewer/rev/canvaskit/')
  assert.deepEqual(grant.editor, {
    enabled: true,
    launchUrl: '/editor/launch-token',
    refreshUrl: '/editor/refresh',
  })
  assert.equal(grant.image.width, 375)
  assert.equal(grant.renderer, 'openpencil')
  assert.equal(grant.rendererBinary, '/Applications/OpenPencil.app/Contents/MacOS/openpencil-desktop')
  assert.equal(grant.fidelity, 'exact')
  assert.deepEqual(grant.warnings, ['one warning'])
})

test('supports schema v2 and legacy source URL aliases', () => {
  const grant = client.grantOf(settled({
    $dshOpenPencil: {
      schemaVersion: 2,
      sourceUrl: '/legacy-document-token',
      sourcePath: '/designs/legacy.op',
    },
  }))
  assert.equal(grant.schemaVersion, 2)
  assert.equal(grant.document.url, '/legacy-document-token')
  assert.equal(grant.document.path, '/designs/legacy.op')
})

test('parses live and published pipeline identity for controlled workbench handoff', async () => {
  const liveGrant = client.presentationGrantOfMeta({
    $dshOpenPencil: {
      schemaVersion: 2,
      document: { url: '/document/live' },
      editor: { enabled: true, launchUrl: '/editor/live' },
      draftId: 'd'.repeat(32),
      liveDraft: true,
    },
  })
  const publishedGrant = client.presentationGrantOfMeta({
    $dshOpenPencil: {
      schemaVersion: 2,
      document: { url: '/document/published' },
      editor: { enabled: true, launchUrl: '/editor/published' },
      draftId: 'd'.repeat(32),
      liveDraft: false,
    },
  })
  assert.equal(liveGrant.draftId, 'd'.repeat(32))
  assert.equal(liveGrant.liveDraft, true)
  assert.equal(publishedGrant.liveDraft, false)

  const current = { sessionId: 'session-a', grant: liveGrant }
  const successor = { sessionId: 'session-a', grant: publishedGrant }
  assert.equal(client.isPublishedSuccessorOfLiveDraft(current, successor), true)
  let closes = 0
  assert.equal(await client.closeLiveDraftForPublishedSuccessor(current, successor, {
    requestClose: async () => { closes += 1; return true },
  }), true)
  assert.equal(closes, 1)

  let registerController
  const delayedClose = client.closeLiveDraftForPublishedSuccessor(
    current,
    successor,
    undefined,
    () => new Promise(resolve => { registerController = resolve }),
  )
  assert.equal(closes, 1, 'a fast finish waits for the mounted live controller instead of consuming the handoff')
  registerController({ requestClose: async () => { closes += 1; return true } })
  assert.equal(await delayedClose, true)
  assert.equal(closes, 2)

  for (const other of [
    { ...successor, sessionId: 'session-b' },
    { ...successor, grant: { ...publishedGrant, draftId: 'e'.repeat(32) } },
    { ...successor, grant: { ...publishedGrant, liveDraft: true } },
  ]) {
    assert.equal(client.isPublishedSuccessorOfLiveDraft(current, other), false)
    assert.equal(await client.closeLiveDraftForPublishedSuccessor(current, other, {
      requestClose: async () => { closes += 1; return true },
    }), false)
  }
  assert.equal(closes, 2, 'unrelated automatic opens must never close an occupied workbench')
})

test('accepts an openpencil_new document-only auto-open grant', () => {
  const grant = client.grantOf(settled({
    $dshOpenPencil: {
      schemaVersion: 2,
      document: {
        path: '/designs/generated.op',
        url: '/document-token',
        downloadUrl: '/document-token?download=1',
        sha256: DOCUMENT_SHA256,
      },
      editor: {
        enabled: true,
        launchUrl: '/editor/launch-token',
      },
      autoOpenEditor: true,
    },
  }))
  assert.equal(grant.image, undefined)
  assert.equal(grant.document.path, '/designs/generated.op')
  assert.equal(grant.editor.launchUrl, '/editor/launch-token')
  assert.equal(grant.autoOpenEditor, true)
  assert.equal(client.openPencilPresentationTitle('openpencil_new', 'en'), 'OpenPencil design')
  assert.equal(client.openPencilPresentationTitle('openpencil_pipeline_begin', 'en'), 'OpenPencil design')
  assert.equal(client.openPencilPresentationTitle('openpencil_pipeline_finish', 'en'), 'OpenPencil design')
  assert.equal(client.openPencilPresentationTitle('openpencil_pipeline_inspect', 'en'), 'OpenPencil render')
  assert.equal(client.openPencilPresentationTitle('openpencil_render', 'en'), 'OpenPencil render')
  assert.equal(client.openPencilPresentationTitle('openpencil_new', 'zh'), 'OpenPencil 设计')
  assert.equal(client.shouldArmLiveAutoOpen(false, 101, 100), true, 'a fast settled live result must still auto-open')
  assert.equal(client.shouldArmLiveAutoOpen(true, 101, 100), false)
  assert.equal(client.shouldArmLiveAutoOpen(false, 99, 100), false, 'historical replay must not auto-open')
})

test('a consumed live auto-open call stays consumed across card remounts', () => {
  const key = `session-remount:${Date.now()}`
  client.rememberLiveAutoOpenCall(key)
  assert.equal(client.takeLiveAutoOpenCall(key), true)
  client.rememberLiveAutoOpenCall(key)
  assert.equal(client.takeLiveAutoOpenCall(key), false)
})

test('live auto-open retries false and rejected hosts, then accepts exactly once', async () => {
  const key = `session-retry:${Date.now()}`
  let calls = 0
  client.rememberLiveAutoOpenCall(key)

  assert.equal(await client.attemptLiveAutoOpenCall(key, () => {
    calls += 1
    return false
  }), false)
  assert.equal(await client.attemptLiveAutoOpenCall(key, async () => {
    calls += 1
    throw new Error('host not mounted')
  }), false)
  assert.equal(await client.attemptLiveAutoOpenCall(key, async () => {
    calls += 1
    return true
  }), true)

  client.rememberLiveAutoOpenCall(key)
  assert.equal(await client.attemptLiveAutoOpenCall(key, () => {
    calls += 1
    return true
  }), false)
  assert.equal(calls, 3, 'only a true host answer permanently accepts the call')
})

test('live auto-open suppresses duplicate requests while one host answer is pending', async () => {
  const key = `session-opening:${Date.now()}`
  let resolveOpen
  let calls = 0
  client.rememberLiveAutoOpenCall(key)
  const pending = client.attemptLiveAutoOpenCall(key, () => {
    calls += 1
    return new Promise(resolve => { resolveOpen = resolve })
  })

  assert.equal(await client.attemptLiveAutoOpenCall(key, () => {
    calls += 1
    return true
  }), false)
  assert.equal(calls, 1)
  resolveOpen(false)
  assert.equal(await pending, false)
  assert.equal(await client.attemptLiveAutoOpenCall(key, () => {
    calls += 1
    return true
  }), true)
  assert.equal(calls, 2)
})

test('extracts only a valid document fingerprint from one canonical text result', () => {
  assert.equal(client.documentSha256FromCanonicalResult(canonicalRenderResult()), DOCUMENT_SHA256)
  assert.equal(client.documentSha256FromCanonicalResult(canonicalRenderResult('not-a-sha256')), undefined)
  assert.equal(client.documentSha256FromCanonicalResult(canonicalRenderResult(
    DOCUMENT_SHA256,
    [{ type: 'text', text: '{}' }],
  )), undefined)
  assert.equal(client.documentSha256FromCanonicalResult({
    ...canonicalRenderResult(),
    isError: true,
  }), undefined)
  assert.equal(client.documentSha256FromCanonicalResult({
    kind: 'tool-result',
    isError: false,
    content: [{ type: 'text', text: ' '.repeat(1024 * 1024 + 1) }],
  }), undefined, 'oversized historical results must be rejected before JSON parsing')
  const screenshotBlock = {
    kind: 'tool-result',
    isError: false,
    content: [{ type: 'text', text: JSON.stringify({ screenshot: { sha256: SCREENSHOT_SHA256 } }) }],
  }
  assert.equal(client.documentSha256FromCanonicalResult(screenshotBlock), SCREENSHOT_SHA256)
})

test('accepts only an identical two-object run_code echo around a canonical fingerprint', () => {
  const first = JSON.stringify({
    path: '/private/render.png',
    document: { path: '/private/snapshot.op', sha256: DOCUMENT_SHA256 },
  })
  const sameStructureDifferentKeyOrder = JSON.stringify({
    document: { sha256: DOCUMENT_SHA256, path: '/private/snapshot.op' },
    path: '/private/render.png',
  }, null, 2)
  const duplicate = canonicalTextResult(` \n${first}\n\t${sameStructureDifferentKeyOrder}\r `)
  assert.equal(client.documentSha256FromCanonicalResult(duplicate), DOCUMENT_SHA256)
  assert.deepEqual(client.presentationHydrationRequestOf({
    block: duplicate,
    toolName: 'openpencil_render',
    sessionId: 'session-duplicate-echo',
    callId: 'call-duplicate-echo',
    embeddedGrant: undefined,
  }), {
    sessionId: 'session-duplicate-echo',
    callId: 'call-duplicate-echo',
    documentSha256: DOCUMENT_SHA256,
  })

  const sameFingerprintDifferentObject = JSON.stringify({
    path: '/private/other.png',
    document: { path: '/private/snapshot.op', sha256: DOCUMENT_SHA256 },
  })
  const differentFingerprint = JSON.stringify({
    path: '/private/render.png',
    document: { path: '/private/snapshot.op', sha256: SCREENSHOT_SHA256 },
  })
  for (const rejected of [
    `${first}${first}`,
    `logged result: ${first}`,
    `${first}\nreturned result`,
    `${first}\n${sameFingerprintDifferentObject}`,
    `${first}\n${differentFingerprint}`,
    `${first}\n${first}\n${first}`,
    `${first}\n{`,
  ]) {
    assert.equal(client.documentSha256FromCanonicalResult(canonicalTextResult(rejected)), undefined)
  }
})

test('hydrates a nested render grant with an exact same-origin fingerprint request', async () => {
  const calls = []
  const fetcher = async (input, init) => {
    calls.push({ input, init })
    return { ok: true, json: async () => hydratedEnvelope() }
  }
  const request = {
    sessionId: 'session-nested',
    callId: 'call-nested',
    documentSha256: DOCUMENT_SHA256,
  }
  const grant = await client.requestPresentationGrant(
    request,
    client.presentationGrantOfMeta,
    { fetcher },
  )

  assert.equal(calls.length, 1)
  assert.equal(calls[0].input, '/_dsh/dsh-openpencil/presentation')
  assert.equal(calls[0].init.method, 'POST')
  assert.equal(calls[0].init.credentials, 'same-origin')
  assert.equal(calls[0].init.headers['content-type'], 'application/json')
  assert.deepEqual(JSON.parse(calls[0].init.body), request)
  assert.deepEqual(Object.keys(JSON.parse(calls[0].init.body)), [
    'sessionId', 'callId', 'documentSha256',
  ])
  assert.equal(grant.document.url, '/_dsh/dsh-openpencil/document/signed')
})

test('presentation hydration fails closed on HTTP and malformed response data', async () => {
  const request = {
    sessionId: 'session-failure',
    callId: 'call-failure',
    documentSha256: DOCUMENT_SHA256,
  }
  assert.equal(await client.requestPresentationGrant(request, client.presentationGrantOfMeta, {
    fetcher: async () => ({ ok: false, json: async () => { throw new Error('must not read') } }),
  }), undefined)
  assert.equal(await client.requestPresentationGrant(request, client.presentationGrantOfMeta, {
    fetcher: async () => ({ ok: true, json: async () => ({}) }),
  }), undefined)
  assert.equal(await client.requestPresentationGrant(request, client.presentationGrantOfMeta, {
    fetcher: async () => ({
      ok: true,
      json: async () => ({ $dshOpenPencil: { schemaVersion: 99, document: { url: '/forged' } } }),
    }),
  }), undefined)
  assert.equal(await client.requestPresentationGrant(request, () => { throw new Error('bad parser') }, {
    fetcher: async () => ({ ok: true, json: async () => hydratedEnvelope() }),
  }), undefined)
})

test('presentation hydration retries a transient 404 and stops after success', async () => {
  const clock = controlledHydrationRetryTimer()
  let calls = 0
  const request = {
    sessionId: 'session-eventual',
    callId: 'call-eventual',
    documentSha256: DOCUMENT_SHA256,
  }
  const pending = client.requestPresentationGrantWithRetry(
    request,
    client.presentationGrantOfMeta,
    {
      fetcher: async () => {
        calls += 1
        return calls === 1
          ? { ok: false, json: async () => { throw new Error('must not read') } }
          : { ok: true, json: async () => hydratedEnvelope() }
      },
      timer: clock.timer,
    },
  )
  await flushAsync()
  assert.equal(calls, 1)
  assert.deepEqual(clock.scheduled.map(handle => handle.delayMs), [100])

  clock.runNext()
  const grant = await pending
  assert.equal(calls, 2)
  assert.equal(grant.document.url, '/_dsh/dsh-openpencil/document/signed')
  assert.equal(clock.scheduled.length, 0, 'success must not schedule another retry')
})

test('presentation hydration exhausts its finite retry budget', async () => {
  const clock = controlledHydrationRetryTimer()
  let calls = 0
  const pending = client.requestPresentationGrantWithRetry({
    sessionId: 'session-missing',
    callId: 'call-missing',
    documentSha256: DOCUMENT_SHA256,
  }, client.presentationGrantOfMeta, {
    fetcher: async () => {
      calls += 1
      return { ok: false, json: async () => { throw new Error('must not read') } }
    },
    timer: clock.timer,
  })

  const observedDelays = []
  for (const expectedDelay of [100, 250, 500, 1_000]) {
    await flushAsync()
    assert.equal(clock.scheduled.length, 1)
    observedDelays.push(clock.runNext().delayMs)
    assert.equal(observedDelays.at(-1), expectedDelay)
  }
  assert.equal(await pending, undefined)
  assert.equal(calls, 5, 'one initial exchange plus four retries')
  assert.deepEqual(observedDelays, [100, 250, 500, 1_000])
  assert.equal(clock.scheduled.length, 0, 'exhaustion must stop polling')
})

test('unmount abort cancels a scheduled presentation hydration retry', async () => {
  const clock = controlledHydrationRetryTimer()
  const controller = new AbortController()
  let calls = 0
  const pending = client.requestPresentationGrantWithRetry({
    sessionId: 'session-unmount',
    callId: 'call-unmount',
    documentSha256: DOCUMENT_SHA256,
  }, client.presentationGrantOfMeta, {
    fetcher: async () => {
      calls += 1
      return { ok: false, json: async () => { throw new Error('must not read') } }
    },
    signal: controller.signal,
    timer: clock.timer,
  })
  await flushAsync()
  assert.equal(clock.scheduled.length, 1)

  controller.abort()
  assert.equal(await pending, undefined)
  assert.equal(clock.cancelled.length, 1)
  clock.runNext()
  await flushAsync()
  assert.equal(calls, 1, 'an unmounted card must not issue another exchange')
})

test('changing hydration key aborts and cancels the old retry', async () => {
  const clock = controlledHydrationRetryTimer()
  const oldController = new AbortController()
  const calls = []
  const fetcher = async (_input, init) => {
    const request = JSON.parse(init.body)
    calls.push(request)
    return request.documentSha256 === DOCUMENT_SHA256
      ? { ok: false, json: async () => { throw new Error('must not read') } }
      : { ok: true, json: async () => hydratedEnvelope() }
  }
  const oldPending = client.requestPresentationGrantWithRetry({
    sessionId: 'session-key-change',
    callId: 'call-old',
    documentSha256: DOCUMENT_SHA256,
  }, client.presentationGrantOfMeta, {
    fetcher,
    signal: oldController.signal,
    timer: clock.timer,
  })
  await flushAsync()
  assert.equal(clock.scheduled.length, 1)

  oldController.abort()
  const nextGrant = await client.requestPresentationGrantWithRetry({
    sessionId: 'session-key-change',
    callId: 'call-new',
    documentSha256: SCREENSHOT_SHA256,
  }, client.presentationGrantOfMeta, {
    fetcher,
    timer: clock.timer,
  })
  assert.equal(await oldPending, undefined)
  assert.equal(nextGrant.document.url, '/_dsh/dsh-openpencil/document/signed')
  assert.equal(clock.cancelled.length, 1)
  assert.equal(clock.cancelled[0].delayMs, 100)
  clock.runNext()
  await flushAsync()
  assert.deepEqual(calls.map(call => call.callId), ['call-old', 'call-new'])
})

test('presentation hydration coalesces concurrent requests and isolates subscriber aborts', async () => {
  let resolveResponse
  const response = new Promise(resolve => { resolveResponse = resolve })
  const calls = []
  const fetcher = async (input, init) => {
    calls.push({ input, init })
    return response
  }
  const request = {
    sessionId: 'session-dedupe',
    callId: 'call-dedupe',
    documentSha256: DOCUMENT_SHA256,
  }
  const firstAbort = new AbortController()
  const first = client.requestPresentationGrant(request, client.presentationGrantOfMeta, {
    fetcher,
    signal: firstAbort.signal,
  })
  const second = client.requestPresentationGrant(request, client.presentationGrantOfMeta, { fetcher })
  await flushAsync()
  assert.equal(calls.length, 1)
  firstAbort.abort()
  assert.equal(await first, undefined)
  assert.equal(calls[0].init.signal.aborted, false, 'one subscriber must not cancel another')

  resolveResponse({ ok: true, json: async () => hydratedEnvelope() })
  const grant = await second
  assert.equal(grant.document.url, '/_dsh/dsh-openpencil/document/signed')
})

test('an abandoned hydration does not poison an immediate remount retry', async () => {
  const calls = []
  const fetcher = async (_input, init) => {
    calls.push(init)
    if (calls.length === 1) {
      return await new Promise((_resolve, reject) => {
        init.signal.addEventListener('abort', () => reject(new Error('aborted')), { once: true })
      })
    }
    return { ok: true, json: async () => hydratedEnvelope() }
  }
  const request = {
    sessionId: 'session-remount',
    callId: 'call-remount',
    documentSha256: DOCUMENT_SHA256,
  }
  const firstAbort = new AbortController()
  const first = client.requestPresentationGrant(request, client.presentationGrantOfMeta, {
    fetcher,
    signal: firstAbort.signal,
  })
  await flushAsync()
  firstAbort.abort()
  const retry = client.requestPresentationGrant(request, client.presentationGrantOfMeta, { fetcher })
  assert.equal(await first, undefined)
  const grant = await retry
  assert.equal(calls.length, 2)
  assert.equal(calls[0].signal.aborted, true)
  assert.equal(grant.document.url, '/_dsh/dsh-openpencil/document/signed')
})

test('pre-aborted hydration never starts network work', async () => {
  let calls = 0
  const controller = new AbortController()
  controller.abort()
  assert.equal(await client.requestPresentationGrant({
    sessionId: 'session-aborted',
    callId: 'call-aborted',
    documentSha256: DOCUMENT_SHA256,
  }, client.presentationGrantOfMeta, {
    signal: controller.signal,
    fetcher: async () => {
      calls += 1
      return { ok: true, json: async () => hydratedEnvelope() }
    },
  }), undefined)
  assert.equal(calls, 0)
})

test('embedded presentation metadata prevents a hydration request', async () => {
  let calls = 0
  const block = {
    ...canonicalRenderResult(),
    meta: hydratedEnvelope(),
  }
  const embeddedGrant = client.grantOf(block)
  const request = client.presentationHydrationRequestOf({
    block,
    toolName: 'openpencil_render',
    sessionId: 'session-embedded',
    callId: 'call-embedded',
    embeddedGrant,
  })
  if (request !== undefined) {
    await client.requestPresentationGrant(request, client.presentationGrantOfMeta, {
      fetcher: async () => {
        calls += 1
        return { ok: true, json: async () => hydratedEnvelope() }
      },
    })
  }
  assert.equal(request, undefined)
  assert.equal(calls, 0)
})

test('only canonical OpenPencil presentation tools can request hydration', () => {
  const block = canonicalRenderResult()
  assert.deepEqual(client.presentationHydrationRequestOf({
    block,
    toolName: 'openpencil_render',
    sessionId: 'session-canonical',
    callId: 'call-canonical',
    embeddedGrant: undefined,
  }), {
    sessionId: 'session-canonical',
    callId: 'call-canonical',
    documentSha256: DOCUMENT_SHA256,
  })
  const screenshotBlock = {
    kind: 'tool-result',
    isError: false,
    content: [{ type: 'text', text: JSON.stringify({ screenshot: { sha256: SCREENSHOT_SHA256 } }) }],
  }
  assert.deepEqual(client.presentationHydrationRequestOf({
    block: screenshotBlock,
    toolName: 'openpencil_pipeline_inspect',
    sessionId: 'session-pipeline-inspect',
    callId: 'call-pipeline-inspect',
    embeddedGrant: undefined,
  }), {
    sessionId: 'session-pipeline-inspect',
    callId: 'call-pipeline-inspect',
    documentSha256: SCREENSHOT_SHA256,
  })
  assert.deepEqual(client.presentationHydrationRequestOf({
    block: screenshotBlock,
    toolName: 'openpencil_pipeline_batch',
    sessionId: 'session-pipeline-batch',
    callId: 'call-pipeline-batch',
    embeddedGrant: undefined,
  }), {
    sessionId: 'session-pipeline-batch',
    callId: 'call-pipeline-batch',
    documentSha256: SCREENSHOT_SHA256,
  })
  assert.equal(client.presentationHydrationRequestOf({
    block: {
      kind: 'tool-result',
      isError: false,
      content: [{ type: 'text', text: JSON.stringify({ previewUnavailable: true }) }],
    },
    toolName: 'openpencil_pipeline_batch',
    sessionId: 'session-pipeline-batch',
    callId: 'call-pipeline-batch-no-artifact',
    embeddedGrant: undefined,
  }), undefined)
  assert.deepEqual(client.presentationHydrationRequestOf({
    block: screenshotBlock,
    toolName: 'openpencil_pipeline_finish',
    sessionId: 'session-pipeline-finish-pending',
    callId: 'call-pipeline-finish-pending',
    embeddedGrant: undefined,
  }), {
    sessionId: 'session-pipeline-finish-pending',
    callId: 'call-pipeline-finish-pending',
    documentSha256: SCREENSHOT_SHA256,
  })
  assert.deepEqual(client.presentationHydrationRequestOf({
    block,
    toolName: 'openpencil_new',
    sessionId: 'session-new',
    callId: 'call-new',
    embeddedGrant: undefined,
  }), {
    sessionId: 'session-new',
    callId: 'call-new',
    documentSha256: DOCUMENT_SHA256,
  })
  assert.deepEqual(client.presentationHydrationRequestOf({
    block,
    toolName: 'openpencil_pipeline_begin',
    sessionId: 'session-pipeline-begin',
    callId: 'call-pipeline-begin',
    embeddedGrant: undefined,
  }), {
    sessionId: 'session-pipeline-begin',
    callId: 'call-pipeline-begin',
    documentSha256: DOCUMENT_SHA256,
  })
  assert.deepEqual(client.presentationHydrationRequestOf({
    block,
    toolName: 'openpencil_pipeline_finish',
    sessionId: 'session-pipeline',
    callId: 'call-pipeline',
    embeddedGrant: undefined,
  }), {
    sessionId: 'session-pipeline',
    callId: 'call-pipeline',
    documentSha256: DOCUMENT_SHA256,
  })
  assert.equal(client.presentationHydrationRequestOf({
    block,
    toolName: 'design_render',
    sessionId: 'session-legacy',
    callId: 'call-legacy',
    embeddedGrant: undefined,
  }), undefined)
  assert.equal(client.presentationHydrationRequestOf({
    block,
    toolName: 'openpencil_render',
    sessionId: 's'.repeat(257),
    callId: 'call-too-long-session',
    embeddedGrant: undefined,
  }), undefined)
  assert.equal(client.presentationHydrationRequestOf({
    block,
    toolName: 'openpencil_render',
    sessionId: 'session-too-long-call',
    callId: 'c'.repeat(513),
    embeddedGrant: undefined,
  }), undefined)
})

test('preserves ordered top-level frame grants for the gallery', () => {
  const grant = client.grantOf(settled({
    $dshOpenPencil: {
      schemaVersion: 2,
      frames: [
        {
          path: '/cache/a.png', previewUrl: '/preview-a', downloadUrl: '/preview-a?download=1',
          width: 390, height: 844, id: 'n171', name: 'Music Home', index: 0,
        },
        {
          path: '/cache/b.png', previewUrl: '/preview-b', downloadUrl: '/preview-b?download=1',
          width: 390, height: 844, id: 'n543', name: 'Search', index: 1,
        },
        { path: '/cache/broken.png', previewUrl: '/broken' },
      ],
    },
  }))
  assert.equal(grant.image.previewUrl, '/preview-a')
  assert.equal(grant.frames.length, 2)
  assert.deepEqual(grant.frames.map(frame => ({ id: frame.id, name: frame.name, index: frame.index })), [
    { id: 'n171', name: 'Music Home', index: 0 },
    { id: 'n543', name: 'Search', index: 1 },
  ])
})

test('gallery helpers clamp selection and prefer frame names', () => {
  assert.equal(client.normalizeFrameIndex(-10, 4), 0)
  assert.equal(client.normalizeFrameIndex(99, 4), 3)
  assert.equal(client.normalizeFrameIndex(1.9, 4), 1)
  assert.equal(client.frameLabel({ name: 'Premium' }, 3), 'Premium')
  assert.equal(client.frameLabel({ id: 'n669' }, 3), 'n669')
  assert.equal(client.frameLabel({}, 3), 'Frame 4')
})

test('gallery zoom helpers use bounded 25% stops and a clear percentage', () => {
  assert.equal(client.GALLERY_ZOOM_MIN, 0.25)
  assert.equal(client.GALLERY_ZOOM_MAX, 4)
  assert.equal(client.GALLERY_ZOOM_STEP, 0.25)
  assert.equal(client.clampGalleryZoom(-5), 0.25)
  assert.equal(client.clampGalleryZoom(9), 4)
  assert.equal(client.clampGalleryZoom(Number.NaN), 1)
  assert.equal(client.nextGalleryZoom(1, 1), 1.25)
  assert.equal(client.nextGalleryZoom(1, -1), 0.75)
  assert.equal(client.nextGalleryZoom(0.61, 1), 0.75)
  assert.equal(client.nextGalleryZoom(0.61, -1), 0.5)
  assert.equal(client.nextGalleryZoom(0.25, -1), 0.25)
  assert.equal(client.nextGalleryZoom(4, 1), 4)
  assert.equal(client.galleryZoomPercent(1.246), '125%')
  assert.equal(client.galleryZoomPercent(0.08), '8%')
  assert.equal(client.galleryZoomPercent(0.004), '0.4%')
  assert.equal(client.galleryZoomPercent(0.0001), '0.1%')
})

test('gallery fit-view zoom contains the entire frame and permits very small automatic scales', () => {
  assert.equal(client.calculateGalleryFitViewZoom(1000, 500, 500, 1000), 0.5)
  assert.equal(client.calculateGalleryFitViewZoom(300, 560, 375, 5600), 0.1)
  assert.equal(client.calculateGalleryFitViewZoom(5000, 5000, 100, 100), 4)
  assert.equal(client.calculateGalleryFitViewZoom(0, 560, 375, 1000), 1)
  assert.equal(client.nextGalleryZoom(0.1, 1), 0.25, 'manual zoom exits a tiny fit at its minimum')
})

test('gallery card sizing expands to content and restores the compact viewport', () => {
  assert.equal(client.GALLERY_COMPACT_MAX_HEIGHT, 560)
  assert.equal(client.galleryViewportMaxHeight(false), 560)
  assert.equal(client.galleryViewportMaxHeight(true), undefined)
})

test('gallery toolbar controls share a vertically-centered box', () => {
  assert.equal(client.GALLERY_TOOLBAR_CONTROL_HEIGHT, 28)
  assert.deepEqual(client.GALLERY_TOOLBAR_CONTROL_LAYOUT, {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    boxSizing: 'border-box',
    height: 28,
    lineHeight: 1,
    verticalAlign: 'middle',
  })
  assert.deepEqual(client.GALLERY_TOOLBAR_CONTROL_CONTENT_LAYOUT, {
    display: 'inline-block',
    lineHeight: 1,
    transform: 'translateY(-1px)',
    pointerEvents: 'none',
  })
})

test('gallery and render-card copy follow the resolved DSH locale', () => {
  const zhGallery = client.frameGalleryCopy('zh')
  assert.equal(zhGallery.reset, '重置')
  assert.equal(zhGallery.fitFrame, '适应画面')
  assert.equal(zhGallery.fitContent, '适应内容')
  assert.equal(zhGallery.restoreCard, '还原卡片')
  assert.equal(zhGallery.previous, '上一页')
  assert.equal(zhGallery.next, '下一页')
  assert.equal(client.frameLabel({}, 3, 'zh'), '页面 4')

  const enGallery = client.frameGalleryCopy('en')
  assert.equal(enGallery.reset, 'Reset')
  assert.equal(enGallery.fitFrame, 'Fit frame')
  assert.equal(client.frameLabel({}, 3, 'en'), 'Frame 4')

  const zhCard = client.designRenderCopy('zh')
  assert.equal(zhCard.designRender, 'OpenPencil 渲染')
  assert.equal(zhCard.openInteractiveCanvas, '打开交互画布')
  assert.equal(zhCard.editCanvas, '编辑画布')
  assert.equal(zhCard.editInSidebar, '在侧边栏编辑')
  assert.equal(zhCard.downloadPng, '下载 PNG')
  assert.equal(zhCard.recoveringPreview, '正在恢复 OpenPencil 预览…')
  assert.equal(zhCard.pendingPreview, '当前流水线阶段正在等待最终预览。')
  assert.equal(zhCard.noPreview, '此结果未包含可显示的预览产物。')

  const enCard = client.designRenderCopy('en')
  assert.equal(enCard.designRender, 'OpenPencil render')
  assert.equal(enCard.openInteractiveCanvas, 'Open interactive canvas')
  assert.equal(enCard.editCanvas, 'Edit canvas')
  assert.equal(enCard.recoveringPreview, 'Recovering the OpenPencil preview…')
  assert.equal(enCard.pendingPreview, 'This pipeline stage is waiting for its final preview.')
  assert.equal(enCard.noPreview, 'This result does not include a preview artifact.')
})

test('pending pipeline finish states are labelled as waiting for preview', () => {
  const pending = JSON.stringify({
    published: false,
    stage: 'needs_preview',
  })
  assert.equal(client.pipelineResultAwaitsPreview('openpencil_pipeline_finish', pending), true)
  assert.equal(client.pipelineResultAwaitsPreview('openpencil_pipeline_finish', `${pending}\n${pending}`), true)
  assert.equal(client.pipelineResultAwaitsPreview('openpencil_pipeline_finish', JSON.stringify({
    published: true,
  })), false)
  assert.equal(client.pipelineResultAwaitsPreview('openpencil_render', JSON.stringify({
    published: false,
    stage: 'needs_preview',
  })), false)
  assert.equal(client.pipelineResultAwaitsPreview('openpencil_pipeline_finish', '{invalid'), false)
  assert.equal(client.fallbackPresentationText(pending, true), null)
  assert.equal(client.fallbackPresentationText(pending, false), pending)
})

test('hides only settled successful presentation cards with no displayable state', () => {
  const emptySuccess = {
    settled: true,
    error: false,
    hasGrant: false,
    hydrationPending: false,
    awaitingPreview: false,
  }
  assert.equal(client.shouldHidePresentationCard(emptySuccess), true)
  assert.equal(client.shouldHidePresentationCard({ ...emptySuccess, settled: false }), false)
  assert.equal(client.shouldHidePresentationCard({ ...emptySuccess, error: true }), false)
  assert.equal(client.shouldHidePresentationCard({ ...emptySuccess, hasGrant: true }), false)
  assert.equal(client.shouldHidePresentationCard({ ...emptySuccess, hydrationPending: true }), false)
  assert.equal(
    client.shouldHidePresentationCard({ ...emptySuccess, awaitingPreview: true }),
    true,
    'a settled preview-retry result without a displayable artifact must not render an empty card',
  )
})

test('live OpenPencil selection is parsed, scoped by DSH session, and labelled bilingually', () => {
  const emptyBeforePoll = client.getOpenPencilSelectionSnapshot('session-empty')
  assert.strictEqual(
    client.getOpenPencilSelectionSnapshot('session-empty'),
    emptyBeforePoll,
    'empty snapshots must stay referentially stable for useSyncExternalStore',
  )
  const selection = client.liveSelectionOf({
    sourcePath: '/designs/home.op', activePageId: 'page-1', selectedIds: ['n42'], updatedAt: 7,
    nodes: [{ id: 'n42', type: 'text', name: 'Hero title', x: 12, y: 20, width: 320, height: 48 }],
  })
  assert.ok(selection)
  assert.equal(client.hasOpenPencilSelection(selection), true)
  assert.equal(client.hasOpenPencilSelection({ ...selection, selectedIds: [] }), false)
  assert.deepEqual(client.OPENPENCIL_SELECTION_DOCK_LAYOUT, {
    boxSizing: 'border-box',
    flex: 'none',
    width: 'calc(100% - var(--dsh-composer-side-clearance, 16px) - var(--dsh-composer-side-clearance, 16px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px))',
    maxWidth: 'calc(var(--dsh-composer-card-max-width, 780px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px))',
    margin: '0 auto',
  })
  assert.equal(client.selectionNodeLabel(selection, 'en'), 'Hero title')
  assert.equal(client.selectionNodeLabel(selection, 'zh'), 'Hero title')
  assert.equal(client.selectionNodeDetail(selection, 'en'), 'text · 320 × 48 · n42')

  client.publishOpenPencilSelection('session-a', selection)
  assert.equal(client.getOpenPencilSelectionSnapshot('session-a').selection.nodes[0].id, 'n42')
  assert.equal(client.getOpenPencilSelectionSnapshot('session-b').selection, undefined)
  client.clearOpenPencilSelection('session-b')
  assert.ok(client.getOpenPencilSelectionSnapshot('session-a').selection)
  client.clearOpenPencilSelection('session-a', '/designs/home.op')
  assert.equal(client.getOpenPencilSelectionSnapshot('session-a').selection, undefined)
})

test('selection labels retain id-only and multi-select fallbacks', () => {
  const multi = {
    sourcePath: '/designs/home.op', activePageId: 'page-1', selectedIds: ['n1', 'n2'], nodes: [], updatedAt: 1,
  }
  assert.equal(client.selectionNodeLabel(multi, 'zh'), '已选择 2 个节点')
  assert.equal(client.selectionNodeLabel(multi, 'en'), '2 nodes selected')
  assert.equal(client.selectionNodeDetail(multi, 'en'), 'n1 · n2')
})

test('gallery zoom shortcuts require Ctrl or Command', () => {
  assert.equal(client.galleryZoomShortcut('+', false), undefined)
  assert.equal(client.galleryZoomShortcut('=', true), 'in')
  assert.equal(client.galleryZoomShortcut('+', true), 'in')
  assert.equal(client.galleryZoomShortcut('-', true), 'out')
  assert.equal(client.galleryZoomShortcut('_', true), 'out')
  assert.equal(client.galleryZoomShortcut('0', true), 'reset')
  assert.equal(client.galleryZoomShortcut('ArrowRight', true), undefined)
})

test('gallery keyboard zoom never reverses direction at automatic fit limits', () => {
  assert.equal(client.galleryZoomCommandTarget(0.1, 'out'), undefined)
  assert.equal(client.galleryZoomCommandTarget(0.1, 'in'), 0.25)
  assert.equal(client.galleryZoomCommandTarget(4, 'in'), undefined)
  assert.equal(client.galleryZoomCommandTarget(1, 'out'), 0.75)
  assert.equal(client.galleryZoomCommandTarget(0.4, 'reset'), 1)
})

test('rejects partial viewer metadata instead of guessing an asset revision', () => {
  const grant = client.grantOf(settled({
    $dshOpenPencil: {
      schemaVersion: 1,
      document: { url: '/document-token' },
      viewer: { sdkUrl: '/viewer/sdk.js' },
    },
  }))
  assert.equal(grant.viewer, undefined)
})

test('rejects disabled or partial editor grants', () => {
  const disabled = client.grantOf(settled({
    $dshOpenPencil: {
      schemaVersion: 2,
      document: { url: '/document-token' },
      editor: { enabled: false, launchUrl: '/launch' },
    },
  }))
  assert.equal(disabled.editor, undefined)
  const partial = client.grantOf(settled({
    $dshOpenPencil: {
      schemaVersion: 2,
      document: { url: '/document-token' },
      editor: { enabled: true },
    },
  }))
  assert.equal(partial.editor, undefined)
})

test('the page-wide canvas coordinator closes the previous owner only', () => {
  const closed = []
  const first = Symbol('first')
  const second = Symbol('second')
  const releaseFirst = client.claimCanvas(first, () => { closed.push('first') })
  const releaseSecond = client.claimCanvas(second, () => { closed.push('second') })
  assert.deepEqual(closed, ['first'])
  releaseFirst()

  const third = Symbol('third')
  const releaseThird = client.claimCanvas(third, () => { closed.push('third') })
  assert.deepEqual(closed, ['first', 'second'])
  releaseSecond()
  releaseThird()
})

test('sizes the canvas backing store from its CSS box and DPR', () => {
  const canvas = { clientWidth: 375, clientHeight: 640, width: 300, height: 150 }
  assert.deepEqual(client.sizeCanvasForDisplay(canvas, 2), {
    cssWidth: 375,
    cssHeight: 640,
    dpr: 2,
  })
  assert.equal(canvas.width, 750)
  assert.equal(canvas.height, 1280)
})

test('canvas backing sizing safely handles an unavailable layout or DPR', () => {
  const canvas = { clientWidth: 0, clientHeight: 0, width: 300, height: 150 }
  assert.deepEqual(client.sizeCanvasForDisplay(canvas, Number.NaN), {
    cssWidth: 1,
    cssHeight: 1,
    dpr: 1,
  })
  assert.equal(canvas.width, 1)
  assert.equal(canvas.height, 1)
})

test('editor bridge accepts only typed JSON-string messages', () => {
  assert.deepEqual(client.parseEditorInbound(JSON.stringify({
    type: 'op-bridge/listening',
  })), {
    type: 'op-bridge/listening',
  })
  assert.deepEqual(client.parseEditorInbound(JSON.stringify({
    type: 'op-bridge/dirty-changed', generation: 2, revision: 5, dirty: true,
  })), {
    type: 'op-bridge/dirty-changed', generation: 2, revision: 5, dirty: true,
  })
  assert.deepEqual(client.parseEditorInbound(JSON.stringify({ type: 'op-shell/save' })), { type: 'op-shell/save' })
  assert.equal(client.parseEditorInbound({ type: 'op-shell/save' }), undefined)
  assert.equal(client.parseEditorInbound('{"type":"op-bridge/ready","generation":-1,"revision":0}'), undefined)
  assert.equal(client.parseEditorInbound('{"type":"foreign"}'), undefined)
  assert.deepEqual(
    client.parseEditorInbound(JSON.stringify({ type: 'op-bridge/listening' })),
    { type: 'op-bridge/listening' },
  )
  assert.deepEqual(
    client.parseEditorInbound(JSON.stringify({ type: 'op-bridge/listening', extra: true })),
    { type: 'op-bridge/listening' },
    'extra keys are ignored like every other inbound message',
  )
})

test('editor bridge accepts the first ready edge only', () => {
  const latch = { current: false }
  assert.equal(client.beginEditorInitRetry.takeReady(latch), true)
  assert.equal(latch.current, true)
  assert.equal(client.beginEditorInitRetry.takeReady(latch), false, 'duplicate ready must not reopen the boot document')
})

test('editor bridge pins loopback iframe source and origin', () => {
  assert.equal(client.editorOrigin('http://127.0.0.1:49152/?embed=vscode'), 'http://127.0.0.1:49152')
  assert.equal(
    client.editorIframeUrlWithTheme('http://127.0.0.1:49152/?embed=vscode', 'light'),
    'http://127.0.0.1:49152/?embed=vscode&theme=light',
  )
  assert.equal(
    client.editorIframeUrlWithTheme('http://localhost:49152/?embed=vscode&theme=light#canvas', 'dark'),
    'http://localhost:49152/?embed=vscode&theme=dark#canvas',
  )
  assert.equal(
    client.editorIframeUrlWithLocale('http://127.0.0.1:49152/?embed=vscode&theme=dark', 'zh-CN'),
    'http://127.0.0.1:49152/?embed=vscode&theme=dark&locale=zh-CN',
  )
  assert.equal(
    client.editorIframeUrlWithLocale('http://localhost:49152/?embed=vscode&locale=zh-CN#canvas', 'en-US'),
    'http://localhost:49152/?embed=vscode&locale=en-US#canvas',
  )
  assert.throws(() => client.editorOrigin('https://example.com/editor'), /loopback origin/)

  const frame = {}
  const data = JSON.stringify({ type: 'op-bridge/opened', generation: 4 })
  assert.deepEqual(client.editorMessageFrom(
    { source: frame, origin: 'http://127.0.0.1:49152', data },
    frame,
    'http://127.0.0.1:49152',
  ), { type: 'op-bridge/opened', generation: 4 })
  assert.equal(client.editorMessageFrom(
    { source: {}, origin: 'http://127.0.0.1:49152', data },
    frame,
    'http://127.0.0.1:49152',
  ), undefined)
  assert.equal(client.editorMessageFrom(
    { source: frame, origin: 'http://127.0.0.1:7', data },
    frame,
    'http://127.0.0.1:49152',
  ), undefined)
})

test('the listening handshake passes the same iframe source and origin gate', () => {
  const frame = {}
  const data = JSON.stringify({ type: 'op-bridge/listening' })
  assert.deepEqual(client.editorMessageFrom(
    { source: frame, origin: 'http://127.0.0.1:49152', data },
    frame,
    'http://127.0.0.1:49152',
  ), { type: 'op-bridge/listening' })
  assert.equal(client.editorMessageFrom(
    { source: {}, origin: 'http://127.0.0.1:49152', data },
    frame,
    'http://127.0.0.1:49152',
  ), undefined)
  assert.equal(client.editorMessageFrom(
    { source: frame, origin: 'http://127.0.0.1:7', data },
    frame,
    'http://127.0.0.1:49152',
  ), undefined)
  assert.equal(client.editorMessageFrom(
    { source: frame, origin: 'http://127.0.0.1:49152', data: JSON.stringify({ type: 'foreign' }) },
    frame,
    'http://127.0.0.1:49152',
  ), undefined, 'unknown message types are still ignored')
})

test('editor bridge emits the strict resolved DSH theme message', () => {
  assert.equal(
    client.encodeEditorOutbound({ type: 'op-bridge/theme', colorScheme: 'dark' }),
    '{"type":"op-bridge/theme","colorScheme":"dark"}',
  )
  assert.deepEqual(client.inject, ['slots', 'theme', 'locale'])
})

function initRetryClock() {
  const scheduled = []
  const cancelled = []
  return {
    scheduled,
    cancelled,
    timer: {
      schedule(callback, delayMs) {
        const handle = { callback, delayMs }
        scheduled.push(handle)
        return handle
      },
      cancel(handle) { cancelled.push(handle) },
    },
  }
}

test('managed editor init starts before iframe load and retries until stopped', () => {
  const sent = []
  const clock = initRetryClock()
  const controller = client.beginEditorInitRetry(
    () => { sent.push('init') },
    () => { sent.push('timeout') },
    clock.timer,
    { intervalMs: 500, timeoutMs: 3_000 },
  )

  assert.deepEqual(sent, ['init'], 'first init must not wait for the iframe load event')
  assert.deepEqual(clock.scheduled.map(handle => handle.delayMs), [500, 3_000], 'retry interval plus timeout budget')
  clock.scheduled[0].callback()
  assert.deepEqual(sent, ['init', 'init'])

  controller.stop()
  assert.equal(clock.cancelled.length, 2, 'stop cancels the pending retry and the timeout budget')
  clock.scheduled[0].callback()
  assert.deepEqual(sent, ['init', 'init'], 'stopped retry callbacks are inert')
})

test('managed editor init exhausts the longer budget exactly once', () => {
  const sent = []
  const clock = initRetryClock()
  client.beginEditorInitRetry(
    () => { sent.push('init') },
    () => { sent.push('timeout') },
    clock.timer,
    { intervalMs: 500, timeoutMs: 3_000 },
  )

  assert.equal(clock.scheduled[1].delayMs, 3_000)
  clock.scheduled[1].callback()
  assert.deepEqual(sent, ['init', 'timeout'], 'the timeout budget still surfaces an error')
  clock.scheduled[0].callback()
  assert.deepEqual(sent, ['init', 'timeout'], 'exhaustion stops the retry interval')
  clock.scheduled[1].callback()
  assert.deepEqual(sent, ['init', 'timeout'], 'the exhausted budget never fires twice')
})

test('managed editor init defaults to a 500 ms interval and a 60 s budget', () => {
  const clock = initRetryClock()
  client.beginEditorInitRetry(() => {}, () => {}, clock.timer)
  assert.deepEqual(clock.scheduled.map(handle => handle.delayMs), [500, 60_000])
})

test('an editor that never announces listening keeps retrying far beyond the former 20-attempt cap', () => {
  const sent = []
  const clock = initRetryClock()
  const controller = client.beginEditorInitRetry(
    () => { sent.push('init') },
    () => { sent.push('timeout') },
    clock.timer,
    { intervalMs: 500, timeoutMs: 60_000 },
  )
  // The old 20 x 500 ms budget (~10 s) killed a first Wasm load on slow
  // machines. An old editor (no op-bridge/listening) must keep receiving
  // init until ready arrives or the 60 s budget runs out.
  for (let tick = 0; tick < 40; tick += 1) {
    const next = clock.scheduled.find(handle => handle.delayMs === 500 && !handle.fired)
    assert.ok(next, 'each retry tick schedules the next one')
    next.fired = true
    next.callback()
  }
  assert.equal(sent.length, 41, '40 retries after the immediate init')
  assert.equal(sent.includes('timeout'), false, 'the longer budget must not exhaust at 40 attempts')
  controller.stop()
  assert.equal(sent.includes('timeout'), false)
})

test('op-bridge/listening resends init immediately and stops the periodic retries', () => {
  const sent = []
  const clock = initRetryClock()
  const controller = client.beginEditorInitRetry(
    () => { sent.push('init') },
    () => { sent.push('timeout') },
    clock.timer,
    { intervalMs: 500, timeoutMs: 3_000 },
  )

  controller.acknowledgeListening()
  assert.deepEqual(sent, ['init', 'init'], 'listening must resend init without waiting for the next tick')
  assert.equal(clock.cancelled.length, 2, 'listening cancels the retry interval and restarts the timeout budget')
  clock.scheduled[0].callback()
  assert.deepEqual(sent, ['init', 'init'], 'cancelled retry callbacks are inert after listening')

  controller.stop()
  clock.scheduled[2].callback()
  assert.deepEqual(sent, ['init', 'init'], 'stopping after listening disarms the restarted budget')
})

test('a listening editor that never becomes ready still times out once', () => {
  const sent = []
  const clock = initRetryClock()
  const controller = client.beginEditorInitRetry(
    () => { sent.push('init') },
    () => { sent.push('timeout') },
    clock.timer,
    { intervalMs: 500, timeoutMs: 3_000 },
  )

  controller.acknowledgeListening()
  const restarted = clock.scheduled[2]
  assert.equal(restarted.delayMs, 3_000, 'listening restarts one full bounded budget for boot to finish')
  restarted.callback()
  assert.deepEqual(sent, ['init', 'init', 'timeout'])
  restarted.callback()
  assert.deepEqual(sent, ['init', 'init', 'timeout'], 'the timeout fires exactly once')
})

test('a duplicate listening message resends init but never restarts the budget again', () => {
  const sent = []
  const clock = initRetryClock()
  const controller = client.beginEditorInitRetry(
    () => { sent.push('init') },
    () => { sent.push('timeout') },
    clock.timer,
    { intervalMs: 500, timeoutMs: 3_000 },
  )

  controller.acknowledgeListening()
  controller.acknowledgeListening()
  assert.deepEqual(sent, ['init', 'init', 'init'], 'every listening message resends init immediately')
  assert.equal(clock.cancelled.length, 2, 'only the first listening restarts the timers')
  clock.scheduled[1].callback()
  assert.deepEqual(sent, ['init', 'init', 'init'], 'the original budget is inert after the restart')
})

test('registers canonical OpenPencil publication/render views and client-only legacy replay aliases', () => {
  const registrations = []
  client.apply({
    on() { return () => {} },
    theme: { getTheme: () => ({ active: { colorScheme: 'light' } }) },
    locale: { getLocale: () => ({ active: 'en' }) },
    slots: {
      inject(_name, install) { return install() },
      register(definition, component) {
        registrations.push({ definition, component })
        return () => {}
      },
    },
  })

  assert.equal(client.OPENPENCIL_RENDER_TOOL_NAME, 'openpencil_render')
  assert.equal(client.OPENPENCIL_NEW_TOOL_NAME, 'openpencil_new')
  assert.equal(client.OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME, 'openpencil_pipeline_begin')
  assert.equal(client.OPENPENCIL_PIPELINE_BATCH_TOOL_NAME, 'openpencil_pipeline_batch')
  assert.equal(client.OPENPENCIL_PIPELINE_FINISH_TOOL_NAME, 'openpencil_pipeline_finish')
  assert.equal(client.OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME, 'openpencil_pipeline_inspect')
  assert.equal(client.LEGACY_DESIGN_RENDER_TOOL_NAME, 'design_render')
  // DSH 0.1.5 settled the details seat: the host declares tool.call.toolview
  // and tool.call.images only, so the plugin no longer registers a
  // tool.details.toolview view for each tool. The editor opens through the
  // page-owned workbench modal instead — which is what every shipped host
  // already did, since slots.inject() waits forever on an absent slot.
  assert.deepEqual(registrations.map(({ definition }) => definition), [
    { name: 'tool.call.toolview', key: 'openpencil_render' },
    { name: 'tool.call.toolview', key: 'openpencil_new' },
    { name: 'tool.call.toolview', key: 'openpencil_pipeline_begin' },
    { name: 'tool.call.toolview', key: 'openpencil_pipeline_finish' },
    { name: 'tool.call.toolview', key: 'design_render' },
    { name: 'tool.call.toolview', key: 'openpencil_pipeline_inspect' },
    { name: 'tool.call.toolview', key: 'openpencil_pipeline_batch' },
    { name: 'conversation.input.dock', id: 'openpencil-selection', order: 30 },
  ])
  assert.equal(registrations[1].component, registrations[0].component, 'new uses the existing auto-open call view')
  assert.equal(registrations[2].component, registrations[0].component, 'pipeline begin uses the auto-open call view')
  assert.equal(registrations[3].component, registrations[0].component, 'pipeline finish uses the auto-open call view')
  assert.equal(registrations[5].component, registrations[0].component, 'pipeline inspect reuses the PNG gallery call view')
  assert.equal(registrations[6].component, registrations[0].component, 'pipeline batch reuses the PNG gallery call view')
})

test('the plugin never asks for a details slot the host does not declare', () => {
  const registrations = []
  const pending = []
  client.apply({
    on() { return () => {} },
    theme: { getTheme: () => ({ active: { colorScheme: 'light' } }) },
    locale: { getLocale: () => ({ active: 'en' }) },
    slots: {
      inject(name, install) {
        if (name === 'tool.details.toolview') {
          pending.push(name)
          return () => {}
        }
        return install()
      },
      register(definition) {
        registrations.push(definition)
        return () => {}
      },
    },
  })

  // This used to assert five parked injections: the plugin optimistically
  // injected a details slot no shipped host declared, and each one waited
  // forever. 0.1.5 removed the type that declaration named, so the plugin now
  // asks for nothing it cannot get — `pending` must stay empty.
  assert.deepEqual(pending, [])
  assert.deepEqual(registrations, [
    { name: 'tool.call.toolview', key: 'openpencil_render' },
    { name: 'tool.call.toolview', key: 'openpencil_new' },
    { name: 'tool.call.toolview', key: 'openpencil_pipeline_begin' },
    { name: 'tool.call.toolview', key: 'openpencil_pipeline_finish' },
    { name: 'tool.call.toolview', key: 'design_render' },
    { name: 'tool.call.toolview', key: 'openpencil_pipeline_inspect' },
    { name: 'tool.call.toolview', key: 'openpencil_pipeline_batch' },
    { name: 'conversation.input.dock', id: 'openpencil-selection', order: 30 },
  ])
})

test('editor launch prefers native details and falls back to the plugin modal', () => {
  const calls = []
  assert.equal(client.requestOpenPencilEditor(
    () => { calls.push('details') },
    () => { calls.push('modal') },
  ), 'details')
  assert.deepEqual(calls, ['details'])

  assert.equal(client.requestOpenPencilEditor(
    undefined,
    () => { calls.push('modal') },
  ), 'modal')
  assert.deepEqual(calls, ['details', 'modal'])
})

test('fallback workbench state is page-owned and survives a Tool card lifecycle', () => {
  const request = {
    sessionId: 'session-a',
    grant: { editor: { launchUrl: '/editor/a' } },
  }
  const store = client.createEditorWorkbenchStore()
  const snapshots = []
  const unsubscribe = store.subscribe(() => { snapshots.push(store.getSnapshot()) })

  assert.equal(store.open(request), true)
  assert.equal(store.getSnapshot(), request)
  // Nothing in this store is tied to the DesignRenderView component that sent
  // the command, so removing that card does not issue a close operation.
  assert.deepEqual(snapshots, [request])
  unsubscribe()
  assert.equal(store.getSnapshot(), request)

  store.close()
  assert.equal(store.getSnapshot(), undefined)
})

test('fallback workbench retains a dirty owner when replacement is denied', () => {
  const first = { sessionId: 'session-a', grant: { editor: { launchUrl: '/editor/a' } } }
  const second = { sessionId: 'session-b', grant: { editor: { launchUrl: '/editor/b' } } }
  let replacementChecks = 0
  let repeats = 0
  const store = client.createEditorWorkbenchStore(() => {
    replacementChecks += 1
    return false
  }, () => { repeats += 1 })

  assert.equal(store.open(first), true)
  assert.equal(store.open(first), true, 'opening the same editor only refocuses it')
  assert.equal(repeats, 1)
  assert.equal(replacementChecks, 0)
  assert.equal(store.open(second), false)
  assert.equal(replacementChecks, 1)
  assert.equal(store.getSnapshot(), first)
})

test('workbench editor identity remounts only the managed document process', () => {
  const grant = { editor: { launchUrl: '/editor/a' } }
  assert.equal(client.editorWorkbenchEditorKey(grant, 'session-a'), 'session-a\n/editor/a')
  assert.notEqual(
    client.editorWorkbenchEditorKey(grant, 'session-a'),
    client.editorWorkbenchEditorKey({ editor: { launchUrl: '/editor/b' } }, 'session-a'),
  )
  assert.notEqual(
    client.editorWorkbenchEditorKey(grant, 'session-a'),
    client.editorWorkbenchEditorKey(grant, 'session-b'),
  )
})

test('workbench disposal captures an idle dirty draft without starting a save', async () => {
  let awaitedSaves = 0
  let captures = 0
  const controller = {
    requestClose: async () => true,
    awaitExistingSave: async () => {
      awaitedSaves += 1
      return true
    },
    captureRecovery: async () => {
      captures += 1
      return true
    },
  }

  assert.equal(
    await client.preserveEditorBeforeWorkbenchDispose({ dirty: true, phase: 'ready' }, controller),
    'recovered',
  )
  assert.equal(awaitedSaves, 0, 'idle dirty disposal must not invent a source save')
  assert.equal(captures, 1)
})

test('workbench disposal joins only an existing save and falls back to recovery', async () => {
  let awaitedSaves = 0
  let captures = 0
  const controller = {
    requestClose: async () => true,
    awaitExistingSave: async () => {
      awaitedSaves += 1
      return false
    },
    captureRecovery: async () => {
      captures += 1
      return true
    },
  }

  assert.equal(
    await client.preserveEditorBeforeWorkbenchDispose({ dirty: true, phase: 'saving' }, controller),
    'recovered',
  )
  assert.equal(awaitedSaves, 1)
  assert.equal(captures, 1)

  controller.awaitExistingSave = async () => {
    awaitedSaves += 1
    return true
  }
  assert.equal(
    await client.preserveEditorBeforeWorkbenchDispose({ dirty: true, phase: 'saving' }, controller),
    'saved',
  )
  assert.equal(awaitedSaves, 2)
  assert.equal(captures, 1, 'a completed in-flight save needs no recovery capture')
})

test('workbench disposal leaves a clean editor untouched', async () => {
  let lifecycleCalls = 0
  const controller = {
    requestClose: async () => true,
    awaitExistingSave: async () => { lifecycleCalls += 1; return true },
    captureRecovery: async () => { lifecycleCalls += 1; return true },
  }

  assert.equal(
    await client.preserveEditorBeforeWorkbenchDispose({ dirty: false, phase: 'ready' }, controller),
    'clean',
  )
  assert.equal(lifecycleCalls, 0)
})

test('unrecovered workbench disposal retains the daemon and skips client DELETE', async () => {
  for (const captureRecovery of [
    async () => false,
    async () => { throw new Error('capture unavailable') },
  ]) {
    let retains = 0
    let clientDeletes = 0
    const controller = {
      requestClose: async () => true,
      awaitExistingSave: async () => true,
      captureRecovery,
      retainServerSessionOnUnmount() { retains += 1 },
    }

    assert.equal(
      await client.preserveEditorBeforeWorkbenchDispose({ dirty: true, phase: 'ready' }, controller),
      'unrecovered',
    )
    assert.equal(retains, 1, 'capture failure must arm server-session retention before React unmount')
    assert.equal(
      client.applyManagedEditorUnmountPolicy({
        retainServerSession: retains > 0,
        dirty: true,
        hasLiveLaunch: true,
      }, () => { clientDeletes += 1 }),
      'retained',
    )
    assert.equal(clientDeletes, 0, 'unrecovered HMR disposal must not issue client DELETE')
  }
})

test('native-style dirty unmount retains its live daemon without a lifecycle controller', () => {
  let clientDeletes = 0
  assert.equal(
    client.applyManagedEditorUnmountPolicy({
      retainServerSession: false,
      dirty: true,
      hasLiveLaunch: true,
    }, () => { clientDeletes += 1 }),
    'retained',
  )
  assert.equal(clientDeletes, 0)
})

test('native-style clean unmount keeps the normal guarded close path', () => {
  let clientDeletes = 0
  assert.equal(
    client.applyManagedEditorUnmountPolicy({
      retainServerSession: false,
      dirty: false,
      hasLiveLaunch: true,
    }, () => { clientDeletes += 1 }),
    'closed',
  )
  assert.equal(clientDeletes, 1)
})

test('live draft unmount always detaches without retaining a pipeline-owned daemon', () => {
  let clientDeletes = 0
  assert.equal(
    client.applyManagedEditorUnmountPolicy({
      retainServerSession: true,
      dirty: true,
      hasLiveLaunch: true,
      liveDraft: true,
    }, () => { clientDeletes += 1 }),
    'closed',
  )
  assert.equal(clientDeletes, 1, 'the close capability detaches the panel; it does not stop the draft daemon')
})

test('live draft canvas has no independent save action', () => {
  assert.equal(client.managedEditorAllowsSave(undefined), true)
  assert.equal(client.managedEditorAllowsSave({}), true)
  assert.equal(client.managedEditorAllowsSave({ liveDraft: false }), true)
  assert.equal(client.managedEditorAllowsSave({ liveDraft: true }), false)
})

test('a successful explicit close is not reclassified as a retained dirty unmount', () => {
  let cleanupCalls = 0
  assert.equal(
    client.applyManagedEditorUnmountPolicy({
      retainServerSession: false,
      dirty: true,
      hasLiveLaunch: false,
    }, () => { cleanupCalls += 1 }),
    'closed',
  )
  assert.equal(cleanupCalls, 1)
})

test('guarded editor close surfaces a server conflict instead of orphaning silently', async () => {
  const launch = {
    sessionId: 'managed-a',
    closeUrl: '/editor/close-a',
  }
  await assert.rejects(
    client.closeManagedEditorLaunch(launch, {
      fetcher: async () => new Response(null, { status: 409 }),
    }),
    /close failed \(409\)/,
  )
  await assert.doesNotReject(client.closeManagedEditorLaunch(launch, {
    fetcher: async () => new Response(null, { status: 204 }),
  }))
})

test('editor bridge maps and emits the resolved DSH locale as BCP 47', () => {
  assert.equal(client.editorLocaleFromDsh('zh'), 'zh-CN')
  assert.equal(client.editorLocaleFromDsh('en'), 'en-US')
  assert.equal(
    client.encodeEditorOutbound({ type: 'op-bridge/locale', locale: 'zh-CN' }),
    '{"type":"op-bridge/locale","locale":"zh-CN"}',
  )
})

test('editor panel chrome follows the resolved editor locale', () => {
  const zh = client.editorPanelCopy('zh-CN')
  assert.equal(zh.save, '保存')
  assert.equal(zh.saving, '保存中…')
  assert.equal(zh.unsaved, '未保存')
  assert.equal(zh.saved, '已保存')
  assert.equal(zh.loading, '正在加载可编辑的 OpenPencil 画布…')
  assert.equal(zh.errorTitle, 'OpenPencil 编辑器不可用')
  assert.equal(zh.editorBusy, '另一个 OpenPencil 编辑器仍有未保存的更改。')
  assert.equal(zh.discard, 'OpenPencil 中有未保存的更改，确定关闭并放弃吗？')
  assert.equal(zh.pngFallback, '打开 PNG 预览')
  assert.equal(zh.editorTitle('home.op'), 'OpenPencil 编辑器：home.op')
  assert.equal(zh.saveConflict(7), 'OpenPencil 保存冲突（服务器版本 7）')

  const en = client.editorPanelCopy('en-US')
  assert.equal(en.save, 'Save')
  assert.equal(en.saving, 'Saving…')
  assert.equal(en.loading, 'Loading editable OpenPencil canvas…')
  assert.equal(en.editorBusy, 'Another OpenPencil editor still has unsaved changes.')
  assert.equal(en.discard, 'OpenPencil has unsaved changes. Close and discard them?')
  assert.equal(en.editorTitle('home.op'), 'OpenPencil editor: home.op')
  assert.equal(en.syncConflict(7), 'The source changed outside this editor (server v7). Save was stopped.')
})

test('fallback editor workbench chrome follows the resolved editor locale', () => {
  assert.equal(client.OPENPENCIL_EDITOR_CLOSE_BUTTON_ATTRIBUTE, 'data-openpencil-editor-close')
  assert.equal(client.EDITOR_WORKBENCH_CLOSE_BUTTON_MIN_WIDTH, 72)
  assert.deepEqual(client.editorModalCopy('zh-CN'), {
    title: 'OpenPencil 编辑器',
    close: '关闭',
    fullscreen: '全屏',
    restore: '退出全屏',
    resize: '拖动调整编辑区宽度',
    discard: 'OpenPencil 中有未保存的更改，确定关闭并放弃吗？',
  })
  assert.deepEqual(client.editorModalCopy('en-US'), {
    title: 'OpenPencil editor',
    close: 'Close',
    fullscreen: 'Full screen',
    restore: 'Exit full screen',
    resize: 'Drag to resize the editor',
    discard: 'OpenPencil has unsaved changes. Close and discard them?',
  })
})

test('fallback editor workbench reserves a real split dock and fullscreens before the conversation gets cramped', () => {
  assert.equal(client.EDITOR_WORKBENCH_FULLSCREEN_BREAKPOINT, 1480)
  assert.equal(client.EDITOR_WORKBENCH_MIN_WIDTH, 640)
  assert.equal(client.EDITOR_WORKBENCH_MAX_WIDTH, 960)
  assert.equal(client.EDITOR_WORKBENCH_LEFT_CLEARANCE, 840)
  assert.equal(client.EDITOR_WORKBENCH_RESIZE_STEP, 32)
  assert.equal(client.editorWorkbenchUsesFullscreen(1479), true)
  assert.equal(client.editorWorkbenchUsesFullscreen(1480), false)
  assert.equal(client.automaticEditorWorkbenchLayout(1280), 'fullscreen', 'real 1280px DSH view auto-opens as a fullscreen workbench')
  assert.equal(client.automaticEditorWorkbenchLayout(1480), 'dock')
  assert.deepEqual(client.editorWorkbenchWidthBounds(2048), { min: 640, max: 960, initial: 720 })
  assert.deepEqual(client.editorWorkbenchWidthBounds(1600), { min: 640, max: 760, initial: 720 })
  assert.deepEqual(client.editorWorkbenchWidthBounds(1480), { min: 640, max: 640, initial: 640 })
  assert.equal(client.clampEditorWorkbenchWidth(2000, 1600), 760)
  assert.equal(client.clampEditorWorkbenchWidth(100, 1280), 640)
  assert.equal(client.resizedEditorWorkbenchWidth(720, 1000, 900, 1600), 760)
  assert.equal(client.resizedEditorWorkbenchWidth(720, 1000, 1200, 1280), 640)
  const clampedAtMax = client.resizedEditorWorkbenchWidth(760, 900, 700, 1600)
  assert.equal(clampedAtMax, 760)
  assert.equal(
    client.resizedEditorWorkbenchWidth(clampedAtMax, 700, 710, 1600),
    750,
    'incremental drag coordinates leave a width bound immediately when the pointer reverses',
  )
  const preferredWidth = 720
  assert.equal(client.clampEditorWorkbenchWidth(preferredWidth, 1100), 640, 'a narrow viewport only clamps the effective width')
  assert.equal(client.clampEditorWorkbenchWidth(preferredWidth, 1920), 720, 'widening restores the unchanged preference')
})

test('fallback dock owns and exactly restores the DSH root margin', () => {
  const root = { style: { marginRight: '', minWidth: '' }, dataset: {} }
  const lease = client.claimEditorWorkbenchDock(root, 'editor-a', 720, 0)
  assert.ok(lease)
  assert.equal(root.style.marginRight, '720px')
  assert.equal(root.style.minWidth, '0')
  assert.equal(root.dataset[client.OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE], 'editor-a')
  lease.update(803.6)
  assert.equal(root.style.marginRight, '804px')
  assert.equal(client.claimEditorWorkbenchDock(root, 'editor-b', 640, 0), undefined)
  lease.release()
  assert.equal(root.style.marginRight, '')
  assert.equal(root.style.minWidth, '')
  assert.equal(root.dataset[client.OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE], undefined)

  root.style.marginRight = '320px'
  assert.equal(client.claimEditorWorkbenchDock(root, 'editor-c', 640, 320), undefined)
})

test('fullscreen editor workbench traps focus at both Tab boundaries', () => {
  assert.equal(client.editorWorkbenchFocusTargetIndex(3, 0, true), 2, 'Shift+Tab wraps first to last')
  assert.equal(client.editorWorkbenchFocusTargetIndex(3, 2, false), 0, 'Tab wraps last to first')
  assert.equal(client.editorWorkbenchFocusTargetIndex(3, 1, false), -1, 'interior Tab keeps native order')
  assert.equal(client.editorWorkbenchFocusTargetIndex(3, 1, true), -1, 'interior Shift+Tab keeps native order')
  assert.equal(client.editorWorkbenchFocusTargetIndex(3, -1, false), 0, 'escaped focus returns to first')
  assert.equal(client.editorWorkbenchFocusTargetIndex(3, -1, true), 2, 'escaped reverse focus returns to last')
  assert.equal(client.editorWorkbenchFocusTargetIndex(0, -1, false), -1, 'an empty surface falls back to itself')
})

test('side editor workbench only owns Escape while focus is inside', () => {
  assert.equal(client.editorWorkbenchShouldHandleEscape(false, false), false)
  assert.equal(client.editorWorkbenchShouldHandleEscape(false, true), true)
  assert.equal(client.editorWorkbenchShouldHandleEscape(true, false), true)
  assert.equal(client.editorWorkbenchShouldHandleEscape(true, true), true)
})

test('fallback editor modal confirms only when the managed editor is dirty', () => {
  const cleanRoot = { querySelector() { return null } }
  let confirmations = 0
  assert.equal(client.confirmEditorModalClose(cleanRoot, 'discard?', () => {
    confirmations += 1
    return false
  }), true)
  assert.equal(confirmations, 0)

  const dirtyRoot = { querySelector(selector) {
    assert.equal(selector, '[data-tool-details-dirty="true"]')
    return {}
  } }
  assert.equal(client.confirmEditorModalClose(dirtyRoot, 'discard?', message => {
    confirmations += 1
    assert.equal(message, 'discard?')
    return false
  }), false)
  assert.equal(client.confirmEditorModalClose(dirtyRoot, 'discard?', () => {
    confirmations += 1
    return true
  }), true)
  assert.equal(confirmations, 2)
})

test('selection polling stops and clears state on terminal 404 and 410 responses', async () => {
  for (const status of [404, 410]) {
    const clock = controlledPollTimer()
    let stops = 0
    let calls = 0
    client.startEditorSelectionPolling({
      url: `/editor/selection/${status}`,
      fetcher: async () => {
        calls += 1
        return new Response(null, { status })
      },
      onValue() { assert.fail('terminal responses must not publish a selection') },
      onStop() { stops += 1 },
      timer: clock.timer,
    })
    await flushAsync()
    assert.equal(calls, 1)
    assert.equal(stops, 1, `${status} must clear the live selection`)
    assert.equal(clock.scheduled.length, 0, `${status} must not schedule another poll`)
  }
})

test('selection polling retries transient failures and cleanup cancels the next timer', async () => {
  const clock = controlledPollTimer()
  const values = []
  let calls = 0
  let stops = 0
  const stop = client.startEditorSelectionPolling({
    url: '/editor/selection/transient',
    fetcher: async () => {
      calls += 1
      if (calls === 1) return new Response(null, { status: 503 })
      return Response.json({ selection: { selectedIds: ['n1'] } })
    },
    onValue(value) { values.push(value) },
    onStop() { stops += 1 },
    intervalMs: 123,
    timer: clock.timer,
  })
  await flushAsync()
  assert.equal(clock.scheduled.length, 1)
  assert.equal(clock.scheduled[0].delayMs, 123)

  clock.scheduled.shift().callback()
  await flushAsync()
  assert.equal(calls, 2)
  assert.deepEqual(values, [{ selection: { selectedIds: ['n1'] } }])
  assert.equal(clock.scheduled.length, 1)

  const pending = clock.scheduled[0]
  stop()
  stop()
  assert.deepEqual(clock.cancelled, [pending])
  assert.equal(stops, 1, 'cleanup must clear selection exactly once')
})

test('selection polling cleanup aborts an in-flight request without scheduling a retry', async () => {
  const clock = controlledPollTimer()
  let observedSignal
  let stops = 0
  const stop = client.startEditorSelectionPolling({
    url: '/editor/selection/in-flight',
    fetcher: async (_url, init = {}) => {
      observedSignal = init.signal
      return new Promise((_resolve, reject) => {
        observedSignal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')), { once: true })
      })
    },
    onValue() { assert.fail('an aborted request must not publish a selection') },
    onStop() { stops += 1 },
    timer: clock.timer,
  })
  await flushAsync()
  assert.equal(observedSignal.aborted, false)
  stop()
  await flushAsync()
  assert.equal(observedSignal.aborted, true)
  assert.equal(stops, 1)
  assert.equal(clock.scheduled.length, 0)
})

test('editor control capabilities remain on the DSH origin', () => {
  assert.equal(
    client.editorControlUrl('/_dsh/dsh-openpencil/editor/launch'),
    'http://127.0.0.1:3080/_dsh/dsh-openpencil/editor/launch',
  )
  assert.throws(
    () => client.editorControlUrl('https://example.com/save'),
    /same-origin/,
  )
})

test('editor recovery controls stay same-origin and restore only after an explicit action', async () => {
  const recovery = {
    id: 'a'.repeat(43),
    capturedAt: 1_800_000_000_000,
    bytes: 96,
    sourceName: 'design.op',
    sourceChangedSinceCapture: false,
    cacheLabel: `dsh-openpencil/recovery/${'a'.repeat(43)}.json`,
  }
  const launch = {
    sessionId: 'managed-session',
    recoveryUrl: '/_dsh/dsh-openpencil/editor/session/managed-session/recovery',
    recovery,
  }
  assert.equal(
    client.editorRecoveryItemUrl(launch, recovery.id),
    `http://127.0.0.1:3080/_dsh/dsh-openpencil/editor/session/managed-session/recovery/${recovery.id}`,
  )
  const calls = []
  const fetcher = async (url, init) => {
    calls.push({ url, init })
    return Response.json({ ok: true, version: 4, docJson: '{"version":"1.0","children":[]}' })
  }
  const docJson = await client.restoreManagedEditorRecovery(launch, recovery, fetcher)
  assert.equal(docJson, '{"version":"1.0","children":[]}')
  assert.equal(calls.length, 1)
  assert.equal(calls[0].init.method, 'POST')
  assert.equal(calls[0].init.credentials, 'same-origin')
  assert.equal(client.editorRecoveryCopy('zh-CN').available('design.op').includes('仍需点击“保存”'), true)
})

test('editor recovery parser rejects path-like or malformed public metadata', () => {
  assert.equal(client.editorRecoverySummaryOf({
    id: '../private/source.op',
    capturedAt: Date.now(),
    bytes: 3,
    sourceName: 'source.op',
    sourceChangedSinceCapture: false,
    cacheLabel: '/Users/private/recovery.json',
  }), undefined)
})

test('expired editor launch refreshes once and prefers current launch docJson', async () => {
  const calls = []
  const fetcher = async (url, init = {}) => {
    calls.push({ url, init })
    if (calls.length === 1) return new Response(null, { status: 410 })
    if (calls.length === 2) {
      return Response.json({ launchUrl: '/_dsh/dsh-openpencil/editor/fresh-cap' })
    }
    return Response.json({
      sessionId: 'editor-1',
      iframeUrl: 'http://127.0.0.1:49152/?embed=vscode',
      token: 'daemon-secret',
      saveUrl: '/_dsh/dsh-openpencil/editor/save/editor-1',
      closeUrl: '/_dsh/dsh-openpencil/editor/close/editor-1',
      docJson: '{"source":"current"}',
    })
  }
  const prepared = await client.prepareManagedEditor({
    enabled: true,
    launchUrl: '/_dsh/dsh-openpencil/editor/old-cap',
  }, {
    path: '/workspace/design.op',
    url: '/_dsh/dsh-openpencil/render/document-cap',
  }, { fetcher })

  assert.equal(prepared.documentJson, '{"source":"current"}')
  assert.equal(calls.length, 3, 'launch docJson must avoid fetching the immutable historical snapshot')
  assert.equal(calls[0].url, 'http://127.0.0.1:3080/_dsh/dsh-openpencil/editor/old-cap')
  assert.equal(calls[1].url, 'http://127.0.0.1:3080/_dsh/dsh-openpencil/editor/refresh')
  assert.deepEqual(JSON.parse(calls[1].init.body), {
    launchUrl: '/_dsh/dsh-openpencil/editor/old-cap',
    sourcePath: '/workspace/design.op',
    documentUrl: '/_dsh/dsh-openpencil/render/document-cap',
  })
  assert.equal(calls[2].url, 'http://127.0.0.1:3080/_dsh/dsh-openpencil/editor/fresh-cap')
})

test('old editor host falls back to the immutable document only when launch omits docJson', async () => {
  const calls = []
  const fetcher = async (url, init = {}) => {
    calls.push({ url, init })
    if (calls.length === 1) {
      return Response.json({
        sessionId: 'editor-old',
        iframeUrl: 'http://127.0.0.1:49153/?embed=vscode',
        token: 'daemon-secret',
        saveUrl: '/editor/save/editor-old',
        closeUrl: '/editor/close/editor-old',
        docJson: 42,
      })
    }
    return new Response('{"source":"snapshot"}', { status: 200 })
  }
  const prepared = await client.prepareManagedEditor({
    enabled: true,
    launchUrl: '/editor/launch-old-host',
  }, {
    path: '/workspace/design.op',
    url: '/render/immutable-document',
  }, { fetcher })
  assert.equal(prepared.documentJson, '{"source":"snapshot"}')
  assert.equal(calls.length, 2)
  assert.equal(calls[1].url, 'http://127.0.0.1:3080/render/immutable-document')
})

test('current or renewed editor contracts never reopen an immutable historical snapshot', async () => {
  const launchWithoutDoc = {
    sessionId: 'editor-current',
    iframeUrl: 'http://127.0.0.1:49154/?embed=vscode',
    token: 'daemon-secret',
    saveUrl: '/editor/save/current',
    closeUrl: '/editor/close/current',
  }
  let currentCalls = 0
  await assert.rejects(client.prepareManagedEditor({
    enabled: true,
    launchUrl: '/editor/current-launch',
    refreshUrl: '/editor/current-refresh',
  }, {
    path: '/workspace/design.op',
    url: '/render/old-snapshot',
  }, {
    fetcher: async () => {
      currentCalls += 1
      return Response.json(launchWithoutDoc)
    },
  }), /omitted current docJson/)
  assert.equal(currentCalls, 2, 'current contract must close its launched daemon without fetching the historical snapshot')

  let renewedCalls = 0
  await assert.rejects(client.prepareManagedEditor({
    enabled: true,
    launchUrl: '/editor/replayed-old-cap',
  }, {
    path: '/workspace/design.op',
    url: '/render/old-snapshot',
  }, {
    fetcher: async () => {
      renewedCalls += 1
      if (renewedCalls === 1) return new Response(null, { status: 410 })
      if (renewedCalls === 2) return Response.json({ launchUrl: '/editor/renewed-cap' })
      return Response.json(launchWithoutDoc)
    },
  }), /omitted current docJson/)
  assert.equal(renewedCalls, 4, 'renewed launch must close its daemon instead of reading the historical document')
})

test('cancelled editor mount closes exactly the launch that completed after cancellation', async () => {
  const calls = []
  const fetcher = async (url, init = {}) => {
    calls.push({ url, init })
    if (calls.length === 1) {
      return Response.json({
        sessionId: 'cancelled-session',
        iframeUrl: 'http://127.0.0.1:49155/?embed=vscode',
        token: 'daemon-secret',
        saveUrl: '/editor/session/cancelled-session/save',
        closeUrl: '/editor/session/cancelled-session',
        docJson: '{"source":"current"}',
      })
    }
    return Response.json({ ok: true })
  }

  const prepared = await client.prepareManagedEditorForMount({
    enabled: true,
    launchUrl: '/editor/cancelled-launch',
    refreshUrl: '/editor/cancelled-refresh',
  }, {
    path: '/workspace/design.op',
    url: '/render/document',
  }, () => false, { fetcher, sessionId: 'dsh-session' })

  assert.equal(prepared, undefined)
  assert.equal(calls.length, 2)
  assert.equal(calls[1].url, 'http://127.0.0.1:3080/editor/session/cancelled-session')
  assert.equal(calls[1].init.method, 'DELETE')
  assert.equal(calls[1].init.keepalive, true)
  assert.deepEqual(JSON.parse(calls[1].init.body), {
    sessionId: 'cancelled-session',
    dirty: false,
  })
})

test('editor launch refresh is limited to 410 and one retry', async () => {
  let notGoneCalls = 0
  await assert.rejects(client.launchManagedEditor({
    enabled: true,
    launchUrl: '/editor/not-found',
  }, {
    path: '/workspace/design.op',
    url: '/render/document',
  }, {
    fetcher: async () => {
      notGoneCalls += 1
      return new Response(null, { status: 404 })
    },
  }), /launch failed \(404\)/)
  assert.equal(notGoneCalls, 1, '404 must not invoke refresh')

  let goneCalls = 0
  const goneUrls = []
  await assert.rejects(client.launchManagedEditor({
    enabled: true,
    launchUrl: '/editor/stale',
    refreshUrl: '/editor/custom-refresh',
  }, {
    path: '/workspace/design.op',
    url: '/render/document',
  }, {
    fetcher: async (url) => {
      goneCalls += 1
      goneUrls.push(url)
      if (goneCalls === 2) return Response.json({ launchUrl: '/editor/still-stale' })
      return new Response(null, { status: 410 })
    },
  }), /launch failed \(410\)/)
  assert.equal(goneCalls, 3, 'a second 410 must not create a refresh loop')
  assert.equal(goneUrls[1], 'http://127.0.0.1:3080/editor/custom-refresh')
})

test('editor responses surface only a bounded top-level error string', async () => {
  const editor = {
    enabled: true,
    launchUrl: '/editor/error-detail',
  }
  const document = {
    path: '/workspace/design.op',
    url: '/render/document',
  }
  const launch = response => client.launchManagedEditor(editor, document, {
    fetcher: async () => response,
  })

  await assert.rejects(launch(Response.json({
    ok: false,
    error: '  OpenPencil editor web bundle was not ready  ',
    token: 'must-not-appear',
    iframeUrl: 'https://must-not-appear.example/editor',
    message: 'must-not-appear',
  }, { status: 500 })), error => {
    assert.equal(
      error.message,
      'OpenPencil editor launch failed (500): OpenPencil editor web bundle was not ready',
    )
    return true
  })

  for (const response of [
    Response.json({
      ok: false,
      error: { message: 'must-not-appear' },
      token: 'must-not-appear',
      iframeUrl: 'https://must-not-appear.example/editor',
    }, { status: 500 }),
    new Response('not json: must-not-appear', {
      status: 500,
      headers: { 'content-type': 'text/plain' },
    }),
  ]) {
    await assert.rejects(launch(response), {
      message: 'OpenPencil editor launch failed (500)',
    })
  }

  await assert.rejects(launch(Response.json({
    ok: false,
    error: 'x'.repeat(513),
    token: 'must-not-appear',
  }, { status: 500 })), {
    message: 'OpenPencil editor launch failed (500)',
  })
})

test('successful editor JSON responses retain the launch contract', async () => {
  const launch = await client.launchManagedEditor({
    enabled: true,
    launchUrl: '/editor/success-response',
  }, {
    path: '/workspace/design.op',
    url: '/render/document',
  }, {
    fetcher: async () => Response.json({
      sessionId: 'success-session',
      iframeUrl: 'http://127.0.0.1:49156/?embed=vscode',
      token: 'daemon-secret',
      saveUrl: '/editor/save/success-session',
      closeUrl: '/editor/close/success-session',
      docJson: '{"source":"current"}',
    }),
  })

  assert.equal(launch.sessionId, 'success-session')
  assert.equal(launch.docJson, '{"source":"current"}')
  assert.equal(launch.saveUrl, 'http://127.0.0.1:3080/editor/save/success-session')
})

test('live draft launch keeps the pipeline daemon authoritative and skips snapshot loading', async () => {
  const calls = []
  const prepared = await client.prepareManagedEditor({
    enabled: true,
    launchUrl: '/editor/live-launch',
  }, {
    path: '/private/draft.op',
    url: '/render/immutable-document-that-must-not-load',
  }, {
    fetcher: async (url, init = {}) => {
      calls.push({ url, init })
      return Response.json({
        sessionId: 'live-session',
        iframeUrl: 'http://127.0.0.1:49157/?embed=vscode',
        token: 'daemon-secret',
        saveUrl: '/editor/live/save',
        closeUrl: '/editor/live/detach',
        liveDraft: true,
        readyUrl: '/editor/live/ready',
      })
    },
  })

  assert.equal(calls.length, 1, 'live boot must not fetch or reopen an immutable document')
  assert.equal(prepared.documentJson, '')
  assert.equal(prepared.launch.liveDraft, true)
  assert.equal(prepared.launch.readyUrl, 'http://127.0.0.1:3080/editor/live/ready')
})

test('bridge ready attaches a live draft without posting open-document', async () => {
  const requests = []
  const posted = []
  const result = await client.completeManagedEditorBridgeReady({
    sessionId: 'live-session',
    iframeUrl: 'http://127.0.0.1:49157/?embed=vscode',
    token: 'daemon-secret',
    saveUrl: 'http://127.0.0.1:3080/editor/live/save',
    closeUrl: 'http://127.0.0.1:3080/editor/live/detach',
    liveDraft: true,
    readyUrl: 'http://127.0.0.1:3080/editor/live/ready',
  }, '{"stale":"snapshot"}', message => { posted.push(message) }, {
    fetcher: async (url, init) => {
      requests.push({ url, init })
      return new Response(null, { status: 204 })
    },
  })

  assert.equal(result, 'live-draft-attached')
  assert.deepEqual(posted, [], 'a stale snapshot must never replace pipeline mutations')
  assert.equal(requests.length, 1)
  assert.equal(requests[0].url, 'http://127.0.0.1:3080/editor/live/ready')
  assert.equal(requests[0].init.method, 'POST')
  assert.equal(requests[0].init.credentials, 'same-origin')
  assert.deepEqual(JSON.parse(requests[0].init.body), { sessionId: 'live-session' })
})

test('bridge ready preserves normal managed-editor document opening', async () => {
  const posted = []
  let networkCalls = 0
  const result = await client.completeManagedEditorBridgeReady({
    sessionId: 'persisted-session',
    iframeUrl: 'http://127.0.0.1:49158/?embed=vscode',
    token: 'daemon-secret',
    saveUrl: 'http://127.0.0.1:3080/editor/save',
    closeUrl: 'http://127.0.0.1:3080/editor/close',
  }, '{"current":"document"}', message => { posted.push(message) }, {
    fetcher: async () => {
      networkCalls += 1
      return new Response(null, { status: 500 })
    },
  })

  assert.equal(result, 'document-open-requested')
  assert.deepEqual(posted, [{ type: 'op-bridge/open-document', json: '{"current":"document"}' }])
  assert.equal(networkCalls, 0)
})

test('live draft launch requires a ready capability and surfaces attach failure', async () => {
  await assert.rejects(client.launchManagedEditor({
    enabled: true,
    launchUrl: '/editor/live-missing-ready',
  }, {
    path: '/private/draft.op',
    url: '/render/document',
  }, {
    fetcher: async () => Response.json({
      sessionId: 'live-missing-ready',
      iframeUrl: 'http://127.0.0.1:49159/?embed=vscode',
      token: 'daemon-secret',
      saveUrl: '/editor/live/save',
      closeUrl: '/editor/live/detach',
      liveDraft: true,
    }),
  }), /live draft launch omitted readyUrl/)

  await assert.rejects(client.completeManagedEditorBridgeReady({
    sessionId: 'live-failed',
    iframeUrl: 'http://127.0.0.1:49159/?embed=vscode',
    token: 'daemon-secret',
    saveUrl: 'http://127.0.0.1:3080/editor/live/save',
    closeUrl: 'http://127.0.0.1:3080/editor/live/detach',
    liveDraft: true,
    readyUrl: 'http://127.0.0.1:3080/editor/live/ready',
  }, '', () => {}, {
    fetcher: async () => new Response(null, { status: 409 }),
  }), /live draft attach failed \(409\)/)
})

test('refresh conflict is surfaced without launching or reading the historical document', async () => {
  const calls = []
  await assert.rejects(client.prepareManagedEditor({
    enabled: true,
    launchUrl: '/editor/expired-cap',
    refreshUrl: '/editor/refresh-cap',
  }, {
    path: '/workspace/design.op',
    url: '/render/signed-document-cap',
  }, {
    fetcher: async (url, init = {}) => {
      calls.push({ url, init })
      if (calls.length === 1) return new Response(null, { status: 410 })
      return Response.json({
        error: 'source-changed',
        message: 'render again before editing',
      }, { status: 409 })
    },
  }), /refresh failed \(409\): source-changed/)
  assert.equal(calls.length, 2, '409 must not launch again or fetch the historical document')
  assert.equal(calls[1].url, 'http://127.0.0.1:3080/editor/refresh-cap')
})

test('save successor survives a reload and is preferred when the same card reopens', async () => {
  const storage = memoryStorage()
  const original = {
    enabled: true,
    launchUrl: '/editor/original-card-cap',
    refreshUrl: '/editor/original-refresh',
  }
  const successor = client.rememberEditorSuccessor(original.launchUrl, {
    ok: true,
    token: 'must-not-persist',
    docJson: '{"must":"not persist"}',
    sourcePath: '/workspace/must-not-persist.op',
    editor: {
      enabled: true,
      launchUrl: '/editor/successor-cap',
      refreshUrl: '/editor/successor-refresh',
      token: 'nested-token-must-not-persist',
    },
  }, { storage })

  assert.deepEqual(successor, {
    enabled: true,
    launchUrl: 'http://127.0.0.1:3080/editor/successor-cap',
    refreshUrl: 'http://127.0.0.1:3080/editor/successor-refresh',
  })
  const key = client.editorSuccessorStorageKey(original.launchUrl)
  const persisted = storage.values.get(key)
  assert.deepEqual(JSON.parse(persisted), {
    launchUrl: 'http://127.0.0.1:3080/editor/successor-cap',
    refreshUrl: 'http://127.0.0.1:3080/editor/successor-refresh',
  })
  assert.equal(persisted.includes('token'), false)
  assert.equal(persisted.includes('workspace'), false)
  assert.equal(persisted.includes('must persist'), false)

  // A fresh component/module instance resolves from the same tab storage.
  const afterReload = client.editorGrantForBoot(original, { storage })
  assert.deepEqual(afterReload, successor)
  const calls = []
  const prepared = await client.prepareManagedEditor(afterReload, {
    path: '/workspace/design.op',
    url: '/render/original-document-cap',
  }, {
    fetcher: async (url) => {
      calls.push(url)
      return Response.json({
        sessionId: 'successor-session',
        iframeUrl: 'http://127.0.0.1:49160/?embed=vscode',
        token: 'ephemeral-daemon-token',
        saveUrl: '/editor/save/successor-session',
        closeUrl: '/editor/close/successor-session',
        docJson: '{"source":"after-save"}',
      })
    },
  })
  assert.equal(calls[0], 'http://127.0.0.1:3080/editor/successor-cap')
  assert.equal(prepared.documentJson, '{"source":"after-save"}')
})

test('successor storage rejects cross-origin or corrupt capabilities and falls back safely', () => {
  const storage = memoryStorage()
  const original = {
    enabled: true,
    launchUrl: '/editor/original-safe-cap',
    refreshUrl: '/editor/original-safe-refresh',
  }
  const key = client.editorSuccessorStorageKey(original.launchUrl)
  storage.setItem(key, JSON.stringify({
    launchUrl: 'https://attacker.example/launch',
    refreshUrl: '/editor/apparently-safe-refresh',
    token: 'injected',
  }))
  assert.equal(client.editorGrantForBoot(original, { storage }), original)
  assert.equal(storage.getItem(key), null, 'invalid stored successors should be discarded')

  storage.setItem(key, '{not-json')
  assert.equal(client.editorGrantForBoot(original, { storage }), original)
  assert.equal(storage.getItem(key), null, 'corrupt stored successors should be discarded')

  client.rememberEditorSuccessor(original.launchUrl, {
    editor: {
      enabled: true,
      launchUrl: '/editor/temporarily-valid',
      refreshUrl: '/editor/temporarily-valid-refresh',
    },
  }, { storage })
  assert.notEqual(storage.getItem(key), null)
  assert.equal(client.rememberEditorSuccessor(original.launchUrl, {
    editor: {
      enabled: true,
      launchUrl: '/editor/new-launch',
      refreshUrl: 'https://attacker.example/refresh',
    },
  }, { storage }), undefined)
  assert.equal(storage.getItem(key), null, 'an invalid save successor must not leave a stale capability')

  const deniedStorage = {
    getItem() { throw new Error('sessionStorage denied') },
    setItem() { throw new Error('sessionStorage denied') },
    removeItem() { throw new Error('sessionStorage denied') },
  }
  assert.equal(client.editorGrantForBoot(original, { storage: deniedStorage }), original)
  assert.doesNotThrow(() => client.rememberEditorSuccessor(original.launchUrl, {
    editor: {
      enabled: true,
      launchUrl: '/editor/safe-successor',
      refreshUrl: '/editor/safe-successor-refresh',
    },
  }, { storage: deniedStorage }))
  assert.throws(
    () => client.editorSuccessorStorageKey('https://attacker.example/original'),
    /same-origin/,
  )
})

test('the page-wide editor coordinator closes only the previous owner', () => {
  const closed = []
  const first = Symbol('first-editor')
  const second = Symbol('second-editor')
  const releaseFirst = client.claimEditor(first, () => { closed.push('first') })
  const releaseSecond = client.claimEditor(second, () => { closed.push('second') })
  assert.equal(typeof releaseFirst, 'function')
  assert.equal(typeof releaseSecond, 'function')
  assert.deepEqual(closed, ['first'])
  releaseFirst()
  const third = Symbol('third-editor')
  const releaseThird = client.claimEditor(third, () => { closed.push('third') })
  assert.deepEqual(closed, ['first', 'second'])
  releaseSecond()
  releaseThird()
})

test('the page-wide editor coordinator lets a dirty owner veto takeover', () => {
  const calls = []
  const first = Symbol('dirty-editor')
  const denied = Symbol('denied-editor')
  const accepted = Symbol('accepted-editor')
  const releaseFirst = client.claimEditor(first, () => {
    calls.push('asked-dirty-owner')
    return false
  })
  assert.equal(typeof releaseFirst, 'function')
  assert.equal(client.claimEditor(denied, () => { calls.push('denied-owner') }), undefined)
  assert.deepEqual(calls, ['asked-dirty-owner'])

  releaseFirst()
  const releaseAccepted = client.claimEditor(accepted, () => { calls.push('accepted-owner') })
  assert.equal(typeof releaseAccepted, 'function')
  releaseAccepted()
})

test('dirty editor close requires explicit confirmation', () => {
  let calls = 0
  assert.equal(client.confirmEditorClose(false, () => { calls += 1; return false }), true)
  assert.equal(calls, 0)
  assert.equal(client.confirmEditorClose(true, () => { calls += 1; return false }), false)
  assert.equal(client.confirmEditorClose(true, () => { calls += 1; return true }), true)
  assert.equal(calls, 2)
})
