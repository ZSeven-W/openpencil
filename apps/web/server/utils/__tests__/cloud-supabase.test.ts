import { readdirSync, statSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { getBearerToken, getCloudEnv, getCloudSupabase } from '../cloud-supabase';

const createClientMock = vi.hoisted(() => vi.fn());

vi.mock('@supabase/supabase-js', () => ({
  createClient: createClientMock,
}));

function makeEvent(authorization?: string) {
  return {
    req: {
      headers: new Headers(authorization ? { authorization } : {}),
    },
  } as never;
}

function getCloudRouteFiles(dir = join(process.cwd(), 'server/api/cloud')): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const fullPath = join(dir, entry);
    return statSync(fullPath).isDirectory() ? getCloudRouteFiles(fullPath) : [fullPath];
  });
}

describe('cloud-supabase auth helpers', () => {
  afterEach(() => {
    createClientMock.mockReset();
    vi.unstubAllEnvs();
  });

  it('extracts bearer tokens', () => {
    expect(getBearerToken(makeEvent('Bearer abc123'))).toBe('abc123');
  });

  it('accepts bearer tokens case-insensitively and trims surrounding whitespace', () => {
    expect(getBearerToken(makeEvent('bearer   token-value  '))).toBe('token-value');
  });

  it('rejects missing bearer tokens with 401', () => {
    expect(() => getBearerToken(makeEvent())).toThrow(/Missing bearer token/);
  });

  it('rejects malformed bearer headers with 401', () => {
    expect(() => getBearerToken(makeEvent('Basic abc123'))).toThrow(/Missing bearer token/);
  });

  it('requires Supabase server configuration', () => {
    vi.stubEnv('SUPABASE_URL', '');
    vi.stubEnv('SUPABASE_ANON_KEY', '');
    vi.stubEnv('VITE_SUPABASE_URL', '');
    vi.stubEnv('VITE_SUPABASE_ANON_KEY', '');

    expect(() => getCloudEnv()).toThrow(/Supabase is not configured/);
  });

  it('does not create a Supabase client when auth is missing', async () => {
    vi.stubEnv('SUPABASE_URL', 'https://example.supabase.co');
    vi.stubEnv('SUPABASE_ANON_KEY', 'anon-key');

    await expect(getCloudSupabase(makeEvent())).rejects.toThrow(/Missing bearer token/);
    expect(createClientMock).not.toHaveBeenCalled();
  });

  it('rejects invalid bearer tokens with 401', async () => {
    vi.stubEnv('SUPABASE_URL', 'https://example.supabase.co');
    vi.stubEnv('SUPABASE_ANON_KEY', 'anon-key');
    const getUser = vi.fn(async () => ({
      data: { user: null },
      error: new Error('invalid token'),
    }));
    createClientMock.mockReturnValue({ auth: { getUser } });

    await expect(getCloudSupabase(makeEvent('Bearer invalid-token'))).rejects.toThrow(
      /Invalid bearer token/,
    );
    expect(getUser).toHaveBeenCalledWith('invalid-token');
  });

  it('creates a user-scoped Supabase client for valid bearer tokens', async () => {
    vi.stubEnv('SUPABASE_URL', 'https://example.supabase.co');
    vi.stubEnv('SUPABASE_ANON_KEY', 'anon-key');
    const user = { id: 'user-1', email: 'user@example.test' };
    const client = {
      auth: {
        getUser: vi.fn(async () => ({ data: { user }, error: null })),
      },
    };
    createClientMock.mockReturnValue(client);

    const context = await getCloudSupabase(makeEvent('Bearer valid-token'));

    expect(context).toEqual({ supabase: client, user, token: 'valid-token' });
    expect(createClientMock).toHaveBeenCalledWith(
      'https://example.supabase.co',
      'anon-key',
      expect.objectContaining({
        global: {
          headers: {
            Authorization: 'Bearer valid-token',
          },
        },
      }),
    );
  });

  it('keeps every /api/cloud route behind getCloudSupabase', () => {
    const routeFiles = getCloudRouteFiles().filter((file) => file.endsWith('.ts'));
    expect(routeFiles.length).toBeGreaterThan(0);

    const unguardedRoutes = routeFiles.filter(
      (file) => !readFileSync(file, 'utf-8').includes('getCloudSupabase(event)'),
    );
    expect(unguardedRoutes).toEqual([]);
  });
});
