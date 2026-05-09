/**
 * Param coercion helpers for v1 element builders.
 *
 * Replaces the early-2026 "throw on invalid params" pattern with
 * fuzzy-coerce + warning. Reason: ab-v8 KPI showed ~14/95 obvious-T
 * fails were v1 schema throws on hallucinated enum values and missing
 * required arrays — the model meant the right component, just had a
 * bad value, and the whole tool call was dropped.
 *
 * After coercion the builder still emits a structurally-valid tree
 * with a placeholder. Warnings are pushed to a process-global sink so
 * orchestrators can surface them to the LLM (or just inspect them in
 * tests). v0 builders intentionally keep the throw behavior — only v1
 * goes through this helper.
 */

export interface CoerceWarning {
  builder: string;
  param: string;
  given: unknown;
  fallback: unknown;
  reason: string;
}

const warningSink: CoerceWarning[] = [];

function emit(w: CoerceWarning): void {
  warningSink.push(w);
  // eslint-disable-next-line no-console
  console.warn(
    `[coerce-params] ${w.builder}.${w.param}: ${w.reason} ` +
      `(given=${safeStringify(w.given)}, fallback=${safeStringify(w.fallback)})`,
  );
}

function safeStringify(v: unknown): string {
  try {
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}

/** Read warnings collected since the last clear. */
export function getCoerceWarnings(): CoerceWarning[] {
  return [...warningSink];
}

export function clearCoerceWarnings(): void {
  warningSink.length = 0;
}

/**
 * Coerce a value to an allowed enum. Returns `defaultValue` for
 * `undefined`/`null` (no warning — that's the legitimate "unset" case).
 * Returns `defaultValue` and emits a warning for any other invalid value.
 */
export function coerceEnum<T extends string>(
  value: unknown,
  validValues: readonly T[],
  defaultValue: T,
  builder: string,
  param: string,
): T {
  if (value === undefined || value === null) return defaultValue;
  if (typeof value === 'string' && (validValues as readonly string[]).includes(value)) {
    return value as T;
  }
  emit({
    builder,
    param,
    given: value,
    fallback: defaultValue,
    reason: `invalid enum value; valid: ${JSON.stringify(validValues)}`,
  });
  return defaultValue;
}

/**
 * Coerce a value that should be a non-empty array. Falls back to `fallback`
 * on `undefined`/`null`/non-array/empty-array, with a warning.
 */
export function coerceNonEmptyArray<T>(
  value: unknown,
  fallback: T[],
  builder: string,
  param: string,
): T[] {
  if (Array.isArray(value) && value.length > 0) {
    return value as T[];
  }
  emit({
    builder,
    param,
    given: value,
    fallback,
    reason: !Array.isArray(value) ? `expected non-empty array, got ${typeof value}` : 'array empty',
  });
  return fallback;
}

/**
 * Coerce a value that should be a non-empty array of non-empty strings.
 * Filters out non-string / empty entries; if nothing remains, returns
 * `fallback` and warns.
 */
export function coerceStringArray(
  value: unknown,
  fallback: string[],
  builder: string,
  param: string,
): string[] {
  if (Array.isArray(value)) {
    const filtered = value.filter((k): k is string => typeof k === 'string' && k.length > 0);
    if (filtered.length > 0) return filtered;
  }
  emit({
    builder,
    param,
    given: value,
    fallback,
    reason: !Array.isArray(value)
      ? `expected string[], got ${typeof value}`
      : 'no non-empty strings in array',
  });
  return fallback;
}

/**
 * Coerce a value that should be a non-empty array of finite numbers.
 * Filters out non-finite entries; if the array is empty after filtering
 * or wasn't an array, returns `fallback` and warns.
 */
export function coerceNumberArray(
  value: unknown,
  fallback: number[],
  builder: string,
  param: string,
): number[] {
  if (Array.isArray(value)) {
    const filtered = value.filter((v): v is number => typeof v === 'number' && Number.isFinite(v));
    if (filtered.length > 0) return filtered;
  }
  emit({
    builder,
    param,
    given: value,
    fallback,
    reason: !Array.isArray(value)
      ? `expected number[], got ${typeof value}`
      : 'no finite numbers in array',
  });
  return fallback;
}

/**
 * Coerce a value that should be a non-empty string. Falls back silently
 * for `undefined`/`null`/empty/whitespace-only; for other non-string
 * types, warns and falls back.
 */
export function coerceNonEmptyString(
  value: unknown,
  fallback: string,
  builder: string,
  param: string,
): string {
  if (typeof value === 'string') {
    return value.trim().length > 0 ? value : fallback;
  }
  if (value === undefined || value === null) return fallback;
  emit({
    builder,
    param,
    given: value,
    fallback,
    reason: `expected non-empty string, got ${typeof value}`,
  });
  return fallback;
}
