/**
 * Parse a managed editor daemon handshake without ever echoing the raw line.
 * The line contains a short-lived bearer token and must be treated as secret.
 */
export function parseManagedHandshake(line, platformId) {
  let handshake
  try {
    handshake = JSON.parse(line)
  } catch {
    throw new Error(`${platformId}: invalid managed handshake JSON`)
  }
  if (
    handshake === null
    || typeof handshake !== 'object'
    || Array.isArray(handshake)
    || handshake.ok !== true
    || !Number.isInteger(handshake.port)
    || typeof handshake.token !== 'string'
  ) {
    throw new Error(`${platformId}: incomplete managed handshake`)
  }
  return handshake
}
