// apps/desktop/git/repo-session.ts
//
// In-memory registry of open repositories. Each entry is allocated by
// engineDetect/Init/Open and disposed by engineClose. The renderer treats
// repoId as an opaque string and passes it back on every subsequent IPC
// call.
//
// This map is module-level (singleton). It does NOT survive a main-process
// restart, which matches the spec's session-scoped contract for repoId.

import { randomUUID } from 'node:crypto';
import type { IsoRepoHandle } from './git-iso';
import type { InflightMerge } from './merge-session';

export interface CandidateFileInfo {
  /** absolute path */
  path: string;
  /** path relative to RepoSession.handle.dir, used for UI display */
  relativePath: string;
  /** # of commits on refs/heads/<currentBranch> that include this file */
  milestoneCount: number;
  /** # of commits on refs/openpencil/autosaves/<currentBranch> for this file */
  autosaveCount: number;
  /** timestamp (seconds) of the most recent touching commit, null if never */
  lastCommitAt: number | null;
  lastCommitMessage: string | null;
}

export interface RepoSession {
  repoId: string;
  handle: IsoRepoHandle;
  /** Which .op file the panel is currently tracking. May be null right after
   * an `open`/`clone` call where no auto-binding happened — the renderer
   * must call bindTrackedFile to set it before commit/restore work. */
  trackedFilePath: string | null;
  /** Cached candidate list, populated by engineDetect/Init/Open and refreshed
   * on demand by engineListCandidates. */
  candidateFiles: CandidateFileInfo[];
  /** Engine kind reported to the renderer. In Phase 2a always 'iso'. */
  engineKind: 'iso' | 'sys';
  /** In-flight merge state. Set by engineBranchMerge when conflicts are
   * present, cleared by engineApplyMerge / engineAbortMerge. Null when no
   * merge is in progress. */
  inflightMerge: InflightMerge | null;
}

const sessions = new Map<string, RepoSession>();

/**
 * Allocate a fresh session for a newly-opened repo. Returns the new repoId.
 * The caller fills in trackedFilePath / candidateFiles before returning to
 * the IPC layer.
 */
export function registerSession(args: {
  handle: IsoRepoHandle;
  trackedFilePath: string | null;
  candidateFiles: CandidateFileInfo[];
  engineKind: 'iso' | 'sys';
}): RepoSession {
  const repoId = randomUUID();
  const session: RepoSession = {
    repoId,
    handle: args.handle,
    trackedFilePath: args.trackedFilePath,
    candidateFiles: args.candidateFiles,
    engineKind: args.engineKind,
    inflightMerge: null,
  };
  sessions.set(repoId, session);
  return session;
}

/**
 * Look up an existing session by repoId. Returns undefined if the id is
 * unknown — the engine layer translates this to a GitError('no-file').
 */
export function getSession(repoId: string): RepoSession | undefined {
  return sessions.get(repoId);
}

/**
 * Mutate the trackedFilePath of an existing session. Used by
 * engineBindTrackedFile.
 */
export function updateTrackedFile(repoId: string, trackedFilePath: string): boolean {
  const session = sessions.get(repoId);
  if (!session) return false;
  session.trackedFilePath = trackedFilePath;
  return true;
}

/**
 * Mutate the candidateFiles cache of an existing session. Used by
 * engineListCandidates after re-walking the worktree.
 */
export function updateCandidates(repoId: string, candidateFiles: CandidateFileInfo[]): boolean {
  const session = sessions.get(repoId);
  if (!session) return false;
  session.candidateFiles = candidateFiles;
  return true;
}

/**
 * Stash an in-flight merge on the session. Used by engineBranchMerge after
 * detecting .op-level conflicts.
 */
export function setInflightMerge(repoId: string, merge: InflightMerge): boolean {
  const session = sessions.get(repoId);
  if (!session) return false;
  session.inflightMerge = merge;
  return true;
}

/**
 * Clear the in-flight merge. Used by engineApplyMerge (after a successful
 * commit) and engineAbortMerge (after the user discards).
 */
export function clearInflightMerge(repoId: string): boolean {
  const session = sessions.get(repoId);
  if (!session) return false;
  session.inflightMerge = null;
  return true;
}

/**
 * Drop a session from the registry. Called by engineClose. The handle's
 * gitdir/file descriptors are managed by isomorphic-git internally — there's
 * nothing to flush here.
 */
export function unregisterSession(repoId: string): boolean {
  return sessions.delete(repoId);
}

/**
 * Wipe the entire registry. Used in tests' afterEach to keep sessions from
 * leaking between cases. Production code should never call this.
 */
export function clearAllSessions(): void {
  sessions.clear();
}

/** Test/debug helper: count active sessions. Not part of the IPC surface. */
export function sessionCount(): number {
  return sessions.size;
}
