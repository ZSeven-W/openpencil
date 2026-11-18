import { test } from 'node:test'
import assert from 'node:assert/strict'
import { publishAndVerify, REGISTRY_TIMEOUT_MS } from '../scripts/registry-convergence.mjs'

const expected = { integrity: 'sha512-good', tag: '1.0.0' }
function run({ result = { status: 0 }, states = [expected], timeoutMs = 100, pollMs = 10 } = {}) {
  let time = 0, reads = 0, publishes = 0
  return {
    promise: publishAndVerify({
      specifier: 'example@1.0.0', version: '1.0.0', integrity: expected.integrity, tag: 'latest',
      publish: () => { publishes++; return result },
      readState: () => states[Math.min(reads++, states.length - 1)],
      timeoutMs, pollMs, now: () => time, wait: async ms => { time += ms }, log() {},
    }),
    publishes: () => publishes,
  }
}

test('a long CDN delay and later tag visibility complete after one publish', async () => {
  assert.equal(REGISTRY_TIMEOUT_MS, 30 * 60 * 1000)
  const r = run({ states: [{}, { integrity: expected.integrity }, expected], timeoutMs: REGISTRY_TIMEOUT_MS, pollMs: 7 * 60 * 1000 })
  await r.promise
  assert.equal(r.publishes(), 1)
})
test('the specific staged 409 waits for matching integrity and tag', async () => {
  const r = run({ result: { status: 1, stderr: 'E409 Cannot publish over previously staged version' }, states: [{}, expected] })
  await r.promise
  assert.equal(r.publishes(), 1)
})
test('a staged version never becoming visible is a failure', async () => {
  await assert.rejects(run({ result: { status: 1, stderr: '409 Cannot publish over previously staged version' }, states: [{}] }).promise, /timed out/)
})
test('different visible integrity fails instead of skipping a staged conflict', async () => {
  await assert.rejects(run({ states: [{ integrity: 'sha512-other' }] }).promise, /unexpected integrity/)
})
test('matching integrity without the expected tag is not completion', async () => {
  await assert.rejects(run({ states: [{ integrity: expected.integrity, tag: '0.9.0' }] }).promise, /timed out/)
})
test('auth errors and unrelated 409 conflicts fail immediately', async () => {
  for (const stderr of ['E401 auth failed', '409 version already exists']) {
    await assert.rejects(run({ result: { status: 1, stderr } }).promise, /npm publish failed/)
  }
})
