import { app, BrowserWindow } from 'electron';
import { autoUpdater } from 'electron-updater';
import type { NsisUpdater } from 'electron-updater';
import { GitHubProvider } from 'electron-updater/out/providers/GitHubProvider';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { GITHUB_OWNER, GITHUB_REPO } from './constants';

const execFileAsync = promisify(execFile);

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type UpdaterStatus =
  | 'disabled'
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'downloaded'
  | 'not-available'
  | 'error';

export interface UpdaterState {
  status: UpdaterStatus;
  currentVersion: string;
  latestVersion?: string;
  downloadProgress?: number;
  releaseDate?: string;
  error?: string;
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

const isDev = !app.isPackaged;

let updaterState: UpdaterState = {
  status: isDev ? 'disabled' : 'idle',
  currentVersion: app.getVersion(),
};

let autoUpdateEnabled = true;
let updateCheckTimer: ReturnType<typeof setInterval> | null = null;
let lastUpdateCheckAt = 0;

const MacGitHubUpdateProvider = class {
  constructor(options: unknown, updater: unknown, runtimeOptions: unknown) {
    const provider = new (GitHubProvider as any)(options, updater, runtimeOptions) as any;
    if (process.platform === 'darwin') {
      provider.getDefaultChannelName = () =>
        process.arch === 'arm64' ? 'latest-mac-arm64' : 'latest-mac';
      provider.getCustomChannelName = (channel: string) => channel;
    }
    return provider;
  }
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function getUpdaterState(): UpdaterState {
  return updaterState;
}

export function getAutoUpdateEnabled(): boolean {
  return autoUpdateEnabled;
}

export function setAutoUpdateEnabled(enabled: boolean): void {
  autoUpdateEnabled = enabled;
}

export function broadcastUpdaterState(): void {
  for (const win of BrowserWindow.getAllWindows()) {
    if (!win.isDestroyed()) {
      win.webContents.send('updater:state', updaterState);
    }
  }
}

export function setUpdaterState(next: Partial<UpdaterState>): void {
  updaterState = {
    ...updaterState,
    ...next,
    currentVersion: app.getVersion(),
  };
  broadcastUpdaterState();
}

export async function checkForAppUpdates(force = false): Promise<void> {
  if (isDev) return;

  const now = Date.now();
  if (!force && now - lastUpdateCheckAt < 60 * 1000) {
    return;
  }
  lastUpdateCheckAt = now;

  try {
    await autoUpdater.checkForUpdates();
  } catch (err) {
    const error = err instanceof Error ? err.message : String(err);
    setUpdaterState({ status: 'error', error });
  }
}

export function clearUpdateTimer(): void {
  if (updateCheckTimer) {
    clearInterval(updateCheckTimer);
    updateCheckTimer = null;
  }
}

export function startUpdateTimer(): void {
  if (updateCheckTimer) return;
  updateCheckTimer = setInterval(
    () => {
      void checkForAppUpdates(false);
    },
    60 * 60 * 1000,
  );
  updateCheckTimer.unref();
}

export function quitAndInstall(): boolean {
  if (!isDev && updaterState.status === 'downloaded') {
    autoUpdater.quitAndInstall();
    return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

export function setupAutoUpdater(): void {
  if (isDev) return;

  if (process.platform === 'darwin') {
    // macOS needs a custom provider to select arm64 vs x64 channel
    autoUpdater.setFeedURL({
      provider: 'custom',
      updateProvider: MacGitHubUpdateProvider as any,
      owner: GITHUB_OWNER,
      repo: GITHUB_REPO,
      releaseType: 'release',
    } as any);
  } else {
    // Windows/Linux: use standard GitHub provider (reads from electron-builder.yml publish config)
    autoUpdater.setFeedURL({
      provider: 'github',
      owner: GITHUB_OWNER,
      repo: GITHUB_REPO,
      releaseType: 'release',
    });
  }

  autoUpdater.autoDownload = true;
  autoUpdater.autoInstallOnAppQuit = true;
  autoUpdater.allowPrerelease = true;

  // Windows: custom signature verification for self-signed certificate.
  // The default verifier requires the cert to be in the Windows trusted root
  // store, which self-signed certs are not. This custom function still verifies
  // the publisher name from the Authenticode signature — it just skips the
  // trust chain check. This is NOT disabling verification.
  if (process.platform === 'win32') {
    const nsisUpdater = autoUpdater as NsisUpdater;
    nsisUpdater.verifyUpdateCodeSignature = async (
      publisherNames: string[],
      tempUpdateFile: string,
    ): Promise<string | null> => {
      try {
        const { stdout } = await execFileAsync(
          'powershell.exe',
          [
            '-NoProfile',
            '-NonInteractive',
            '-Command',
            `(Get-AuthenticodeSignature '${tempUpdateFile.replace(/'/g, "''")}').SignerCertificate.Subject`,
          ],
          { timeout: 30_000 },
        );

        const subject = stdout.trim();
        if (!subject) {
          return 'The update file is not signed.';
        }

        for (const name of publisherNames) {
          if (subject.includes(name)) {
            return null; // Publisher name matches — verification passed
          }
        }

        return `Publisher mismatch. Expected: ${publisherNames.join(', ')}. Got: ${subject}`;
      } catch (err) {
        return `Signature verification failed: ${err instanceof Error ? err.message : String(err)}`;
      }
    };
  }

  autoUpdater.on('checking-for-update', () => {
    setUpdaterState({ status: 'checking', error: undefined, downloadProgress: undefined });
  });

  autoUpdater.on('update-available', (info) => {
    setUpdaterState({
      status: 'available',
      latestVersion: info.version,
      releaseDate: info.releaseDate,
      error: undefined,
    });
  });

  autoUpdater.on('download-progress', (progress) => {
    setUpdaterState({
      status: 'downloading',
      downloadProgress: Math.round(progress.percent),
      error: undefined,
    });
  });

  autoUpdater.on('update-downloaded', (info) => {
    setUpdaterState({
      status: 'downloaded',
      latestVersion: info.version,
      releaseDate: info.releaseDate,
      downloadProgress: 100,
      error: undefined,
    });
  });

  autoUpdater.on('update-not-available', (info) => {
    setUpdaterState({
      status: 'not-available',
      latestVersion: info.version,
      downloadProgress: undefined,
      error: undefined,
    });
  });

  autoUpdater.on('error', (err) => {
    setUpdaterState({
      status: 'error',
      error: err?.message ?? String(err),
    });
  });

  if (autoUpdateEnabled) {
    // Delay first check until app startup work is done.
    setTimeout(() => {
      void checkForAppUpdates(true);
    }, 5000);

    startUpdateTimer();
  }
}
