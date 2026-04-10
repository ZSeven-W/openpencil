// apps/web/src/stores/git-store-types.ts
//
// Type declarations extracted from git-store.ts to keep that file under
// the 800-LoC cap. Pure types — no runtime code, no actions, no helpers.
// Imported by git-store.ts and any consumer that needs to type-check
// against GitState / RepoMeta / GitStore.

import type {
  GitAuthCreds,
  GitBranchInfo,
  GitCandidateFileInfo,
  GitCommitMeta,
  GitConflictBag,
  GitConflictResolution,
  GitPublicSshKeyInfo,
} from '@/services/git-types';

// ---------------------------------------------------------------------------
// Types: GitState union + RepoMeta + ConflictBagState + PendingAction
// ---------------------------------------------------------------------------

export interface RepoMeta {
  repoId: string;
  mode: 'single-file' | 'folder';
  rootPath: string;
  gitdir: string;
  engineKind: 'iso' | 'sys';
  trackedFilePath: string | null;
  candidateFiles: GitCandidateFileInfo[];
  currentBranch: string;
  branches: GitBranchInfo[];
  workingDirty: boolean;
  otherFilesDirty: number;
  otherFilesPaths: string[];
  ahead: number;
  behind: number;
}

/**
 * Renderer-side wrapper that tracks per-conflict resolution state in a Map
 * keyed by conflictId. Built by hydrateConflictBag() from the wire-format
 * GitConflictBag when branchMerge/pull returns conflicts.
 *
 * Invariant: the backend emits conflictIds in distinct namespaces —
 * `node:<pageId|_>:<nodeId>` for node conflicts and `docField:<field>` for
 * doc-field conflicts. The two Maps therefore share no keys, and
 * resolveConflict can probe them in sequence without ambiguity. If the
 * backend ever changes this, resolveConflict's branch logic becomes
 * ambiguous and must be updated to carry an explicit kind tag.
 */
export interface ConflictBagState {
  nodeConflicts: Map<
    string,
    GitConflictBag['nodeConflicts'][number] & { resolution?: GitConflictResolution }
  >;
  docFieldConflicts: Map<
    string,
    GitConflictBag['docFieldConflicts'][number] & { resolution?: GitConflictResolution }
  >;
}

export interface PendingAction {
  label: string;
  run: () => Promise<void>;
}

export type GitState =
  | { kind: 'no-file' }
  | { kind: 'no-repo' }
  | { kind: 'wizard-clone' }
  | { kind: 'initializing' }
  | { kind: 'needs-tracked-file'; repo: RepoMeta }
  | { kind: 'ready'; repo: RepoMeta; saveRequiredFor?: PendingAction }
  | {
      kind: 'conflict';
      repo: RepoMeta;
      conflicts: ConflictBagState;
      saveRequiredFor?: PendingAction;
    }
  | { kind: 'error'; message: string; recoverable: boolean };

// ---------------------------------------------------------------------------
// Store interface
// ---------------------------------------------------------------------------

export interface GitStore {
  state: GitState;
  panelOpen: boolean;
  log: GitCommitMeta[];
  sshKeys: GitPublicSshKeyInfo[];

  // Phase 4a: author identity (cached + persisted via prefs)
  authorIdentity: { name: string; email: string } | null;
  authorPromptVisible: boolean;

  // Phase 4b: auto-bind banner (transient flag set when openRepo/cloneRepo
  // auto-binds a single candidate file; cleared by acknowledge actions or
  // closeRepo)
  lastAutoBindedPath: string | null;

  // Phase 4c: commit input draft (ephemeral, not persisted)
  commitMessage: string;

  // Phase 4c: autosave error display (last error from the subscriber)
  autosaveError: string | null;

  // Phase 4c: subscriber lifecycle handle (internal, never read by UI)
  __autosaveUnsub: (() => void) | null;

  // Panel lifecycle
  togglePanel: () => void;
  openPanel: () => void;
  closePanel: () => void;

  // Phase 4a: author identity actions
  loadAuthorIdentity: () => Promise<void>;
  setAuthorIdentity: (name: string, email: string) => Promise<void>;
  showAuthorPrompt: () => void;
  hideAuthorPrompt: () => void;

  // Phase 4b: auto-bind banner actions
  acknowledgeAutoBind: () => void;
  acknowledgeAutoBindAndOpen: () => Promise<void>;

  // Phase 4c: commit input actions
  setCommitMessage: (text: string) => void;
  clearCommitMessage: () => void;
  cancelSaveRequired: () => void;

  // Phase 4c: overflow menu actions
  enterTrackedFilePicker: () => void;
  clearAuthorIdentity: () => Promise<void>;

  // Phase 4c: autosave subscriber lifecycle
  initAutosaveSubscriber: () => void;
  disposeAutosaveSubscriber: () => void;
  clearAutosaveError: () => void;

  // Repo discovery / creation
  detectRepo: (filePath: string) => Promise<void>;
  initRepo: (filePath: string) => Promise<void>;
  openRepo: (repoPath: string, currentFilePath?: string) => Promise<void>;
  cloneRepo: (opts: { url: string; dest: string; auth?: GitAuthCreds }) => Promise<void>;
  bindTrackedFile: (filePath: string) => Promise<void>;
  refreshCandidates: () => Promise<void>;
  closeRepo: () => Promise<void>;

  // Status / log / diff
  refreshStatus: () => Promise<void>;
  loadLog: (opts: { ref: 'main' | 'autosaves' | string; limit: number }) => Promise<void>;
  computeDiff: (
    from: string,
    to: string,
  ) => Promise<{
    summary: {
      framesChanged: number;
      nodesAdded: number;
      nodesRemoved: number;
      nodesModified: number;
    };
    patches: unknown[];
  }>;

  // Commit / restore / promote (all MUTATING, gated by withCleanWorkingTree)
  commitMilestone: (message: string, author: { name: string; email: string }) => Promise<void>;
  commitAutosave: (message: string, author: { name: string; email: string }) => Promise<void>;
  restoreCommit: (commitHash: string) => Promise<void>;
  promoteAutosave: (
    autosaveHash: string,
    message: string,
    author: { name: string; email: string },
  ) => Promise<void>;

  // Branches (switch/merge MUTATING, others read-only)
  refreshBranches: () => Promise<void>;
  createBranch: (opts: { name: string; fromCommit?: string }) => Promise<void>;
  switchBranch: (name: string) => Promise<void>;
  deleteBranch: (name: string) => Promise<void>;
  mergeBranch: (fromBranch: string) => Promise<void>;

  // Merge orchestration
  resolveConflict: (conflictId: string, choice: GitConflictResolution) => Promise<void>;
  applyMerge: () => Promise<void>;
  abortMerge: () => Promise<void>;

  // Remote (pull/push MUTATING, fetch read-only)
  fetchRemote: (auth?: GitAuthCreds) => Promise<void>;
  pull: (auth?: GitAuthCreds) => Promise<void>;
  push: (auth?: GitAuthCreds) => Promise<void>;

  // Auth
  storeAuth: (host: string, creds: GitAuthCreds) => Promise<void>;
  getAuth: (host: string) => Promise<GitAuthCreds | null>;
  clearAuth: (host: string) => Promise<void>;

  // SSH keys
  refreshSshKeys: () => Promise<void>;
  generateSshKey: (opts: { host: string; comment: string }) => Promise<GitPublicSshKeyInfo>;
  importSshKey: (opts: { privateKeyPath: string; host: string }) => Promise<GitPublicSshKeyInfo>;
  deleteSshKey: (keyId: string) => Promise<void>;

  // Retry the queued action after a successful save (Phase 4 wires the button)
  retrySaveRequired: () => Promise<void>;
}
