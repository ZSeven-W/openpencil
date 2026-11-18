import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

const RECOVERY_KEY = Buffer.alloc(32, 7)
const MAX_DOCUMENT_BYTES = 64 * 1024 * 1024

function sha256(value) {
  return createHash('sha256').update(value).digest('hex')
}

async function recoveryHarness(now = 1_800_000_000_000) {
  const root = await mkdtemp(join(tmpdir(), 'dsh-openpencil-recovery-'))
  const sourcePath = join(root, 'private-design-name.op')
  const sourceDocumentJson = JSON.stringify({ version: '1.0', children: [] })
  await writeFile(sourcePath, sourceDocumentJson)
  const module = await import(`../lib/editor-recovery.js?test=${Date.now()}-${Math.random()}`)
  const store = new module.EditorRecoveryStore(RECOVERY_KEY, join(root, 'cache'), () => now)
  return {
    ...module,
    store,
    root,
    sourcePath,
    sourceDocumentJson,
    sourceSha256: sha256(sourceDocumentJson),
    async cleanup() { await rm(root, { recursive: true, force: true }) },
  }
}

test('recovery store skips clean documents and persists an opaque dirty snapshot', async () => {
  const harness = await recoveryHarness()
  try {
    const clean = await harness.store.capture({
      sourcePath: harness.sourcePath,
      sourceSha256: harness.sourceSha256,
      sourceDocumentJson: `\n${harness.sourceDocumentJson}\n`,
      daemonDocument: { documentJson: harness.sourceDocumentJson, version: 3 },
      reason: 'plugin-dispose',
    })
    assert.equal(clean, undefined)

    const dirtyDocumentJson = JSON.stringify({ version: '1.0', children: [{ id: 'unsaved' }] })
    const recovery = await harness.store.capture({
      sourcePath: harness.sourcePath,
      sourceSha256: harness.sourceSha256,
      sourceDocumentJson: harness.sourceDocumentJson,
      daemonDocument: { documentJson: dirtyDocumentJson, version: 4 },
      reason: 'client-dispose',
    })
    assert.ok(recovery)
    assert.match(recovery.id, /^[A-Za-z0-9_-]{43}$/)
    assert.equal(recovery.sourceName, 'private-design-name.op')
    assert.equal(recovery.sourceChangedSinceCapture, false)
    assert.equal(recovery.cacheLabel, `dsh-openpencil/recovery/${recovery.id}.json`)

    const names = await readdir(join(harness.root, 'cache'))
    assert.deepEqual(names, [`${recovery.id}.json`])
    assert.equal(names[0].includes('private-design-name'), false, 'cache filename must not expose the source path')
    const record = JSON.parse(await readFile(join(harness.root, 'cache', names[0]), 'utf8'))
    assert.equal(record.documentJson, dirtyDocumentJson)
    assert.equal('daemonToken' in record, false)

    assert.deepEqual(await harness.store.read(harness.sourcePath, recovery.id), { documentJson: dirtyDocumentJson })
    assert.equal((await harness.store.find(harness.sourcePath, sha256('externally changed'))).sourceChangedSinceCapture, true)
    assert.equal(await harness.store.discard(harness.sourcePath, recovery.id), true)
    assert.equal(await harness.store.find(harness.sourcePath, harness.sourceSha256), undefined)
  } finally {
    await harness.cleanup()
  }
})

test('recovery store rejects tampering and expires snapshots after seven days', async () => {
  const now = 1_800_000_000_000
  const harness = await recoveryHarness(now)
  try {
    const recovery = await harness.store.capture({
      sourcePath: harness.sourcePath,
      sourceSha256: harness.sourceSha256,
      sourceDocumentJson: harness.sourceDocumentJson,
      daemonDocument: { documentJson: '{"version":"1.0","children":[{"id":"draft"}]}', version: 9 },
      reason: 'plugin-dispose',
    })
    assert.ok(recovery)
    const cachePath = join(harness.root, 'cache', `${recovery.id}.json`)
    const tampered = JSON.parse(await readFile(cachePath, 'utf8'))
    tampered.documentJson = '{"version":"1.0","children":[{"id":"tampered"}]}'
    await writeFile(cachePath, JSON.stringify(tampered))
    assert.equal(await harness.store.read(harness.sourcePath, recovery.id), undefined)

    const expired = await harness.store.capture({
      sourcePath: harness.sourcePath,
      sourceSha256: harness.sourceSha256,
      sourceDocumentJson: harness.sourceDocumentJson,
      daemonDocument: { documentJson: '{"version":"1.0","children":[{"id":"old"}]}', version: 2 },
      reason: 'plugin-dispose',
    })
    assert.ok(expired)
    const expiredPath = join(harness.root, 'cache', `${expired.id}.json`)
    const expiredRecord = JSON.parse(await readFile(expiredPath, 'utf8'))
    expiredRecord.capturedAt = now - 8 * 24 * 60 * 60 * 1000
    await writeFile(expiredPath, JSON.stringify(expiredRecord))
    assert.equal(await harness.store.find(harness.sourcePath, harness.sourceSha256), undefined)
    assert.equal((await readdir(join(harness.root, 'cache'))).includes(`${expired.id}.json`), false)
  } finally {
    await harness.cleanup()
  }
})

test('recovery store survives a controller restart with the stable render access key', async () => {
  const harness = await recoveryHarness()
  try {
    const recovery = await harness.store.capture({
      sourcePath: harness.sourcePath,
      sourceSha256: harness.sourceSha256,
      sourceDocumentJson: harness.sourceDocumentJson,
      daemonDocument: { documentJson: '{"version":"1.0","children":[{"id":"restart-safe"}]}', version: 5 },
      reason: 'plugin-dispose',
    })
    assert.ok(recovery)
    const restarted = new harness.EditorRecoveryStore(RECOVERY_KEY, join(harness.root, 'cache'), () => 1_800_000_000_100)
    assert.equal((await restarted.find(harness.sourcePath, harness.sourceSha256)).id, recovery.id)
  } finally {
    await harness.cleanup()
  }
})

test('recovery store keeps escape-heavy documents whose envelope exceeds the document limit', async () => {
  const harness = await recoveryHarness()
  try {
    // This is well below the host document limit, but its outer record is over
    // the former MAX_DOCUMENT_BYTES + 1 MiB read limit because quotes are
    // escaped a second time in the recovery envelope.
    const dirtyDocumentJson = JSON.stringify({ payload: '"'.repeat(18 * 1024 * 1024) })
    assert.ok(Buffer.byteLength(dirtyDocumentJson) < MAX_DOCUMENT_BYTES)
    const recovery = await harness.store.capture({
      sourcePath: harness.sourcePath,
      sourceSha256: harness.sourceSha256,
      sourceDocumentJson: harness.sourceDocumentJson,
      daemonDocument: { documentJson: dirtyDocumentJson, version: 10 },
      reason: 'plugin-dispose',
    })
    assert.ok(recovery)
    const cachePath = join(harness.root, 'cache', `${recovery.id}.json`)
    assert.ok(Buffer.byteLength(await readFile(cachePath)) > MAX_DOCUMENT_BYTES + 1024 * 1024)
    assert.equal((await harness.store.read(harness.sourcePath, recovery.id)).documentJson, dirtyDocumentJson)
  } finally {
    await harness.cleanup()
  }
})

test('recovery store accepts the maximum host document and rejects one byte over it before writing', async () => {
  const harness = await recoveryHarness()
  try {
    const emptyDocumentBytes = Buffer.byteLength(JSON.stringify({ payload: '' }))
    const maximumDocumentJson = JSON.stringify({ payload: 'a'.repeat(MAX_DOCUMENT_BYTES - emptyDocumentBytes) })
    assert.equal(Buffer.byteLength(maximumDocumentJson), MAX_DOCUMENT_BYTES)
    const recovery = await harness.store.capture({
      sourcePath: harness.sourcePath,
      sourceSha256: harness.sourceSha256,
      sourceDocumentJson: harness.sourceDocumentJson,
      daemonDocument: { documentJson: maximumDocumentJson, version: 11 },
      reason: 'client-dispose',
    })
    assert.ok(recovery)
    assert.equal(Buffer.byteLength((await harness.store.read(harness.sourcePath, recovery.id)).documentJson), MAX_DOCUMENT_BYTES)
    await harness.store.discard(harness.sourcePath, recovery.id)

    const oversizedDocumentJson = JSON.stringify({ payload: 'a'.repeat(MAX_DOCUMENT_BYTES - emptyDocumentBytes + 1) })
    assert.equal(Buffer.byteLength(oversizedDocumentJson), MAX_DOCUMENT_BYTES + 1)
    await assert.rejects(
      harness.store.capture({
        sourcePath: harness.sourcePath,
        sourceSha256: harness.sourceSha256,
        sourceDocumentJson: harness.sourceDocumentJson,
        daemonDocument: { documentJson: oversizedDocumentJson, version: 12 },
        reason: 'client-dispose',
      }),
      /OpenPencil recovery document size is invalid/,
    )
    assert.deepEqual(await readdir(join(harness.root, 'cache')), [])
  } finally {
    await harness.cleanup()
  }
})

test('recovery store rejects an oversized metadata envelope before creating a cache file', async () => {
  const harness = await recoveryHarness()
  try {
    const oversizedSourcePath = `/${'private-path-segment'.repeat(64 * 1024)}`
    await assert.rejects(
      harness.store.capture({
        sourcePath: oversizedSourcePath,
        sourceSha256: harness.sourceSha256,
        sourceDocumentJson: harness.sourceDocumentJson,
        daemonDocument: { documentJson: '{"version":"1.0","children":[{"id":"draft"}]}', version: 13 },
        reason: 'plugin-dispose',
      }),
      /OpenPencil recovery snapshot exceeds the cache size limit/,
    )
    await assert.rejects(readdir(join(harness.root, 'cache')), error => error?.code === 'ENOENT')
  } finally {
    await harness.cleanup()
  }
})

test('daemon recovery reads a bounded snapshot and restores against the current version', async () => {
  const harness = await recoveryHarness()
  try {
    const calls = []
    const token = 'test-daemon-token-that-is-not-a-secret'
    const fetcher = async (url, init = {}) => {
      calls.push({ url, init })
      if (init.method === 'POST') return Response.json({ ok: true, version: 13 })
      return Response.json({ document: { version: '1.0', children: [{ id: 'draft' }] }, version: 12 })
    }
    const current = await harness.readManagedDaemonDocument('http://127.0.0.1:43123', token, fetcher)
    assert.equal(current.version, 12)
    assert.equal(JSON.parse(current.documentJson).children[0].id, 'draft')
    const restored = await harness.restoreManagedDaemonDocument(
      'http://127.0.0.1:43123', token,
      { documentJson: '{"version":"1.0","children":[{"id":"recovered"}]}', version: current.version },
      fetcher,
    )
    assert.equal(restored, 13)
    assert.equal(calls.length, 2)
    assert.equal(calls[0].init.headers.authorization, undefined)
    assert.equal(calls[0].init.headers['x-openpencil-token'], undefined)
    assert.equal(calls[1].init.headers.authorization, undefined)
    assert.equal(calls[1].init.headers['x-openpencil-token'], undefined)
    assert.deepEqual(JSON.parse(calls[1].init.body), {
      document: { version: '1.0', children: [{ id: 'recovered' }] },
      sourceClientId: 'dsh-openpencil-recovery',
      baseVersion: 12,
    })
  } finally {
    await harness.cleanup()
  }
})
