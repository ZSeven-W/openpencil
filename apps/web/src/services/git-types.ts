// apps/web/src/services/git-types.ts
//
// Renderer-side type mirror of the desktop git IPC surface. Kept in sync
// with apps/desktop/preload.ts's GitAPI interface by hand — when a new IPC
// channel lands in Phase 2+ or later phases, update both files.
//
// The GitErrorCode union mirrors apps/desktop/git/error.ts verbatim.

export type GitErrorCode =
  // Emitted by Phase 1b-2c:
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
  branchDelete: (repoId: string, name: string) => Promise<void>;

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

  // Phase 4a: author identity probe (system git config). Returns null if
  // git is unavailable or either user.name/user.email key is unset.
  getSystemAuthor: () => Promise<{ name: string; email: string } | null>;
}
