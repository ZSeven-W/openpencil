// apps/web/src/types/electron.d.ts
//
// Type definition for window.electronAPI, exposed by apps/desktop/preload.ts.
// Kept in sync with preload.ts by hand — when new IPC channels land in the
// desktop bridge, update both files.
//
// This file is a module (it imports from services/git-types) so `export {}`
// below marks it as such. The `declare global` block makes the Window
// augmentation (and the ambient Updater* / ElectronAPI types that existing
// code references globally) visible project-wide.

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
    /** Phase 3+: git IPC surface. See apps/web/src/services/git-types.ts. */
    git: GitAPI;
  }

  interface Window {
    electronAPI?: ElectronAPI;
  }
}

export {};
