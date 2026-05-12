import { describe, expect, it, vi } from 'vitest';
import { getFilePathFromArgs, setupAppOpenHandlers } from '../../app-open-handlers';

function makeApp() {
  const handlers = new Map<string, (...args: any[]) => void>();
  return {
    handlers,
    app: {
      on: vi.fn((event: string, callback: (...args: any[]) => void) => {
        handlers.set(event, callback);
      }),
      isReady: vi.fn(() => true),
      requestSingleInstanceLock: vi.fn(() => true),
      quit: vi.fn(),
    },
  };
}

describe('app open handlers', () => {
  it('extracts OpenPencil document paths from argv', () => {
    expect(getFilePathFromArgs(['--flag', '/tmp/design.op'])).toBe('/tmp/design.op');
    expect(getFilePathFromArgs(['/tmp/design.pen'])).toBe('/tmp/design.pen');
    expect(getFilePathFromArgs(['--flag', '/tmp/readme.md'])).toBeNull();
  });

  it('stores pending OAuth callback when the window is still loading', () => {
    const { app, handlers } = makeApp();
    const setPendingCloudAuthCallbackUrl = vi.fn();
    const webContents = { isLoading: vi.fn(() => true), send: vi.fn() };

    setupAppOpenHandlers({
      app: app as never,
      getMainWindow: () =>
        ({
          isDestroyed: () => false,
          webContents,
          isMinimized: () => false,
          restore: vi.fn(),
          focus: vi.fn(),
        }) as never,
      setPendingFilePath: vi.fn(),
      setPendingCloudAuthCallbackUrl,
    });

    handlers
      .get('second-instance')
      ?.({}, ['/Applications/OpenPencil.app', 'openpencil://auth/callback?code=github-code']);

    expect(setPendingCloudAuthCallbackUrl).toHaveBeenCalledWith(
      'openpencil://auth/callback?code=github-code',
    );
    expect(webContents.send).not.toHaveBeenCalled();
  });

  it('sends OAuth callback to a ready window and clears pending state', () => {
    const { app, handlers } = makeApp();
    const setPendingCloudAuthCallbackUrl = vi.fn();
    const webContents = { isLoading: vi.fn(() => false), send: vi.fn() };

    setupAppOpenHandlers({
      app: app as never,
      getMainWindow: () =>
        ({
          isDestroyed: () => false,
          webContents,
          isMinimized: () => false,
          restore: vi.fn(),
          focus: vi.fn(),
        }) as never,
      setPendingFilePath: vi.fn(),
      setPendingCloudAuthCallbackUrl,
    });

    handlers.get('open-url')?.(
      { preventDefault: vi.fn() },
      'openpencil://auth/callback?code=github-code',
    );

    expect(setPendingCloudAuthCallbackUrl).toHaveBeenNthCalledWith(
      1,
      'openpencil://auth/callback?code=github-code',
    );
    expect(setPendingCloudAuthCallbackUrl).toHaveBeenNthCalledWith(2, null);
    expect(webContents.send).toHaveBeenCalledWith(
      'cloud-auth:oauth-callback',
      'openpencil://auth/callback?code=github-code',
    );
  });
});
