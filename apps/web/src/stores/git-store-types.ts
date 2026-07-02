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
  GitRemoteInfo,
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
  /**
   * Phase 6a: cached remote metadata for the single 'origin' remote, or
   * null when no probe has been issued yet. Hydrated by `refreshRemote()`,
   * mutated by `setRemoteUrl()`. The store reads `.git/config` only — no
   * network. The Phase 6b pull/push controls and the Phase 6c remote
   * settings UI both branch on this field.
   */
  remote: GitRemoteInfo | null;
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

/**
 * Recoverable GitErrorCodes that the clone wizard catches inline instead of
 * letting them escape into the generic `error` state. Defined here so the
 * store action and the wizard component agree on the exact set.
 */
export const CLONE_INLINE_ERROR_CODES = [
  'clone-network',
  'network',
  'timeout',
  'auth-required',
  'auth-failed',
  'auth-token-invalid',
  'clone-failed',
  'clone-target-exists',
] as const;

export type CloneInlineErrorCode = (typeof CLONE_INLINE_ERROR_CODES)[number];

/**
 * Phase 6b: recoverable auth-related GitErrorCodes the pull / push buttons
 * catch inline and surface the shared auth form for. `auth-required` covers
 * the "no credentials at all" case; `auth-failed` and `auth-token-invalid`
 * cover "stored credentials rejected by the server". Anything else escapes
 * to the generic error state. Pull and push share a single constant — the
 * auth form is common to both flows and there's no planned divergence.
 */
export const REMOTE_AUTH_ERROR_CODES = [
  'auth-required',
  'auth-failed',
  'auth-token-invalid',
] as const;

export type RemoteAuthErrorCode = (typeof REMOTE_AUTH_ERROR_CODES)[number];

export type GitState =
  | { kind: 'no-file' }
  | { kind: 'no-repo' }
  | {
      kind: 'wizard-clone';
      /**
       * True while `cloneRepo()` is in flight from inside the wizard. The
       * wizard stays mounted across the round-trip (no transition to
       * `initializing`) so the form's URL/dest/token inputs survive a
       * recoverable failure. The clone form reads this directly instead of
       * keeping a local `useState` that would be lost on unmount.
       */
      busy: boolean;
      /**
       * Inline error surfaced under the clone form. Set when cloneRepo()
       * caught a recoverable code (see CLONE_INLINE_ERROR_CODES). The wizard
       * stays mounted so the user can fix the URL/auth and retry without
       * losing form state. Cleared on the next cloneRepo() attempt or
       * cancelCloneWizard().
       */
      error: { code: CloneInlineErrorCode; message: string } | null;
    }
  | { kind: 'initializing' }
  | { kind: 'needs-tracked-file'; repo: RepoMeta }
  | { kind: 'ready'; repo: RepoMeta; saveRequiredFor?: PendingAction }
  | {
      kind: 'conflict';
      repo: RepoMeta;
      conflicts: ConflictBagState;
      /**
       * Phase 6b: paths (relative to repo root) of non-`.op` files the
       * backend reported as unresolved. Empty array means the conflict is
       * purely over `.op` node/field data and the existing per-node
       * resolution UI covers it. Non-empty means the user must either
       * resolve those files externally and hit "continue", or abort the
       * merge entirely — the conflict banner renders a strip with both
       * recovery affordances when this is non-empty.
       */
      unresolvedFiles: string[];
      /**
       * Phase 7b: inline error from the last applyMerge() call that threw
       * `merge-still-conflicted`. Cleared when the user resolves more
       * conflicts and retries, or when refreshStatus() reconciles the state.
       * Null means no finalize error is pending.
       */
      finalizeError: string | null;
      /**
       * I2: true when the panel was reopened mid-merge and the in-memory
       * conflict state was lost (session.inflightMerge === null, MERGE_HEAD
       * on disk). The banner renders an abort-only UI when this is true.
       * False (or absent) in all normal merge flows.
       */
      reopenedMidMerge: boolean;
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
  /**
   * Phase 7b: exit the tracked-file picker.
   * - If the picker was entered from `ready` (repo.trackedFilePath non-null)
   *   → transition back to `ready` with the same repo.
   * - If the picker is the first post-open/post-clone screen
   *   (repo.trackedFilePath === null) → close the transient repo session
   *   and return to `no-file`.
   */
  exitTrackedFilePicker: () => Promise<void>;
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
  deleteBranch: (name: string, opts?: { force?: boolean }) => Promise<void>;
  mergeBranch: (fromBranch: string) => Promise<void>;

  // Merge orchestration
  resolveConflict: (conflictId: string, choice: GitConflictResolution) => Promise<void>;
  applyMerge: () => Promise<void>;
  abortMerge: () => Promise<void>;

  // Remote (pull/push MUTATING, fetch read-only)
  fetchRemote: (auth?: GitAuthCreds) => Promise<void>;
  pull: (auth?: GitAuthCreds) => Promise<void>;
  push: (auth?: GitAuthCreds) => Promise<void>;

  // Phase 6a: clone wizard + remote metadata/config
  /**
   * Transition any state into `wizard-clone` with no inline error. The
   * empty-state clone card is the only entry point in 6a; later phases
   * may add a settings entry from `ready`.
   */
  enterCloneWizard: () => void;
  /**
   * Always transitions back to `no-file`. The git-panel.tsx detect-repo
   * effect immediately rehydrates the correct `no-repo` / `ready` state
   * from the currently-open document path, so we don't need a smarter
   * cancel target here.
   */
  cancelCloneWizard: () => void;
  /**
   * Refresh the cached `repo.remote` from the desktop side via remoteGet.
   * Reads only `.git/config` — no network. No-op when state has no repo.
   */
  refreshRemote: () => Promise<void>;
  /**
   * Set or clear the single 'origin' remote. Pass a non-empty url to
   * add/update; pass `null` to remove. Updates `repo.remote` immediately
   * from the IPC return value so a single round-trip is enough — callers
   * MUST NOT rely on a follow-up refreshRemote() to see the new value.
   */
  setRemoteUrl: (url: string | null) => Promise<void>;

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
