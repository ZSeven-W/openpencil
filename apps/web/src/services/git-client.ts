// apps/web/src/services/git-client.ts
//
// Thin IPC wrapper around window.electronAPI.git.*. Every method forwards
// the call to the preload bridge and rehydrates any thrown GitError from
// the Electron-serialized marker format. Callers (the store) can then use
// `instanceof GitError` reliably.
//
// This module is stateless. The withCleanWorkingTree gate lives in the
// store (not the client) so that a tripped gate can queue a PendingAction
// for retry after the user saves.

import { GitError, rehydrateGitError } from './git-error';
import type {
  GitAPI,
  GitAuthCreds,
  GitBranchInfo,
  GitCandidateFileInfo,
  GitCommitMeta,
  GitConflictBag,
  GitConflictResolution,
  GitPublicSshKeyInfo,
  GitRemoteInfo,
  GitRepoOpenInfo,
  GitStatusInfo,
} from './git-types';

/**
 * Lazy accessor for window.electronAPI.git. Throws if we're not running
 * inside Electron — the caller should gate on isElectron() first. The
 * defensive throw here surfaces misuse loudly (e.g. the top-bar button
 * rendered in a browser accidentally).
 */
function getApi(): GitAPI {
  if (typeof window === 'undefined' || !window.electronAPI?.git) {
    throw new GitError(
      'engine-crash',
      'git-client: window.electronAPI.git is unavailable (not running in Electron)',
      { recoverable: false },
    );
  }
  return window.electronAPI.git;
}

/**
 * Run an IPC call and rehydrate any thrown GitError. Non-GitError failures
 * (network timeouts, Electron internal errors, malformed payloads) are
 * re-thrown as-is so the store can distinguish "the backend returned a
 * known failure mode" from "something truly unexpected happened".
 */
async function invoke<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (err) {
    const gitErr = rehydrateGitError(err);
    if (gitErr) throw gitErr;
    throw err;
  }
}

/**
 * Public: check whether window.electronAPI.git is wired. Used by the
 * top-bar button to decide whether to render at all, and by the store's
 * boot helper to short-circuit no-op in browser mode.
 */
export function isGitApiAvailable(): boolean {
  return typeof window !== 'undefined' && !!window.electronAPI?.git;
}

// ---------------------------------------------------------------------------
// Public surface — one method per IPC channel. Each is a two-line wrapper:
// get the api handle, forward the call through invoke().
// ---------------------------------------------------------------------------

export const gitClient = {
  // ---- Detect / open / init / clone ---------------------------------------
  detect: (filePath: string) => invoke(() => getApi().detect(filePath)),
  init: (filePath: string) => invoke(() => getApi().init(filePath)),
  open: (repoPath: string, currentFilePath?: string) =>
    invoke(() => getApi().open(repoPath, currentFilePath)),
  clone: (opts: { url: string; dest: string; auth?: GitAuthCreds }) =>
    invoke(() => getApi().clone(opts)),
  bindTrackedFile: (repoId: string, filePath: string) =>
    invoke(() => getApi().bindTrackedFile(repoId, filePath)),
  listCandidates: (repoId: string) => invoke(() => getApi().listCandidates(repoId)),
  close: (repoId: string) => invoke(() => getApi().close(repoId)),

  // ---- Status / log / diff -----------------------------------------------
  status: (repoId: string) => invoke(() => getApi().status(repoId)),
  log: (repoId: string, opts: { ref: 'main' | 'autosaves' | string; limit: number }) =>
    invoke(() => getApi().log(repoId, opts)),
  diff: (repoId: string, fromCommit: string, toCommit: string) =>
    invoke(() => getApi().diff(repoId, fromCommit, toCommit)),

  // ---- Commit / restore / promote ----------------------------------------
  commit: (
    repoId: string,
    opts: {
      kind: 'milestone' | 'autosave';
      message: string;
      author: { name: string; email: string };
    },
  ) => invoke(() => getApi().commit(repoId, opts)),
  restore: (repoId: string, commitHash: string) =>
    invoke(() => getApi().restore(repoId, commitHash)),
  promote: (
    repoId: string,
    autosaveHash: string,
    message: string,
    author: { name: string; email: string },
  ) => invoke(() => getApi().promote(repoId, autosaveHash, message, author)),

  // ---- Branches ----------------------------------------------------------
  branchList: (repoId: string) => invoke(() => getApi().branchList(repoId)),
  branchCreate: (repoId: string, opts: { name: string; fromCommit?: string }) =>
    invoke(() => getApi().branchCreate(repoId, opts)),
  branchSwitch: (repoId: string, name: string) => invoke(() => getApi().branchSwitch(repoId, name)),
  branchDelete: (repoId: string, name: string, opts?: { force?: boolean }) =>
    invoke(() => getApi().branchDelete(repoId, name, opts)),
  branchMerge: (repoId: string, fromBranch: string) =>
    invoke(() => getApi().branchMerge(repoId, fromBranch)),

  // ---- Merge orchestration -----------------------------------------------
  resolveConflict: (repoId: string, conflictId: string, choice: GitConflictResolution) =>
    invoke(() => getApi().resolveConflict(repoId, conflictId, choice)),
  applyMerge: (repoId: string) => invoke(() => getApi().applyMerge(repoId)),
  abortMerge: (repoId: string) => invoke(() => getApi().abortMerge(repoId)),

  // ---- Phase 4a: author identity probe -----------------------------------
  getSystemAuthor: () => invoke(() => getApi().getSystemAuthor()),

  // ---- Remote ------------------------------------------------------------
  fetch: (repoId: string, auth?: GitAuthCreds) => invoke(() => getApi().fetch(repoId, auth)),
  pull: (repoId: string, auth?: GitAuthCreds) => invoke(() => getApi().pull(repoId, auth)),
  push: (repoId: string, auth?: GitAuthCreds) => invoke(() => getApi().push(repoId, auth)),

  // ---- Phase 6a: remote metadata + config (no network) -------------------
  remoteGet: (repoId: string) => invoke(() => getApi().remoteGet(repoId)),
  remoteSet: (repoId: string, url: string | null) => invoke(() => getApi().remoteSet(repoId, url)),

  // ---- Auth --------------------------------------------------------------
  authStore: (host: string, creds: GitAuthCreds) => invoke(() => getApi().authStore(host, creds)),
  authGet: (host: string) => invoke(() => getApi().authGet(host)),
  authClear: (host: string) => invoke(() => getApi().authClear(host)),

  // ---- SSH keys ----------------------------------------------------------
  sshListKeys: () => invoke(() => getApi().sshListKeys()),
  sshGenerateKey: (opts: { host: string; comment: string }) =>
    invoke(() => getApi().sshGenerateKey(opts)),
  sshImportKey: (opts: { privateKeyPath: string; host: string }) =>
    invoke(() => getApi().sshImportKey(opts)),
  sshDeleteKey: (keyId: string) => invoke(() => getApi().sshDeleteKey(keyId)),
};

// Re-export the types most consumers need, so importing sites don't also
// need to pull from git-types.
export type {
  GitRepoOpenInfo,
  GitStatusInfo,
  GitCommitMeta,
  GitBranchInfo,
  GitCandidateFileInfo,
  GitConflictBag,
  GitConflictResolution,
  GitAuthCreds,
  GitPublicSshKeyInfo,
  GitRemoteInfo,
};
