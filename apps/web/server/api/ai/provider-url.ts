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
