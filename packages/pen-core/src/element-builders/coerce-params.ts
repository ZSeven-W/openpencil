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
 * Title → canonical lucide icon name. Used by nav builders (bottom-nav,
 * sidebar-nav) to correct common AI mistakes where the model picks an
 * icon that doesn't match the tab's purpose. The most frequent slip is
 * Cart→`shopping-bag` instead of `shopping-cart` (a bag is for carrying,
 * a cart is for checkout — visually they are different glyphs in lucide).
 *
 * Reads as: "if the user-visible label is X, the canonical icon is Y".
 * Keep entries lowercase for case-insensitive match. Bilingual where
 * the AI commonly emits the Chinese label.
 */
const NAV_TITLE_TO_CANONICAL_ICON: Record<string, string> = {
  cart: 'shopping-cart',
  购物车: 'shopping-cart',
  bag: 'shopping-bag',
  购物袋: 'shopping-bag',
  home: 'house',
  首页: 'house',
  search: 'search',
  搜索: 'search',
  profile: 'user',
  account: 'user',
  我的: 'user',
  账户: 'user',
  orders: 'clipboard-list',
  订单: 'clipboard-list',
  inbox: 'inbox',
  收件箱: 'inbox',
  notifications: 'bell',
  通知: 'bell',
  messages: 'message-circle',
  消息: 'message-circle',
  settings: 'settings',
  设置: 'settings',
  favorites: 'heart',
  likes: 'heart',
  收藏: 'heart',
  explore: 'compass',
  discover: 'compass',
  发现: 'compass',
};

/**
 * If the nav tab's `title` has a canonical icon AND the model's emitted
 * `icon` is one of the known wrong-glyph choices for that title, swap to
 * the canonical name and emit a coerce warning. Other icons are passed
 * through untouched (the model may have a deliberate non-default choice).
 *
 * Examples (warning + swap):
 *   { title: 'Cart', icon: 'shopping-bag' } → 'shopping-cart'
 *   { title: 'Profile', icon: 'profile' }   → 'user'
 *   { title: 'Home',   icon: 'home' }       → 'house' (lucide canonical)
 *
 * Pass-through (no warning):
 *   { title: 'Custom', icon: 'rocket' }     → 'rocket'
 *   { title: 'Cart',   icon: 'shopping-cart' } → 'shopping-cart'
 *
 * Reused across bottom-nav-v1 + sidebar-nav-v1 to keep the convention
 * single-sourced.
 */
export function coerceNavTabIcon(title: string, icon: string, builder: string): string {
  const lowerTitle = title.trim().toLowerCase();
  const canonical =
    NAV_TITLE_TO_CANONICAL_ICON[lowerTitle] ?? NAV_TITLE_TO_CANONICAL_ICON[title.trim()];
  if (!canonical) return icon;
  if (icon === canonical) return icon;
  // Only swap when the emitted icon is a known wrong-glyph choice for
  // this title. Otherwise the user may have intentionally picked a
  // custom glyph (e.g. Cart → 'package' for a delivery app).
  const KNOWN_WRONG_ALTS: Record<string, string[]> = {
    'shopping-cart': ['shopping-bag', 'bag', 'package', 'tote'],
    user: ['profile', 'account', 'avatar', 'circle-user-round'],
    house: ['home'],
    bell: ['notification', 'alarm'],
    'message-circle': ['message', 'chat'],
    'clipboard-list': ['list', 'orders', 'receipt'],
  };
  if (!KNOWN_WRONG_ALTS[canonical]?.includes(icon)) return icon;
  emit({
    builder,
    param: 'icon',
    given: icon,
    fallback: canonical,
    reason: `nav tab "${title}" canonical icon is ${canonical} (got ${icon})`,
  });
  return canonical;
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
