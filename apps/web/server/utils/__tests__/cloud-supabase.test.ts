import { describe, expect, it } from 'vitest';
import { getBearerToken } from '../cloud-supabase';

function makeEvent(authorization?: string) {
  return {
    req: {
      headers: new Headers(authorization ? { authorization } : {}),
    },
  } as never;
}

describe('cloud-supabase auth helpers', () => {
  it('extracts bearer tokens', () => {
    expect(getBearerToken(makeEvent('Bearer abc123'))).toBe('abc123');
  });

  it('rejects missing bearer tokens with 401', () => {
    expect(() => getBearerToken(makeEvent())).toThrow(/Missing bearer token/);
  });
});
