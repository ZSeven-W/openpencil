// apps/web/src/stores/git-store.ts
//
// Zustand store implementing the GitState state machine. Every mutating
// action goes through withCleanWorkingTree so the renderer can never
// overwrite disk with an out-of-sync tree. Pure helpers and the dirty/
// runOrError wrappers live in git-store-helpers.ts to keep this file under
// the 800-LoC cap.
//
// NOTE: this file is ~848 lines (48 over cap). Phase 7c extracted
// makeReloadAfterApply to git-store-helpers.ts, but applyMerge's reload
// orchestration and noop handling added lines back. Further extraction
// deferred — see Phase 8+ for a dedicated refactor.

import { create } from 'zustand';
import { gitClient } from '@/services/git-client';
import { GitError, isGitError } from '@/services/git-error';
import { useDocumentStore } from '@/stores/document-store';
import { documentEvents } from '@/utils/document-events';
import { loadOpFileFromPath } from '@/utils/load-op-file';
import {
  buildConflictState,
  classifyCloneError,
  currentLogRef,
  dropSaveRequired,
  makeAutosaveHandler,
  makeReloadAfterApply,
  makeSyncAfterHeadMove,
  metaFromOpenInfo,
  patchRepoRemote,
  requireRepoId,
  resolveAuthorIdentity,
} from './git-store-helpers';
import type { GitStore, PendingAction } from './git-store-types';

export const useGitStore = create<GitStore>((set, get) => {
  /**
   * Phase 6b: shared head-move sync (refreshStatus + refreshBranches +
   * reload tracked file + loadLog). Called from pull/switchBranch/mergeBranch
   * clean paths so all three head-moving actions stay in lockstep.
   */
  const syncAfterHeadMove = makeSyncAfterHeadMove(get);

  /**
   * Phase 7c: post-apply reload (reload tracked file + refreshStatus +
   * loadLog). Called from applyMerge() on both the normal-success and noop
   * paths. Extracted to git-store-helpers.ts to keep this file under the cap.
   */
  const reloadAfterApply = makeReloadAfterApply(get);

  /**
   * Guard a mutating action on `useDocumentStore.getState().isDirty`. Dirty →
   * stash a PendingAction and throw GitError('save-required'); the UI shows
   * an inline alert and retrySaveRequired re-runs the action after save.
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

  /** Run an action; thrown GitError transitions to the generic error state. */
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
      set({ state: { kind: 'error', message, recoverable } });
      throw err;
    }
  }

  return {
    state: { kind: 'no-file' },
    panelOpen: false,
    log: [],
    sshKeys: [],

    // Phase 4a: author identity (loadAuthorIdentity runs the lookup chain)
    authorIdentity: null,
    authorPromptVisible: false,

    // Phase 4b: auto-bind banner flag (set by openRepo/cloneRepo when
    // auto-binding a single candidate; cleared by acknowledge actions)
    lastAutoBindedPath: null,

    // Phase 4c: commit input draft (ephemeral)
    commitMessage: '',

    // Phase 4c: autosave error display (last error from the subscriber)
    autosaveError: null,

    // Phase 4c: subscriber lifecycle handle (internal)
    __autosaveUnsub: null,

    // ---- Panel lifecycle -------------------------------------------------
    togglePanel: () => set((s) => ({ panelOpen: !s.panelOpen })),
    openPanel: () => set({ panelOpen: true }),
    closePanel: () => set({ panelOpen: false }),

    // ---- Phase 4a: author identity actions ------------------------------
    loadAuthorIdentity: async () => {
      // The resolution chain (prefs → system git → null) lives in
      // git-store-helpers.ts to keep this file under the 800-LoC cap.
      const id = await resolveAuthorIdentity();
      set({ authorIdentity: id });
    },

    setAuthorIdentity: async (name, email) => {
      // Persist to OpenPencil prefs first so a panel reopen rehydrates from
      // them in step 1 of the chain. If preference IPC fails (e.g. browser
      // mode), still update the in-memory cache so the current session
      // works. SSR guard: skip the IPC entirely (bare `window` would
      // ReferenceError) but still update the in-memory cache.
      if (typeof window !== 'undefined') {
        try {
          await window.electronAPI?.setPreference('git.authorName', name);
          await window.electronAPI?.setPreference('git.authorEmail', email);
        } catch {
          /* swallow — in-memory cache below still serves the session */
        }
      }
      set({ authorIdentity: { name, email } });
    },

    showAuthorPrompt: () => set({ authorPromptVisible: true }),
    hideAuthorPrompt: () => set({ authorPromptVisible: false }),

    // ---- Phase 4b: auto-bind banner actions ------------------------------
    acknowledgeAutoBind: () => set({ lastAutoBindedPath: null }),
    acknowledgeAutoBindAndOpen: async () => {
      const path = get().lastAutoBindedPath;
      if (!path) return;
      // Load the file into the editor via the shared helper. Fire-and-
      // forget — failures are silent (the helper returns false but the
      // banner clears regardless to avoid nagging).
      await loadOpFileFromPath(path);
      set({ lastAutoBindedPath: null });
    },

    // ---- Phase 4c: commit input actions ---------------------------------
    setCommitMessage: (text) => set({ commitMessage: text }),
    clearCommitMessage: () => set({ commitMessage: '' }),
    cancelSaveRequired: () => set((s) => ({ state: dropSaveRequired(s.state) })),

    // ---- Phase 4c: overflow menu actions --------------------------------
    enterTrackedFilePicker: () =>
      set((s) => {
        if (s.state.kind !== 'ready') return s;
        return { state: { kind: 'needs-tracked-file', repo: s.state.repo } };
      }),

    // ---- Phase 7b: exit the tracked-file picker -------------------------
    exitTrackedFilePicker: async () => {
      const state = get().state;
      if (state.kind !== 'needs-tracked-file') return;
      if (state.repo.trackedFilePath !== null) {
        // Entered from ready (re-binding): go back to ready.
        set({ state: { kind: 'ready', repo: state.repo } });
      } else {
        // Entered as first post-open/clone screen: close the transient
        // session and return to no-file so the empty state renders.
        try {
          await gitClient.close(state.repo.repoId);
        } catch {
          // Best-effort: even if close fails, reset state to avoid a stale UI.
        }
        set({ state: { kind: 'no-file' } });
      }
    },

    clearAuthorIdentity: async () => {
      // Remove the OpenPencil prefs first so a reload doesn't rehydrate.
      // We must REMOVE the keys (not `setPreference(..., '')`), otherwise
      // the lookup chain in resolveAuthorIdentity will see empty-string
      // sentinels on disk and treat them as set-but-blank instead of
      // absent — diverging from the documented "clear cache AND remove
      // both prefs keys" contract.
      if (typeof window !== 'undefined') {
        try {
          await window.electronAPI?.removePreference('git.authorName');
          await window.electronAPI?.removePreference('git.authorEmail');
        } catch {
          /* swallow — in-memory clear below still wins for this session */
        }
      }
      set({ authorIdentity: null });
    },

    // ---- Phase 4c: autosave subscriber lifecycle ------------------------
    initAutosaveSubscriber: () => {
      // Idempotent: if already wired, return.
      if (get().__autosaveUnsub !== null) return;
      const handler = makeAutosaveHandler(get, set);
      const unsub = documentEvents.on('saved', handler);
      set({ __autosaveUnsub: unsub });
    },

    disposeAutosaveSubscriber: () => {
      const unsub = get().__autosaveUnsub;
      if (unsub) {
        unsub();
        set({ __autosaveUnsub: null });
      }
    },

    clearAutosaveError: () => set({ autosaveError: null }),

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
        // branches, workingDirty, ahead/behind, remote) by polling status,
        // branches, and remote metadata. Also reconciles in-flight merge
        // state if the backend reports one.
        await get().refreshStatus();
        await get().refreshBranches();
        await get().refreshRemote();
      });
    },

    initRepo: async (filePath) => {
      set({ state: { kind: 'initializing' } });
      await runOrError(async () => {
        const info = await gitClient.init(filePath);
        set({ state: { kind: 'ready', repo: metaFromOpenInfo(info) } });
        await get().refreshStatus();
        await get().refreshBranches();
        await get().refreshRemote();
      });
    },

    openRepo: async (repoPath, currentFilePath) => {
      set({ state: { kind: 'initializing' } });
      await runOrError(async () => {
        const info = await gitClient.open(repoPath, currentFilePath);

        // Phase 4b auto-bind: if the repo has exactly one candidate and
        // open() didn't already set trackedFilePath, bind it now and skip
        // the picker entirely. Surface the auto-bind banner so the user
        // can also load the file into the editor if they want.
        if (info.trackedFilePath === null && info.candidates.length === 1) {
          const only = info.candidates[0];
          await gitClient.bindTrackedFile(info.repoId, only.path);
          set({
            state: {
              kind: 'ready',
              repo: { ...metaFromOpenInfo(info), trackedFilePath: only.path },
            },
            lastAutoBindedPath: only.path,
          });
          await get().refreshStatus();
          await get().refreshBranches();
          await get().refreshRemote();
          return;
        }

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
        await get().refreshRemote();
      });
    },

    cloneRepo: async (opts) => {
      // Phase 6a: a wizard-launched clone catches recoverable errors inline
      // (so the form keeps its state for retry); a CLI-driven clone treats
      // every code as fatal. classifyCloneError() encodes that policy.
      //
      // CRITICAL: when entering from the wizard we must NOT transition to
      // `initializing` mid-flight — that would unmount the <GitPanelCloneForm>
      // and wipe the URL/dest/token inputs on a recoverable retry. Instead we
      // stay in `wizard-clone` and flip a `busy` flag the form reads as its
      // loading indicator.
      const prevWasWizard = get().state.kind === 'wizard-clone';
      if (prevWasWizard) {
        set({ state: { kind: 'wizard-clone', busy: true, error: null } });
      } else {
        set({ state: { kind: 'initializing' } });
      }
      try {
        const info = await gitClient.clone(opts);

        // Phase 4b auto-bind: single candidate → ready + banner. Multi /
        // zero candidates land in needs-tracked-file per spec line 109. Both
        // branches naturally leave the wizard, so the form unmounts cleanly.
        if (info.candidates.length === 1) {
          const only = info.candidates[0];
          await gitClient.bindTrackedFile(info.repoId, only.path);
          set({
            state: {
              kind: 'ready',
              repo: { ...metaFromOpenInfo(info), trackedFilePath: only.path },
            },
            lastAutoBindedPath: only.path,
          });
        } else {
          set({ state: { kind: 'needs-tracked-file', repo: metaFromOpenInfo(info) } });
        }
        await get().refreshStatus();
        await get().refreshBranches();
        await get().refreshRemote();
      } catch (err) {
        const decision = classifyCloneError(err, prevWasWizard);
        if (decision.kind === 'inline') {
          // Keep the wizard mounted with the form state intact; flip busy
          // off and surface the inline banner so the user can retry.
          set({
            state: {
              kind: 'wizard-clone',
              busy: false,
              error: { code: decision.code, message: decision.message },
            },
          });
          return;
        }
        set({
          state: {
            kind: 'error',
            message: decision.message,
            recoverable: decision.recoverable,
          },
        });
        throw err;
      }
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
      set({ state: { kind: 'no-file' }, log: [], lastAutoBindedPath: null });
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
      // populates `mergeInProgress`, `conflicts`, and (Phase 6b)
      // `unresolvedFiles`. We mirror all three into the renderer state
      // machine so a panel reopened mid-merge sees the conflict view
      // AND the non-`.op` file banner.
      const current = get().state;
      const unresolved = status.unresolvedFiles ?? [];
      const reopenedMidMerge = status.reopenedMidMerge ?? false;
      // I2: also enter conflict state for the panel-reopen degraded mode —
      // mergeInProgress + reopenedMidMerge is true even when unresolvedFiles
      // is empty (tracked .op was filtered out) and conflicts is null.
      if (
        status.mergeInProgress &&
        (status.conflicts || unresolved.length > 0 || reopenedMidMerge)
      ) {
        // Backend reports an in-flight merge.
        if (current.kind === 'conflict') {
          // Already in conflict state: preserve in-memory resolutions and
          // finalizeError — the .op conflict bag does not mutate during a
          // merge session, and the user's resolution choices must survive
          // the 3-second polling cycle. Only unresolvedFiles can change as
          // the user resolves non-.op files externally.
          set({
            state: {
              ...current,
              unresolvedFiles: unresolved,
              reopenedMidMerge,
            },
          });
        } else if (current.kind === 'ready') {
          // Promote ready → conflict with a fresh bag (new entry into conflict state).
          set({
            state: buildConflictState(
              current.repo,
              status.conflicts ?? null,
              unresolved,
              null,
              reopenedMidMerge,
            ),
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
        // Phase 4c: refresh the log and clear the draft on success so
        // the history list shows the new commit and the input empties.
        await get().loadLog({ ref: currentLogRef(get()), limit: 50 });
        get().clearCommitMessage();
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
        // The IPC overwrote the tracked .op file on disk. Reload it into
        // document-store so the in-memory document matches the restored
        // tree — otherwise the next Cmd+S / autosave would write the old
        // in-memory content back to disk, silently undoing the restore.
        // HEAD itself is unchanged by restore, so the log does not need
        // a refresh.
        const state = get().state;
        if ((state.kind === 'ready' || state.kind === 'conflict') && state.repo.trackedFilePath) {
          await loadOpFileFromPath(state.repo.trackedFilePath);
        }
      }, 'restore');
    },

    promoteAutosave: async (autosaveHash, message, author) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.promote(repoId, autosaveHash, message, author);
        // Promote writes a new milestone commit at the autosave's tree.
        // Reload the document for the same reason as restoreCommit — the
        // on-disk tree may diverge from the in-memory document.
        const state = get().state;
        if ((state.kind === 'ready' || state.kind === 'conflict') && state.repo.trackedFilePath) {
          await loadOpFileFromPath(state.repo.trackedFilePath);
        }
        await get().loadLog({ ref: currentLogRef(get()), limit: 50 });
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
        // HEAD moved. syncAfterHeadMove refreshes status/branches, reloads
        // the on-disk tracked file into document-store, and refreshes the
        // history list for the now-active branch.
        await syncAfterHeadMove();
      }, 'switch branch');
    },

    deleteBranch: async (name, opts) => {
      const repoId = requireRepoId(get().state);
      await gitClient.branchDelete(repoId, name, opts);
      await get().refreshBranches();
    },

    mergeBranch: async (fromBranch) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        const result = await gitClient.branchMerge(repoId, fromBranch);
        if (result.result === 'conflict' && result.conflicts) {
          set((s) => {
            if (s.state.kind !== 'ready') return s;
            return { state: buildConflictState(s.state.repo, result.conflicts!, []) };
          });
          // Conflict path: state is fully hydrated — skip the sync cascade.
          return;
        }
        if (result.result === 'conflict-non-op') {
          // I3: non-.op conflict — merge is in flight but engine couldn't apply
          // .op merge because non-`.op` files are unresolved. refreshStatus
          // performs the full repo-meta update AND promotes ready → conflict
          // with the unresolvedFiles list via the shared mergeInProgress branch.
          await get().refreshStatus();
          return;
        }
        // Success paths (fast-forward, merge): HEAD moved. Delegate the
        // cascade to the shared helper (see switchBranch for details).
        await syncAfterHeadMove();
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
      // Update the local Map with the recorded resolution. Also clear any
      // stale finalizeError so the banner doesn't show the old error after
      // the user fixes another conflict.
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
        return {
          state: {
            ...s.state,
            conflicts: { nodeConflicts, docFieldConflicts },
            finalizeError: null,
          },
        };
      });
    },

    applyMerge: async () => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        try {
          await gitClient.applyMerge(repoId);
        } catch (err) {
          // Phase 7b: `merge-still-conflicted` surfaces inline on the banner
          // rather than transitioning to the generic error card. The user must
          // resolve remaining conflicts and retry applyMerge.
          if (isGitError(err) && err.code === 'merge-still-conflicted') {
            set((s) => {
              if (s.state.kind === 'conflict') {
                return { state: { ...s.state, finalizeError: err.message } };
              }
              return s;
            });
            // Immediately refresh status so the unresolved-file list is current.
            await get().refreshStatus();
            return; // do NOT re-throw — banner owns the error display
          }
          throw err;
        }
        // Phase 7c: success (including noop: true) → transition conflict → ready
        // and clear any stale finalizeError, then reload the tracked .op file
        // and refresh the history log.
        set((s) => {
          if (s.state.kind === 'conflict') {
            return { state: { kind: 'ready', repo: s.state.repo } };
          }
          return s;
        });
        // Reload the tracked file and refresh log. reloadAfterApply reads the
        // current state (now 'ready') so it can find trackedFilePath.
        await reloadAfterApply();
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
        if (result.result === 'fast-forward' || result.result === 'merge') {
          // Clean head-move. Delegate the cascade so pull behaves like
          // switchBranch / mergeBranch success paths — refresh status +
          // branches, reload the tracked .op file, refresh the log.
          await syncAfterHeadMove();
          return;
        }
        if (result.result === 'conflict') {
          // `.op` conflict bag. Transition ready → conflict with no
          // unresolved non-op files; the manual-resolution UI covers
          // everything here (landing in Phase 7).
          set((s) => {
            if (s.state.kind !== 'ready') return s;
            return { state: buildConflictState(s.state.repo, result.conflicts ?? null, []) };
          });
          return;
        }
        // result === 'conflict-non-op': the merge is in flight but the
        // engine could not apply the .op merge because non-`.op` files
        // are unresolved. refreshStatus performs the full repo-meta
        // update (branch / ahead / behind / working dirty) AND promotes
        // ready → conflict with the unresolvedFiles list via the shared
        // mergeInProgress branch — no manual state build needed.
        await get().refreshStatus();
      }, 'pull');
    },

    push: async (auth) => {
      const repoId = requireRepoId(get().state);
      // Note: push IPC currently throws GitError('push-rejected') or
      // GitError('auth-failed') on failure rather than returning a tagged
      // result. We let those escape from here (not via runOrError) so the
      // remote-controls button can catch and branch on err.code: a rejected
      // push opens the "pull first" retry strip; an auth-failed push opens
      // the shared auth form. Anything else propagates as a normal throw
      // and the button shows a compact inline error.
      await withCleanWorkingTree(async () => {
        await gitClient.push(repoId, auth);
        // Success: refresh status so ahead/behind zero out and the "nothing
        // to push" hint takes over. No head move → no syncAfterHeadMove.
        await get().refreshStatus();
      }, 'push');
    },

    // ---- Auth -----------------------------------------------------------
    storeAuth: (host, creds) => gitClient.authStore(host, creds),
    getAuth: (host) => gitClient.authGet(host),
    clearAuth: (host) => gitClient.authClear(host),

    // ---- Phase 6a: clone wizard + remote metadata -----------------------
    enterCloneWizard: () => set({ state: { kind: 'wizard-clone', busy: false, error: null } }),

    cancelCloneWizard: () => {
      // Always land in no-file. The git-panel.tsx detect-repo effect will
      // immediately rehydrate the correct no-repo / ready state from the
      // currently-open document path on the next render.
      set({ state: { kind: 'no-file' } });
    },

    refreshRemote: async () => {
      const state = get().state;
      if (
        state.kind !== 'ready' &&
        state.kind !== 'conflict' &&
        state.kind !== 'needs-tracked-file'
      ) {
        return;
      }
      const remote = await gitClient.remoteGet(state.repo.repoId);
      set((s) => ({ state: patchRepoRemote(s.state, remote) }));
    },

    setRemoteUrl: async (url) => {
      const repoId = requireRepoId(get().state);
      // Normalize empty/whitespace-only strings to null so the desktop
      // side can treat blank input as "remove origin" — form layer
      // doesn't have to coerce.
      const normalized = url === null || url.trim() === '' ? null : url.trim();
      const remote = await gitClient.remoteSet(repoId, normalized);
      // Update renderer state IMMEDIATELY from the IPC return value. Per
      // the Phase 6a contract, callers MUST NOT rely on a follow-up
      // refreshRemote() to see the new value.
      set((s) => ({ state: patchRepoRemote(s.state, remote) }));
    },

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
      set((s) => ({ state: dropSaveRequired(s.state) }));
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
    authorIdentity: null,
    authorPromptVisible: false,
    lastAutoBindedPath: null,
    commitMessage: '',
    autosaveError: null,
    __autosaveUnsub: null,
  });
}
