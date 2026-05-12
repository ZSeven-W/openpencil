import type { App, BrowserWindow } from 'electron';
import { extname } from 'node:path';
import { CLOUD_AUTH_CALLBACK_CHANNEL, extractCloudAuthCallbackUrl } from './cloud/oauth-deep-link';

interface AppOpenHandlersDeps {
  app: App;
  getMainWindow: () => BrowserWindow | null;
  setPendingFilePath: (filePath: string | null) => void;
  setPendingCloudAuthCallbackUrl: (url: string | null) => void;
}

/** Extract .op file path from command-line arguments. */
export function getFilePathFromArgs(args: string[]): string | null {
  for (const arg of args) {
    if (arg.startsWith('-') || arg.startsWith('--')) continue;
    const ext = extname(arg).toLowerCase();
    if (ext === '.op' || ext === '.pen') return arg;
  }
  return null;
}

export function setupAppOpenHandlers(deps: AppOpenHandlersDeps): void {
  const { app, getMainWindow, setPendingFilePath, setPendingCloudAuthCallbackUrl } = deps;

  function sendOpenFile(filePath: string): void {
    const mainWindow = getMainWindow();
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send('file:open', filePath);
    } else {
      setPendingFilePath(filePath);
    }
  }

  function sendCloudAuthCallback(url: string): void {
    const mainWindow = getMainWindow();
    setPendingCloudAuthCallbackUrl(url);
    if (mainWindow && !mainWindow.isDestroyed() && !mainWindow.webContents.isLoading()) {
      setPendingCloudAuthCallbackUrl(null);
      mainWindow.webContents.send(CLOUD_AUTH_CALLBACK_CHANNEL, url);
    }
  }

  app.on('open-file', (event, filePath) => {
    event.preventDefault();
    if (app.isReady()) {
      sendOpenFile(filePath);
    } else {
      setPendingFilePath(filePath);
    }
  });

  app.on('open-url', (event, url) => {
    event.preventDefault();
    if (!extractCloudAuthCallbackUrl([url])) return;
    if (app.isReady()) {
      sendCloudAuthCallback(url);
    } else {
      setPendingCloudAuthCallbackUrl(url);
    }
  });

  const gotTheLock = app.requestSingleInstanceLock();
  if (!gotTheLock) {
    app.quit();
    return;
  }

  app.on('second-instance', (_event, argv) => {
    const oauthCallbackUrl = extractCloudAuthCallbackUrl(argv);
    if (oauthCallbackUrl) sendCloudAuthCallback(oauthCallbackUrl);

    const filePath = getFilePathFromArgs(argv);
    if (filePath) sendOpenFile(filePath);

    const mainWindow = getMainWindow();
    if (mainWindow) {
      if (mainWindow.isMinimized()) mainWindow.restore();
      mainWindow.focus();
    }
  });
}
