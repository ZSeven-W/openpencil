// apps/web/src/stores/git-store-helpers.ts
//
// Pure helper functions and constants extracted from git-store.ts to
// keep that file under the 800-LoC cap. None of these touch the Zustand
// store directly — they are stateless pure functions consumed by the
// store actions in git-store.ts.

import { gitClient } from '@/services/git-client';
import { GitError, isGitError } from '@/services/git-error';
import { loadOpFileFromPath } from '@/utils/load-op-file';
import type {
  GitConflictBag,
  GitConflictResolution,
  GitRemoteInfo,
  GitRepoOpenInfo,
} from '@/services/git-types';
import {
  CLONE_INLINE_ERROR_CODES,
  REMOTE_AUTH_ERROR_CODES,
  type CloneInlineErrorCode,
  type ConflictBagState,
  type GitState,
  type GitStore,
  type RemoteAuthErrorCode,
  type RepoMeta,
} from './git-store-types';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Build an empty RepoMeta shell from a GitRepoOpenInfo. Branches and status
 * fields are filled in by refreshStatus + refreshBranches which the store
 * calls after every init/open/clone/bind.
 */
export function metaFromOpenInfo(info: GitRepoOpenInfo): RepoMeta {
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
    remote: null, // refreshRemote() will populate this
  };
}

/**
 * Hydrate a wire-format GitConflictBag into the Map-backed ConflictBagState.
 */
export function hydrateConflictBag(bag: GitConflictBag): ConflictBagState {
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
 * Patch the cached remote on whichever repo-bearing state is active. No-op
 * outside `ready`/`conflict`/`needs-tracked-file`. Used by `refreshRemote`
 * and `setRemoteUrl` so they don't have to inline the same narrowing twice.
 */
export function patchRepoRemote(state: GitState, remote: GitRemoteInfo): GitState {
  if (state.kind === 'ready' || state.kind === 'conflict' || state.kind === 'needs-tracked-file') {
    return { ...state, repo: { ...state.repo, remote } };
  }
  return state;
}

/**
 * Strip the `saveRequiredFor` pending action from a `ready`/`conflict`
 * state. No-op on any other variant. Extracted so the store's
 * `cancelSaveRequired` and `retrySaveRequired` don't have to repeat the
 * same destructure-and-cast dance (shaves ~10 lines off git-store.ts, which
 * sits at the 800-LoC cap).
 */
export function dropSaveRequired(state: GitState): GitState {
  if (state.kind === 'ready' || state.kind === 'conflict') {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const { saveRequiredFor: _omit, ...rest } = state;
    return rest as GitState;
  }
  return state;
}

/**
 * Get the current repoId or throw. Most actions require an active session.
 */
export function requireRepoId(state: GitState): string {
  if (state.kind === 'ready' || state.kind === 'conflict' || state.kind === 'needs-tracked-file') {
    return state.repo.repoId;
  }
  throw new GitError('no-file', 'No active repository for this action', {
    recoverable: false,
  });
}

/**
 * Resolve the author identity through the documented chain:
 *   1. OpenPencil prefs (`git.authorName` + `git.authorEmail`)
 *   2. System git config via gitClient.getSystemAuthor()
 *   3. null (the inline author form takes over)
 *
 * Extracted from git-store.ts to keep the store file under the 800-LoC
 * cap. The action wrapper in the store calls this and `set()`s the result.
 */
export async function resolveAuthorIdentity(): Promise<{ name: string; email: string } | null> {
  // SSR guard — bare `window` would ReferenceError even under ?.
  if (typeof window === 'undefined') return null;

  // Step 1: prefs
  try {
    const prefs = await window.electronAPI?.getPreferences();
    const prefName = prefs?.['git.authorName'];
    const prefEmail = prefs?.['git.authorEmail'];
    if (prefName && prefEmail) {
      return { name: prefName, email: prefEmail };
    }
  } catch {
    // prefs unavailable — fall through
  }

  // Step 2: system git config (only if Electron + sys git is wired)
  if (window.electronAPI?.git) {
    try {
      const sys = await gitClient.getSystemAuthor();
      if (sys) return sys;
    } catch {
      // sys git unavailable / config not set — fall through
    }
  }

  return null;
}

/**
 * Classify a thrown clone error. Used by `cloneRepo` in the store to
 * decide whether the error should stay inline on the wizard (recoverable
 * codes when `prevWasWizard` is true) or transition to the generic error
 * card. Returns a discriminated descriptor — the caller does the actual
 * `set()`.
 *
 * `prevWasWizard` is the snapshot of `state.kind === 'wizard-clone'`
 * captured BEFORE the clone IPC. The two-bucket policy is documented in
 * the Phase 6a plan: a CLI-driven clone (no wizard mounted) treats every
 * code as fatal so the user doesn't get teleported into a wizard they
 * never opened.
 */
export function classifyCloneError(
  err: unknown,
  prevWasWizard: boolean,
):
  | { kind: 'inline'; code: CloneInlineErrorCode; message: string }
  | { kind: 'fatal'; message: string; recoverable: boolean } {
  if (prevWasWizard && isGitError(err)) {
    const code = err.code;
    if ((CLONE_INLINE_ERROR_CODES as readonly string[]).includes(code)) {
      return {
        kind: 'inline',
        code: code as CloneInlineErrorCode,
        message: err.message,
      };
    }
  }
  const message = isGitError(err) ? err.message : err instanceof Error ? err.message : String(err);
  const recoverable = isGitError(err) ? err.recoverable : false;
  return { kind: 'fatal', message, recoverable };
}

/**
 * Build the autosave subscriber handler. Extracted from git-store.ts to keep
 * that file under the 800-LoC cap. The returned async function:
 *   - reads the current store state via the captured `get`/`set`
 *   - silently no-ops outside `ready` state, on browser-download saves, and
 *     when the saved file isn't the bound tracked file
 *   - composes a minimal "auto: HH:MM" message and commits an autosave
 *   - records any error in `autosaveError` instead of throwing
 *
 * Spec §"Autosave failures are silent" — we never re-throw from here.
 */
export function makeAutosaveHandler(
  get: () => GitStore,
  set: (partial: Partial<GitStore>) => void,
): (event: { filePath: string | null; fileName: string }) => Promise<void> {
  return async (event) => {
    const current = get().state;
    if (current.kind !== 'ready') return; // silent no-op
    if (event.filePath === null) return; // browser-download fallback
    if (event.filePath !== current.repo.trackedFilePath) return; // different file

    const now = new Date();
    const hh = String(now.getHours()).padStart(2, '0');
    const mm = String(now.getMinutes()).padStart(2, '0');
    const message = `auto: ${hh}:${mm}`;

    const author = get().authorIdentity ?? { name: 'Unknown', email: 'unknown@local' };

    try {
      await get().commitAutosave(message, author);
      set({ autosaveError: null });
    } catch (err) {
      if (isGitError(err)) {
        set({ autosaveError: err.message });
      } else if (err instanceof Error) {
        set({ autosaveError: err.message });
      } else {
        set({ autosaveError: 'unknown autosave error' });
      }
    }
  };
}

/**
 * Phase 6b: Active branch ref for log queries. Returns the current branch
 * when a repo-bearing state is active, falling back to 'main' outside a
 * repo. Extracted out of git-store.ts (where it was inlined) so the shared
 * `syncAfterHeadMove` helper can reuse it.
 */
export function currentLogRef(store: GitStore): string {
  const s = store.state;
  return s.kind === 'ready' || s.kind === 'conflict' || s.kind === 'needs-tracked-file'
    ? s.repo.currentBranch
    : 'main';
}

/**
 * Phase 6b: build the shared post-head-move sync routine. Invoked from
 * `pull` (fast-forward / clean merge), `switchBranch`, and `mergeBranch`
 * (clean paths). Each head-moving action needs the exact same cascade:
 *
 *   1. refreshStatus()   — updates branch + ahead/behind + merge state
 *   2. refreshBranches() — keeps the branch picker in sync
 *   3. reload the tracked .op file from disk — HEAD moved, the on-disk
 *      blob changed, and the in-memory document must match or the next
 *      save silently overwrites the new HEAD with stale content
 *   4. loadLog()         — the GitPanelReady log effect keys on state.kind
 *      which does NOT change across head moves; without an explicit reload
 *      the history list would stay stale until the panel remounts
 *
 * Consolidating this in one helper prevents the three call sites from
 * drifting out of sync (pre-6b, `pull` skipped steps 3 and 4 entirely,
 * which is why a successful pull never refreshed the canvas or history).
 */
export function makeSyncAfterHeadMove(get: () => GitStore): () => Promise<void> {
  return async () => {
    await get().refreshStatus();
    await get().refreshBranches();
    const state = get().state;
    if ((state.kind === 'ready' || state.kind === 'conflict') && state.repo.trackedFilePath) {
      await loadOpFileFromPath(state.repo.trackedFilePath);
    }
    await get().loadLog({ ref: currentLogRef(get()), limit: 50 });
  };
}

/**
 * Phase 7c: after applyMerge() succeeds, reload the tracked .op file from
 * disk and refresh the history log so the UI reflects the merged result.
 *
 * Mirrors the reload done in makeSyncAfterHeadMove but scoped to the post-
 * merge path: we do NOT call refreshBranches() here because the branch did
 * not change during a merge (we stayed on the same branch and integrated the
 * incoming changes). refreshStatus() is called to update dirty/ahead-behind
 * so the "push N commits" button picks up the new merge commit.
 */
export function makeReloadAfterApply(get: () => GitStore): () => Promise<void> {
  return async () => {
    const state = get().state;
    // After applyMerge transitions to 'ready', reload the tracked file.
    if (state.kind === 'ready' && state.repo.trackedFilePath) {
      await loadOpFileFromPath(state.repo.trackedFilePath);
    }
    // Refresh status to pick up the new merge commit in ahead/behind counts.
    await get().refreshStatus();
    // Reload the log so the history list shows the merge commit.
    await get().loadLog({ ref: currentLogRef(get()), limit: 50 });
  };
}

/**
 * Phase 6b: classify a thrown remote-action error. Returns an `'auth'`
 * verdict for recoverable auth codes (so the pull/push buttons can open
 * the shared auth form) or `'other'` for everything else (so the caller
 * falls through to its generic error handling). Mirrors the clone-side
 * `classifyCloneError` shape so the store doesn't grow four slightly
 * different classifier patterns.
 *
 * The `which` argument is retained for call-site readability and to leave
 * a slot for a pull/push divergence if one ever lands — today both flows
 * share `REMOTE_AUTH_ERROR_CODES`.
 */
export function classifyRemoteAuthError(
  err: unknown,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  _which: 'pull' | 'push',
): { kind: 'auth'; code: RemoteAuthErrorCode; message: string } | { kind: 'other' } {
  if (!isGitError(err)) return { kind: 'other' };
  if ((REMOTE_AUTH_ERROR_CODES as readonly string[]).includes(err.code)) {
    return {
      kind: 'auth',
      code: err.code as RemoteAuthErrorCode,
      message: err.message,
    };
  }
  return { kind: 'other' };
}

/**
 * Phase 6b: turn a wire-format conflict bag (plus the optional unresolved
 * non-`.op` file list) into the renderer-side conflict-state payload. Used
 * by `pull`, `mergeBranch`, and `refreshStatus` so all three transitions
 * into `conflict` share the exact same shape.
 */
export function buildConflictState(
  repo: RepoMeta,
  bag: GitConflictBag | null,
  unresolvedFiles: string[],
  finalizeError: string | null = null,
  reopenedMidMerge = false,
): Extract<GitState, { kind: 'conflict' }> {
  return {
    kind: 'conflict',
    repo,
    conflicts: bag
      ? hydrateConflictBag(bag)
      : { nodeConflicts: new Map(), docFieldConflicts: new Map() },
    unresolvedFiles,
    finalizeError,
    reopenedMidMerge,
  };
}
