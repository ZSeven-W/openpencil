// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';

const supabaseMocks = vi.hoisted(() => ({
  getSupabaseBrowserClient: vi.fn(),
  getSupabaseBrowserConfig: vi.fn(),
  getSession: vi.fn(),
  onAuthStateChange: vi.fn(),
  signInWithPassword: vi.fn(),
  signInWithOAuth: vi.fn(),
  exchangeCodeForSession: vi.fn(),
  signOut: vi.fn(),
  isElectronCloudAuthAvailable: vi.fn(),
}));

vi.mock('@/services/cloud/supabase-client', () => ({
  getSupabaseBrowserClient: () => supabaseMocks.getSupabaseBrowserClient(),
  getSupabaseBrowserConfig: () => supabaseMocks.getSupabaseBrowserConfig(),
  isElectronCloudAuthAvailable: () => supabaseMocks.isElectronCloudAuthAvailable(),
}));

import { __cloudAuthTestExports, useCloudAuthStore } from '../cloud-auth-store';

describe('cloud-auth-store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    __cloudAuthTestExports.resetSubscriptions();
    supabaseMocks.getSupabaseBrowserConfig.mockReturnValue({
      url: 'https://example.supabase.co',
      anonKey: 'anon-key',
    });
    supabaseMocks.getSupabaseBrowserClient.mockReturnValue({
      auth: {
        getSession: supabaseMocks.getSession,
        onAuthStateChange: supabaseMocks.onAuthStateChange,
        signInWithPassword: supabaseMocks.signInWithPassword,
        signInWithOAuth: supabaseMocks.signInWithOAuth,
        exchangeCodeForSession: supabaseMocks.exchangeCodeForSession,
        signOut: supabaseMocks.signOut,
      },
    });
    supabaseMocks.getSession.mockResolvedValue({
      data: { session: null },
      error: null,
    });
    supabaseMocks.onAuthStateChange.mockReturnValue({
      data: { subscription: { unsubscribe: vi.fn() } },
    });
    supabaseMocks.signInWithPassword.mockResolvedValue({
      data: { session: null, user: null },
      error: null,
    });
    supabaseMocks.signInWithOAuth.mockResolvedValue({
      data: { url: 'https://github.com/login/oauth/authorize' },
      error: null,
    });
    supabaseMocks.exchangeCodeForSession.mockResolvedValue({
      data: { session: null },
      error: null,
    });
    supabaseMocks.signOut.mockResolvedValue({ error: null });
    supabaseMocks.isElectronCloudAuthAvailable.mockReturnValue(false);
    delete (window as typeof window & { electronAPI?: ElectronAPI }).electronAPI;
    window.history.pushState({}, '', '/editor/file-1?draft=1');
    useCloudAuthStore.setState({
      status: 'anonymous',
      session: null,
      user: null,
      error: null,
      initialized: true,
    });
  });

  it('starts GitHub OAuth with the current app path as redirect target', async () => {
    const ok = await useCloudAuthStore.getState().signInWithGitHub();

    expect(ok).toBe(true);
    expect(supabaseMocks.signInWithOAuth).toHaveBeenCalledWith({
      provider: 'github',
      options: {
        redirectTo: `${window.location.origin}/editor/file-1`,
      },
    });
    expect(useCloudAuthStore.getState().status).toBe('loading');
  });

  it('starts desktop GitHub OAuth in the system browser with the app protocol callback', async () => {
    const openOAuthUrl = vi.fn(async () => {});
    const onOAuthCallback = vi.fn(() => vi.fn());
    (window as typeof window & { electronAPI?: ElectronAPI }).electronAPI = {
      isElectron: true,
      cloudAuth: {
        getItem: vi.fn(),
        setItem: vi.fn(),
        removeItem: vi.fn(),
        getPendingOAuthCallback: vi.fn(async () => null),
        openOAuthUrl,
        onOAuthCallback,
      },
    } as never;
    supabaseMocks.isElectronCloudAuthAvailable.mockReturnValue(true);

    const ok = await useCloudAuthStore.getState().signInWithGitHub();

    expect(ok).toBe(true);
    expect(supabaseMocks.signInWithOAuth).toHaveBeenCalledWith({
      provider: 'github',
      options: {
        redirectTo: 'openpencil://auth/callback',
        skipBrowserRedirect: true,
      },
    });
    expect(openOAuthUrl).toHaveBeenCalledWith('https://github.com/login/oauth/authorize');
  });

  it('exchanges desktop OAuth callback codes and stores the returned session', async () => {
    const oauthCallbackRef: { current: ((url: string) => void) | null } = { current: null };
    const session = {
      access_token: 'desktop-token',
      user: { id: 'user-desktop', email: 'desktop@example.test' },
    };
    (window as typeof window & { electronAPI?: ElectronAPI }).electronAPI = {
      isElectron: true,
      cloudAuth: {
        getItem: vi.fn(),
        setItem: vi.fn(),
        removeItem: vi.fn(),
        getPendingOAuthCallback: vi.fn(async () => null),
        openOAuthUrl: vi.fn(),
        onOAuthCallback: vi.fn((callback) => {
          oauthCallbackRef.current = callback;
          return vi.fn();
        }),
      },
    } as never;
    supabaseMocks.isElectronCloudAuthAvailable.mockReturnValue(true);
    supabaseMocks.exchangeCodeForSession.mockResolvedValue({
      data: { session },
      error: null,
    });

    await useCloudAuthStore.getState().initialize();
    oauthCallbackRef.current?.('openpencil://auth/callback?code=github-code');
    await vi.waitFor(() => {
      expect(supabaseMocks.exchangeCodeForSession).toHaveBeenCalledWith('github-code');
    });

    expect(useCloudAuthStore.getState()).toMatchObject({
      status: 'authenticated',
      session,
      user: session.user,
      error: null,
    });
  });

  it('exchanges a pending desktop OAuth callback during initialization', async () => {
    const session = {
      access_token: 'pending-desktop-token',
      user: { id: 'user-pending', email: 'pending@example.test' },
    };
    (window as typeof window & { electronAPI?: ElectronAPI }).electronAPI = {
      isElectron: true,
      cloudAuth: {
        getItem: vi.fn(),
        setItem: vi.fn(),
        removeItem: vi.fn(),
        getPendingOAuthCallback: vi.fn(
          async () => 'openpencil://auth/callback?code=pending-github-code',
        ),
        openOAuthUrl: vi.fn(),
        onOAuthCallback: vi.fn(() => vi.fn()),
      },
    } as never;
    supabaseMocks.isElectronCloudAuthAvailable.mockReturnValue(true);
    supabaseMocks.exchangeCodeForSession.mockResolvedValue({
      data: { session },
      error: null,
    });

    await useCloudAuthStore.getState().initialize();

    expect(supabaseMocks.exchangeCodeForSession).toHaveBeenCalledWith('pending-github-code');
    expect(useCloudAuthStore.getState()).toMatchObject({
      status: 'authenticated',
      session,
      user: session.user,
    });
  });

  it('restores an existing persisted session during initialization', async () => {
    const session = {
      access_token: 'restored-token',
      user: { id: 'user-1', email: 'user@example.test' },
    };
    supabaseMocks.getSession.mockResolvedValue({
      data: { session },
      error: null,
    });

    await useCloudAuthStore.getState().initialize();

    expect(useCloudAuthStore.getState()).toMatchObject({
      status: 'authenticated',
      session,
      user: session.user,
      initialized: true,
    });
  });

  it('signs in with email and stores the returned session in state', async () => {
    const session = {
      access_token: 'email-token',
      user: { id: 'user-2', email: 'email@example.test' },
    };
    supabaseMocks.signInWithPassword.mockResolvedValue({
      data: { session, user: session.user },
      error: null,
    });

    const ok = await useCloudAuthStore.getState().signIn('email@example.test', 'password123');

    expect(ok).toBe(true);
    expect(supabaseMocks.signInWithPassword).toHaveBeenCalledWith({
      email: 'email@example.test',
      password: 'password123',
    });
    expect(useCloudAuthStore.getState()).toMatchObject({
      status: 'authenticated',
      session,
      user: session.user,
    });
  });

  it('returns a restored access token for cloud API calls', async () => {
    const session = {
      access_token: 'api-token',
      user: { id: 'user-3', email: 'api@example.test' },
    };
    supabaseMocks.getSession.mockResolvedValue({
      data: { session },
      error: null,
    });

    const token = await useCloudAuthStore.getState().getAccessToken();

    expect(token).toBe('api-token');
    expect(useCloudAuthStore.getState()).toMatchObject({
      status: 'authenticated',
      session,
      user: session.user,
    });
  });

  it('clears session state on sign out', async () => {
    useCloudAuthStore.setState({
      status: 'authenticated',
      session: {
        access_token: 'token',
        user: { id: 'user-4', email: 'out@example.test' },
      } as never,
      user: { id: 'user-4', email: 'out@example.test' } as never,
      initialized: true,
    });

    await useCloudAuthStore.getState().signOut();

    expect(supabaseMocks.signOut).toHaveBeenCalled();
    expect(useCloudAuthStore.getState()).toMatchObject({
      status: 'anonymous',
      session: null,
      user: null,
      initialized: true,
    });
  });

  it('reports unconfigured state when Supabase is unavailable', async () => {
    supabaseMocks.getSupabaseBrowserClient.mockReturnValue(null);
    supabaseMocks.getSupabaseBrowserConfig.mockReturnValue(null);

    const ok = await useCloudAuthStore.getState().signInWithGitHub();

    expect(ok).toBe(false);
    expect(supabaseMocks.signInWithOAuth).not.toHaveBeenCalled();
    expect(useCloudAuthStore.getState()).toMatchObject({
      status: 'unconfigured',
      error: 'Supabase is not configured.',
    });
  });
});
