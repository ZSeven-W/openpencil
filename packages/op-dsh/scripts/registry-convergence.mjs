import { setTimeout as delay } from 'node:timers/promises'

export const REGISTRY_TIMEOUT_MS = 30 * 60 * 1000

// A staged conflict is an acknowledgement to wait, never proof of publication.
// Only matching visible integrity AND the expected tag complete the release.
export async function publishAndVerify({
  specifier, version, integrity, tag, publish, readState,
  timeoutMs = REGISTRY_TIMEOUT_MS, pollMs = 10_000,
  now = Date.now, wait = delay, log = line => process.stdout.write(line + '\n'),
}) {
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0 || !Number.isFinite(pollMs) || pollMs <= 0) {
    throw new Error('registry convergence timeout and poll interval must be positive')
  }
  const result = publish()
  if (result.status !== 0) {
    const output = String(result.stderr ?? '') + String(result.stdout ?? '')
    if (!/409/u.test(output) || !/Cannot publish over previously staged version/iu.test(output)) {
      throw new Error(`${specifier}: npm publish failed: ${output || result.error?.message || result.signal || result.status}`)
    }
    log(`${specifier} is already staged; waiting for registry confirmation`)
  }
  const deadline = now() + timeoutMs
  let state
  for (;;) {
    state = readState()
    if (state.integrity !== undefined && state.integrity !== integrity) {
      throw new Error(`${specifier} became visible with unexpected integrity ${state.integrity}`)
    }
    if (state.integrity === integrity && state.tag === version) {
      log(`${specifier} published with matching integrity and dist-tag ${tag}`)
      return
    }
    const remaining = deadline - now()
    if (remaining <= 0) break
    log(`${specifier}: waiting for registry integrity and ${tag}=${version}; tag=${JSON.stringify(state.tag)}`)
    await wait(Math.min(pollMs, remaining))
  }
  throw new Error(`${specifier}: registry confirmation timed out after ${timeoutMs}ms; expected integrity ${integrity} and ${tag}=${version}, last state=${JSON.stringify(state)}`)
}
