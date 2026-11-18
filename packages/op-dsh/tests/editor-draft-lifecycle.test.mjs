import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { chmod, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

const PNG_BASE64 = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII='

const fakeHostSource = `#!/usr/bin/env node
const fs = require('node:fs')
const http = require('node:http')
const logPath = process.env.FAKE_DRAFT_LOG
const fileIndex = process.argv.indexOf('--file')
const sourcePath = process.argv[fileIndex + 1]
const handshakeDelay = Number(process.env.FAKE_DRAFT_HANDSHAKE_DELAY_MS || 0)
const dropResetResponseOnce = process.env.FAKE_DRAFT_DROP_RESET_RESPONSE_ONCE === '1'
const mutateDuringScreenshotOnce = process.env.FAKE_DRAFT_MUTATE_DURING_SCREENSHOT_ONCE === '1'
let document = JSON.parse(fs.readFileSync(sourcePath, 'utf8'))
let version = 1
let resetConsumed = false
let resetResponseDropped = false
let screenshotMutationConsumed = false
let active = 0
let maxActive = 0
function log(value) { fs.appendFileSync(logPath, JSON.stringify(value) + '\\n') }
function controllerUsesNoCredentials(req) {
  return req.headers.authorization === undefined
    && req.headers['x-openpencil-token'] === undefined
}
const server = http.createServer((req, res) => {
  if (req.url === '/api/mcp/sync-reset' && req.method === 'POST') {
    if (!controllerUsesNoCredentials(req)) {
      res.statusCode = 401
      res.end(JSON.stringify({ ok: false }))
      return
    }
    if (resetConsumed) {
      log({ event: 'sync-reset', skipped: true, version, authorization: req.headers.authorization, token: req.headers['x-openpencil-token'] })
      res.setHeader('content-type', 'application/json')
      res.end(JSON.stringify({ ok: true, skipped: true, version }))
      return
    }
    document = JSON.parse(fs.readFileSync(sourcePath, 'utf8'))
    version += 1
    resetConsumed = true
    log({ event: 'sync-reset', skipped: false, version, authorization: req.headers.authorization, token: req.headers['x-openpencil-token'] })
    if (dropResetResponseOnce && !resetResponseDropped) {
      resetResponseDropped = true
      req.socket.destroy()
      return
    }
    res.setHeader('content-type', 'application/json')
    res.end(JSON.stringify({ ok: true, version }))
    return
  }
  if (req.url === '/api/mcp/document' && req.method === 'GET') {
    if (!controllerUsesNoCredentials(req)) {
      res.statusCode = 401
      res.end(JSON.stringify({ ok: false }))
      return
    }
    log({ event: 'document-get', authorization: req.headers.authorization, token: req.headers['x-openpencil-token'] })
    res.setHeader('content-type', 'application/json')
    res.end(JSON.stringify({ document, version }))
    return
  }
  if (req.url === '/api/mcp/document' && req.method === 'POST') {
    log({
      event: 'document-post',
      authorization: req.headers.authorization,
      token: req.headers['x-openpencil-token'],
    })
    res.statusCode = 500
    res.end(JSON.stringify({ ok: false, error: 'browser write reached daemon' }))
    return
  }
  if (req.url === '/api/mcp/version') {
    if (!controllerUsesNoCredentials(req)) {
      res.statusCode = 401
      res.end(JSON.stringify({ ok: false }))
      return
    }
    res.setHeader('content-type', 'application/json')
    res.end(JSON.stringify({ version }))
    return
  }
  if (req.url === '/mcp' && req.method === 'POST') {
    if (!controllerUsesNoCredentials(req)) {
      res.statusCode = 401
      res.end(JSON.stringify({ ok: false }))
      return
    }
    let body = ''
    req.on('data', chunk => { body += chunk })
    req.on('end', () => {
      const request = JSON.parse(body)
      const tool = request.params.name
      const args = request.params.arguments || {}
      active += 1
      maxActive = Math.max(maxActive, active)
      log({
        event: 'start',
        tool,
        active,
        maxActive,
        authorization: req.headers.authorization,
        token: req.headers['x-openpencil-token'],
      })
      const delay = Number(args.delayMs || 0)
      setTimeout(() => {
        let value
        if (tool === 'batch_design') {
          const children = Array.isArray(document.children) ? document.children : []
          document = { ...document, children: [...children, { id: 'n-' + version, name: String(args.script || '') }] }
          version += 1
          value = { applied: true }
        } else if (tool === 'apply_design_system') {
          document = { ...document, designSystem: args.name }
          version += 1
          value = { applied: true }
        } else if (tool === 'set_variables') {
          document = { ...document, variables: args.variables }
          version += 1
          value = { applied: true }
        } else if (tool === 'finalize_design' || tool === 'enrich_images') {
          value = { repairs: 0, advisories: [] }
        } else if (tool === 'get_screenshot') {
          if (mutateDuringScreenshotOnce && !screenshotMutationConsumed) {
            screenshotMutationConsumed = true
            document = { ...document, sameVersionScreenshotMutation: true }
          }
          active -= 1
          log({ event: 'end', tool, active, maxActive })
          res.setHeader('content-type', 'application/json')
          res.end(JSON.stringify({
            jsonrpc: '2.0', id: request.id,
            result: { content: [
              { type: 'image', data: '${PNG_BASE64}', mimeType: 'image/png' },
              { type: 'text', text: JSON.stringify({ nodeId: args.nodeId, image_base64: '${PNG_BASE64}', token: 'fake-draft-token-123456789' }) },
            ] },
          }))
          return
        } else if (tool === 'get_design_prompt' && args.mutateSameVersion === true) {
          document = { ...document, sameVersionMutation: String(args.label || 'changed') }
          value = { ok: true, mutatedWithoutVersion: true }
        } else {
          value = { ok: true, tool, token: 'fake-draft-token-123456789', sourcePath }
        }
        active -= 1
        log({ event: 'end', tool, active, maxActive })
        res.setHeader('content-type', 'application/json')
        res.end(JSON.stringify({
          jsonrpc: '2.0', id: request.id,
          result: { content: [{ type: 'text', text: JSON.stringify(value) }] },
        }))
      }, delay)
    })
    return
  }
  res.statusCode = 200
  res.setHeader('content-type', 'application/javascript')
  res.end('/* fake managed editor */')
})
let closing = false
function close() {
  if (closing) return
  closing = true
  server.close(() => process.exit(0))
  setTimeout(() => process.exit(0), 200).unref()
}
server.listen(0, '127.0.0.1', () => {
  const port = server.address().port
  log({ event: 'host', pid: process.pid, sourcePath })
  setTimeout(() => {
    process.stdout.write(JSON.stringify({ ok: true, port, token: 'fake-draft-token-123456789', version: 'test' }) + '\\n')
  }, handshakeDelay)
})
process.stdin.resume()
process.stdin.on('end', close)
process.on('SIGTERM', close)
`

function isAlive(pid) {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

async function waitFor(predicate, label) {
  const deadline = Date.now() + 3_000
  while (Date.now() < deadline) {
    const value = await predicate()
    if (value) return value
    await new Promise(resolve => setTimeout(resolve, 10))
  }
  throw new Error(`timed out waiting for ${label}`)
}

async function logEntries(logPath) {
  const text = await readFile(logPath, 'utf8').catch(() => '')
  return text.trim() === '' ? [] : text.trim().split('\n').map(line => JSON.parse(line))
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex')
}

async function createHarness(options = {}) {
  const root = await mkdtemp(join(tmpdir(), 'dsh-openpencil-draft-test-'))
  const binary = join(root, 'fake-host.cjs')
  const logPath = join(root, 'events.jsonl')
  await Promise.all([writeFile(binary, fakeHostSource), writeFile(logPath, '')])
  await chmod(binary, 0o755)
  const previousLog = process.env.FAKE_DRAFT_LOG
  const previousHandshakeDelay = process.env.FAKE_DRAFT_HANDSHAKE_DELAY_MS
  const previousDropResetResponse = process.env.FAKE_DRAFT_DROP_RESET_RESPONSE_ONCE
  const previousScreenshotMutation = process.env.FAKE_DRAFT_MUTATE_DURING_SCREENSHOT_ONCE
  process.env.FAKE_DRAFT_LOG = logPath
  process.env.FAKE_DRAFT_HANDSHAKE_DELAY_MS = String(options.handshakeDelayMs ?? 0)
  process.env.FAKE_DRAFT_DROP_RESET_RESPONSE_ONCE = options.dropResetResponseOnce === true ? '1' : '0'
  process.env.FAKE_DRAFT_MUTATE_DURING_SCREENSHOT_ONCE = options.mutateDuringScreenshotOnce === true ? '1' : '0'
  const { DesignDraftController } = await import(`../lib/design-draft-controller.js?draft=${Date.now()}-${Math.random()}`)
  const controller = new DesignDraftController({
    binary,
    webBundleDir: root,
    canvasKitDir: root,
    openPencilVersion: 'test',
    revision: '0'.repeat(40),
    source: 'override',
  }, options)
  return {
    root,
    logPath,
    controller,
    async host() {
      return waitFor(async () => (await logEntries(logPath)).find(entry => entry.event === 'host'), 'draft host')
    },
    async cleanup() {
      await controller.dispose()
      if (previousLog === undefined) delete process.env.FAKE_DRAFT_LOG
      else process.env.FAKE_DRAFT_LOG = previousLog
      if (previousHandshakeDelay === undefined) delete process.env.FAKE_DRAFT_HANDSHAKE_DELAY_MS
      else process.env.FAKE_DRAFT_HANDSHAKE_DELAY_MS = previousHandshakeDelay
      if (previousDropResetResponse === undefined) delete process.env.FAKE_DRAFT_DROP_RESET_RESPONSE_ONCE
      else process.env.FAKE_DRAFT_DROP_RESET_RESPONSE_ONCE = previousDropResetResponse
      if (previousScreenshotMutation === undefined) delete process.env.FAKE_DRAFT_MUTATE_DURING_SCREENSHOT_ONCE
      else process.env.FAKE_DRAFT_MUTATE_DURING_SCREENSHOT_ONCE = previousScreenshotMutation
      await rm(root, { recursive: true, force: true })
    },
  }
}

test('draft persists across calls, isolates owners, rejects paths, and serializes concurrent mutations', async () => {
  const harness = await createHarness()
  try {
    const begun = await harness.controller.begin({
      ownerSessionId: 'owner-a',
      target: { id: 'artifact-a', label: 'Login' },
      signal: new AbortController().signal,
    })
    assert.equal(JSON.stringify(begun).includes('dsh-openpencil-draft-'), false)
    await assert.rejects(
      harness.controller.call(begun.draftId, 'owner-b', 'get_design_prompt', {}),
      /different DSH session/,
    )
    await assert.rejects(
      harness.controller.call(begun.draftId, 'owner-a', 'get_design_prompt', { filePath: '/tmp/escape.op' }),
      /cannot include filePath/,
    )

    const first = harness.controller.call(begun.draftId, 'owner-a', 'batch_design', {
      script: 'first', delayMs: 80,
    })
    const second = harness.controller.call(begun.draftId, 'owner-a', 'batch_design', {
      script: 'second', delayMs: 20,
    })
    const [firstResult, secondResult] = await Promise.all([first, second])
    assert.equal(firstResult.version, 2)
    assert.equal(secondResult.version, 3)
    const snapshot = await harness.controller.snapshot(begun.draftId, 'owner-a')
    assert.deepEqual(JSON.parse(snapshot.documentJson).children.map(node => node.name), ['first', 'second'])
    const variableResult = await harness.controller.call(begun.draftId, 'owner-a', 'set_variables', {
      variables: { accent: { type: 'color', value: '#A9642F' } },
    })
    assert.equal(variableResult.version, 4)
    const tokenSnapshot = await harness.controller.snapshot(begun.draftId, 'owner-a')
    assert.equal(JSON.parse(tokenSnapshot.documentJson).variables.accent.value, '#A9642F')
    const entries = await logEntries(harness.logPath)
    assert.equal(Math.max(...entries.map(entry => entry.maxActive ?? 0)), 1)

    const context = await harness.controller.call(begun.draftId, 'owner-a', 'get_design_prompt', {})
    assert.equal(JSON.stringify(context).includes('fake-draft-token'), false)
    assert.equal(JSON.stringify(context).includes('dsh-openpencil-draft-'), false)
  } finally {
    await harness.cleanup()
  }
})

test('live launch consumes reset server-side so writes never race browser bootstrap and reopen stays skipped', async () => {
  const harness = await createHarness()
  try {
    const begun = await harness.controller.begin({
      ownerSessionId: 'owner-live',
      target: { id: 'live-target', label: 'Live canvas' },
      signal: new AbortController().signal,
    })
    await harness.controller.call(begun.draftId, 'owner-live', 'batch_design', { script: 'before attach' })
    const live = await harness.controller.prepareLiveLaunch(begun.draftId, 'owner-live')
    assert.equal(live.draftId, begun.draftId)
    assert.match(live.baseUrl, /^http:\/\/127\.0\.0\.1:\d+$/)
    assert.equal(JSON.parse(live.documentJson).children[0].name, 'before attach')
    assert.equal(JSON.parse(await readFile(live.sourcePath, 'utf8')).children[0].name, 'before attach')

    assert.notEqual(live.token, 'fake-draft-token-123456789', 'browser must receive an independent proxy capability')
    assert.equal((await fetch(`${live.baseUrl}/`)).status, 200, 'static Web shell remains tokenless')
    assert.equal((await fetch(`${live.baseUrl}/api/mcp/document`)).status, 401)
    assert.equal((await fetch(`${live.baseUrl}/api/mcp/document`, {
      headers: { authorization: `Bearer ${live.token}` },
    })).status, 401, 'Authorization is not a live browser capability')
    assert.equal((await fetch(`${live.baseUrl}/api/mcp/events`, {
      headers: { 'x-openpencil-token': live.token },
    })).status, 403, 'tokenless EventSource is omitted in favor of version/document polling')
    assert.equal((await fetch(`${live.baseUrl}/api/settings/providers`, {
      headers: { 'x-openpencil-token': live.token },
    })).status, 403, 'unknown privileged GET routes fail closed')
    const browserPush = await fetch(`${live.baseUrl}/api/mcp/document`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${live.token}`,
        'x-openpencil-token': live.token,
        'content-type': 'application/json',
      },
      body: JSON.stringify({ document: { version: '1.0.0', children: [] }, baseVersion: live.version }),
    })
    assert.equal(browserPush.status, 403)
    assert.equal((await browserPush.json()).code, 'read-only-live-canvas')
    assert.equal(
      (await logEntries(harness.logPath)).some(entry => entry.event === 'document-post'),
      false,
      'browser document POST must be rejected before the daemon',
    )

    await harness.controller.call(begun.draftId, 'owner-live', 'batch_design', { script: 'after attach' })
    const agentWrite = (await logEntries(harness.logPath)).findLast(entry => entry.event === 'start' && entry.tool === 'batch_design')
    assert.equal(agentWrite.authorization, undefined, 'Agent writes must not send an Authorization bearer')
    assert.equal(agentWrite.token, undefined, 'Agent writes must not send X-OpenPencil-Token')
    const browserRead = await fetch(`${live.baseUrl}/api/mcp/document`, {
      headers: { 'x-openpencil-token': live.token },
    })
    assert.equal(browserRead.status, 200)
    const upstreamRead = (await logEntries(harness.logPath)).findLast(entry => entry.event === 'document-get')
    assert.equal(upstreamRead.authorization, undefined, 'the read-only proxy must strip browser Authorization upstream')
    assert.equal(upstreamRead.token, undefined, 'the read-only proxy must strip its browser capability upstream')
    const browserReset = await fetch(`${live.baseUrl}/api/mcp/sync-reset`, {
      method: 'POST',
      headers: { 'x-openpencil-token': live.token, 'content-type': 'application/json' },
      body: '{}',
    })
    assert.equal(browserReset.status, 200)
    assert.equal((await browserReset.json()).skipped, true, 'browser bootstrap must only see the consumed reset')
    const nativeReset = (await logEntries(harness.logPath)).find(entry => entry.event === 'sync-reset' && entry.skipped === false)
    assert.equal(nativeReset.authorization, undefined, 'daemon reset must not send Authorization')
    assert.equal(nativeReset.token, undefined, 'daemon reset must not send X-OpenPencil-Token')
    assert.equal(harness.controller.markLiveReady(begun.draftId, 'owner-live', live.attachId), true)
    const snapshot = await harness.controller.snapshot(begun.draftId, 'owner-live')
    assert.deepEqual(JSON.parse(snapshot.documentJson).children.map(node => node.name), [
      'before attach',
      'after attach',
    ])
    assert.equal(harness.controller.markLiveReady(begun.draftId, 'owner-live', live.attachId), false)
    assert.equal(harness.controller.detachLive(begun.draftId, 'owner-live', live.attachId), true)

    const resetsBeforeReopen = (await logEntries(harness.logPath)).filter(entry => entry.event === 'sync-reset').length
    const reopened = await harness.controller.prepareLiveLaunch(begun.draftId, 'owner-live')
    assert.notEqual(reopened.attachId, live.attachId)
    assert.equal(
      (await logEntries(harness.logPath)).filter(entry => entry.event === 'sync-reset').length,
      resetsBeforeReopen,
      'reopening the same draft must not try to consume the one-shot reset again',
    )
    await harness.controller.call(begun.draftId, 'owner-live', 'batch_design', { script: 'after reopen' })
    const reopenedSnapshot = await harness.controller.snapshot(begun.draftId, 'owner-live')
    assert.deepEqual(JSON.parse(reopenedSnapshot.documentJson).children.map(node => node.name), [
      'before attach',
      'after attach',
      'after reopen',
    ])
    const finished = await harness.controller.finish(begun.draftId, 'owner-live', {
      requireCurrentScreenshot: false,
      publish: async snapshot => JSON.parse(snapshot.documentJson).children.map(node => node.name),
    })
    assert.deepEqual(finished.published, ['before attach', 'after attach', 'after reopen'])
  } finally {
    await harness.cleanup()
  }
})

test('live launch confirms an uncertain reset response by versioned daemon readback', async () => {
  const harness = await createHarness({ dropResetResponseOnce: true })
  try {
    const begun = await harness.controller.begin({
      ownerSessionId: 'owner-reset-loss',
      target: { id: 'reset-loss-target' },
      signal: new AbortController().signal,
    })
    await harness.controller.call(begun.draftId, 'owner-reset-loss', 'batch_design', { script: 'survives response loss' })
    const live = await harness.controller.prepareLiveLaunch(begun.draftId, 'owner-reset-loss')
    assert.equal(JSON.parse(live.documentJson).children[0].name, 'survives response loss')
    const browserReset = await fetch(`${live.baseUrl}/api/mcp/sync-reset`, {
      method: 'POST',
      headers: { 'x-openpencil-token': live.token, 'content-type': 'application/json' },
      body: '{}',
    })
    assert.equal((await browserReset.json()).skipped, true)
    await harness.controller.call(begun.draftId, 'owner-reset-loss', 'batch_design', { script: 'continues safely' })
    const snapshot = await harness.controller.snapshot(begun.draftId, 'owner-reset-loss')
    assert.deepEqual(JSON.parse(snapshot.documentJson).children.map(node => node.name), [
      'survives response loss',
      'continues safely',
    ])
  } finally {
    await harness.cleanup()
  }
})

test('finish requires a current screenshot, preserves a draft on publish failure, and closes before queued work after success', async () => {
  const harness = await createHarness()
  try {
    const begun = await harness.controller.begin({
      ownerSessionId: 'owner-publish', target: { id: 'publish-target' }, signal: new AbortController().signal,
    })
    await harness.controller.call(begun.draftId, 'owner-publish', 'batch_design', { script: 'initial' })
    await harness.controller.screenshot(begun.draftId, 'owner-publish')
    await harness.controller.finalize(begun.draftId, 'owner-publish')
    await assert.rejects(
      harness.controller.finish(begun.draftId, 'owner-publish', { publish: async () => ({ ok: true }) }),
      error => {
        assert.equal(error?.code, 'OPENPENCIL_DRAFT_PREVIEW_REQUIRED')
        assert.equal(error.currentVersion, error.finalizedVersion)
        assert.notEqual(error.screenshotFinalizedVersion, error.currentVersion)
        return true
      },
    )
    const screenshot = await harness.controller.screenshot(begun.draftId, 'owner-publish')
    assert.equal(screenshot.mimeType, 'image/png')
    assert.ok(screenshot.bytes.length > 32)
    assert.deepEqual(screenshot.metadata, { nodeId: 'root', token: '[redacted]' })

    await assert.rejects(
      harness.controller.finish(begun.draftId, 'owner-publish', {
        publish: async snapshot => {
          assert.equal(JSON.stringify(snapshot).includes('dsh-openpencil-draft-'), false)
          throw new Error('publish failed')
        },
      }),
      /publish failed/,
    )
    const retryMutation = await harness.controller.call(
      begun.draftId, 'owner-publish', 'batch_design', { script: 'after failed publish' },
    )
    assert.equal(retryMutation.changed, true)
    await harness.controller.finalize(begun.draftId, 'owner-publish')
    await harness.controller.screenshot(begun.draftId, 'owner-publish')

    let releasePublish
    const publishing = harness.controller.finish(begun.draftId, 'owner-publish', {
      publish: () => new Promise(resolve => { releasePublish = resolve }),
    })
    await waitFor(() => releasePublish, 'publish callback')
    const queued = harness.controller.call(begun.draftId, 'owner-publish', 'batch_design', { script: 'too late' })
    releasePublish({ stored: true })
    const finished = await publishing
    assert.deepEqual(finished.published, { stored: true })
    await assert.rejects(queued, /ended before its operation started/)
    await assert.rejects(
      harness.controller.snapshot(begun.draftId, 'owner-publish'),
      /has ended/,
    )
  } finally {
    await harness.cleanup()
  }
})

test('finish rejects a checkpoint when a queued mutation wins the serialized publication lock', async () => {
  const harness = await createHarness()
  try {
    const begun = await harness.controller.begin({
      ownerSessionId: 'owner-concurrent-finish',
      target: { id: 'concurrent-finish-target' },
      signal: new AbortController().signal,
    })
    await harness.controller.call(begun.draftId, 'owner-concurrent-finish', 'batch_design', { script: 'checkpoint' })
    await harness.controller.finalize(begun.draftId, 'owner-concurrent-finish')
    const checkpoint = await harness.controller.snapshot(begun.draftId, 'owner-concurrent-finish')
    const checkpointSha256 = sha256(checkpoint.documentJson)
    await harness.controller.screenshot(begun.draftId, 'owner-concurrent-finish')

    const mutation = harness.controller.call(
      begun.draftId,
      'owner-concurrent-finish',
      'batch_design',
      { script: 'wins before publish', delayMs: 50 },
    )
    let publishCalls = 0
    const publishing = harness.controller.finish(begun.draftId, 'owner-concurrent-finish', {
      expectedVersion: checkpoint.version,
      expectedDocumentSha256: checkpointSha256,
      publish: async () => {
        publishCalls += 1
        return { stored: true }
      },
    })
    await mutation
    await assert.rejects(publishing, error => {
      assert.equal(error?.code, 'OPENPENCIL_DRAFT_CHECKPOINT_DRIFT')
      assert.equal(error.expectedVersion, checkpoint.version)
      assert.equal(error.expectedDocumentSha256, checkpointSha256)
      assert.ok(error.currentVersion > checkpoint.version)
      return true
    })
    assert.equal(publishCalls, 0)

    await harness.controller.finalize(begun.draftId, 'owner-concurrent-finish')
    const current = await harness.controller.snapshot(begun.draftId, 'owner-concurrent-finish')
    await harness.controller.screenshot(begun.draftId, 'owner-concurrent-finish')
    const recovered = await harness.controller.finish(begun.draftId, 'owner-concurrent-finish', {
      expectedVersion: current.version,
      expectedDocumentSha256: sha256(current.documentJson),
      publish: async () => ({ stored: true }),
    })
    assert.deepEqual(recovered.published, { stored: true })
  } finally {
    await harness.cleanup()
  }
})

test('a child screenshot cannot satisfy the finalized root publication proof', async () => {
  const harness = await createHarness()
  try {
    const begun = await harness.controller.begin({
      ownerSessionId: 'owner-child-proof', target: { id: 'child-proof-target' }, signal: new AbortController().signal,
    })
    await harness.controller.call(begun.draftId, 'owner-child-proof', 'batch_design', { script: 'child proof' })
    await harness.controller.finalize(begun.draftId, 'owner-child-proof')
    const checkpoint = await harness.controller.snapshot(begun.draftId, 'owner-child-proof')
    const checkpointSha256 = sha256(checkpoint.documentJson)
    const child = await harness.controller.screenshot(begun.draftId, 'owner-child-proof', { nodeId: 'n-1' })
    assert.equal(child.documentSha256, checkpointSha256)
    assert.deepEqual(child.metadata, { nodeId: 'n-1', token: '[redacted]' })

    await assert.rejects(
      harness.controller.finish(begun.draftId, 'owner-child-proof', {
        expectedVersion: checkpoint.version,
        expectedDocumentSha256: checkpointSha256,
        publish: async () => ({ stored: true }),
      }),
      error => {
        assert.equal(error?.code, 'OPENPENCIL_DRAFT_PREVIEW_REQUIRED')
        assert.equal(error.screenshotVersion, undefined)
        assert.equal(error.screenshotFinalizedVersion, undefined)
        return true
      },
    )

    const root = await harness.controller.screenshot(begun.draftId, 'owner-child-proof')
    assert.equal(root.documentSha256, checkpointSha256)
    const finished = await harness.controller.finish(begun.draftId, 'owner-child-proof', {
      expectedVersion: checkpoint.version,
      expectedDocumentSha256: checkpointSha256,
      publish: async () => ({ stored: true }),
    })
    assert.deepEqual(finished.published, { stored: true })
  } finally {
    await harness.cleanup()
  }
})

test('same-version document byte drift invalidates both checkpoint and screenshot proof', async () => {
  const harness = await createHarness()
  try {
    const begun = await harness.controller.begin({
      ownerSessionId: 'owner-byte-drift', target: { id: 'byte-drift-target' }, signal: new AbortController().signal,
    })
    await harness.controller.call(begun.draftId, 'owner-byte-drift', 'batch_design', { script: 'same version baseline' })
    await harness.controller.finalize(begun.draftId, 'owner-byte-drift')
    const checkpoint = await harness.controller.snapshot(begun.draftId, 'owner-byte-drift')
    const checkpointSha256 = sha256(checkpoint.documentJson)
    await harness.controller.screenshot(begun.draftId, 'owner-byte-drift')

    await harness.controller.call(begun.draftId, 'owner-byte-drift', 'get_design_prompt', {
      mutateSameVersion: true,
      label: 'same-version-drift',
    })
    const drifted = await harness.controller.snapshot(begun.draftId, 'owner-byte-drift')
    assert.equal(drifted.version, checkpoint.version)
    assert.notEqual(sha256(drifted.documentJson), checkpointSha256)

    await assert.rejects(
      harness.controller.finish(begun.draftId, 'owner-byte-drift', {
        expectedVersion: checkpoint.version,
        expectedDocumentSha256: checkpointSha256,
        publish: async () => ({ stored: true }),
      }),
      error => {
        assert.equal(error?.code, 'OPENPENCIL_DRAFT_CHECKPOINT_DRIFT')
        assert.equal(error.currentVersion, checkpoint.version)
        assert.notEqual(error.currentDocumentSha256, checkpointSha256)
        return true
      },
    )

    await harness.controller.finalize(begun.draftId, 'owner-byte-drift')
    const recoveredCheckpoint = await harness.controller.snapshot(begun.draftId, 'owner-byte-drift')
    await harness.controller.screenshot(begun.draftId, 'owner-byte-drift')
    const recovered = await harness.controller.finish(begun.draftId, 'owner-byte-drift', {
      expectedVersion: recoveredCheckpoint.version,
      expectedDocumentSha256: sha256(recoveredCheckpoint.documentJson),
      publish: async () => ({ stored: true }),
    })
    assert.deepEqual(recovered.published, { stored: true })
  } finally {
    await harness.cleanup()
  }
})

test('screenshot rejects same-version document changes that happen while rendering', async () => {
  const harness = await createHarness({ mutateDuringScreenshotOnce: true })
  try {
    const begun = await harness.controller.begin({
      ownerSessionId: 'owner-screenshot-drift',
      target: { id: 'screenshot-drift-target' },
      signal: new AbortController().signal,
    })
    await harness.controller.call(begun.draftId, 'owner-screenshot-drift', 'batch_design', { script: 'render baseline' })
    await harness.controller.finalize(begun.draftId, 'owner-screenshot-drift')
    await assert.rejects(
      harness.controller.screenshot(begun.draftId, 'owner-screenshot-drift'),
      /changed while its exact root preview artifact was rendered/,
    )
    const screenshot = await harness.controller.screenshot(begun.draftId, 'owner-screenshot-drift')
    assert.match(screenshot.documentSha256, /^[a-f0-9]{64}$/)
  } finally {
    await harness.cleanup()
  }
})

test('owner abort and controller disposal stop daemons and cancellation retires uncertain mutations', async () => {
  const harness = await createHarness()
  try {
    const first = await harness.controller.begin({
      ownerSessionId: 'owner-abort', target: { id: 'abort-target' }, signal: new AbortController().signal,
    })
    const firstHost = await harness.host()
    assert.equal(await harness.controller.abortOwner('owner-abort'), 1)
    await waitFor(() => !isAlive(firstHost.pid), 'owner daemon exit')
    await assert.rejects(harness.controller.snapshot(first.draftId, 'owner-abort'), /has ended/)

    const second = await harness.controller.begin({
      ownerSessionId: 'owner-cancel', target: { id: 'cancel-target' }, signal: new AbortController().signal,
    })
    const abort = new AbortController()
    const mutation = harness.controller.call(second.draftId, 'owner-cancel', 'batch_design', {
      script: 'cancelled', delayMs: 1_000,
    }, { signal: abort.signal })
    await waitFor(async () => (await logEntries(harness.logPath)).filter(entry => entry.event === 'start').length >= 1, 'mutation start')
    abort.abort(new Error('cancel mutation'))
    await assert.rejects(mutation)
    await assert.rejects(harness.controller.snapshot(second.draftId, 'owner-cancel'), /has ended/)

    const third = await harness.controller.begin({
      ownerSessionId: 'owner-dispose', target: { id: 'dispose-target' }, signal: new AbortController().signal,
    })
    const firstDispose = harness.controller.dispose()
    assert.equal(harness.controller.dispose(), firstDispose)
    await firstDispose
    await assert.rejects(harness.controller.snapshot(third.draftId, 'owner-dispose'), /has ended/)
  } finally {
    await harness.cleanup()
  }
})

test('draft limits enforce one active draft per owner and the configured global cap', async () => {
  const harness = await createHarness({ maxDrafts: 2 })
  try {
    await harness.controller.begin({
      ownerSessionId: 'owner-one', target: { id: 'one' }, signal: new AbortController().signal,
    })
    await assert.rejects(harness.controller.begin({
      ownerSessionId: 'owner-one', target: { id: 'duplicate' }, signal: new AbortController().signal,
    }), /already has an active/)
    await harness.controller.begin({
      ownerSessionId: 'owner-two', target: { id: 'two' }, signal: new AbortController().signal,
    })
    await assert.rejects(harness.controller.begin({
      ownerSessionId: 'owner-three', target: { id: 'three' }, signal: new AbortController().signal,
    }), /capacity is full/)
  } finally {
    await harness.cleanup()
  }
})

test('owner disposal cancels and joins a draft that is still starting', async () => {
  const harness = await createHarness({ handshakeDelayMs: 1_000 })
  try {
    const beginning = harness.controller.begin({
      ownerSessionId: 'owner-starting', target: { id: 'starting' }, signal: new AbortController().signal,
    })
    const host = await harness.host()
    assert.equal(await harness.controller.abortOwner('owner-starting'), 1)
    await assert.rejects(beginning, /owner session ended during startup/)
    await waitFor(() => !isAlive(host.pid), 'starting daemon exit')
    const entries = await logEntries(harness.logPath)
    await assert.rejects(readFile(entries.find(entry => entry.event === 'host').sourcePath), error => error?.code === 'ENOENT')
  } finally {
    await harness.cleanup()
  }
})
