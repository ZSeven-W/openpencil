/**
 * Recursively strip dangerous prototype-pollution keys from parsed JSON objects.
 * Call on any user-supplied or file-parsed JSON before using it in the application.
 */

// '__proto__' and 'prototype' enable classic prototype pollution.
// 'constructor' is stripped because obj.constructor.prototype can also be
// used to reach and mutate Object.prototype in certain exploit chains.
const DANGEROUS_KEYS = new Set(['__proto__', 'constructor', 'prototype'])

export function sanitizeObject<T>(obj: T, seen = new WeakSet<object>()): T {
  if (!obj || typeof obj !== 'object') return obj
  if (seen.has(obj as object)) return obj
  seen.add(obj as object)
  if (Array.isArray(obj)) return obj.map((item) => sanitizeObject(item, seen)) as T
  const clean: Record<string, unknown> = {}
  for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
    if (DANGEROUS_KEYS.has(k)) continue
    clean[k] = sanitizeObject(v, seen)
  }
  return clean as T
}
