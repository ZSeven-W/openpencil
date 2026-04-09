// apps/web/src/stores/git-store.ts
//
// Zustand store implementing the GitState state machine from the spec
// (line 847-898). Every mutating action goes through withCleanWorkingTree
// so the renderer can never overwrite the disk with an out-of-sync tree.
//
// Phase 3 ships the complete action surface (all 31 IPC channels wrapped)
// but NO autosave subscriber, NO panel UI, NO i18n. Phase 4 wires the
// panel and the documentEvents.on('saved') hook.

import { create } from 'zustand';
import { gitClient } from '@/services/git-client';
import { GitError, isGitError } from '@/services/git-error';
import type {
  GitAuthCreds,
  GitBranchInfo,
  GitCandidateFileInfo,
  GitCommitMeta,
  GitConflictBag,
  GitConflictResolution,
  GitPublicSshKeyInfo,
  GitRepoOpenInfo,
} from '@/services/git-types';
import { useDocumentStore } from '@/stores/document-store';

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

interface GitStore {
  state: GitState;
  panelOpen: boolean;
  log: GitCommitMeta[];
  sshKeys: GitPublicSshKeyInfo[];

  // Panel lifecycle
  togglePanel: () => void;
  openPanel: () => void;
  closePanel: () => void;

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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Build an empty RepoMeta shell from a GitRepoOpenInfo. Branches and status
 * fields are filled in by refreshStatus + refreshBranches which the store
 * calls after every init/open/clone/bind.
 */
function metaFromOpenInfo(info: GitRepoOpenInfo): RepoMeta {
  return {
    repoId: info.repoId,
    mode: info.mode,
    rootPath: info.rootPath,
    gitdir: info.gitdir,
    engineKind: info.engineKind,
    trackedFilePath: info.trackedFilePath,
    candidateFiles: info.candidates,
    currentBranch: 'main', // refreshStatus will correct this
    branches: [],
    workingDirty: false,
    otherFilesDirty: 0,
    otherFilesPaths: [],
    ahead: 0,
    behind: 0,
  };
}

/**
 * Hydrate a wire-format GitConflictBag into the Map-backed ConflictBagState.
 */
function hydrateConflictBag(bag: GitConflictBag): ConflictBagState {
  const nodeConflicts = new Map<
    string,
    GitConflictBag['nodeConflicts'][number] & { resolution?: GitConflictResolution }
  >();
  for (const c of bag.nodeConflicts) nodeConflicts.set(c.id, { ...c });

  const docFieldConflicts = new Map<
    string,
    GitConflictBag['docFieldConflicts'][number] & { resolution?: GitConflictResolution }
  >();
  for (const c of bag.docFieldConflicts) docFieldConflicts.set(c.id, { ...c });

  return { nodeConflicts, docFieldConflicts };
}

/**
 * Get the current repoId or throw. Most actions require an active session.
 */
function requireRepoId(state: GitState): string {
  if (state.kind === 'ready' || state.kind === 'conflict' || state.kind === 'needs-tracked-file') {
    return state.repo.repoId;
  }
  throw new GitError('no-file', 'No active repository for this action', {
    recoverable: false,
  });
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useGitStore = create<GitStore>((set, get) => {
  /**
   * Guard a mutating action on `useDocumentStore.getState().isDirty`. If
   * dirty, stash a PendingAction on the current state and throw
   * GitError('save-required'). The UI catches and shows an inline alert;
   * on save-and-retry the store re-runs the action.
   */
  async function withCleanWorkingTree<T>(action: () => Promise<T>, label: string): Promise<T> {
    if (useDocumentStore.getState().isDirty) {
      const pending: PendingAction = {
        label,
        run: async () => {
          await action();
        },
      };
      set((s) => {
        if (s.state.kind === 'ready' || s.state.kind === 'conflict') {
          return { state: { ...s.state, saveRequiredFor: pending } };
        }
        return s;
      });
      throw new GitError('save-required', 'Document has unsaved changes');
    }
    return action();
  }

  /**
   * Wrap any action so thrown GitError transitions the store to error state.
   * Phase 3 uses this for the user-initiated init/open/clone paths. Other
   * actions propagate the error to the caller for local handling.
   */
  async function runOrError<T>(action: () => Promise<T>): Promise<T | undefined> {
    try {
      return await action();
    } catch (err) {
      const message = isGitError(err)
        ? err.message
        : err instanceof Error
          ? err.message
          : String(err);
      const recoverable = isGitError(err) ? err.recoverable : false;
      set({
        state: {
          kind: 'error',
          message,
          recoverable,
        },
      });
      throw err;
    }
  }

  return {
    state: { kind: 'no-file' },
    panelOpen: false,
    log: [],
    sshKeys: [],

    // ---- Panel lifecycle -------------------------------------------------
    togglePanel: () => set((s) => ({ panelOpen: !s.panelOpen })),
    openPanel: () => set({ panelOpen: true }),
    closePanel: () => set({ panelOpen: false }),

    // ---- Repo discovery / creation --------------------------------------
    detectRepo: async (filePath) => {
      set({ state: { kind: 'initializing' } });
      await runOrError(async () => {
        const result = await gitClient.detect(filePath);
        if (result.mode === 'none') {
          set({ state: { kind: 'no-repo' } });
          return;
        }
        set({ state: { kind: 'ready', repo: metaFromOpenInfo(result) } });
        // Hydrate the placeholder fields in metaFromOpenInfo (currentBranch,
        // branches, workingDirty, ahead/behind) by polling status + branches.
        // Also reconciles in-flight merge state if the backend reports one.
        await get().refreshStatus();
        await get().refreshBranches();
      });
    },

    initRepo: async (filePath) => {
      set({ state: { kind: 'initializing' } });
      await runOrError(async () => {
        const info = await gitClient.init(filePath);
        set({ state: { kind: 'ready', repo: metaFromOpenInfo(info) } });
        await get().refreshStatus();
        await get().refreshBranches();
      });
    },

    openRepo: async (repoPath, currentFilePath) => {
      set({ state: { kind: 'initializing' } });
      await runOrError(async () => {
        const info = await gitClient.open(repoPath, currentFilePath);
        const meta = metaFromOpenInfo(info);
        if (info.trackedFilePath === null) {
          set({ state: { kind: 'needs-tracked-file', repo: meta } });
        } else {
          set({ state: { kind: 'ready', repo: meta } });
        }
        // refreshStatus + refreshBranches both work in needs-tracked-file
        // (requireRepoId accepts it). They populate currentBranch / branches /
        // dirty counts even before the user picks a tracked file, so the
        // picker can show "main · 3 ahead" header info.
        await get().refreshStatus();
        await get().refreshBranches();
      });
    },

    cloneRepo: async (opts) => {
      set({ state: { kind: 'initializing' } });
      await runOrError(async () => {
        const info = await gitClient.clone(opts);
        // Per spec line 109: clone ALWAYS lands in needs-tracked-file.
        set({ state: { kind: 'needs-tracked-file', repo: metaFromOpenInfo(info) } });
        await get().refreshStatus();
        await get().refreshBranches();
      });
    },

    bindTrackedFile: async (filePath) => {
      const state = get().state;
      if (state.kind !== 'needs-tracked-file' && state.kind !== 'ready') {
        throw new GitError('no-file', 'No repo to bind tracked file to', {
          recoverable: false,
        });
      }
      const repoId = state.repo.repoId;
      await runOrError(async () => {
        await gitClient.bindTrackedFile(repoId, filePath);
        // Transition needs-tracked-file → ready.
        set((s) => {
          if (s.state.kind === 'needs-tracked-file') {
            return {
              state: {
                kind: 'ready',
                repo: { ...s.state.repo, trackedFilePath: filePath },
              },
            };
          }
          if (s.state.kind === 'ready') {
            return {
              state: { ...s.state, repo: { ...s.state.repo, trackedFilePath: filePath } },
            };
          }
          return s;
        });
        // After binding, status() can return file-specific dirty info (the
        // backend's engineStatus uses session.trackedFilePath to compute
        // workingDirty against the autosave-ref blob).
        await get().refreshStatus();
      });
    },

    refreshCandidates: async () => {
      const repoId = requireRepoId(get().state);
      const candidates = await gitClient.listCandidates(repoId);
      set((s) => {
        if (
          s.state.kind === 'ready' ||
          s.state.kind === 'conflict' ||
          s.state.kind === 'needs-tracked-file'
        ) {
          return {
            state: { ...s.state, repo: { ...s.state.repo, candidateFiles: candidates } },
          };
        }
        return s;
      });
    },

    closeRepo: async () => {
      const state = get().state;
      // Every state that holds a RepoMeta has an active main-process session
      // — including needs-tracked-file. Calling close() on all of them
      // prevents session leaks when the user opens or clones a repo and then
      // closes the panel before binding a tracked file.
      if (
        state.kind === 'ready' ||
        state.kind === 'conflict' ||
        state.kind === 'needs-tracked-file'
      ) {
        try {
          await gitClient.close(state.repo.repoId);
        } catch {
          // Best-effort: even if close fails (e.g. backend already cleaned
          // up the session), we still want to reset the renderer state to
          // avoid a stale UI. Swallow and continue.
        }
      }
      set({ state: { kind: 'no-file' }, log: [] });
    },

    // ---- Status / log / diff --------------------------------------------
    refreshStatus: async () => {
      const repoId = requireRepoId(get().state);
      const status = await gitClient.status(repoId);

      // Step 1: copy the basic repo fields into RepoMeta. Applies to all
      // states that hold a repo (ready / conflict / needs-tracked-file).
      set((s) => {
        if (
          s.state.kind === 'ready' ||
          s.state.kind === 'conflict' ||
          s.state.kind === 'needs-tracked-file'
        ) {
          return {
            state: {
              ...s.state,
              repo: {
                ...s.state.repo,
                currentBranch: status.branch,
                workingDirty: status.workingDirty,
                otherFilesDirty: status.otherFilesDirty,
                otherFilesPaths: status.otherFilesPaths,
                ahead: status.ahead,
                behind: status.behind,
              },
            },
          };
        }
        return s;
      });

      // Step 2: reconcile the conflict state. Phase 2c's engineStatus
      // populates `mergeInProgress` and `conflicts` from the backend
      // session's inflightMerge. We mirror that into the renderer state
      // machine so a panel reopened mid-merge sees the conflict view.
      const current = get().state;
      if (status.mergeInProgress && status.conflicts) {
        // Backend reports an in-flight merge. Promote ready → conflict, or
        // refresh the conflict bag if we're already in conflict (e.g.
        // resolutions changed since last status).
        if (current.kind === 'ready') {
          set({
            state: {
              kind: 'conflict',
              repo: current.repo,
              conflicts: hydrateConflictBag(status.conflicts),
            },
          });
        } else if (current.kind === 'conflict') {
          set({
            state: {
              ...current,
              conflicts: hydrateConflictBag(status.conflicts),
            },
          });
        }
      } else if (!status.mergeInProgress && current.kind === 'conflict') {
        // Backend says no merge in flight, but the renderer is in conflict
        // state — the merge was finalized externally (e.g. terminal git, or
        // applyMerge from another window). Transition back to ready.
        set({ state: { kind: 'ready', repo: current.repo } });
      }
    },

    loadLog: async (opts) => {
      const repoId = requireRepoId(get().state);
      const commits = await gitClient.log(repoId, opts);
      set({ log: commits });
    },

    computeDiff: async (from, to) => {
      const repoId = requireRepoId(get().state);
      return gitClient.diff(repoId, from, to);
    },

    // ---- Commit / restore / promote (gated) -----------------------------
    commitMilestone: async (message, author) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.commit(repoId, { kind: 'milestone', message, author });
      }, 'commit milestone');
    },

    commitAutosave: async (message, author) => {
      const repoId = requireRepoId(get().state);
      // Autosave is not in the withCleanWorkingTree set per spec — the
      // autosave subscriber (Phase 4) runs AFTER a successful save, so the
      // document is clean by construction.
      await gitClient.commit(repoId, { kind: 'autosave', message, author });
    },

    restoreCommit: async (commitHash) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.restore(repoId, commitHash);
      }, 'restore');
    },

    promoteAutosave: async (autosaveHash, message, author) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.promote(repoId, autosaveHash, message, author);
      }, 'promote autosave');
    },

    // ---- Branches -------------------------------------------------------
    refreshBranches: async () => {
      const repoId = requireRepoId(get().state);
      const branches = await gitClient.branchList(repoId);
      set((s) => {
        if (
          s.state.kind === 'ready' ||
          s.state.kind === 'conflict' ||
          s.state.kind === 'needs-tracked-file'
        ) {
          return { state: { ...s.state, repo: { ...s.state.repo, branches } } };
        }
        return s;
      });
    },

    createBranch: async (opts) => {
      const repoId = requireRepoId(get().state);
      await gitClient.branchCreate(repoId, opts);
      await get().refreshBranches();
    },

    switchBranch: async (name) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.branchSwitch(repoId, name);
        await get().refreshStatus();
        await get().refreshBranches();
      }, 'switch branch');
    },

    deleteBranch: async (name) => {
      const repoId = requireRepoId(get().state);
      await gitClient.branchDelete(repoId, name);
      await get().refreshBranches();
    },

    mergeBranch: async (fromBranch) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        const result = await gitClient.branchMerge(repoId, fromBranch);
        if (result.result === 'conflict' && result.conflicts) {
          set((s) => {
            if (s.state.kind === 'ready') {
              return {
                state: {
                  kind: 'conflict',
                  repo: s.state.repo,
                  conflicts: hydrateConflictBag(result.conflicts!),
                },
              };
            }
            return s;
          });
          // On conflict we do NOT refresh — the conflict state is already
          // hydrated from the result above, and refreshStatus would be
          // redundant (it would just re-confirm mergeInProgress=true).
          return;
        }
        // Success paths (fast-forward, merge) advance HEAD and may update the
        // branch list — refresh both so the panel header and branch dropdown
        // stay in sync without waiting for a user-triggered refresh.
        await get().refreshStatus();
        await get().refreshBranches();
      }, 'merge branch');
    },

    // ---- Merge orchestration --------------------------------------------
    resolveConflict: async (conflictId, choice) => {
      const state = get().state;
      if (state.kind !== 'conflict') {
        throw new GitError('engine-crash', 'resolveConflict called outside conflict state', {
          recoverable: false,
        });
      }
      await gitClient.resolveConflict(state.repo.repoId, conflictId, choice);
      // Update the local Map with the recorded resolution.
      set((s) => {
        if (s.state.kind !== 'conflict') return s;
        const nodeConflicts = new Map(s.state.conflicts.nodeConflicts);
        const docFieldConflicts = new Map(s.state.conflicts.docFieldConflicts);
        if (nodeConflicts.has(conflictId)) {
          const c = nodeConflicts.get(conflictId)!;
          nodeConflicts.set(conflictId, { ...c, resolution: choice });
        } else if (docFieldConflicts.has(conflictId)) {
          const c = docFieldConflicts.get(conflictId)!;
          docFieldConflicts.set(conflictId, { ...c, resolution: choice });
        }
        return { state: { ...s.state, conflicts: { nodeConflicts, docFieldConflicts } } };
      });
    },

    applyMerge: async () => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.applyMerge(repoId);
        // Transition conflict → ready.
        set((s) => {
          if (s.state.kind === 'conflict') {
            return { state: { kind: 'ready', repo: s.state.repo } };
          }
          return s;
        });
      }, 'apply merge');
    },

    abortMerge: async () => {
      const repoId = requireRepoId(get().state);
      await gitClient.abortMerge(repoId);
      set((s) => {
        if (s.state.kind === 'conflict') {
          return { state: { kind: 'ready', repo: s.state.repo } };
        }
        return s;
      });
    },

    // ---- Remote ---------------------------------------------------------
    fetchRemote: async (auth) => {
      const repoId = requireRepoId(get().state);
      await gitClient.fetch(repoId, auth);
      await get().refreshStatus();
    },

    pull: async (auth) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        const result = await gitClient.pull(repoId, auth);
        if (result.result === 'conflict' && result.conflicts) {
          set((s) => {
            if (s.state.kind === 'ready') {
              return {
                state: {
                  kind: 'conflict',
                  repo: s.state.repo,
                  conflicts: hydrateConflictBag(result.conflicts!),
                },
              };
            }
            return s;
          });
        }
        await get().refreshStatus();
      }, 'pull');
    },

    push: async (auth) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.push(repoId, auth);
        await get().refreshStatus();
      }, 'push');
    },

    // ---- Auth -----------------------------------------------------------
    storeAuth: (host, creds) => gitClient.authStore(host, creds),
    getAuth: (host) => gitClient.authGet(host),
    clearAuth: (host) => gitClient.authClear(host),

    // ---- SSH keys -------------------------------------------------------
    refreshSshKeys: async () => {
      const keys = await gitClient.sshListKeys();
      set({ sshKeys: keys });
    },
    generateSshKey: async (opts) => {
      const key = await gitClient.sshGenerateKey(opts);
      await get().refreshSshKeys();
      return key;
    },
    importSshKey: async (opts) => {
      const key = await gitClient.sshImportKey(opts);
      await get().refreshSshKeys();
      return key;
    },
    deleteSshKey: async (keyId) => {
      await gitClient.sshDeleteKey(keyId);
      await get().refreshSshKeys();
    },

    // ---- Retry queued action --------------------------------------------
    retrySaveRequired: async () => {
      const state = get().state;
      if (state.kind !== 'ready' && state.kind !== 'conflict') return;
      const pending = state.saveRequiredFor;
      if (!pending) return;
      // Save first via the document store.
      const saved = await useDocumentStore.getState().save();
      if (!saved) return;
      // Clear the pending flag, then re-run.
      set((s) => {
        if (s.state.kind === 'ready' || s.state.kind === 'conflict') {
          // eslint-disable-next-line @typescript-eslint/no-unused-vars
          const { saveRequiredFor: _omit, ...rest } = s.state;
          return { state: rest as GitState };
        }
        return s;
      });
      await pending.run();
    },
  };
});

// Test-only helper for resetting the store between tests.
export function __resetGitStore(): void {
  useGitStore.setState({
    state: { kind: 'no-file' },
    panelOpen: false,
    log: [],
    sshKeys: [],
  });
}
