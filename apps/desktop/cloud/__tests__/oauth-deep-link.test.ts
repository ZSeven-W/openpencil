import { describe, expect, it } from 'vitest';
import {
  CLOUD_AUTH_CALLBACK_URL,
  extractCloudAuthCallbackUrl,
  isCloudAuthCallbackUrl,
  normalizeCloudOAuthAuthorizeUrl,
} from '../oauth-deep-link';

describe('OAuth deep link helpers', () => {
  it('recognizes the OpenPencil cloud auth callback URL', () => {
    expect(isCloudAuthCallbackUrl(`${CLOUD_AUTH_CALLBACK_URL}?code=github-code`)).toBe(true);
  });

  it('rejects other protocols and paths', () => {
    expect(isCloudAuthCallbackUrl('https://example.test/auth/callback?code=github-code')).toBe(
      false,
    );
    expect(isCloudAuthCallbackUrl('openpencil://files/callback?code=github-code')).toBe(false);
    expect(isCloudAuthCallbackUrl('not a url')).toBe(false);
  });

  it('extracts a callback URL from second-instance arguments', () => {
    expect(
      extractCloudAuthCallbackUrl([
        '/Applications/OpenPencil.app/Contents/MacOS/OpenPencil',
        '--flag',
        `${CLOUD_AUTH_CALLBACK_URL}?code=github-code`,
      ]),
    ).toBe(`${CLOUD_AUTH_CALLBACK_URL}?code=github-code`);
  });

  it('allows only http and https OAuth authorization URLs', () => {
    expect(normalizeCloudOAuthAuthorizeUrl('https://example.supabase.co/auth/v1/authorize')).toBe(
      'https://example.supabase.co/auth/v1/authorize',
    );
    expect(() => normalizeCloudOAuthAuthorizeUrl('openpencil://auth/callback')).toThrow(
      /http or https/,
    );
  });
});
