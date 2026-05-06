/**
 * Recursively
 * 从解析的 JSON 对象中去除危险的原型污染键。在应用程序中使用任何用户提供的或文件解析的 JSON 之前，先对其执行 Call。
 */

// '__proto__' 和 'prototype' 会造成经典的原型污染。 'constructor' 被删除，因为
// obj.constructor.prototype 也可用于在某些漏洞利用链中访问和变异 Object.prototype。
const DANGEROUS_KEYS = new Set(['__proto__', 'constructor', 'prototype']);

export function sanitizeObject<T>(obj: T, seen = new WeakSet<object>()): T {
  if (!obj || typeof obj !== 'object') return obj;
  if (seen.has(obj as object)) return obj;
  seen.add(obj as object);
  if (Array.isArray(obj)) return obj.map((item) => sanitizeObject(item, seen)) as T;
  const clean: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
    if (DANGEROUS_KEYS.has(k)) continue;
    clean[k] = sanitizeObject(v, seen);
  }
  return clean as T;
}
