// apps/web/src/services/git-types.ts
//
// Renderer-桌面 git IPC 表面的侧型镜像。 Kept 同步
// 手动使用 apps/desktop/preload.ts 的 GitAPI 界面 — 当一个新的 IPC
// 频道进入 Phase 2+ 或更高版本，请更新这两个文件。
//
// The GitErrorCode 联合镜像 apps/desktop/git/error.ts 逐字。

export type GitErrorCode =
  // Emitted 由 Phase 1b-2c：
  | 'init-failed'
  | 'open-failed'
  | 'not-a-repo'
  | 'commit-empty'
  | 'branch-exists'
  | 'branch-current'
  | 'branch-unmerged'
  | 'engine-crash'
  | 'no-file'
  | 'clone-failed'
  | 'clone-target-exists'
  | 'clone-network'
  | 'auth-required'
  | 'auth-failed'
  | 'auth-token-invalid'
  | 'network'
  | 'timeout'
  | 'commit-author-missing'
  | 'pull-non-fast-forward'
  | 'push-rejected'
  | 'push-no-remote'
  | 'branch-switch-dirty'
  | 'merge-conflict'
  | 'merge-conflict-non-op'
  | 'merge-still-conflicted'
  | 'merge-abort-failed'
  | 'restore-dirty'
  | 'ssh-not-supported-iso'
  | 'ssh-key-missing'
  | 'concurrent-busy'
  | 'external-modified'
  | 'save-required';

export type GitAuthCreds =
  | { kind: 'token'; username: string; token: string }
  | { kind: 'ssh'; keyId: string };

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
  /**
   * I2：在合并过程中重新打
   * 开面板时为 true — MERGE_HEAD 存在于磁盘上，但 session.inflightMerge 为 null（新会话）。
   * The 渲染器使用它来显示仅中止的 UI 而不是正常的冲突视图。 False （或不存在）在所有正常的合并流中。
   *
   */
  reopenedMidMerge?: boolean;
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

export interface GitPublicSshKeyInfo {
  id: string;
  host: string;
  publicKey: string;
  fingerprint: string;
  comment: string;
}

/**
 * Renderer-单个“
 *
 * 原始”远程的可见远程元数据。 Phase 6a 的合约：只有一个遥控器 — `origin`。 The
 * 渲染器从不检查多远程设置
 * ；如果用户在 `.git/config` 中有多个遥控器，则仅报告 `origin`。 `url` 是配置的 URL 或在 origin
 * 不存在时为 null。 `host` 是从 URL（HTTPS、ssh:// 和 SCP 样式 git@host:path）解析的，对于无法解析的
 * URLs / null URLs，则为 null。
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

  clone: (opts: { url: string; dest: string; auth?: GitAuthCreds }) => Promise<GitRepoOpenInfo>;
  fetch: (repoId: string, auth?: GitAuthCreds) => Promise<{ ahead: number; behind: number }>;
  pull: (
    repoId: string,
    auth?: GitAuthCreds,
  ) => Promise<{
    result: 'fast-forward' | 'merge' | 'conflict' | 'conflict-non-op';
    conflicts?: GitConflictBag;
  }>;
  push: (repoId: string, auth?: GitAuthCreds) => Promise<{ result: 'ok' }>;

  authStore: (host: string, creds: GitAuthCreds) => Promise<void>;
  authGet: (host: string) => Promise<GitAuthCreds | null>;
  authClear: (host: string) => Promise<void>;

  sshListKeys: () => Promise<GitPublicSshKeyInfo[]>;
  sshGenerateKey: (opts: { host: string; comment: string }) => Promise<GitPublicSshKeyInfo>;
  sshImportKey: (opts: { privateKeyPath: string; host: string }) => Promise<GitPublicSshKeyInfo>;
  sshDeleteKey: (keyId: string) => Promise<void>;

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

  // Phase 4a：作者身份探测（系统 git 配置）。如果 git 不可用或未设置 user.name/user.email 键，则
  // Returns null。
  getSystemAuthor: () => Promise<{ name: string; email: string } | null>;

  // Phase 6a：远程元数据+配置。 remoteGet 只读 .git/config
  // （没有网络）。 remoteSet 恰好拥有一个远程（'origin'） - 传递一个
  // 非空 url 到 set/update 或 `null` 删除它。 Both 通话
// return the fresh GitRemoteInfo so the renderer can update state from
  // 单程往返。
  remoteGet: (repoId: string) => Promise<GitRemoteInfo>;
  remoteSet: (repoId: string, url: string | null) => Promise<GitRemoteInfo>;
}
