// apps/web/src/stores/git-store-helpers.ts
//
// Pure helper functions and constants extracted from git-store.ts to
// keep that file under the 800-LoC cap. None of these touch the Zustand
// store directly — they are stateless pure functions consumed by the
// store actions in git-store.ts.

import { GitError } from '@/services/git-error';
import type { GitConflictBag, GitConflictResolution, GitRepoOpenInfo } from '@/services/git-types';
import type { ConflictBagState, GitState, RepoMeta } from './git-store-types';

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
