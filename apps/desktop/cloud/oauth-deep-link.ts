export const CLOUD_AUTH_PROTOCOL = 'openpencil';
export const CLOUD_AUTH_CALLBACK_URL = 'openpencil://auth/callback';
export const CLOUD_AUTH_CALLBACK_CHANNEL = 'cloud-auth:oauth-callback';

export function isCloudAuthCallbackUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return (
      url.protocol === `${CLOUD_AUTH_PROTOCOL}:` &&
      url.hostname === 'auth' &&
      url.pathname === '/callback'
    );
  } catch {
    return false;
  }
}

export function extractCloudAuthCallbackUrl(args: string[]): string | null {
  for (const arg of args) {
    if (isCloudAuthCallbackUrl(arg)) return arg;
  }
  return null;
}

export function normalizeCloudOAuthAuthorizeUrl(value: string): string {
  const url = new URL(value);
  if (url.protocol !== 'https:' && url.protocol !== 'http:') {
    throw new Error('OAuth URL must use http or https');
  }
  return url.toString();
}
