export function normalizeBaseURL(baseURL: string): string {
  return baseURL.trim().replace(/\/+$/, '');
}

export function normalizeOptionalBaseURL(baseURL?: string): string | undefined {
  if (!baseURL) return undefined;
  const normalized = normalizeBaseURL(baseURL);
  return normalized.length > 0 ? normalized : undefined;
}

export function normalizeOpenAICompatBaseURL(baseURL?: string): string | undefined {
  const normalized = normalizeOptionalBaseURL(baseURL);
  if (!normalized) return undefined;

  switch (normalized) {
    case 'https://ark.cn-beijing.volces.com/api/v3/v1':
      return 'https://ark.cn-beijing.volces.com/api/v3';
    case 'https://ark.cn-beijing.volces.com/api/coding/v3/v1':
      return 'https://ark.cn-beijing.volces.com/api/coding/v3';
    case 'https://open.bigmodel.cn/api/paas/v4/v1':
      return 'https://open.bigmodel.cn/api/paas/v4';
    case 'https://api.z.ai/api/paas/v4/v1':
    case 'https://open.z.ai/api/paas/v4/v1':
      return 'https://api.z.ai/api/paas/v4';
    case 'https://open.bigmodel.cn/api/coding/paas/v4/v1':
      return 'https://open.bigmodel.cn/api/coding/paas/v4';
    case 'https://api.z.ai/api/coding/paas/v4/v1':
    case 'https://open.z.ai/api/coding/paas/v4/v1':
      return 'https://api.z.ai/api/coding/paas/v4';
    case 'https://generativelanguage.googleapis.com/v1beta/openai/v1':
      return 'https://generativelanguage.googleapis.com/v1beta/openai';
    default:
      return normalized;
  }
}

export function requireOpenAICompatBaseURL(baseURL?: string): string {
  const normalized = normalizeOpenAICompatBaseURL(baseURL);
  if (!normalized) {
    throw new Error('OpenAI-compatible provider requires baseURL');
  }
  return normalized;
}

/**
 * Normalize a team-member's baseURL. Throws if an openai-compat member has no baseURL.
 */
export function normalizeMemberBaseURL(
  memberId: string,
  providerType: string,
  baseURL?: string,
): string | undefined {
  const normalized =
    providerType === 'openai-compat'
      ? normalizeOpenAICompatBaseURL(baseURL)
      : normalizeOptionalBaseURL(baseURL);
  if (providerType === 'openai-compat' && !normalized) {
    throw new Error(`Member "${memberId}" (openai-compat) requires baseURL`);
  }
  return normalized;
}

export function buildProviderModelsURL(baseURL: string): string {
  return `${normalizeOpenAICompatBaseURL(baseURL) ?? normalizeBaseURL(baseURL)}/models`;
}

/**
 * Node's `fetch` (undici) collapses every network-level failure — DNS, TLS,
 * refused connection, timeout — into a single opaque `TypeError: fetch failed`.
 * The real reason lives on `error.cause` as a SystemError with `code`/`syscall`/
 * `hostname`. Surface that so users can act on it ("ENOTFOUND api.foo.com",
 * "self-signed certificate", "ECONNREFUSED 127.0.0.1:443") instead of staring
 * at "fetch failed".
 *
 * Walks the `cause` chain (some failures wrap multiple times) and unwraps
 * AggregateError (undici emits one when DNS returns multiple A records and
 * every connect attempt fails) so each leaf reason makes it to the user.
 */
export function formatFetchError(error: unknown): string {
  const reasons = collectFetchErrorReasons(error);
  if (reasons.length === 0) {
    return error instanceof Error ? error.message || 'Unknown error' : 'Unknown error';
  }
  return Array.from(new Set(reasons)).join('; ');
}

function collectFetchErrorReasons(error: unknown, depth = 0): string[] {
  if (depth > 5 || !(error instanceof Error)) return [];

  const aggregate = (error as { errors?: unknown }).errors;
  if (Array.isArray(aggregate) && aggregate.length > 0) {
    return aggregate.flatMap((sub) => collectFetchErrorReasons(sub, depth + 1));
  }

  const cause = (error as { cause?: unknown }).cause;
  if (cause instanceof Error) {
    return collectFetchErrorReasons(cause, depth + 1);
  }

  const code = (error as { code?: string }).code;
  const message = error.message?.trim();
  if (code && message && !message.includes(code)) return [`${code}: ${message}`];
  if (message) return [message];
  if (code) return [code];
  return [];
}
