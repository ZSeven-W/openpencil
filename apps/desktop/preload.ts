import { contextBridge, ipcRenderer, type IpcRendererEvent } from 'electron';

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

// ---- Git API types (Phase 2a) ------------------------------------------------
// These mirror the engine's RepoOpenInfo / StatusInfo / CommitMeta / BranchInfo
// shapes so the renderer can type its IPC calls. The marker-based GitError
// rehydration is implemented on the renderer side in Phase 3.

export interface GitCandidateFileInfo {
  path: string;
  relativePath: string;
  milestoneCount: number;
  autosaveCount: number;
  lastCommitAt: number | null;
  lastCommitMessage: string | null;
}

export interface GitRepoOpenInfo {
  repoId: string;
  mode: 'single-file' | 'folder';
  rootPath: string;
  gitdir: string;
  engineKind: 'iso' | 'sys';
  trackedFilePath: string | null;
  candidates: GitCandidateFileInfo[];
}

export interface GitConflictBag {
  nodeConflicts: Array<{
    id: string;
    pageId: string | null;
    nodeId: string;
    reason:
      | 'both-modified-same-field'
      | 'modify-vs-delete'
      | 'add-vs-add-different'
      | 'reparent-conflict';
    base: unknown;
    ours: unknown;
    theirs: unknown;
  }>;
  docFieldConflicts: Array<{
    id: string;
    field: string;
    path: string;
    base: unknown;
    ours: unknown;
    theirs: unknown;
  }>;
}

export type GitConflictResolution =
  | { kind: 'ours' }
  | { kind: 'theirs' }
  | { kind: 'manual-node'; node: unknown }
  | { kind: 'manual-field'; value: unknown };

export interface GitStatusInfo {
  branch: string;
  trackedFilePath: string | null;
  workingDirty: boolean;
  otherFilesDirty: number;
  otherFilesPaths: string[];
  ahead: number;
  behind: number;
  mergeInProgress: boolean;
  unresolvedFiles: string[];
  conflicts: GitConflictBag | null;
}

export interface GitCommitMeta {
  hash: string;
  parentHashes: string[];
  message: string;
  author: { name: string; email: string; timestamp: number };
  kind: 'milestone' | 'autosave';
}

export interface GitBranchInfo {
  name: string;
  isCurrent: boolean;
  ahead: number;
  behind: number;
  lastCommit: { hash: string; message: string; timestamp: number } | null;
}

/**
 * Phase 6a: renderer-visible remote metadata for the single 'origin' remote.
 * Mirrors apps/web/src/services/git-types.ts so the renderer can type its
 * IPC calls without importing from the desktop side.
 */
export interface GitRemoteInfo {
  name: 'origin';
  url: string | null;
  host: string | null;
}

export interface GitAPI {
  detect: (filePath: string) => Promise<{ mode: 'none' } | GitRepoOpenInfo>;
  init: (filePath: string) => Promise<GitRepoOpenInfo>;
  open: (repoPath: string, currentFilePath?: string) => Promise<GitRepoOpenInfo>;
  bindTrackedFile: (repoId: string, filePath: string) => Promise<{ trackedFilePath: string }>;
  listCandidates: (repoId: string) => Promise<GitCandidateFileInfo[]>;
  close: (repoId: string) => Promise<void>;
  status: (repoId: string) => Promise<GitStatusInfo>;
  log: (
    repoId: string,
    opts: { ref: 'main' | 'autosaves' | string; limit: number },
  ) => Promise<GitCommitMeta[]>;
  commit: (
    repoId: string,
    opts: {
      kind: 'milestone' | 'autosave';
      message: string;
      author: { name: string; email: string };
    },
  ) => Promise<{ hash: string }>;
  restore: (repoId: string, commitHash: string) => Promise<void>;
  promote: (
    repoId: string,
    autosaveHash: string,
    message: string,
    author: { name: string; email: string },
  ) => Promise<{ hash: string }>;
  branchList: (repoId: string) => Promise<GitBranchInfo[]>;
  branchCreate: (repoId: string, opts: { name: string; fromCommit?: string }) => Promise<void>;
  branchSwitch: (repoId: string, name: string) => Promise<void>;
  branchDelete: (repoId: string, name: string, opts?: { force?: boolean }) => Promise<void>;

  // ---- Phase 2b: remote ops ----
  clone: (opts: {
    url: string;
    dest: string;
    auth?: { kind: 'token'; username: string; token: string } | { kind: 'ssh'; keyId: string };
  }) => Promise<GitRepoOpenInfo>;
  fetch: (
    repoId: string,
    auth?: { kind: 'token'; username: string; token: string } | { kind: 'ssh'; keyId: string },
  ) => Promise<{ ahead: number; behind: number }>;
  pull: (
    repoId: string,
    auth?: { kind: 'token'; username: string; token: string } | { kind: 'ssh'; keyId: string },
  ) => Promise<{
    result: 'fast-forward' | 'merge' | 'conflict' | 'conflict-non-op';
    conflicts?: GitConflictBag;
  }>;
  push: (
    repoId: string,
    auth?: { kind: 'token'; username: string; token: string } | { kind: 'ssh'; keyId: string },
  ) => Promise<{ result: 'ok' }>;

  // ---- Phase 2b: auth ----
  authStore: (
    host: string,
    creds: { kind: 'token'; username: string; token: string } | { kind: 'ssh'; keyId: string },
  ) => Promise<void>;
  authGet: (
    host: string,
  ) => Promise<
    { kind: 'token'; username: string; token: string } | { kind: 'ssh'; keyId: string } | null
  >;
  authClear: (host: string) => Promise<void>;

  // ---- Phase 2b: ssh keys (privateKeyPath stripped) ----
  sshListKeys: () => Promise<
    Array<{
      id: string;
      host: string;
      publicKey: string;
      fingerprint: string;
      comment: string;
    }>
  >;
  sshGenerateKey: (opts: { host: string; comment: string }) => Promise<{
    id: string;
    host: string;
    publicKey: string;
    fingerprint: string;
    comment: string;
  }>;
  sshImportKey: (opts: { privateKeyPath: string; host: string }) => Promise<{
    id: string;
    host: string;
    publicKey: string;
    fingerprint: string;
    comment: string;
  }>;
  sshDeleteKey: (keyId: string) => Promise<void>;

  // ---- Phase 2c: merge orchestration ----
  diff: (
    repoId: string,
    fromCommit: string,
    toCommit: string,
  ) => Promise<{
    summary: {
      framesChanged: number;
      nodesAdded: number;
      nodesRemoved: number;
      nodesModified: number;
    };
    patches: unknown[];
  }>;
  branchMerge: (
    repoId: string,
    fromBranch: string,
  ) => Promise<{
    result: 'fast-forward' | 'merge' | 'conflict' | 'conflict-non-op';
    conflicts?: GitConflictBag;
  }>;
  resolveConflict: (
    repoId: string,
    conflictId: string,
    choice: GitConflictResolution,
  ) => Promise<void>;
  applyMerge: (repoId: string) => Promise<{ hash: string; noop: boolean }>;
  abortMerge: (repoId: string) => Promise<void>;

  // Phase 4a: author identity probe (system git config)
  getSystemAuthor: () => Promise<{ name: string; email: string } | null>;

  // Phase 6a: remote metadata + config (no network)
  remoteGet: (repoId: string) => Promise<GitRemoteInfo>;
  remoteSet: (repoId: string, url: string | null) => Promise<GitRemoteInfo>;
}

export interface ElectronAPI {
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
  getLogDir: () => Promise<string>;
  setTheme: (theme: 'dark' | 'light', colors?: { bg: string; fg: string }) => void;
  getPreferences: () => Promise<Record<string, string>>;
  setPreference: (key: string, value: string) => Promise<void>;
  removePreference: (key: string) => Promise<void>;
  confirmClose: () => void;
  confirmUnsavedChanges: (payload: {
    message: string;
    detail?: string;
    yesLabel: string;
    noLabel: string;
    cancelLabel: string;
  }) => Promise<'save' | 'discard' | 'cancel'>;
  syncRecentFiles: (files: Array<{ fileName: string; filePath: string }>) => void;
  updater: {
    getState: () => Promise<UpdaterState>;
    checkForUpdates: () => Promise<UpdaterState>;
    quitAndInstall: () => Promise<boolean>;
    getAutoCheck: () => Promise<boolean>;
    setAutoCheck: (enabled: boolean) => Promise<boolean>;
    onStateChange: (callback: (state: UpdaterState) => void) => () => void;
  };
  git: GitAPI;
}

const api: ElectronAPI = {
  isElectron: true,

  openFile: () => ipcRenderer.invoke('dialog:openFile'),

  openImageFile: () => ipcRenderer.invoke('dialog:openImageFile'),

  openDirectory: () => ipcRenderer.invoke('dialog:openDirectory'),

  saveFile: (content: string, defaultPath?: string) =>
    ipcRenderer.invoke('dialog:saveFile', { content, defaultPath }),

  saveToPath: (filePath: string, content: string) =>
    ipcRenderer.invoke('dialog:saveToPath', { filePath, content }),

  setTheme: (theme: 'dark' | 'light', colors?: { bg: string; fg: string }) =>
    ipcRenderer.invoke('theme:set', theme, colors),

  getPreferences: () => ipcRenderer.invoke('prefs:getAll'),

  setPreference: (key: string, value: string) => ipcRenderer.invoke('prefs:set', key, value),

  removePreference: (key: string) => ipcRenderer.invoke('prefs:remove', key),

  onMenuAction: (callback: (action: string) => void) => {
    const listener = (_event: IpcRendererEvent, action: string) => {
      callback(action);
    };
    ipcRenderer.on('menu:action', listener);
    return () => {
      ipcRenderer.removeListener('menu:action', listener);
    };
  },

  onOpenFile: (callback: (filePath: string) => void) => {
    const listener = (_event: IpcRendererEvent, filePath: string) => {
      callback(filePath);
    };
    ipcRenderer.on('file:open', listener);
    return () => {
      ipcRenderer.removeListener('file:open', listener);
    };
  },

  readFile: (filePath: string) => ipcRenderer.invoke('file:read', filePath),

  getPendingFile: () => ipcRenderer.invoke('file:getPending'),

  syncRecentFiles: (files: Array<{ fileName: string; filePath: string }>) =>
    ipcRenderer.send('recent-files:sync', files),

  confirmClose: () => ipcRenderer.send('window:confirmClose'),

  confirmUnsavedChanges: (payload) => ipcRenderer.invoke('dialog:confirmUnsavedChanges', payload),

  getLogDir: () => ipcRenderer.invoke('log:getDir'),

  updater: {
    getState: () => ipcRenderer.invoke('updater:getState'),
    checkForUpdates: () => ipcRenderer.invoke('updater:checkForUpdates'),
    quitAndInstall: () => ipcRenderer.invoke('updater:quitAndInstall'),
    getAutoCheck: () => ipcRenderer.invoke('updater:getAutoCheck'),
    setAutoCheck: (enabled: boolean) => ipcRenderer.invoke('updater:setAutoCheck', enabled),
    onStateChange: (callback: (state: UpdaterState) => void) => {
      const listener = (_event: IpcRendererEvent, state: UpdaterState) => {
        callback(state);
      };
      ipcRenderer.on('updater:state', listener);
      return () => {
        ipcRenderer.removeListener('updater:state', listener);
      };
    },
  },

  git: {
    detect: (filePath: string) => ipcRenderer.invoke('git:detect', filePath),
    init: (filePath: string) => ipcRenderer.invoke('git:init', filePath),
    open: (repoPath: string, currentFilePath?: string) =>
      ipcRenderer.invoke('git:open', repoPath, currentFilePath),
    bindTrackedFile: (repoId: string, filePath: string) =>
      ipcRenderer.invoke('git:bindTrackedFile', repoId, filePath),
    listCandidates: (repoId: string) => ipcRenderer.invoke('git:listCandidates', repoId),
    close: (repoId: string) => ipcRenderer.invoke('git:close', repoId),
    status: (repoId: string) => ipcRenderer.invoke('git:status', repoId),
    log: (repoId, opts) => ipcRenderer.invoke('git:log', repoId, opts),
    commit: (repoId, opts) => ipcRenderer.invoke('git:commit', repoId, opts),
    restore: (repoId: string, commitHash: string) =>
      ipcRenderer.invoke('git:restore', repoId, commitHash),
    promote: (repoId, autosaveHash, message, author) =>
      ipcRenderer.invoke('git:promote', repoId, autosaveHash, message, author),
    branchList: (repoId: string) => ipcRenderer.invoke('git:branchList', repoId),
    branchCreate: (repoId, opts) => ipcRenderer.invoke('git:branchCreate', repoId, opts),
    branchSwitch: (repoId: string, name: string) =>
      ipcRenderer.invoke('git:branchSwitch', repoId, name),
    branchDelete: (repoId: string, name: string, opts?: { force?: boolean }) =>
      ipcRenderer.invoke('git:branchDelete', repoId, name, opts),

    // Phase 2b: remote ops
    clone: (opts) => ipcRenderer.invoke('git:clone', opts),
    fetch: (repoId, auth) => ipcRenderer.invoke('git:fetch', repoId, auth),
    pull: (repoId, auth) => ipcRenderer.invoke('git:pull', repoId, auth),
    push: (repoId, auth) => ipcRenderer.invoke('git:push', repoId, auth),

    // Phase 2b: auth
    authStore: (host, creds) => ipcRenderer.invoke('git:authStore', host, creds),
    authGet: (host) => ipcRenderer.invoke('git:authGet', host),
    authClear: (host) => ipcRenderer.invoke('git:authClear', host),

    // Phase 2b: ssh keys
    sshListKeys: () => ipcRenderer.invoke('git:sshListKeys'),
    sshGenerateKey: (opts) => ipcRenderer.invoke('git:sshGenerateKey', opts),
    sshImportKey: (opts) => ipcRenderer.invoke('git:sshImportKey', opts),
    sshDeleteKey: (keyId) => ipcRenderer.invoke('git:sshDeleteKey', keyId),

    // Phase 2c: merge orchestration
    diff: (repoId, fromCommit, toCommit) =>
      ipcRenderer.invoke('git:diff', repoId, fromCommit, toCommit),
    branchMerge: (repoId, fromBranch) => ipcRenderer.invoke('git:branchMerge', repoId, fromBranch),
    resolveConflict: (repoId, conflictId, choice) =>
      ipcRenderer.invoke('git:resolveConflict', repoId, conflictId, choice),
    applyMerge: (repoId) => ipcRenderer.invoke('git:applyMerge', repoId),
    abortMerge: (repoId) => ipcRenderer.invoke('git:abortMerge', repoId),

    // Phase 4a: author identity probe
    getSystemAuthor: () => ipcRenderer.invoke('git:getSystemAuthor'),

    // Phase 6a: remote metadata + config
    remoteGet: (repoId: string) => ipcRenderer.invoke('git:remoteGet', repoId),
    remoteSet: (repoId: string, url: string | null) =>
      ipcRenderer.invoke('git:remoteSet', repoId, url),
  },
};

contextBridge.exposeInMainWorld('electronAPI', api);
