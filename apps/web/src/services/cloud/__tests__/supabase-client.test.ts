import { afterEach, describe, expect, it, vi } from 'vitest';

const createClientMock = vi.hoisted(() => vi.fn());

vi.mock('@supabase/supabase-js', () => ({
  createClient: createClientMock,
}));

async function importFreshClientModule() {
  vi.resetModules();
  return import('../supabase-client');
}

describe('supabase-client', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    createClientMock.mockReset();
    vi.unstubAllEnvs();
  });

  it('uses Electron secure auth storage when the desktop bridge is available', async () => {
    vi.stubEnv('VITE_SUPABASE_URL', 'https://example.supabase.co');
    vi.stubEnv('VITE_SUPABASE_ANON_KEY', 'anon-key');
    const cloudAuth = {
      getItem: vi.fn(async () => 'stored-session'),
      setItem: vi.fn(async () => {}),
      removeItem: vi.fn(async () => {}),
      openOAuthUrl: vi.fn(async () => {}),
      onOAuthCallback: vi.fn(() => vi.fn()),
    };
    vi.stubGlobal('window', { electronAPI: { isElectron: true, cloudAuth } });
    createClientMock.mockReturnValue({ auth: {} });

    const { getSupabaseBrowserClient } = await importFreshClientModule();
    getSupabaseBrowserClient();

    const options = createClientMock.mock.calls[0]?.[2];
    expect(options.auth.persistSession).toBe(true);
    expect(options.auth.storageKey).toBe('openpencil-cloud-auth');
    expect(options.auth.detectSessionInUrl).toBe(false);
    expect(options.auth.flowType).toBe('pkce');
    await expect(options.auth.storage.getItem('auth-key')).resolves.toBe('stored-session');
    await options.auth.storage.setItem('auth-key', 'auth-value');
    await options.auth.storage.removeItem('auth-key');
    expect(cloudAuth.getItem).toHaveBeenCalledWith('auth-key');
    expect(cloudAuth.setItem).toHaveBeenCalledWith('auth-key', 'auth-value');
    expect(cloudAuth.removeItem).toHaveBeenCalledWith('auth-key');
  });

  it('falls back to default browser storage outside Electron', async () => {
    vi.stubEnv('VITE_SUPABASE_URL', 'https://example.supabase.co');
    vi.stubEnv('VITE_SUPABASE_ANON_KEY', 'anon-key');
    vi.stubGlobal('window', {});
    createClientMock.mockReturnValue({ auth: {} });

    const { getSupabaseBrowserClient } = await importFreshClientModule();
    getSupabaseBrowserClient();

    const options = createClientMock.mock.calls[0]?.[2];
    expect(options.auth.storage).toBeUndefined();
    expect(options.auth.storageKey).toBe('openpencil-cloud-auth');
    expect(options.auth.detectSessionInUrl).toBe(true);
    expect(options.auth.flowType).toBe('implicit');
  });
});
