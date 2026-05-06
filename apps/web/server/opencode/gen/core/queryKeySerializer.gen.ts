// This 文件由 @hey-api/openapi-ts 自动生成

/**
 * JSON 友好的联合，镜像 Pinia Colada 可以散列的内容。
 */
export type JsonValue =
  | null
  | string
  | number
  | boolean
  | JsonValue[]
  | { [key: string]: JsonValue };

/**
 * Replacer 将非 JSON 值（bigint、Date 等）转换为安全替代值。
 */
export const queryKeyJsonReplacer = (_key: string, value: unknown) => {
  if (value === undefined || typeof value === 'function' || typeof value === 'symbol') {
    return undefined;
  }
  if (typeof value === 'bigint') {
    return value.toString();
  }
  if (value instanceof Date) {
    return value.toISOString();
  }
  return value;
};

/**
 * Safely 将一个值字符串化并将其解析回 JsonValue。
 */
export const stringifyToJsonValue = (input: unknown): JsonValue | undefined => {
  try {
    const json = JSON.stringify(input, queryKeyJsonReplacer);
    if (json === undefined) {
      return undefined;
    }
    return JSON.parse(json) as JsonValue;
  } catch {
    return undefined;
  }
};

/**
 * Detects 普通对象（包括具有空原型的对象）。
 */
const isPlainObject = (value: unknown): value is Record<string, unknown> => {
  if (value === null || typeof value !== 'object') {
    return false;
  }
  const prototype = Object.getPrototypeOf(value as object);
  return prototype === Object.prototype || prototype === null;
};

/**
 * Turns URLSearchParams 到排序的 JSON 对象中以获得确定性键。
 */
const serializeSearchParams = (params: URLSearchParams): JsonValue => {
  const entries = Array.from(params.entries()).sort(([a], [b]) => a.localeCompare(b));
  const result: Record<string, JsonValue> = {};

  for (const [key, value] of entries) {
    const existing = result[key];
    if (existing === undefined) {
      result[key] = value;
      continue;
    }

    if (Array.isArray(existing)) {
      (existing as string[]).push(value);
    } else {
      result[key] = [existing, value];
    }
  }

  return result;
};

/**
 * Normalizes 将任何接受的值转换为 JSON 友好的查询键形状。
 */
export const serializeQueryKeyValue = (value: unknown): JsonValue | undefined => {
  if (value === null) {
    return null;
  }

  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    return value;
  }

  if (value === undefined || typeof value === 'function' || typeof value === 'symbol') {
    return undefined;
  }

  if (typeof value === 'bigint') {
    return value.toString();
  }

  if (value instanceof Date) {
    return value.toISOString();
  }

  if (Array.isArray(value)) {
    return stringifyToJsonValue(value);
  }

  if (typeof URLSearchParams !== 'undefined' && value instanceof URLSearchParams) {
    return serializeSearchParams(value);
  }

  if (isPlainObject(value)) {
    return stringifyToJsonValue(value);
  }

  return undefined;
};
