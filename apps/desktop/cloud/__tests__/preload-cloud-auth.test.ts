import { beforeEach, describe, expect, it, vi } from 'vitest';

const exposeInMainWorld = vi.fn();
const invoke = vi.fn();
const on = vi.fn();
const removeListener = vi.fn();

vi.mock('electron', () => ({
  clipboard: { writeText: vi.fn() },
  contextBridge: { exposeInMainWorld },
  ipcRenderer: {
    invoke,
    on,
    removeListener,
    send: vi.fn(),
  },
  webUtils: { getPathForFile: vi.fn() },
}));

describe('preload cloud auth bridge', () => {
  beforeEach(() => {
    exposeInMainWorld.mockClear();
    invoke.mockClear();
    on.mockClear();
    removeListener.mockClear();
    vi.resetModules();
  });

  it('exposes cloud auth storage methods', async () => {
    await import('../../preload');

    expect(exposeInMainWorld).toHaveBeenCalledWith(
      'electronAPI',
      expect.objectContaining({
        cloudAuth: expect.objectContaining({
          getItem: expect.any(Function),
          setItem: expect.any(Function),
          removeItem: expect.any(Function),
          getPendingOAuthCallback: expect.any(Function),
          openOAuthUrl: expect.any(Function),
          onOAuthCallback: expect.any(Function),
        }),
      }),
    );
  });

  it('forwards cloud auth storage calls to named IPC channels', async () => {
    await import('../../preload');
    const api = exposeInMainWorld.mock.calls[0]?.[1] as ElectronAPI;

    await api.cloudAuth.getItem('auth-key');
    await api.cloudAuth.setItem('auth-key', 'auth-value');
    await api.cloudAuth.removeItem('auth-key');
    await api.cloudAuth.getPendingOAuthCallback();
    await api.cloudAuth.openOAuthUrl('https://example.supabase.co/auth/v1/authorize');

    expect(invoke.mock.calls).toEqual([
      ['cloud-auth:getItem', 'auth-key'],
      ['cloud-auth:setItem', 'auth-key', 'auth-value'],
      ['cloud-auth:removeItem', 'auth-key'],
      ['cloud-auth:getPendingOAuthCallback'],
      ['cloud-auth:openOAuthUrl', 'https://example.supabase.co/auth/v1/authorize'],
    ]);
  });

  it('subscribes to OAuth callback events and returns an unsubscribe function', async () => {
    await import('../../preload');
    const api = exposeInMainWorld.mock.calls[0]?.[1] as ElectronAPI;
    const callback = vi.fn();

    const unsubscribe = api.cloudAuth.onOAuthCallback(callback);
    const listener = on.mock.calls[0]?.[1];
    listener({}, 'openpencil://auth/callback?code=github-code');
    unsubscribe();

    expect(on).toHaveBeenCalledWith('cloud-auth:oauth-callback', expect.any(Function));
    expect(callback).toHaveBeenCalledWith('openpencil://auth/callback?code=github-code');
    expect(removeListener).toHaveBeenCalledWith('cloud-auth:oauth-callback', listener);
  });
});
