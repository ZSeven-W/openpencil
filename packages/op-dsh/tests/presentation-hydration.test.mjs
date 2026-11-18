import assert from 'node:assert/strict'
import { randomBytes } from 'node:crypto'
import { createServer, request as httpRequest } from 'node:http'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import {
  PRESENTATION_HYDRATION_ROUTE,
  PresentationHydrationController,
  parseHydratableBatchResult,
  parseHydratableAgentRunResult,
  parseHydratableBeginResult,
  parseHydratableInspectionResult,
  parseHydratableNewResult,
  parseHydratablePipelineResult,
  parseHydratableRenderResult,
} from '../lib/presentation-hydration.js'
import {
  RenderAccessController,
  renderDir,
  snapshotDir,
  stateRoot,
} from '../lib/renderer.js'

const IMAGE_SHA = 'a'.repeat(64)
const DOCUMENT_SHA = 'b'.repeat(64)
const DRAFT_ID = 'd'.repeat(32)

function closeServer(server) {
  return new Promise(resolve => server.close(resolve))
}

function renderResult(overrides = {}) {
  const filename = 'render-00000000-0000-4000-8000-000000000001.png'
  const path = join(renderDir(), filename)
  const documentFilename = `${DOCUMENT_SHA}.op`
  return {
    path,
    filename,
    mimeType: 'image/png',
    kind: 'image',
    description: 'Rendered /tmp/design.op with openpencil (exact)',
    sourceTool: 'openpencil_render',
    previewIntent: 'image',
    bytes: 1234,
    width: 375,
    height: 800,
    sha256: IMAGE_SHA,
    sourcePath: '/tmp/design.op',
    renderer: 'openpencil',
    rendererBinary: '/Applications/OpenPencil.app/Contents/MacOS/openpencil-desktop',
    fidelity: 'exact',
    warnings: [],
    frames: [{
      path,
      filename,
      mimeType: 'image/png',
      bytes: 1234,
      width: 375,
      height: 800,
      sha256: IMAGE_SHA,
      id: 'frame-1',
      name: 'Home',
      index: 0,
    }],
    frameCount: 1,
    editable: true,
    document: {
      path: join(snapshotDir(), documentFilename),
      filename: documentFilename,
      mimeType: 'application/json',
      bytes: 4567,
      sha256: DOCUMENT_SHA,
    },
    ...overrides,
  }
}

function newResult(overrides = {}) {
  const documentFilename = `${DOCUMENT_SHA}.op`
  return {
    path: '/tmp/generated.op',
    filename: 'generated.op',
    bytes: 4567,
    sha256: DOCUMENT_SHA,
    created: true,
    applied: true,
    saved: true,
    sourceTool: 'openpencil_new',
    previewIntent: 'document',
    editable: true,
    autoOpenEditor: true,
    document: {
      path: join(snapshotDir(), documentFilename),
      filename: documentFilename,
      mimeType: 'application/json',
      bytes: 4567,
      sha256: DOCUMENT_SHA,
    },
    result: { applied: true, inserted: 1 },
    note: 'Created and saved /tmp/generated.op; DSH requests the managed OpenPencil editor to open automatically when the editor surface is idle.',
    ...overrides,
  }
}

function pipelineResult(overrides = {}) {
  const { result: _result, ...base } = newResult()
  const previewFilename = 'render-00000000-0000-4000-8000-000000000099.png'
  return {
    ...base,
    draftId: DRAFT_ID,
    sourceTool: 'openpencil_pipeline_finish',
    published: true,
    preview: {
      path: join(renderDir(), previewFilename),
      filename: previewFilename,
      mimeType: 'image/png',
      bytes: 1234,
      width: 390,
      height: 844,
      sha256: IMAGE_SHA,
      index: 0,
    },
    note: 'Published /tmp/generated.op atomically and requested idle-only editor auto-open.',
    ...overrides,
  }
}

function beginResult(overrides = {}) {
  const documentFilename = `${DOCUMENT_SHA}.op`
  return {
    draftId: DRAFT_ID,
    path: '/tmp/ecommerce-home.op',
    version: 1,
    createdAt: 123,
    platform: 'web',
    canvas: {
      platform: 'web', width: 1440, seedHeight: 900, finalHeight: 'fit_content',
      fixedViewport: false, rootCount: 1, rootType: 'frame',
    },
    buildContract: {
      version: 'openpencil-script-v12',
      canvas: {
        width: 1440,
        rootHeight: 'fit_content',
        rule: 'Keep the existing root at height:"fit_content".',
      },
      script: {
        runtime: 'sandboxed QuickJS',
        create: 'Use I(parent,node) against the exact rootNodeId; I(null,...) is invalid.',
        repeat: 'Use arrays and loops.',
      },
      generation: {
        first: 'Create the first visible viewport in at most 32 I/K calls and 8 KiB.',
        second: 'Create every remaining region, then finish.',
        limit: 'Exactly two generation scripts.',
      },
      continuationStyle: { rule: 'Use only the values returned beside this contract.' },
      quality: {
        textDefaults: 'Generated text explicitly uses portable Inter, system-ui, sans-serif / 16 / 1.5.',
        contrast: 'Use AA text pairs from the returned palette.',
      },
      repair: 'After finish authorizes repairTargets, use one bounded QuickJS U(nodeId, patch) script.',
      node: {
        parents: 'Only frame and group may contain children.',
        container: 'Use valid width and height sizing.',
        text: 'Set explicit typography.',
        paint: 'Use solid paints.',
        icon: 'Use icon_font.',
        control: 'Use native controls.',
      },
      layoutRules: ['one root'],
    },
    rootNodeId: 'root',
    continuationStyle: {
      version: 'openpencil-continuation-style-v1',
      palette: { page: '#F4F0E8', ink: '#17191D' },
    },
    editorState: { activePageId: 'page-1' },
    styleGuideTags: { tags: ['editorial'] },
    document: {
      path: join(snapshotDir(), documentFilename),
      filename: documentFilename,
      mimeType: 'application/json',
      bytes: 4567,
      sha256: DOCUMENT_SHA,
    },
    sourceTool: 'openpencil_pipeline_begin',
    previewIntent: 'document',
    editable: true,
    autoOpenEditor: true,
    liveCanvas: true,
    published: false,
    next: 'Continue in a few large batches while the live canvas stays open.',
    ...overrides,
  }
}

function legacyBeginResult(version = 'openpencil-batch-v1', overrides = {}) {
  const current = beginResult()
  const {
    repair: _repair,
    quality: _quality,
    ...v6BuildContract
  } = current.buildContract
  if ([
    'openpencil-script-v8',
    'openpencil-script-v9',
    'openpencil-script-v10',
    'openpencil-script-v11',
  ].includes(version)) {
    return {
      ...current,
      buildContract: { ...current.buildContract, version },
      ...overrides,
    }
  }
  if (version === 'openpencil-script-v7') {
    return {
      ...current,
      buildContract: {
        ...v6BuildContract,
        version,
        repair: current.buildContract.repair,
      },
      ...overrides,
    }
  }
  if (version === 'openpencil-script-v6') {
    return {
      ...current,
      buildContract: {
        ...v6BuildContract,
        version,
        operations: 'Only after finish authorizes concrete repairTargets.',
      },
      ...overrides,
    }
  }

  const { rootNodeId: _rootNodeId, continuationStyle: _continuationStyle, ...legacyCurrent } = current
  const {
    generation: _generation,
    continuationStyle: _buildContinuationStyle,
    ...v1WithoutOperations
  } = v6BuildContract
  const v1BuildContract = {
    ...v1WithoutOperations,
    operations: 'Use exact ids for later edits.',
  }
  const firstBatch = {
    required: ['root plus coherent above-the-fold content'],
    forbidden: ['empty-shell-only canvas'],
  }
  const legacyBuildContract = version === 'openpencil-batch-v1'
    ? { ...v1BuildContract, version }
    : version === 'openpencil-batch-v5'
      ? { ...v1BuildContract, version, firstBatch, continuationStyle: { required: 'reuse style' } }
      : { ...v1BuildContract, version, firstBatch }
  return {
    ...legacyCurrent,
    buildContract: legacyBuildContract,
    ...overrides,
  }
}

function inspectionResult(overrides = {}) {
  return {
    draftId: DRAFT_ID,
    kind: 'screenshot',
    version: 2,
    screenshot: {
      path: join(stateRoot(), 'design-draft-inspections', `${IMAGE_SHA}.png`),
      filename: `${IMAGE_SHA}.png`,
      mimeType: 'image/png',
      bytes: 1234,
      sha256: IMAGE_SHA,
      width: 390,
      height: 844,
    },
    next: 'The exact user preview is attached; no model image inspection is required.',
    ...overrides,
  }
}

function batchResult(overrides = {}) {
  return {
    draftId: DRAFT_ID,
    version: 2,
    changed: true,
    generationScriptCount: 1,
    generationScriptLimit: 2,
    rootNodeId: 'root',
    batch: { applied: true },
    canvas: { platform: 'mobile', width: 390 },
    canvasCheck: { valid: true, diagnostics: [] },
    screenshot: inspectionResult().screenshot,
    diagnostics: [],
    canContinue: true,
    next: 'Continue with the final generation script.',
    ...overrides,
  }
}

function visualReviewResult(overrides = {}) {
  const { reason: _reason, ...base } = pendingPipelineResult()
  return {
    ...base,
    stage: 'needs_visual_review',
    digest: ['page 390x? with 3 visible regions (top to bottom):', 'footer: present as the final region.'],
    checklist: ['Vertical rhythm: section spacing is even.', 'Footer: the page ends with the complete footer band.'],
    next: 'Review the digest against every checklist item, then call finish once more.',
    ...overrides,
  }
}

function pendingPipelineResult(overrides = {}) {
  return {
    draftId: DRAFT_ID,
    path: '/tmp/generated.op',
    published: false,
    stage: 'needs_preview',
    reason: 'preview_unavailable',
    version: 2,
    finalization: {
      version: 2,
      changed: true,
      documentChanged: true,
      reused: true,
      documentSha256: DOCUMENT_SHA,
      note: 'Finalization is checkpointed; this is informational only.',
    },
    screenshot: inspectionResult().screenshot,
    diagnostics: [],
    canContinue: true,
    next: 'Call finish once more to publish this exact preview.',
    ...overrides,
  }
}

function historicalEvent(callId, result, content, toolName = result.sourceTool) {
  return {
    type: 'tool/ptc-dispatch',
    data: {
      rootCallId: 'outer',
      parentCallId: 'outer',
      subCallId: callId,
      name: toolName,
      arguments: { path: '/tmp/design.op' },
      isError: false,
      content: content ?? [{ type: 'text', text: JSON.stringify(result) }],
    },
  }
}

async function createHarness({
  sessions = new Map(),
  now,
  ttlMs,
  maxEntries,
  maxRecordBytes,
  maxBytes,
  trustedHosts,
  remoteAddress,
} = {}) {
  const root = await mkdtemp(join(tmpdir(), 'dsh-openpencil-presentation-'))
  const previousDshHome = process.env.DSH_HOME
  process.env.DSH_HOME = join(root, 'dsh-home')
  const render = new RenderAccessController(randomBytes(32))
  const detachRender = render.attachRoute()
  const sessionViews = new WeakMap()
  const editorCalls = []
  const hydration = new PresentationHydrationController({
    sessions: {
      // DSH 0.1.5 replaced `Session.events` with `snapshotEvents()`. The
      // adapter is memoised per stored session because the production history
      // index is keyed on the SESSION OBJECT's identity: handing back a fresh
      // wrapper on every lookup would miss that cache every time and rescan
      // the whole log, which is precisely what the incremental-index test
      // below measures.
      get(id) {
        const stored = sessions.get(String(id))
        if (stored === undefined) return undefined
        let view = sessionViews.get(stored)
        if (view === undefined) {
          view = { snapshotEvents: () => stored.events }
          sessionViews.set(stored, view)
        }
        return view
      },
    },
    render,
    viewer: {
      viewerGrant: {
        sdkUrl: '/_dsh/dsh-openpencil/viewer-assets/revision/sdk.js',
        wasmUrl: '/_dsh/dsh-openpencil/viewer-assets/revision/op_web_sdk_bg.wasm',
        canvasKitBaseUrl: '/_dsh/dsh-openpencil/viewer-assets/revision/canvaskit/',
      },
    },
    editor: {
      grantFor(sourcePath, sourceSha256) {
        editorCalls.push({ sourcePath, sourceSha256 })
        return {
          enabled: true,
          launchUrl: '/_dsh/dsh-openpencil/editor/live/launch',
          refreshUrl: '/_dsh/dsh-openpencil/editor/live/refresh',
        }
      },
      grantForDraft(draftId, ownerSessionId) {
        editorCalls.push({ draftId, ownerSessionId })
        return {
          enabled: true,
          launchUrl: '/_dsh/dsh-openpencil/editor/draft/launch',
          refreshUrl: '/_dsh/dsh-openpencil/editor/draft/refresh',
        }
      },
    },
    trustedHosts,
  }, { now, ttlMs, maxEntries, maxRecordBytes, maxBytes })

  const server = createServer((req, res) => {
    if (remoteAddress !== undefined) {
      Object.defineProperty(req.socket, 'remoteAddress', { configurable: true, value: remoteAddress })
    }
    void hydration.handle(req, res)
  })
  await new Promise((resolve, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', resolve)
  })
  const address = server.address()
  assert.equal(typeof address, 'object')
  const origin = `http://127.0.0.1:${address.port}`
  const request = (body, init = {}) => fetch(`${origin}${init.path ?? PRESENTATION_HYDRATION_ROUTE}`, {
    method: init.method ?? 'POST',
    body: body === undefined ? undefined : JSON.stringify(body),
    headers: {
      ...(init.origin === false ? {} : { origin: init.origin ?? origin }),
      ...(body === undefined ? {} : { 'content-type': init.contentType ?? 'application/json' }),
      ...(init.headers ?? {}),
    },
  })
  const authorityRequest = (body, authority, init = {}) => new Promise((resolve, reject) => {
    const encoded = JSON.stringify(body)
    const headers = {
      host: authority,
      ...(init.origin === false ? {} : { origin: init.origin ?? `http://${authority}` }),
      'content-type': 'application/json',
      'content-length': String(Buffer.byteLength(encoded)),
      ...(init.headers ?? {}),
    }
    const outgoing = httpRequest({
      hostname: '127.0.0.1',
      port: address.port,
      path: PRESENTATION_HYDRATION_ROUTE,
      method: 'POST',
      headers,
    }, response => {
      const chunks = []
      response.on('data', chunk => chunks.push(Buffer.from(chunk)))
      response.on('end', () => resolve({
        status: response.statusCode,
        body: Buffer.concat(chunks).toString('utf8'),
      }))
    })
    outgoing.once('error', reject)
    outgoing.end(encoded)
  })

  return {
    hydration,
    editorCalls,
    request,
    authorityRequest,
    result: renderResult(),
    origin,
    async cleanup() {
      detachRender()
      await closeServer(server)
      if (previousDshHome === undefined) delete process.env.DSH_HOME
      else process.env.DSH_HOME = previousDshHome
      await rm(root, { recursive: true, force: true })
    },
  }
}

function observe(hydration, sessionId, callId, result, toolName = result.sourceTool) {
  hydration.observeToolResult({
    name: toolName,
    callId,
    parent: Symbol('run-code'),
    agent: { id: sessionId, session: { id: sessionId } },
  }, { isError: false, value: result, content: [] })
}

test('nested openpencil_new keeps explicit historical editing but reserves auto-open for the live settlement', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = newResult()
    const callId = 'outer:code:new'
    observe(harness.hydration, 'session-new-live', callId, result)
    sessions.set('session-new-live', { events: [historicalEvent(callId, result)] })
    const live = await harness.request({
      sessionId: 'session-new-live',
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(live.status, 200)
    const liveEnvelope = (await live.json()).$dshOpenPencil
    assert.equal('image' in liveEnvelope, false)
    assert.equal(liveEnvelope.document.path, '/tmp/generated.op')
    assert.match(liveEnvelope.document.url, /^\/_dsh\/dsh-openpencil\/render\//)
    assert.equal(liveEnvelope.editor.launchUrl, '/_dsh/dsh-openpencil/editor/live/launch')
    assert.equal(liveEnvelope.autoOpenEditor, true)
    assert.deepEqual(harness.editorCalls, [{
      sourcePath: '/tmp/generated.op',
      sourceSha256: DOCUMENT_SHA,
    }])

    sessions.set('session-new-history', { events: [historicalEvent(callId, result)] })
    const historical = await harness.request({
      sessionId: 'session-new-history',
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(historical.status, 200)
    const historicalEnvelope = (await historical.json()).$dshOpenPencil
    assert.equal('image' in historicalEnvelope, false)
    assert.equal(historicalEnvelope.document.path, '/tmp/generated.op')
    assert.equal(historicalEnvelope.editor.launchUrl, '/_dsh/dsh-openpencil/editor/live/launch')
    assert.equal('autoOpenEditor' in historicalEnvelope, false)
    assert.equal(harness.editorCalls.length, 2)
  } finally {
    await harness.cleanup()
  }
})

test('nested pipeline_finish restores the same live document grant and idle auto-open capability', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = pipelineResult()
    const callId = 'outer:code:pipeline-finish'
    observe(harness.hydration, 'session-pipeline-live', callId, result)
    sessions.set('session-pipeline-live', { events: [historicalEvent(callId, result)] })
    const response = await harness.request({
      sessionId: 'session-pipeline-live',
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(response.status, 200)
    const envelope = (await response.json()).$dshOpenPencil
    assert.match(envelope.image.previewUrl, /^\/_dsh\/dsh-openpencil\/render\//)
    assert.equal(envelope.frames.length, 1)
    assert.equal(envelope.document.path, '/tmp/generated.op')
    assert.equal(envelope.editor.launchUrl, '/_dsh/dsh-openpencil/editor/live/launch')
    assert.equal(envelope.autoOpenEditor, true)
    assert.equal(envelope.draftId, DRAFT_ID)
    assert.equal(envelope.liveDraft, false)
    assert.deepEqual(harness.editorCalls, [{
      sourcePath: '/tmp/generated.op',
      sourceSha256: DOCUMENT_SHA,
    }])

    sessions.set('session-pipeline-history', { events: [historicalEvent(callId, result)] })
    const historical = await harness.request({
      sessionId: 'session-pipeline-history',
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(historical.status, 200)
    const historicalEnvelope = (await historical.json()).$dshOpenPencil
    assert.match(historicalEnvelope.image.previewUrl, /^\/_dsh\/dsh-openpencil\/render\//)
    assert.equal(historicalEnvelope.editor.launchUrl, '/_dsh/dsh-openpencil/editor/live/launch')
    assert.equal('autoOpenEditor' in historicalEnvelope, false)
    assert.equal(historicalEnvelope.draftId, DRAFT_ID)
    assert.equal(historicalEnvelope.liveDraft, false)
  } finally {
    await harness.cleanup()
  }
})

test('nested v12 pipeline_begin restores an owner-bound live draft grant and never revives it from history alone', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = beginResult()
    const callId = 'outer:code:pipeline-begin'
    observe(harness.hydration, 'session-pipeline-begin', callId, result)
    sessions.set('session-pipeline-begin', {
      events: [historicalEvent(callId, result)],
    })
    const response = await harness.request({
      sessionId: 'session-pipeline-begin',
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(response.status, 200)
    const envelope = (await response.json()).$dshOpenPencil
    assert.equal('image' in envelope, false)
    assert.equal(envelope.document.path, '/tmp/ecommerce-home.op')
    assert.equal(envelope.editor.launchUrl, '/_dsh/dsh-openpencil/editor/draft/launch')
    assert.equal(envelope.autoOpenEditor, true)
    assert.deepEqual(harness.editorCalls, [{
      draftId: DRAFT_ID,
      ownerSessionId: 'session-pipeline-begin',
    }])

    sessions.set('session-pipeline-begin-history', {
      events: [historicalEvent(callId, result)],
    })
    const historical = await harness.request({
      sessionId: 'session-pipeline-begin-history',
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(historical.status, 404, 'an unpublished daemon must not be revived from transcript text')
  } finally {
    await harness.cleanup()
  }
})

test('a live v1 pipeline_begin remains explicitly editable but cannot recover v2 auto-open authority', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = legacyBeginResult()
    const callId = 'outer:code:pipeline-begin-v1'
    observe(harness.hydration, 'session-pipeline-begin-v1', callId, result)
    sessions.set('session-pipeline-begin-v1', {
      events: [historicalEvent(callId, result)],
    })
    const response = await harness.request({
      sessionId: 'session-pipeline-begin-v1',
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(response.status, 200)
    const envelope = (await response.json()).$dshOpenPencil
    assert.equal(envelope.editor.launchUrl, '/_dsh/dsh-openpencil/editor/draft/launch')
    assert.equal('autoOpenEditor' in envelope, false)

    sessions.set('session-pipeline-begin-v1-history', {
      events: [historicalEvent(callId, result)],
    })
    const historical = await harness.request({
      sessionId: 'session-pipeline-begin-v1-history',
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(historical.status, 404)
  } finally {
    await harness.cleanup()
  }
})

test('nested pipeline screenshot hydrates a visible stage image without exposing the private cache path', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = inspectionResult()
    const callId = 'outer:code:pipeline-inspect'
    observe(harness.hydration, 'session-pipeline-inspect', callId, result, 'openpencil_pipeline_inspect')
    sessions.set('session-pipeline-inspect', {
      events: [historicalEvent(callId, result, undefined, 'openpencil_pipeline_inspect')],
    })
    const response = await harness.request({
      sessionId: 'session-pipeline-inspect',
      callId,
      // Legacy field name now carries the canonical artifact fingerprint for
      // either a document or a stage image.
      documentSha256: IMAGE_SHA,
    })
    assert.equal(response.status, 200)
    const envelope = (await response.json()).$dshOpenPencil
    assert.equal(envelope.schemaVersion, 2)
    assert.equal(envelope.image.path, `render-stage-${IMAGE_SHA}.png`)
    assert.match(envelope.image.previewUrl, /^\/_dsh\/dsh-openpencil\/render\//)
    assert.equal(JSON.stringify(envelope).includes('design-draft-inspections'), false)
    assert.equal('document' in envelope, false)
    assert.equal('editor' in envelope, false)
  } finally {
    await harness.cleanup()
  }
})

test('nested pipeline batch hydrates its committed PNG and rejects an artifact-free batch', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = batchResult()
    const callId = 'outer:code:pipeline-batch'
    observe(harness.hydration, 'session-pipeline-batch', callId, result, 'openpencil_pipeline_batch')
    sessions.set('session-pipeline-batch', {
      events: [historicalEvent(callId, result, undefined, 'openpencil_pipeline_batch')],
    })
    const response = await harness.request({
      sessionId: 'session-pipeline-batch',
      callId,
      documentSha256: IMAGE_SHA,
    })
    assert.equal(response.status, 200)
    const envelope = (await response.json()).$dshOpenPencil
    assert.equal(envelope.image.path, `render-stage-${IMAGE_SHA}.png`)
    assert.equal('document' in envelope, false)
    assert.equal('editor' in envelope, false)

    const noArtifact = batchResult({ screenshot: undefined, previewUnavailable: true })
    const noArtifactCallId = 'outer:code:pipeline-batch-no-artifact'
    observe(harness.hydration, 'session-pipeline-batch', noArtifactCallId, noArtifact, 'openpencil_pipeline_batch')
    sessions.get('session-pipeline-batch').events.push(
      historicalEvent(noArtifactCallId, noArtifact, undefined, 'openpencil_pipeline_batch'),
    )
    const absent = await harness.request({
      sessionId: 'session-pipeline-batch',
      callId: noArtifactCallId,
      documentSha256: IMAGE_SHA,
    })
    assert.equal(absent.status, 404)
  } finally {
    await harness.cleanup()
  }
})

test('nested needs_visual_review hydrates its exact checkpointed preview', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = visualReviewResult()
    const callId = 'outer:code:pipeline-finish-review'
    observe(harness.hydration, 'session-pipeline-finish-review', callId, result, 'openpencil_pipeline_finish')
    sessions.set('session-pipeline-finish-review', {
      events: [historicalEvent(callId, result, undefined, 'openpencil_pipeline_finish')],
    })
    const response = await harness.request({
      sessionId: 'session-pipeline-finish-review',
      callId,
      documentSha256: IMAGE_SHA,
    })
    assert.equal(response.status, 200)
    const envelope = (await response.json()).$dshOpenPencil
    assert.equal(envelope.schemaVersion, 2)
    assert.equal(envelope.image.path, `render-stage-${IMAGE_SHA}.png`)
    assert.match(envelope.image.previewUrl, /^\/_dsh\/dsh-openpencil\/render\//)
  } finally {
    await harness.cleanup()
  }
})

test('needs_visual_review with an oversized digest is not hydratable', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = visualReviewResult({ digest: Array.from({ length: 41 }, () => 'line') })
    const callId = 'outer:code:pipeline-finish-review-bad'
    observe(harness.hydration, 'session-review-bad', callId, result, 'openpencil_pipeline_finish')
    sessions.set('session-review-bad', {
      events: [historicalEvent(callId, result, undefined, 'openpencil_pipeline_finish')],
    })
    const response = await harness.request({
      sessionId: 'session-review-bad',
      callId,
      documentSha256: IMAGE_SHA,
    })
    assert.notEqual(response.status, 200)
  } finally {
    await harness.cleanup()
  }
})

test('nested pending pipeline_finish hydrates its exact checkpointed preview', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = pendingPipelineResult()
    const callId = 'outer:code:pipeline-finish-pending'
    observe(harness.hydration, 'session-pipeline-finish-pending', callId, result, 'openpencil_pipeline_finish')
    sessions.set('session-pipeline-finish-pending', {
      events: [historicalEvent(callId, result, undefined, 'openpencil_pipeline_finish')],
    })
    const response = await harness.request({
      sessionId: 'session-pipeline-finish-pending',
      callId,
      documentSha256: IMAGE_SHA,
    })
    assert.equal(response.status, 200)
    const envelope = (await response.json()).$dshOpenPencil
    assert.equal(envelope.schemaVersion, 2)
    assert.equal(envelope.image.path, `render-stage-${IMAGE_SHA}.png`)
    assert.match(envelope.image.previewUrl, /^\/_dsh\/dsh-openpencil\/render\//)
    assert.equal(JSON.stringify(envelope).includes('design-draft-inspections'), false)
    assert.equal('document' in envelope, false)
    assert.equal('editor' in envelope, false)
  } finally {
    await harness.cleanup()
  }
})

test('duplicate nested openpencil_new settlements fail closed', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const result = newResult()
    const callId = 'outer:code:new-duplicate'
    observe(harness.hydration, 'session-new-duplicate', callId, result)
    sessions.set('session-new-duplicate', {
      events: [historicalEvent(callId, result), historicalEvent(callId, result)],
    })
    const response = await harness.request({
      sessionId: 'session-new-duplicate',
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(response.status, 404)
    assert.deepEqual(harness.editorCalls, [])
  } finally {
    await harness.cleanup()
  }
})

test('live nested results hydrate only the presentation envelope and may restore editing', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions, trustedHosts: ['example.test'] })
  try {
    observe(harness.hydration, 'session-live', 'outer:code:1', harness.result)
    sessions.set('session-live', { events: [historicalEvent('outer:code:1', harness.result)] })
    const response = await harness.request({
      sessionId: 'session-live',
      callId: 'outer:code:1',
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(response.status, 200)
    assert.match(response.headers.get('cache-control') ?? '', /no-store/)
    assert.equal(response.headers.get('x-content-type-options'), 'nosniff')
    const body = await response.json()
    assert.deepEqual(Object.keys(body), ['$dshOpenPencil'])
    assert.equal(body.$dshOpenPencil.schemaVersion, 2)
    assert.match(body.$dshOpenPencil.image.previewUrl, /^\/_dsh\/dsh-openpencil\/render\//)
    assert.equal(body.$dshOpenPencil.frames.length, 1)
    assert.equal(body.$dshOpenPencil.document.path, '/tmp/design.op')
    assert.equal(body.$dshOpenPencil.viewer.sdkUrl, '/_dsh/dsh-openpencil/viewer-assets/revision/sdk.js')
    assert.equal(body.$dshOpenPencil.editor.launchUrl, '/_dsh/dsh-openpencil/editor/live/launch')
    assert.deepEqual(harness.editorCalls, [{ sourcePath: '/tmp/design.op', sourceSha256: DOCUMENT_SHA }])

    observe(harness.hydration, 'session-remote', 'outer:code:2', harness.result)
    sessions.set('session-remote', { events: [historicalEvent('outer:code:2', harness.result)] })
    const remoteHost = `example.test:${new URL(harness.origin).port}`
    const remote = await harness.authorityRequest({
      sessionId: 'session-remote',
      callId: 'outer:code:2',
      documentSha256: DOCUMENT_SHA,
    }, remoteHost)
    assert.equal(remote.status, 200)
    assert.equal('editor' in JSON.parse(remote.body).$dshOpenPencil, false, 'non-loopback origins must never receive editor grants')
    assert.equal(harness.editorCalls.length, 1)
  } finally {
    await harness.cleanup()
  }
})

test('editor authorization requires a nested observer and an exact durable result match', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const requestFor = (sessionId, callId) => ({ sessionId, callId, documentSha256: DOCUMENT_SHA })

    harness.hydration.observeToolResult({
      name: 'openpencil_render',
      callId: 'root-call',
      agent: { id: 'session-root', session: { id: 'session-root' } },
    }, { isError: false, value: harness.result, content: [] })
    sessions.set('session-root', { events: [historicalEvent('root-call', harness.result)] })
    const root = await harness.request(requestFor('session-root', 'root-call'))
    assert.equal(root.status, 200)
    assert.equal('editor' in (await root.json()).$dshOpenPencil, false, 'root tool observations must not authorize Code Mode hydration')

    const live = renderResult({ sourcePath: '/tmp/live.op' })
    observe(harness.hydration, 'session-mismatch', 'outer:code:mismatch', live)
    sessions.set('session-mismatch', { events: [historicalEvent('outer:code:mismatch', harness.result)] })
    const mismatch = await harness.request(requestFor('session-mismatch', 'outer:code:mismatch'))
    assert.equal(mismatch.status, 200)
    assert.equal('editor' in (await mismatch.json()).$dshOpenPencil, false, 'a non-matching durable result may preview but cannot receive editor capability')

    observe(harness.hydration, 'session-top-level-noise', 'outer:code:stable', harness.result)
    harness.hydration.observeToolResult({
      name: 'openpencil_render',
      callId: 'outer:code:stable',
      agent: { id: 'session-top-level-noise', session: { id: 'session-top-level-noise' } },
    }, { isError: false, value: renderResult({ sourcePath: '/tmp/top-level-noise.op' }), content: [] })
    sessions.set('session-top-level-noise', { events: [historicalEvent('outer:code:stable', harness.result)] })
    const unaffected = await harness.request(requestFor('session-top-level-noise', 'outer:code:stable'))
    assert.equal('editor' in (await unaffected.json()).$dshOpenPencil, true, 'a top-level result with the same call id must not evict nested authorization')

    observe(harness.hydration, 'session-unsettled', 'outer:code:unsettled', harness.result)
    assert.equal((await harness.request(requestFor('session-unsettled', 'outer:code:unsettled'))).status, 404, 'an unpersisted live observation is never a preview authority')
  } finally {
    await harness.cleanup()
  }
})

test('editor capability also requires the actual peer socket to be loopback', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions, remoteAddress: '192.0.2.44' })
  try {
    const sessionId = 'session-spoofed-loopback'
    const callId = 'outer:code:socket'
    observe(harness.hydration, sessionId, callId, harness.result)
    sessions.set(sessionId, { events: [historicalEvent(callId, harness.result)] })
    const response = await harness.request({ sessionId, callId, documentSha256: DOCUMENT_SHA })
    assert.equal(response.status, 200, 'a remote peer may still receive its authorized preview')
    assert.equal('editor' in (await response.json()).$dshOpenPencil, false, 'spoofed loopback Host and Origin must not mint editor capability')
    assert.deepEqual(harness.editorCalls, [])
  } finally {
    await harness.cleanup()
  }
})

test('IPv4-mapped 127/8 peers are treated as loopback for editor capability', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions, remoteAddress: '::ffff:127.23.4.5' })
  try {
    const sessionId = 'session-mapped-loopback'
    const callId = 'outer:code:mapped'
    observe(harness.hydration, sessionId, callId, harness.result)
    sessions.set(sessionId, { events: [historicalEvent(callId, harness.result)] })
    const response = await harness.request({ sessionId, callId, documentSha256: DOCUMENT_SHA })
    assert.equal(response.status, 200)
    assert.equal('editor' in (await response.json()).$dshOpenPencil, true)
  } finally {
    await harness.cleanup()
  }
})

test('durable Code Mode fallback is strict and never issues an editor grant', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    sessions.set('session-history', { events: [historicalEvent('outer:code:2', harness.result)] })
    const response = await harness.request({
      sessionId: 'session-history',
      callId: 'outer:code:2',
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(response.status, 200)
    const body = await response.json()
    assert.equal(body.$dshOpenPencil.image.width, 375)
    assert.equal('editor' in body.$dshOpenPencil, false)
    assert.deepEqual(harness.editorCalls, [])

    const offRoot = renderResult({ path: '/tmp/render-attacker.png' })
    sessions.set('session-off-root', { events: [historicalEvent('outer:code:3', offRoot)] })
    const rejected = await harness.request({
      sessionId: 'session-off-root',
      callId: 'outer:code:3',
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(rejected.status, 404)
    assert.equal(await rejected.text(), '')

    sessions.set('session-multi', {
      events: [historicalEvent('outer:code:4', harness.result, [
        { type: 'text', text: JSON.stringify(harness.result) },
        { type: 'text', text: '{}' },
      ])],
    })
    const multi = await harness.request({
      sessionId: 'session-multi',
      callId: 'outer:code:4',
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(multi.status, 404)

    sessions.set('session-duplicate', {
      events: [
        historicalEvent('outer:code:5', harness.result),
        historicalEvent('outer:code:5', harness.result),
      ],
    })
    const duplicate = await harness.request({
      sessionId: 'session-duplicate',
      callId: 'outer:code:5',
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(duplicate.status, 404, 'duplicate settlement identities must fail closed')

    sessions.set('session-huge', {
      events: [historicalEvent('outer:code:huge', harness.result, [
        { type: 'text', text: ' '.repeat(16 * 1024 * 1024 + 1) },
      ])],
    })
    const huge = await harness.request({
      sessionId: 'session-huge',
      callId: 'outer:code:huge',
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(huge.status, 404, 'oversized durable text must be rejected before JSON parsing')
  } finally {
    await harness.cleanup()
  }
})

test('durable settlement index consumes only appended events and tombstones later duplicates', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const callId = 'outer:code:indexed'
    const events = Array.from({ length: 128 }, (_unused, index) => ({
      type: 'unrelated',
      data: { index },
    }))
    events.push(historicalEvent(callId, harness.result))
    let indexedReads = 0
    sessions.set('session-indexed', {
      // Match the official Session contract: each append invalidates the
      // immutable snapshot, so the next read returns a different array.
      get events() {
        return new Proxy(Object.freeze([...events]), {
          get(target, property, receiver) {
            if (/^\d+$/u.test(String(property))) indexedReads += 1
            return Reflect.get(target, property, receiver)
          },
        })
      },
    })
    const request = { sessionId: 'session-indexed', callId, documentSha256: DOCUMENT_SHA }
    assert.equal((await harness.request(request)).status, 200)
    const readsAfterInitialIndex = indexedReads
    events.push({ type: 'unrelated', data: {} })
    assert.equal((await harness.request(request)).status, 200, 'an unrelated suffix must preserve the unique settlement')
    assert.ok(
      indexedReads - readsAfterInitialIndex <= 3,
      'a replaced immutable snapshot must scan only its appended suffix',
    )
    events.push(historicalEvent(callId, harness.result))
    assert.equal((await harness.request(request)).status, 404, 'an appended second settlement must invalidate the cached parse and fail closed')
  } finally {
    await harness.cleanup()
  }
})

test('hydration route rejects cross-origin, malformed, mismatched, and non-POST requests', async () => {
  const harness = await createHarness()
  try {
    observe(harness.hydration, 'session-safe', 'outer:code:5', harness.result)
    const request = {
      sessionId: 'session-safe',
      callId: 'outer:code:5',
      documentSha256: DOCUMENT_SHA,
    }
    assert.equal((await harness.request(request, { origin: 'https://attacker.example' })).status, 403)
    assert.equal((await harness.request(request, { headers: { 'sec-fetch-site': 'cross-site' } })).status, 403)
    const remoteHost = `example.test:${new URL(harness.origin).port}`
    assert.equal((await harness.authorityRequest(request, remoteHost)).status, 403, 'an unconfigured Host must fail the DNS-rebinding fence')
    assert.equal((await harness.request({ ...request, documentSha256: 'c'.repeat(64) })).status, 404)
    assert.equal((await harness.request({ ...request, path: '/tmp/design.op' })).status, 400)
    assert.equal((await harness.request(request, { contentType: 'text/plain' })).status, 415)
    const oversizedLength = await harness.authorityRequest(request, new URL(harness.origin).host, {
      headers: { 'content-length': '4097' },
    })
    assert.equal(oversizedLength.status, 413)
    const get = await harness.request(undefined, { method: 'GET' })
    assert.equal(get.status, 405)
    assert.equal(get.headers.get('allow'), 'POST')
    assert.equal((await harness.request(request, { path: `${PRESENTATION_HYDRATION_ROUTE}?x=1` })).status, 404)
  } finally {
    await harness.cleanup()
  }
})

test('live hydration records are TTL-bound and LRU-capped', async () => {
  let now = 1_000
  const sessions = new Map()
  const harness = await createHarness({ sessions, now: () => now, ttlMs: 10, maxEntries: 1 })
  try {
    observe(harness.hydration, 'session-a', 'outer:code:1', harness.result)
    observe(harness.hydration, 'session-b', 'outer:code:1', harness.result)
    sessions.set('session-a', { events: [historicalEvent('outer:code:1', harness.result)] })
    sessions.set('session-b', { events: [historicalEvent('outer:code:1', harness.result)] })
    const forId = sessionId => ({ sessionId, callId: 'outer:code:1', documentSha256: DOCUMENT_SHA })
    const evicted = await harness.request(forId('session-a'))
    assert.equal(evicted.status, 200, 'durable preview remains after the oldest authorization is evicted')
    assert.equal('editor' in (await evicted.json()).$dshOpenPencil, false)
    const retained = await harness.request(forId('session-b'))
    assert.equal(retained.status, 200)
    assert.equal('editor' in (await retained.json()).$dshOpenPencil, true)
    now += 11
    const expired = await harness.request(forId('session-b'))
    assert.equal(expired.status, 200, 'expired authorization must fall back to durable preview')
    assert.equal('editor' in (await expired.json()).$dshOpenPencil, false)
  } finally {
    await harness.cleanup()
  }
})

test('duplicate live settlement identities are tombstoned instead of overwritten', async () => {
  const sessions = new Map()
  const harness = await createHarness({ sessions })
  try {
    const sessionId = 'session-duplicate-live'
    const callId = 'outer:code:duplicate'
    sessions.set(sessionId, { events: [historicalEvent(callId, harness.result)] })
    observe(harness.hydration, sessionId, callId, harness.result)
    observe(harness.hydration, sessionId, callId, renderResult({
      sourcePath: '/tmp/different-source.op',
      editable: false,
    }))

    const response = await harness.request({
      sessionId,
      callId,
      documentSha256: DOCUMENT_SHA,
    })
    assert.equal(response.status, 404, 'a duplicate call id must block both live and historical fallback')
    assert.deepEqual(harness.editorCalls, [])
  } finally {
    await harness.cleanup()
  }
})

test('live cache rejects oversized records and evicts deterministically by aggregate bytes', async () => {
  const sessions = new Map()
  const harness = await createHarness({
    sessions,
    maxEntries: 10,
    maxRecordBytes: 1_024,
    maxBytes: 600,
  })
  try {
    const requestFor = sessionId => ({
      sessionId,
      callId: 'outer:code:bytes',
      documentSha256: DOCUMENT_SHA,
    })
    const resultA = renderResult({ sourcePath: '/tmp/a.op' })
    const resultB = renderResult({ sourcePath: '/tmp/b.op' })
    const resultC = renderResult({ sourcePath: '/tmp/c.op' })
    for (const [sessionId, result] of [['session-byte-a', resultA], ['session-byte-b', resultB]]) {
      observe(harness.hydration, sessionId, 'outer:code:bytes', result)
      sessions.set(sessionId, { events: [historicalEvent('outer:code:bytes', result)] })
    }
    const promoted = await harness.request(requestFor('session-byte-a'))
    assert.equal('editor' in (await promoted.json()).$dshOpenPencil, true, 'reading an authorization must promote it in the LRU')
    observe(harness.hydration, 'session-byte-c', 'outer:code:bytes', resultC)
    sessions.set('session-byte-c', { events: [historicalEvent('outer:code:bytes', resultC)] })
    const evicted = await harness.request(requestFor('session-byte-b'))
    assert.equal('editor' in (await evicted.json()).$dshOpenPencil, false, 'aggregate byte pressure must evict the least-recently-used authorization')
    assert.equal('editor' in (await (await harness.request(requestFor('session-byte-a'))).json()).$dshOpenPencil, true)
    assert.equal('editor' in (await (await harness.request(requestFor('session-byte-c'))).json()).$dshOpenPencil, true)

    const oversized = renderResult({ sourcePath: `/tmp/${'x'.repeat(2_000)}.op` })
    observe(harness.hydration, 'session-byte-oversized', 'outer:code:bytes', oversized)
    sessions.set('session-byte-oversized', { events: [historicalEvent('outer:code:bytes', oversized)] })
    assert.equal((await harness.request(requestFor('session-byte-oversized'))).status, 404, 'an individually oversized record must become a fail-closed tombstone')
  } finally {
    await harness.cleanup()
  }
})

test('strict parser refuses legacy path-only and presentation-bearing values', () => {
  const result = renderResult()
  assert.ok(parseHydratableRenderResult(result))
  const { sha256: _sha256, ...withoutSha } = result
  assert.equal(parseHydratableRenderResult(withoutSha), undefined)
  assert.equal(parseHydratableRenderResult({ ...result, $dshOpenPencil: {} }), undefined)
})

test('strict new-result parser binds the source and immutable document fingerprint', () => {
  const result = newResult()
  assert.ok(parseHydratableNewResult(result))
  assert.equal(parseHydratableNewResult({ ...result, sourceTool: 'openpencil_render' }), undefined)
  assert.equal(parseHydratableNewResult({ ...result, path: '/tmp/other.op' }), undefined)
  assert.equal(parseHydratableNewResult({ ...result, sha256: 'c'.repeat(64) }), undefined)
  assert.equal(parseHydratableNewResult({ ...result, autoOpenEditor: false }), undefined)
  assert.equal(parseHydratableNewResult({ ...result, image: {} }), undefined)
})

test('strict pipeline-result parser accepts only a publication or its exact checkpointed preview wait', () => {
  const result = pipelineResult()
  assert.ok(parseHydratablePipelineResult(result))
  assert.equal(parseHydratablePipelineResult({ ...result, draftId: 'short' }), undefined)
  assert.equal(parseHydratablePipelineResult({ ...result, sourceTool: 'openpencil_new' }), undefined)
  assert.equal(parseHydratablePipelineResult({ ...result, published: false }), undefined)
  assert.equal(parseHydratablePipelineResult({ ...result, result: { applied: true } }), undefined)
  assert.equal(parseHydratablePipelineResult({ ...result, autoOpenEditor: false }), undefined)

  const pending = pendingPipelineResult()
  assert.ok(parseHydratablePipelineResult(pending))
  assert.equal(parseHydratablePipelineResult({ ...pending, stage: 'needs_correction' }), undefined)
  assert.equal(parseHydratablePipelineResult({
    ...pending,
    finalization: { ...pending.finalization, result: { repairRecords: ['large', 'private', 'payload'] } },
  }), undefined)
  assert.equal(parseHydratablePipelineResult({
    ...pending,
    finalization: { ...pending.finalization, version: 3 },
  }), undefined)
  assert.equal(parseHydratablePipelineResult({
    ...pending,
    screenshot: { ...pending.screenshot, path: '/tmp/leaked.png' },
  }), undefined)
  assert.equal(parseHydratablePipelineResult({ ...pending, unexpected: true }), undefined)
})

test('strict batch parser requires one exact PNG artifact', () => {
  const result = batchResult()
  assert.ok(parseHydratableBatchResult(result))
  assert.equal(parseHydratableBatchResult({ ...result, screenshot: undefined, previewUnavailable: true }), undefined)
  assert.equal(parseHydratableBatchResult({
    ...result,
    screenshot: { ...result.screenshot, path: '/tmp/leaked.png' },
  }), undefined)
  assert.equal(parseHydratableBatchResult({ ...result, generationScriptLimit: 8 }), undefined)
  assert.equal(parseHydratableBatchResult({ ...result, unexpected: true }), undefined)
})

function agentRunResult(overrides = {}) {
  return {
    draftId: DRAFT_ID,
    version: 4,
    changed: true,
    agentRun: {
      toolCalls: 9,
      stopReason: 'end_turn',
      landedRoots: 1,
      finalize: { committedScreens: 1, unfilledScreens: 0, qualityChecks: 3, qualityRepairs: 2, qualityNotes: 0 },
    },
    screenshot: inspectionResult().screenshot,
    canContinue: true,
    next: 'Call finish exactly once now.',
    ...overrides,
  }
}

test('strict agent-run parser requires one landed loop plus its exact PNG artifact', () => {
  const result = agentRunResult()
  assert.ok(parseHydratableAgentRunResult(result))
  assert.equal(parseHydratableAgentRunResult({ ...result, changed: false }), undefined)
  assert.equal(parseHydratableAgentRunResult({ ...result, screenshot: undefined, previewUnavailable: true }), undefined)
  assert.equal(parseHydratableAgentRunResult({
    ...result,
    screenshot: { ...result.screenshot, path: '/tmp/leaked.png' },
  }), undefined)
  assert.equal(parseHydratableAgentRunResult({ ...result, unexpected: true }), undefined)
})

test('strict begin parser binds a compact unpublished live-draft contract', () => {
  const result = beginResult()
  const current = parseHydratableBeginResult(result)
  assert.ok(current)
  assert.equal(current.buildContract.version, 'openpencil-script-v12')
  assert.equal(current.rootNodeId, 'root')
  assert.deepEqual(current.continuationStyle, result.continuationStyle)
  for (const version of [
    'openpencil-batch-v1',
    'openpencil-batch-v2',
    'openpencil-batch-v3',
    'openpencil-batch-v4',
    'openpencil-batch-v5',
    'openpencil-script-v6',
    'openpencil-script-v7',
    'openpencil-script-v8',
    'openpencil-script-v9',
    'openpencil-script-v10',
    'openpencil-script-v11',
  ]) {
    assert.ok(parseHydratableBeginResult(legacyBeginResult(version)), `${version} remains replay-compatible`)
  }
  assert.equal(parseHydratableBeginResult({ ...result, draftId: 'short' }), undefined)
  assert.equal(parseHydratableBeginResult({ ...result, liveCanvas: false }), undefined)
  assert.equal(parseHydratableBeginResult({ ...result, platform: 'tablet' }), undefined)
  assert.equal(parseHydratableBeginResult({ ...result, canvas: { ...result.canvas, width: 390 } }), undefined)
  assert.equal(parseHydratableBeginResult({ ...result, rootNodeId: 'bad root id' }), undefined)
  assert.equal(parseHydratableBeginResult({ ...result, continuationStyle: [] }), undefined)
  const { generation: _generation, ...withoutGeneration } = result.buildContract
  assert.equal(parseHydratableBeginResult({ ...result, buildContract: withoutGeneration }), undefined)
  const { repair: _repair, ...withoutRepair } = result.buildContract
  assert.equal(parseHydratableBeginResult({ ...result, buildContract: withoutRepair }), undefined)
  const { quality: _quality, ...withoutQuality } = result.buildContract
  assert.equal(parseHydratableBeginResult({ ...result, buildContract: withoutQuality }), undefined)
  assert.equal(parseHydratableBeginResult({
    ...result,
    buildContract: {
      ...result.buildContract,
      quality: { ...result.buildContract.quality, unexpected: true },
    },
  }), undefined)
  assert.equal(parseHydratableBeginResult({
    ...result,
    buildContract: {
      ...result.buildContract,
      quality: { textDefaults: result.buildContract.quality.textDefaults },
    },
  }), undefined)
  assert.equal(parseHydratableBeginResult({
    ...result,
    buildContract: {
      ...withoutRepair,
      operations: 'Legacy DSL field is forbidden in the v8 contract.',
    },
  }), undefined)
  assert.equal(parseHydratableBeginResult({
    ...result,
    buildContract: { ...result.buildContract, unexpected: true },
  }), undefined)
  assert.equal(parseHydratableBeginResult({ ...result, buildContract: { version: 'unknown' } }), undefined)
  assert.equal(parseHydratableBeginResult({ ...result, designAgentPrompt: 'oversized duplicate' }), undefined)
})

test('strict inspection parser binds the private screenshot cache identity', () => {
  const result = inspectionResult()
  assert.ok(parseHydratableInspectionResult(result))
  assert.equal(parseHydratableInspectionResult({
    ...result,
    screenshot: { ...result.screenshot, path: '/tmp/leaked.png' },
  }), undefined)
  assert.equal(parseHydratableInspectionResult({
    ...result,
    screenshot: { ...result.screenshot, sha256: DOCUMENT_SHA },
  }), undefined)
  assert.equal(parseHydratableInspectionResult({ ...result, kind: 'layout' }), undefined)
})
