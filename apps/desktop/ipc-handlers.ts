import { ipcMain, dialog, shell, type BrowserWindow } from 'electron';
import { relative, resolve, extname, sep } from 'node:path';
import { readFile, writeFile } from 'node:fs/promises';
import { app } from 'electron';

import {
  getUpdaterState,
  checkForAppUpdates,
  quitAndInstall,
  getAutoUpdateEnabled,
  setAutoUpdateEnabled,
  setUpdaterState,
  clearUpdateTimer,
  startUpdateTimer,
} from './auto-updater';
import { getLogDir } from './logger';
import {
  buildUnsavedChangesDialogOptions,
  mapUnsavedChangesResponse,
} from './unsaved-changes-dialog';
import {
  writeCodegenFilesToDirectory,
  type CodegenOutputFile,
} from './file-system/codegen-output';
import {
  commitCodegenOutput,
  getCodegenOutputGitStatus,
  pushCodegenOutput,
} from './file-system/codegen-git';
import { createDefaultCloudSessionStore, type CloudSessionStore } from './cloud/session-store';
import { normalizeCloudOAuthAuthorizeUrl } from './cloud/oauth-deep-link';

interface IpcDeps {
  getMainWindow: () => BrowserWindow | null;
  getPendingFilePath: () => string | null;
  clearPendingFilePath: () => void;
  getPendingOAuthCallbackUrl: () => string | null;
  clearPendingOAuthCallbackUrl: () => void;
  prefsCache: Record<string, string>;
  schedulePrefsWrite: () => void;
  writeAppSettings: (patch: { autoUpdate?: boolean }) => Promise<void>;
}

export function setupIPC(deps: IpcDeps): void {
  const {
    getMainWindow,
    getPendingFilePath,
    clearPendingFilePath,
    getPendingOAuthCallbackUrl,
    clearPendingOAuthCallbackUrl,
    prefsCache,
    schedulePrefsWrite,
    writeAppSettings,
  } = deps;
  const codegenOutputDirectories = new Set<string>();
  let cloudSessionStore: CloudSessionStore | null = null;

  function getCloudSessionStore(): CloudSessionStore {
    cloudSessionStore ??= createDefaultCloudSessionStore();
    return cloudSessionStore;
  }

  function isInsideDirectory(parentDir: string, childPath: string): boolean {
    const rel = relative(parentDir, childPath);
    return rel === '' || (!rel.startsWith('..') && resolve(rel) !== rel);
  }

  function requireCodegenOutputDirectory(rootDir: string): string {
    const resolvedRoot = resolve(rootDir);
    if (resolvedRoot.includes('\0')) {
      throw new Error('Invalid output directory');
    }
    if (!codegenOutputDirectories.has(resolvedRoot)) {
      throw new Error('Output directory must be selected before writing generated files');
    }
    return resolvedRoot;
  }

  function requireCodegenRevealPath(path: string): string {
    const resolvedPath = resolve(path);
    if (resolvedPath.includes('\0')) {
      throw new Error('Invalid path');
    }
    const isAllowed = Array.from(codegenOutputDirectories).some((dir) =>
      isInsideDirectory(dir, resolvedPath),
    );
    if (!isAllowed) {
      throw new Error('Path must be inside a selected generated code output directory');
    }
    return resolvedPath;
  }

  ipcMain.handle('dialog:openFile', async () => {
    const mainWindow = getMainWindow();
    if (!mainWindow) return null;
    const result = await dialog.showOpenDialog(mainWindow, {
      title: 'Open .op file',
      filters: [{ name: 'OpenPencil Files', extensions: ['op', 'pen'] }],
      properties: ['openFile'],
    });
    if (result.canceled || result.filePaths.length === 0) return null;
    const filePath = result.filePaths[0];
    const content = await readFile(filePath, 'utf-8');
    return { filePath, content };
  });

  ipcMain.handle('dialog:openDirectory', async () => {
    const mainWindow = getMainWindow();
    if (!mainWindow) return null;
    const result = await dialog.showOpenDialog(mainWindow, {
      title: 'Open Git repository folder',
      properties: ['openDirectory'],
    });
    if (result.canceled || result.filePaths.length === 0) return null;
    return result.filePaths[0];
  });

  ipcMain.handle('cloud-auth:getItem', (_event, key: string) =>
    getCloudSessionStore().getItem(key),
  );

  ipcMain.handle('cloud-auth:setItem', (_event, key: string, value: string) =>
    getCloudSessionStore().setItem(key, value),
  );

  ipcMain.handle('cloud-auth:removeItem', (_event, key: string) =>
    getCloudSessionStore().removeItem(key),
  );

  ipcMain.handle('cloud-auth:getPendingOAuthCallback', () => {
    const url = getPendingOAuthCallbackUrl();
    if (url) {
      clearPendingOAuthCallbackUrl();
      return url;
    }
    return null;
  });

  ipcMain.handle('cloud-auth:openOAuthUrl', async (_event, url: string) => {
    await shell.openExternal(normalizeCloudOAuthAuthorizeUrl(url));
  });

  ipcMain.handle('codegen:selectOutputDirectory', async () => {
    const mainWindow = getMainWindow();
    if (!mainWindow) return null;
    const result = await dialog.showOpenDialog(mainWindow, {
      title: 'Select generated code output folder',
      properties: ['openDirectory', 'createDirectory'],
    });
    if (result.canceled || result.filePaths.length === 0) return null;
    const selected = resolve(result.filePaths[0]);
    codegenOutputDirectories.add(selected);
    return selected;
  });

  ipcMain.handle(
    'codegen:writeFiles',
    async (_event, payload: { rootDir: string; files: CodegenOutputFile[] }) => {
      const rootDir = requireCodegenOutputDirectory(payload.rootDir);
      return writeCodegenFilesToDirectory({ ...payload, rootDir });
    },
  );

  ipcMain.handle('codegen:revealPath', async (_event, path: string) => {
    const resolved = requireCodegenRevealPath(path);
    shell.showItemInFolder(resolved);
  });

  ipcMain.handle(
    'codegen:gitStatus',
    async (_event, payload: { rootDir: string; files: CodegenOutputFile[] }) => {
      const rootDir = requireCodegenOutputDirectory(payload.rootDir);
      return getCodegenOutputGitStatus({ ...payload, rootDir });
    },
  );

  ipcMain.handle(
    'codegen:gitCommit',
    async (
      _event,
      payload: {
        rootDir: string;
        files: CodegenOutputFile[];
        message: string;
        author: { name: string; email: string };
      },
    ) => {
      const rootDir = requireCodegenOutputDirectory(payload.rootDir);
      return commitCodegenOutput({ ...payload, rootDir });
    },
  );

  ipcMain.handle('codegen:gitPush', async (_event, payload: { rootDir: string }) => {
    const rootDir = requireCodegenOutputDirectory(payload.rootDir);
    return pushCodegenOutput({ rootDir });
  });

  ipcMain.handle(
    'dialog:saveFile',
    async (_event, payload: { content: string; defaultPath?: string }) => {
      const mainWindow = getMainWindow();
      if (!mainWindow) return null;
      const result = await dialog.showSaveDialog(mainWindow, {
        title: 'Save .op file',
        defaultPath: payload.defaultPath,
        filters: [{ name: 'OpenPencil Files', extensions: ['op'] }],
      });
      if (result.canceled || !result.filePath) return null;
      await writeFile(result.filePath, payload.content, 'utf-8');
      return result.filePath;
    },
  );

  ipcMain.handle(
    'dialog:saveToPath',
    async (_event, payload: { filePath: string; content: string }) => {
      const resolved = resolve(payload.filePath);
      if (resolved.includes('\0')) {
        throw new Error('Invalid file path');
      }
      const ext = extname(resolved).toLowerCase();
      if (ext !== '.op' && ext !== '.pen') {
        throw new Error('Only .op and .pen file extensions are allowed');
      }
      const allowedRoots = [app.getPath('home'), app.getPath('temp')];
      const inAllowedDir = allowedRoots.some(
        (root) => resolved === root || resolved.startsWith(root + sep),
      );
      if (!inAllowedDir) {
        throw new Error('File path must be within the user home or temp directory');
      }
      await writeFile(resolved, payload.content, 'utf-8');
      return resolved;
    },
  );

  ipcMain.handle('file:getPending', () => {
    const filePath = getPendingFilePath();
    if (filePath) {
      clearPendingFilePath();
      return filePath;
    }
    return null;
  });

  ipcMain.handle('file:read', async (_event, filePath: string) => {
    const resolved = resolve(filePath);
    const ext = extname(resolved).toLowerCase();
    if (ext !== '.op' && ext !== '.pen') return null;
    try {
      const content = await readFile(resolved, 'utf-8');
      return { filePath: resolved, content };
    } catch {
      return null;
    }
  });

  ipcMain.handle('dialog:openImageFile', async () => {
    const mainWindow = getMainWindow();
    if (!mainWindow) return null;
    const result = await dialog.showOpenDialog(mainWindow, {
      title: 'Open image file',
      filters: [
        {
          name: 'Image Files',
          extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'avif'],
        },
      ],
      properties: ['openFile'],
    });
    if (result.canceled || result.filePaths.length === 0) return null;

    const filePath = result.filePaths[0];
    return {
      filePath,
      name: filePath.split(/[\\/]/).pop() ?? 'image',
      content:
        extname(filePath).toLowerCase() === '.svg' ? await readFile(filePath, 'utf-8') : null,
    };
  });

  ipcMain.handle(
    'dialog:confirmUnsavedChanges',
    async (
      _event,
      payload: {
        message: string;
        detail?: string;
        yesLabel: string;
        noLabel: string;
        cancelLabel: string;
      },
    ) => {
      const mainWindow = getMainWindow();
      if (!mainWindow) return 'cancel';
      const { response } = await dialog.showMessageBox(
        mainWindow,
        buildUnsavedChangesDialogOptions(payload),
      );
      return mapUnsavedChangesResponse(response);
    },
  );

  // Theme 同步 Windows/Linux 标题栏覆盖
  ipcMain.handle(
    'theme:set',
    (_event, theme: 'dark' | 'light', colors?: { bg: string; fg: string }) => {
      const mainWindow = getMainWindow();
      if (!mainWindow || mainWindow.isDestroyed()) return;
      const isWinOrLinux = process.platform === 'win32' || process.platform === 'linux';
      if (!isWinOrLinux) return;
      const isLinux = process.platform === 'linux';
      const fallbackBg = theme === 'dark' ? '#111' : '#fff';
      const fallbackFg = theme === 'dark' ? '#d4d4d8' : '#3f3f46';
      mainWindow.setTitleBarOverlay({
        color: isLinux ? colors?.bg || fallbackBg : 'rgba(0,0,0,0)',
        symbolColor: colors?.fg || fallbackFg,
      });
    },
  );

  // Renderer 偏好设置
  ipcMain.handle('prefs:getAll', () => ({ ...prefsCache }));

  ipcMain.handle('prefs:set', (_event, key: string, value: string) => {
    prefsCache[key] = value;
    schedulePrefsWrite();
  });

  ipcMain.handle('prefs:remove', (_event, key: string) => {
    delete prefsCache[key];
    schedulePrefsWrite();
  });

  // Recent 文件从渲染器同步→主（用于本机菜单）
  ipcMain.on(
    'recent-files:sync',
    (_event, files: Array<{ fileName: string; filePath: string }>) => {
      (global as any).__recentFiles = files;
      // Rebuild 菜单，因此“Open Recent”反映当前状态
      import('./app-menu').then(({ buildAppMenu }) => buildAppMenu());
    },
  );

  ipcMain.handle('log:getDir', () => getLogDir());

  // Updater IPC
  ipcMain.handle('updater:getState', () => getUpdaterState());
  ipcMain.handle('updater:checkForUpdates', async () => {
    await checkForAppUpdates(true);
    return getUpdaterState();
  });
  ipcMain.handle('updater:quitAndInstall', () => quitAndInstall());
  ipcMain.handle('updater:getAutoCheck', () => getAutoUpdateEnabled());

  ipcMain.handle('updater:setAutoCheck', async (_event, enabled: boolean) => {
    setAutoUpdateEnabled(enabled);
    await writeAppSettings({ autoUpdate: enabled });

    if (enabled) {
      startUpdateTimer();
      setUpdaterState({ status: 'idle' });
    } else {
      clearUpdateTimer();
      setUpdaterState({ status: 'disabled' });
    }
    return enabled;
  });
}
