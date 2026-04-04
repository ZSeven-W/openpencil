export function normalizeBaseURL(baseURL: string): string {
  return baseURL.trim().replace(/\/+$/, '');
}

export function normalizeOptionalBaseURL(baseURL?: string): string | undefined {
  if (!baseURL) return undefined;
  const normalized = normalizeBaseURL(baseURL);
  return normalized.length > 0 ? normalized : undefined;
}

export function requireOpenAICompatBaseURL(baseURL?: string): string {
  const normalized = normalizeOptionalBaseURL(baseURL);
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
  const normalized = normalizeOptionalBaseURL(baseURL);
  if (providerType === 'openai-compat' && !normalized) {
    throw new Error(`Member "${memberId}" (openai-compat) requires baseURL`);
  }
  return normalized;
}

export function buildProviderModelsURL(baseURL: string): string {
  return `${normalizeBaseURL(baseURL)}/models`;
}
