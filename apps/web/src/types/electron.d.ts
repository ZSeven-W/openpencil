// apps/web/src/types/electron.d.ts
//
// window.electronAPI 的 Type 定义，由 apps/desktop/preload.ts 公开。
// Kept 与 preload.ts 手动同步 — 当新的 IPC 通道登陆时
// 桌面桥，更新这两个文件。
//
// This 文件是一个模块（它从 services/git-types 导入），所以 `export {}`
// 下面将其标记为这样。 The `declare global` 块使 Window
// 增强（以及现有的环境 Updater* / ElectronAPI 类型
// 全局代码引用）在项目范围内可见。

import type { GitAPI } from '@/services/git-types';

declare global {
  type UpdaterStatus =
    | 'disabled'
    | 'idle'
    | 'checking'
    | 'available'
    | 'downloading'
    | 'downloaded'
    | 'not-available'
    | 'error';

  interface UpdaterState {
    status: UpdaterStatus;
    currentVersion: string;
    latestVersion?: string;
    downloadProgress?: number;
    releaseDate?: string;
    error?: string;
  }

  interface ElectronAPI {
    isElectron: true;
    openFile: () => Promise<{ filePath: string; content: string } | null>;
    openImageFile: () => Promise<{ filePath: string; name: string; content: string | null } | null>;
    openDirectory: () => Promise<string | null>;
    saveFile: (content: string, defaultPath?: string) => Promise<string | null>;
    saveToPath: (filePath: string, content: string) => Promise<string>;
    onMenuAction: (callback: (action: string) => void) => () => void;
    onOpenFile: (callback: (filePath: string) => void) => () => void;
    readFile: (filePath: string) => Promise<{ filePath: string; content: string } | null>;
    getPendingFile: () => Promise<string | null>;
    syncRecentFiles: (files: Array<{ fileName: string; filePath: string }>) => void;
    /** Resolve 拖放时 File 的绝对文件系统路径。 */
    getPathForFile: (file: File) => string | null;
    confirmClose: () => void;
    confirmUnsavedChanges: (payload: {
      message: string;
      detail?: string;
      yesLabel: string;
      noLabel: string;
      cancelLabel: string;
    }) => Promise<'save' | 'discard' | 'cancel'>;
    getLogDir: () => Promise<string>;
    setTheme: (theme: 'dark' | 'light', colors?: { bg: string; fg: string }) => void;
    getPreferences: () => Promise<Record<string, string>>;
    setPreference: (key: string, value: string) => Promise<void>;
    removePreference: (key: string) => Promise<void>;
    updater: {
      getState: () => Promise<UpdaterState>;
      checkForUpdates: () => Promise<UpdaterState>;
      quitAndInstall: () => Promise<boolean>;
      getAutoCheck: () => Promise<boolean>;
      setAutoCheck: (enabled: boolean) => Promise<boolean>;
      onStateChange: (callback: (state: UpdaterState) => void) => () => void;
    };
    /** Phase 3+：git IPC 表面。 See apps/web/src/services/git-types.ts。 */
    git: GitAPI;
  }

  interface Window {
    electronAPI?: ElectronAPI;
  }
}

export {};
