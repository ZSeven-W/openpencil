// apps/desktop/git/git-engine.ts
//
// Engine adapter — the single backend interface for the git layer. Each
// IPC handler is a one-line forward to one of the exported `engineX` fns
// below. The engine owns:
//
//   - repoId allocation (via repoSession)
//   - dual-ref milestone semantics (heads + autosaves)
//   - workingDirty detection via blob OID comparison
//   - candidate file walking for the needs-tracked-file picker
//   - the iso vs sys decision (Phase 2a: always 'iso')
//
// It does NOT own:
//   - low-level git primitives (those live in git-iso.ts)
//   - IPC serialization (that's ipc-handlers.ts)
//   - clone/fetch/push/auth/SSH/merge (Phase 2b/2c)
//
// FILE SIZE DEBT (Phase 7a): This file is ~1982 lines — approximately 2.5×
// the 800-line guideline. The Phase 7a addition (engineBranchMergeFolderMode)
// could not move to worktree-merge.ts because it needs session state
// (repoSession, setInflightMerge) and ref-resolution helpers that live here;
// worktree-merge.ts is intentionally kept as a pure shell-wrapper boundary
// with no session coupling. Decomposition is deferred to a future phase —
// revisit when this file crosses ~2100 lines or when session state is extracted.

import { resolve, basename, relative, join, sep } from 'node:path';
import { promises as fsp } from 'node:fs';
import * as git from 'isomorphic-git';
// eslint-disable-next-line @typescript-eslint/no-require-imports
const httpNode = require('isomorphic-git/http/node') as typeof import('isomorphic-git/http/node');
import * as fs from 'node:fs';

import { GitError, type GitErrorCode } from './error';
import { detectRepo, type RepoDetection } from './repo-detector';
import {
  initSingleFile,
  openRepo,
  commitFile,
  readBlobAtCommit,
  logForRef,
  restoreFileFromCommit,
  listBranches,
  createBranch,
  deleteBranch,
  switchBranch,
  getCurrentBranch,
  setRef,
  readBlobOidAt,
  findMergeBase,
  writeRemoteOrigin,
  type IsoRepoHandle,
  type CommitMetaIso,
} from './git-iso';
import {
  isSystemGitAvailable,
  sysClone,
  sysFetch,
  sysPush,
  sysAheadBehind,
  buildSshCommand,
} from './git-sys';
import {
  sysMergeNoCommit,
  sysListUnresolved,
  readMergeHead,
  sysShowStageBlob,
  sysRestoreOurs,
  sysStageFile,
  sysFinalizeMerge,
  sysAbortMerge,
} from './worktree-merge';
import {
  registerSession,
  getSession,
  updateTrackedFile,
  updateCandidates,
  unregisterSession,
  setInflightMerge,
  clearInflightMerge,
  type RepoSession,
  type CandidateFileInfo,
} from './repo-session';
import type { AuthCreds, AuthStore } from './auth-store';
import type { SshKeyManager } from './ssh-keys';
import { diffDocuments, mergeDocuments, type NodePatch } from '@zseven-w/pen-core';
import type { PenDocument } from '@zseven-w/pen-types';
import { runMerge, applyResolutions } from './merge-orchestrator';
import { buildConflictBag, type ConflictBag, type ConflictResolution } from './merge-session';

// ---------------------------------------------------------------------------
// Public types — these are the wire shapes returned to the IPC layer.
// They mirror the spec's IPC contract section.
// ---------------------------------------------------------------------------

export interface RepoOpenInfo {
  repoId: string;
  mode: 'single-file' | 'folder';
  rootPath: string;
  gitdir: string;
  engineKind: 'iso' | 'sys';
  trackedFilePath: string | null;
  candidates: CandidateFileInfo[];
}

export interface CommitMeta {
  hash: string;
  parentHashes: string[];
  message: string;
  author: { name: string; email: string; timestamp: number };
  kind: 'milestone' | 'autosave';
}

export interface BranchInfo {
  name: string;
  isCurrent: boolean;
  ahead: number; // always 0 in Phase 2a (no remote tracking)
  behind: number; // always 0 in Phase 2a
  lastCommit: { hash: string; message: string; timestamp: number } | null;
}

/**
 * Phase 6a: renderer-visible remote metadata for the single 'origin' remote.
 * Mirrors the wire shape declared in apps/web/src/services/git-types.ts.
 */
export interface RemoteInfo {
  name: 'origin';
  url: string | null;
  host: string | null;
}

export interface StatusInfo {
  /** Current branch from HEAD's symbolic ref. Always populated for normal
   * repos — even on a fresh repo with no commits, isomorphic-git's
   * currentBranch reads HEAD's symbolic value (e.g. 'main') without
   * verifying the heads ref exists. The engine throws 'engine-crash' if
   * HEAD is detached, so callers never see undefined here. */
  branch: string;
  trackedFilePath: string | null;
  workingDirty: boolean;
  otherFilesDirty: number;
  otherFilesPaths: string[];
  ahead: number;
  behind: number;
  mergeInProgress: boolean;
  unresolvedFiles: string[];
  /** Wire-format conflict bag when an in-flight merge has unresolved
   * conflicts. null otherwise. Phase 2c populates this from the session's
   * inflightMerge state. */
  conflicts: ConflictBag | null;
  /**
   * I2: true when the panel was reopened mid-merge — MERGE_HEAD is present
   * on disk but session.inflightMerge is null (new session, lost in-memory
   * state). The renderer uses this to show an abort-only UI instead of the
   * normal conflict resolution view.
   */
  reopenedMidMerge: boolean;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Return the session or throw GitError('no-file'). */
function requireSession(repoId: string): RepoSession {
  const s = getSession(repoId);
  if (!s) {
    throw new GitError('no-file', `Unknown repoId: ${repoId}`, { recoverable: false });
  }
  return s;
}

/**
 * Resolve an alias ref name to a fully-qualified ref:
 *   'main'      → 'refs/heads/<currentBranch>'
 *   'autosaves' → 'refs/openpencil/autosaves/<currentBranch>'
 *   <name>      → 'refs/heads/<name>'
 *
 * In Phase 2a there's no detached HEAD handling — if currentBranch is null
 * we throw 'engine-crash' since the caller should have caught it earlier.
 */
async function getRefAlias(
  handle: IsoRepoHandle,
  alias: 'main' | 'autosaves' | string,
): Promise<string> {
  if (alias === 'main' || alias === 'autosaves') {
    const branch = await getCurrentBranch({ handle });
    if (!branch) {
      throw new GitError('engine-crash', 'No current branch (HEAD detached?)');
    }
    return alias === 'main' ? `refs/heads/${branch}` : `refs/openpencil/autosaves/${branch}`;
  }
  return `refs/heads/${alias}`;
}

// ---------------------------------------------------------------------------
// Phase 2b: SSH key manager + auth store singletons + dispatch helpers.
// Singletons are assigned by ipc-handlers.ts at boot via setSshKeyManager()
// and setAuthStore(). Tests inject fakes via the same setters.
// ---------------------------------------------------------------------------

let sshKeyManager: SshKeyManager | null = null;
let authStore: AuthStore | null = null;

export function setSshKeyManager(mgr: SshKeyManager | null): void {
  sshKeyManager = mgr;
}

export function setAuthStore(store: AuthStore | null): void {
  authStore = store;
}

async function resolveSshKeyPath(keyId: string): Promise<string> {
  if (!sshKeyManager) {
    throw new GitError('ssh-key-missing', 'No SSH key manager configured');
  }
  try {
    return await sshKeyManager.getPrivateKeyPath(keyId);
  } catch {
    throw new GitError('ssh-key-missing', `SSH key ${keyId} not found`);
  }
}

/**
 * Decide whether a network op needs system git.
 *
 * iso (isomorphic-git) only speaks HTTP/HTTPS. Everything else — SSH transport,
 * `file://` URLs, local file paths — must go through sys. SSH key auth always
 * forces sys regardless of URL scheme.
 *
 * - `null` URL (anonymous, no remote info) → iso (default safe path)
 * - `http(s)://...` → iso
 * - everything else → sys
 */
export function shouldUseSys(url: string | null, auth?: AuthCreds): boolean {
  if (auth?.kind === 'ssh') return true;
  if (!url) return false;
  if (url.startsWith('https://') || url.startsWith('http://')) return false;
  return true;
}

/**
 * Extract the hostname from a git remote URL. Handles three formats:
 *   https://host/path           → host
 *   ssh://git@host:22/path      → host
 *   git@host:user/repo.git      → host (the SCP-style SSH form)
 *
 * Returns null for unparseable URLs (e.g. local file paths used in tests).
 */
export function parseHost(url: string): string | null {
  if (url.startsWith('https://') || url.startsWith('http://') || url.startsWith('ssh://')) {
    try {
      return new URL(url).hostname || null;
    } catch {
      return null;
    }
  }
  // SCP-style: user@host:path
  const m = url.match(/^[^@\s]+@([^:\s]+):/);
  if (m) return m[1];
  return null;
}

/**
 * Look up the remote URL configured for `<remote>` (default 'origin') on
 * the given handle. Uses isomorphic-git's listRemotes which only reads
 * .git/config — no network. Returns null if the remote isn't configured
 * or the gitdir is unreadable.
 */
export async function getRemoteUrl(
  handle: IsoRepoHandle,
  remote = 'origin',
): Promise<string | null> {
  try {
    const remotes = await git.listRemotes({ fs, gitdir: handle.gitdir });
    const r = remotes.find((x) => x.remote === remote);
    return r?.url ?? null;
  } catch {
    return null;
  }
}

/**
 * Pick the auth credentials to use for a remote operation. Order of
 * precedence:
 *   1. The explicit `auth` argument passed to the IPC call (highest)
 *   2. A credential previously stored in auth-store, keyed by the URL's host
 *   3. undefined → anonymous / let iso fail with auth-required
 *
 * This is the SINGLE place auth resolution happens. Every network engine fn
 * (clone/fetch/pull/push) calls it before deciding iso vs sys and before
 * passing creds into iso's onAuth callback.
 */
export async function resolveAuthForRemote(
  url: string | null,
  explicit?: AuthCreds,
): Promise<AuthCreds | undefined> {
  if (explicit) return explicit;
  if (!authStore || !url) return undefined;
  const host = parseHost(url);
  if (!host) return undefined;
  return (await authStore.get(host)) ?? undefined;
}

/**
 * Map an iso (isomorphic-git) error to a GitErrorCode. iso errors carry
 * a `.code` property and sometimes a `.data.statusCode` for HTTP failures.
 */
function mapIsoError(err: unknown): GitErrorCode {
  const e = err as { code?: string; data?: { statusCode?: number }; message?: string };
  const status = e.data?.statusCode;
  if (status === 401 || status === 403) return 'auth-failed';
  if (status === 404) return 'clone-failed';
  if (
    e.code === 'UrlParseError' ||
    e.code === 'EAI_AGAIN' ||
    (e.message ?? '').includes('ENOTFOUND')
  ) {
    return 'network';
  }
  if (e.code === 'PushRejectedError') return 'push-rejected';
  if (e.code === 'MergeNotSupportedError' || e.code === 'FastForwardError') {
    return 'pull-non-fast-forward';
  }
  return 'engine-crash';
}

// ---------------------------------------------------------------------------
// Public engine fns
// ---------------------------------------------------------------------------

/**
 * Discover whether the given .op file lives inside a git repo. If found,
 * register a session and auto-bind the file path. If not found, return
 * { mode: 'none' } and allocate no session.
 *
 * The candidate file walk is NOT performed here for the 'none' branch.
 * For the 'single-file' branch, candidates is always [opFile] (a single-file
 * repo can only contain one .op file by definition). For the 'folder' branch,
 * we run the full walk so the picker has data ready.
 */
export async function engineDetect(filePath: string): Promise<{ mode: 'none' } | RepoOpenInfo> {
  const detection = await detectRepo(filePath);
  if (detection.mode === 'none') {
    return { mode: 'none' };
  }
  const handle = await openRepo(detection);
  const candidates =
    detection.mode === 'single-file'
      ? [await buildSingleFileCandidate(handle, filePath)]
      : await walkCandidates(handle);
  const session = registerSession({
    handle,
    trackedFilePath: resolve(filePath),
    candidateFiles: candidates,
    engineKind: 'iso',
  });
  return toOpenInfo(session);
}

/**
 * Initialize a fresh single-file repo at .op-history/<basename>.git next to
 * the file. Auto-binds the file as the tracked file.
 */
export async function engineInit(filePath: string): Promise<RepoOpenInfo> {
  const handle = await initSingleFile({ filePath });
  const candidates = [await buildSingleFileCandidate(handle, filePath)];
  const session = registerSession({
    handle,
    trackedFilePath: resolve(filePath),
    candidateFiles: candidates,
    engineKind: 'iso',
  });
  return toOpenInfo(session);
}

// ---------------------------------------------------------------------------
// Internal helpers (continued)
// ---------------------------------------------------------------------------

function toOpenInfo(session: RepoSession): RepoOpenInfo {
  return {
    repoId: session.repoId,
    mode: session.handle.mode,
    rootPath: session.handle.dir,
    gitdir: session.handle.gitdir,
    engineKind: session.engineKind,
    trackedFilePath: session.trackedFilePath,
    candidates: session.candidateFiles,
  };
}

/**
 * Build a single CandidateFileInfo for a single-file repo. Counts come from
 * the heads + autosaves refs for the current branch (or zeros if no commits
 * yet). Used by engineDetect/Init in single-file mode.
 */
async function buildSingleFileCandidate(
  handle: IsoRepoHandle,
  filePath: string,
): Promise<CandidateFileInfo> {
  const abs = resolve(filePath);
  const rel = basename(abs);
  return computeCandidateMeta(handle, abs, rel);
}

/**
 * Compute the candidate metadata (counts + last commit) for one file by
 * walking the heads and autosaves refs of the current branch and checking
 * blob presence at each commit's tree. Used by both single-file and folder
 * walks.
 */
async function computeCandidateMeta(
  handle: IsoRepoHandle,
  absPath: string,
  relativePath: string,
): Promise<CandidateFileInfo> {
  const branch = await getCurrentBranch({ handle });
  if (!branch) {
    return {
      path: absPath,
      relativePath,
      milestoneCount: 0,
      autosaveCount: 0,
      lastCommitAt: null,
      lastCommitMessage: null,
    };
  }

  const headsRef = `refs/heads/${branch}`;
  const autoRef = `refs/openpencil/autosaves/${branch}`;
  const headsLog = await logForRef({ handle, ref: headsRef, depth: 10000 });
  const autoLog = await logForRef({ handle, ref: autoRef, depth: 10000 });

  // Count commits whose tree contains this file's relativePath. We probe by
  // calling readBlob and catching misses.
  const milestoneCount = await countCommitsTouching(handle, headsLog, relativePath);
  const autosaveCount = await countCommitsTouching(handle, autoLog, relativePath);

  // Most recent touching commit across both refs determines lastCommitAt.
  let lastCommitAt: number | null = null;
  let lastCommitMessage: string | null = null;
  for (const c of [...headsLog, ...autoLog]) {
    if (lastCommitAt === null || c.author.timestamp > lastCommitAt) {
      // Verify the file is present at this commit before counting it as the latest.
      try {
        await git.readBlob({
          fs,
          gitdir: handle.gitdir,
          oid: c.hash,
          filepath: relativePath,
        });
        lastCommitAt = c.author.timestamp;
        lastCommitMessage = c.message.trim();
      } catch {
        // not in this commit
      }
    }
  }

  return {
    path: absPath,
    relativePath,
    milestoneCount,
    autosaveCount,
    lastCommitAt,
    lastCommitMessage,
  };
}

async function countCommitsTouching(
  handle: IsoRepoHandle,
  commits: CommitMetaIso[],
  filepath: string,
): Promise<number> {
  let n = 0;
  for (const c of commits) {
    try {
      await git.readBlob({
        fs,
        gitdir: handle.gitdir,
        oid: c.hash,
        filepath,
      });
      n++;
    } catch {
      // file not in this commit's tree
    }
  }
  return n;
}

/**
 * Walk the worktree for *.op files (and *.pen, since the editor accepts both).
 * Skips:
 *   - dotfiles and dotdirs (.git, .op-history, .DS_Store, ...)
 *   - node_modules
 *   - any directory the user can't read (logged + ignored)
 *
 * Returns CandidateFileInfo[] sorted by lastCommitAt descending (most
 * recently-touched first), then by relativePath ascending as a stable tiebreak.
 */
async function walkCandidates(handle: IsoRepoHandle): Promise<CandidateFileInfo[]> {
  const root = handle.dir;
  const found: string[] = []; // absolute paths

  async function recurse(dir: string): Promise<void> {
    let entries;
    try {
      entries = await fsp.readdir(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      if (entry.name.startsWith('.')) continue;
      if (entry.name === 'node_modules') continue;
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        await recurse(full);
      } else if (entry.isFile()) {
        const lower = entry.name.toLowerCase();
        if (lower.endsWith('.op') || lower.endsWith('.pen')) {
          found.push(full);
        }
      }
    }
  }

  await recurse(root);

  const metas: CandidateFileInfo[] = [];
  for (const abs of found) {
    const rel = relative(root, abs);
    metas.push(await computeCandidateMeta(handle, abs, rel));
  }

  metas.sort((a, b) => {
    const aTs = a.lastCommitAt ?? -1;
    const bTs = b.lastCommitAt ?? -1;
    if (aTs !== bTs) return bTs - aTs; // most recent first
    return a.relativePath.localeCompare(b.relativePath);
  });

  return metas;
}

/**
 * Open an existing repo at `repoPath`. If `currentFilePath` is provided AND
 * the file lives inside the repo's worktree, the file is auto-bound as the
 * tracked file. Otherwise the session's trackedFilePath is left null and
 * the renderer must call engineBindTrackedFile.
 *
 * Phase 2a only handles already-existing folder-mode repos here. Single-file
 * repos enter the engine via engineDetect (which knows the .op file path
 * from the start).
 */
export async function engineOpen(
  repoPath: string,
  currentFilePath?: string,
): Promise<RepoOpenInfo> {
  const absRepo = resolve(repoPath);
  let detection: RepoDetection;
  try {
    // detectRepo expects a file path, not a dir. Probe with a dummy file inside.
    detection = await detectRepo(join(absRepo, '__probe__.op'));
  } catch (err) {
    throw new GitError('open-failed', `Failed to probe ${absRepo}`, { cause: err });
  }
  if (detection.mode !== 'folder') {
    throw new GitError('not-a-repo', `${absRepo} is not a folder-mode git repository`);
  }
  const handle = await openRepo(detection);

  // Decide auto-binding.
  let trackedFilePath: string | null = null;
  if (currentFilePath) {
    const absCurrent = resolve(currentFilePath);
    if (isInside(handle.dir, absCurrent)) {
      trackedFilePath = absCurrent;
    }
  }

  const candidates = await walkCandidates(handle);

  // If no current file but exactly one candidate exists, auto-bind it for
  // convenience. (Spec §"Tracked file binding" allows this single-candidate
  // shortcut.)
  if (!trackedFilePath && candidates.length === 1) {
    trackedFilePath = candidates[0].path;
  }

  const session = registerSession({
    handle,
    trackedFilePath,
    candidateFiles: candidates,
    engineKind: 'iso',
  });
  return toOpenInfo(session);
}

/**
 * Set or replace the trackedFilePath of a session. Returns the new value.
 * Throws 'no-file' for unknown repoId, 'open-failed' if the file is not
 * inside the repo's worktree.
 */
export async function engineBindTrackedFile(
  repoId: string,
  filePath: string,
): Promise<{ trackedFilePath: string }> {
  const session = requireSession(repoId);
  const abs = resolve(filePath);
  if (!isInside(session.handle.dir, abs)) {
    throw new GitError('open-failed', `${abs} is not inside repo ${session.handle.dir}`);
  }
  updateTrackedFile(repoId, abs);
  return { trackedFilePath: abs };
}

/**
 * Re-walk the worktree and refresh the cached candidate list. Returns the
 * fresh list. Used by the picker UI when the user adds files outside
 * OpenPencil and wants to refresh.
 */
export async function engineListCandidates(repoId: string): Promise<CandidateFileInfo[]> {
  const session = requireSession(repoId);
  const fresh = await walkCandidates(session.handle);
  updateCandidates(repoId, fresh);
  return fresh;
}

/**
 * Drop the session. The renderer should call this when the user closes the
 * file (so memory doesn't grow with stale repoIds). It is also called by
 * tests in afterEach via clearAllSessions.
 */
export function engineClose(repoId: string): void {
  unregisterSession(repoId);
}

/**
 * Path containment check. Returns true if `child` is `root` itself or a
 * descendant. Uses the resolved absolute paths.
 */
function isInside(root: string, child: string): boolean {
  const r = resolve(root);
  const c = resolve(child);
  if (c === r) return true;
  return c.startsWith(r + sep);
}

/**
 * Snapshot of the repo's working state. workingDirty is computed by hashing
 * the tracked file's on-disk content and comparing it to the blob OID stored
 * at the tip of refs/openpencil/autosaves/<branch> (falling back to
 * refs/heads/<branch> if the autosave ref doesn't exist yet).
 *
 * Phase 2a constants (filled in by 2b/2c):
 *   - ahead/behind: 0 (no remote tracking yet)
 *   - mergeInProgress: false (no merge orchestration yet)
 *   - unresolvedFiles: [] (same)
 */
export async function engineStatus(repoId: string): Promise<StatusInfo> {
  const session = requireSession(repoId);
  const branch = await getCurrentBranch({ handle: session.handle });
  if (!branch) {
    // Detached HEAD — Phase 2a doesn't support this yet. The renderer should
    // never get here on a normal repo because initSingleFile and isomorphic-git's
    // default init both leave HEAD as a symbolic ref to refs/heads/<defaultBranch>.
    throw new GitError('engine-crash', 'HEAD is detached; Phase 2a does not support this');
  }

  // workingDirty: hash the on-disk file and compare to the autosave-ref tip
  // (falling back to heads-ref if autosave doesn't exist yet). On a fresh
  // repo with no commits both refs are missing → workingDirty is true.
  let workingDirty = false;
  if (session.trackedFilePath) {
    workingDirty = await isWorkingDirty(session.handle, session.trackedFilePath, branch);
  }

  // otherFilesDirty: in single-file mode, always 0. In folder mode, count
  // files that differ from refs/heads/<branch>'s tree (excluding the tracked
  // file itself). The walker takes the union of tree paths AND worktree paths
  // so deleted-from-disk tracked files and untracked dotfiles are both counted.
  let otherFilesDirty = 0;
  let otherFilesPaths: string[] = [];
  if (session.handle.mode === 'folder') {
    const result = await countOtherDirtyFiles(session.handle, session.trackedFilePath, branch);
    otherFilesDirty = result.count;
    otherFilesPaths = result.paths;
  }

  // ahead/behind: compute via sys git if available; otherwise return 0/0.
  // Phase 2b adds this — Phase 2a always returned 0/0.
  let ahead = 0;
  let behind = 0;
  if (await isSystemGitAvailable()) {
    try {
      const ab = await sysAheadBehind({ cwd: session.handle.dir, branch });
      ahead = ab.ahead;
      behind = ab.behind;
    } catch {
      // No remote tracking → leave as 0/0.
    }
  }

  // Populate merge fields from two sources (Phase 7a):
  //   1. session.inflightMerge  — in-memory state for .op-level conflicts
  //   2. on-disk MERGE_HEAD     — survives session close/reopen, covers
  //      non-.op conflicts, and reflects terminal-initiated merges
  let mergeInProgress = false;
  let unresolvedFiles: string[] = [];
  let conflicts: ConflictBag | null = null;
  let reopenedMidMerge = false;

  if (session.inflightMerge) {
    mergeInProgress = true;
    // Build the wire-format bag from the in-flight conflict map.
    const merged = session.inflightMerge.mergeResult;
    conflicts = {
      nodeConflicts: merged.nodeConflicts.map((c) => ({
        ...c,
        id: `node:${c.pageId ?? '_'}:${c.nodeId}`,
      })),
      docFieldConflicts: merged.docFieldConflicts.map((c) => ({
        ...c,
        id: `field:${c.field}:${c.path}`,
      })),
    };
    // Tracked .op file is "unresolved" until all conflicts have resolutions.
    const totalConflicts = merged.nodeConflicts.length + merged.docFieldConflicts.length;
    if (session.inflightMerge.resolutions.size < totalConflicts && session.trackedFilePath) {
      unresolvedFiles = [toPosixPath(relative(session.handle.dir, session.trackedFilePath))];
    }
    // Also check for non-.op files still unresolved on-disk (mixed conflict).
    if (session.handle.mode === 'folder') {
      const onDiskUnresolved = (await readMergeHead(session.handle.gitdir))
        ? await sysListUnresolved({ cwd: session.handle.dir })
        : [];
      const trackedRel = session.trackedFilePath
        ? toPosixPath(relative(session.handle.dir, session.trackedFilePath))
        : null;
      for (const p of onDiskUnresolved) {
        if (p !== trackedRel && !unresolvedFiles.includes(p)) {
          unresolvedFiles.push(p);
        }
      }
    }
  } else if (session.handle.mode === 'folder') {
    // No in-memory merge, but check on-disk MERGE_HEAD (e.g. after panel
    // close/reopen mid-merge, or terminal-initiated merge).
    const mergeHead = await readMergeHead(session.handle.gitdir);
    if (mergeHead) {
      mergeInProgress = true;
      // I2: filter the tracked .op file out of unresolvedFiles so the renderer
      // does not misleadingly label it as a "non-op file". The tracked file
      // appears in the git index with stages 1/2/3 after a conflict, but the
      // renderer has no UI to resolve it in the degraded panel-reopen state.
      const sysUnresolved = await sysListUnresolved({ cwd: session.handle.dir });
      const trackedRel = session.trackedFilePath
        ? toPosixPath(relative(session.handle.dir, session.trackedFilePath))
        : null;
      unresolvedFiles = trackedRel ? sysUnresolved.filter((f) => f !== trackedRel) : sysUnresolved;
      // Signal the degraded panel-reopen state so the renderer can show
      // abort-only UI instead of the normal conflict resolution view.
      reopenedMidMerge = true;
    }
  }

  return {
    branch,
    trackedFilePath: session.trackedFilePath,
    workingDirty,
    otherFilesDirty,
    otherFilesPaths,
    ahead,
    behind,
    mergeInProgress,
    unresolvedFiles,
    conflicts,
    reopenedMidMerge,
  };
}

/**
 * Compute workingDirty for a tracked file. Hashes the disk content and
 * compares to the blob OID at the autosave-ref tip (or heads-ref tip if no
 * autosave ref exists yet).
 */
async function isWorkingDirty(
  handle: IsoRepoHandle,
  trackedFilePath: string,
  branch: string,
): Promise<boolean> {
  let bytes: Buffer;
  try {
    bytes = await fsp.readFile(trackedFilePath);
  } catch {
    // File missing on disk → treat as dirty so the next save action surfaces
    // the I/O error from there. We don't bubble fs errors out of status().
    return true;
  }
  const { oid: workOid } = await git.hashBlob({ object: bytes });

  const rel = toPosixPath(relative(handle.dir, trackedFilePath));
  let refOid = await readBlobOidAt({
    handle,
    ref: `refs/openpencil/autosaves/${branch}`,
    filepath: rel,
  });
  if (refOid === null) {
    refOid = await readBlobOidAt({
      handle,
      ref: `refs/heads/${branch}`,
      filepath: rel,
    });
  }
  if (refOid === null) {
    // Neither ref has the file → dirty.
    return true;
  }
  return refOid !== workOid;
}

/**
 * Count files (other than the tracked file) that are dirty relative to the
 * heads-ref tip. "Dirty" includes:
 *   - file present in tree but missing on disk (deleted)
 *   - file present on disk but missing from tree (untracked)
 *   - file present in both with different blob OIDs (modified)
 *
 * The walker takes the UNION of tree paths and worktree paths so all three
 * cases are caught. Tracked dotfiles like `.gitignore` are included — only
 * `.git/`, `.op-history/`, and `node_modules/` are excluded from the walk.
 *
 * Returns POSIX-separated relative paths matching git's tree format.
 */
async function countOtherDirtyFiles(
  handle: IsoRepoHandle,
  trackedFilePath: string | null,
  branch: string,
): Promise<{ count: number; paths: string[] }> {
  const headsRef = `refs/heads/${branch}`;
  let tip: string;
  try {
    tip = await git.resolveRef({ fs, gitdir: handle.gitdir, ref: headsRef });
  } catch {
    // No heads ref yet → no commits exist, so by definition every file on
    // disk is "untracked but dirty". For Phase 2a we treat the empty-history
    // case as 0 other-dirty (the panel UX shows "no history yet" instead of
    // a wall of untracked files). The folder dirty count becomes meaningful
    // only after the first milestone.
    return { count: 0, paths: [] };
  }

  const treePaths = await listTreePaths(handle, tip);
  const diskPaths = await listWorktreePaths(handle.dir);
  const all = new Set<string>([...treePaths, ...diskPaths]);

  const trackedRel = trackedFilePath ? toPosixPath(relative(handle.dir, trackedFilePath)) : null;
  const dirty: string[] = [];

  for (const rel of all) {
    if (trackedRel && rel === trackedRel) continue;

    // diskOid: null if file missing/unreadable on disk.
    let diskOid: string | null = null;
    try {
      const bytes = await fsp.readFile(join(handle.dir, ...rel.split('/')));
      const { oid } = await git.hashBlob({ object: bytes });
      diskOid = oid;
    } catch {
      diskOid = null;
    }

    // treeOid: null if file is not in the heads tree.
    let treeOid: string | null = null;
    try {
      const blob = await git.readBlob({
        fs,
        gitdir: handle.gitdir,
        oid: tip,
        filepath: rel,
      });
      treeOid = blob.oid;
    } catch {
      treeOid = null;
    }

    // Dirty if either side missing or hashes differ.
    if (diskOid !== treeOid) {
      dirty.push(rel);
    }
  }

  // Stable sort for deterministic UI rendering and tests.
  dirty.sort();
  return { count: dirty.length, paths: dirty };
}

/**
 * Recursively list every blob path inside a commit's root tree. Returns
 * POSIX-separated paths matching git's internal format.
 */
async function listTreePaths(handle: IsoRepoHandle, commit: string): Promise<string[]> {
  const out: string[] = [];
  await walkTree(handle, commit, '', out);
  return out;
}

async function walkTree(
  handle: IsoRepoHandle,
  oid: string,
  prefix: string,
  out: string[],
): Promise<void> {
  // git.readTree accepts either a tree OID or a commit OID (in which case it
  // resolves to that commit's root tree).
  const { tree } = await git.readTree({ fs, gitdir: handle.gitdir, oid });
  for (const entry of tree) {
    const path = prefix ? `${prefix}/${entry.path}` : entry.path;
    if (entry.type === 'tree') {
      await walkTree(handle, entry.oid, path, out);
    } else if (entry.type === 'blob') {
      out.push(path);
    }
  }
}

/**
 * Walk the worktree and return every regular file's POSIX-relative path.
 * Excludes ONLY `.git/`, `.op-history/`, and `node_modules/`. Tracked dotfiles
 * (`.gitignore`, `.editorconfig`, etc.) are included.
 */
async function listWorktreePaths(root: string): Promise<string[]> {
  const out: string[] = [];
  async function recurse(dir: string, prefix: string): Promise<void> {
    let entries;
    try {
      entries = await fsp.readdir(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      if (entry.name === '.git' || entry.name === '.op-history' || entry.name === 'node_modules') {
        continue;
      }
      const full = join(dir, entry.name);
      const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        await recurse(full, rel);
      } else if (entry.isFile()) {
        out.push(rel);
      }
    }
  }
  await recurse(root, '');
  return out;
}

/**
 * Convert an OS-native relative path to POSIX form so it matches git's
 * internal tree format on Windows.
 */
function toPosixPath(p: string): string {
  return p.split(sep).join('/');
}

/**
 * Walk commits from the given ref. The ref alias is resolved by getRefAlias:
 *   'main'      → refs/heads/<currentBranch>
 *   'autosaves' → refs/openpencil/autosaves/<currentBranch>
 *   <name>      → refs/heads/<name>
 *
 * Each commit is decorated with `kind`: 'milestone' if its hash is reachable
 * from refs/heads/<branch>, 'autosave' otherwise.
 */
export async function engineLog(
  repoId: string,
  opts: { ref: 'main' | 'autosaves' | string; limit: number },
): Promise<CommitMeta[]> {
  const session = requireSession(repoId);
  const ref = await getRefAlias(session.handle, opts.ref);
  const commits = await logForRef({ handle: session.handle, ref, depth: opts.limit });

  // Build the set of milestone hashes for the current branch so we can label
  // each entry. We do this by walking the heads ref to a generous depth.
  const branch = await getCurrentBranch({ handle: session.handle });
  const milestoneHashes = new Set<string>();
  if (branch) {
    const headsCommits = await logForRef({
      handle: session.handle,
      ref: `refs/heads/${branch}`,
      depth: 10000,
    });
    for (const c of headsCommits) milestoneHashes.add(c.hash);
  }

  return commits.map<CommitMeta>((c) => ({
    hash: c.hash,
    parentHashes: c.parentHashes,
    message: c.message,
    author: c.author,
    kind: milestoneHashes.has(c.hash) ? 'milestone' : 'autosave',
  }));
}

/**
 * Create a commit on the tracked file. Two kinds:
 *
 *   'milestone': writes the commit to refs/heads/<branch> (parent = heads
 *     tip), then force-jumps refs/openpencil/autosaves/<branch> to the new
 *     hash. Abandons any intermediate autosaves.
 *
 *   'autosave': writes the commit to refs/openpencil/autosaves/<branch>
 *     (parent = autosaves tip, which may be a milestone if no autosaves
 *     since). heads ref is untouched.
 *
 * Throws GitError 'commit-empty' if the working file content is identical
 * to its tip on the relevant ref. Throws 'no-file' for unknown repoId or
 * for a session with no trackedFilePath set.
 */
export async function engineCommit(
  repoId: string,
  opts: {
    kind: 'milestone' | 'autosave';
    message: string;
    author: { name: string; email: string };
  },
): Promise<{ hash: string }> {
  const session = requireSession(repoId);
  if (!session.trackedFilePath) {
    throw new GitError('no-file', 'Session has no trackedFilePath; call bindTrackedFile first');
  }

  const branch = await getCurrentBranch({ handle: session.handle });
  if (!branch) {
    throw new GitError('engine-crash', 'HEAD is detached; Phase 2a does not support this');
  }
  const headsRef = `refs/heads/${branch}`;
  const autoRef = `refs/openpencil/autosaves/${branch}`;

  const rel = toPosixPath(relative(session.handle.dir, session.trackedFilePath));

  if (opts.kind === 'milestone') {
    const { hash } = await commitFile({
      handle: session.handle,
      filepath: rel,
      ref: headsRef,
      message: opts.message,
      author: opts.author,
    });
    await setRef({ handle: session.handle, ref: autoRef, value: hash });
    return { hash };
  }

  // autosave
  //
  // Phase 4c content-hash debounce: if the tracked file's current disk
  // blob matches the autosave tip's blob for the same path, skip the
  // commit and return the existing tip hash. This catches the common
  // "multi-Cmd+S" / "undo-to-clean" case where the user fires N saves
  // with no actual content change.
  const tipBlobOid = await readBlobOidAt({
    handle: session.handle,
    ref: autoRef,
    filepath: rel,
  });
  if (tipBlobOid !== null) {
    const diskContent = await fsp.readFile(session.trackedFilePath);
    const { oid: diskBlobHash } = await git.hashBlob({ object: diskContent });
    if (diskBlobHash === tipBlobOid) {
      // No content change since the last autosave — return the existing
      // tip commit hash as a no-op. We re-resolve the ref here because
      // readBlobOidAt only exposes the blob oid, and the API contract
      // requires a commit hash in the return value.
      const currentTipHash = await git.resolveRef({
        fs,
        gitdir: session.handle.gitdir,
        ref: autoRef,
      });
      return { hash: currentTipHash };
    }
  }

  const { hash } = await commitFile({
    handle: session.handle,
    filepath: rel,
    ref: autoRef,
    message: opts.message,
    author: opts.author,
  });
  return { hash };
}

/**
 * Restore the tracked file's content from a specific commit. Writes the
 * blob to the working tree but does NOT create a new commit — the engine
 * leaves the working tree dirty so the user sees pending changes and can
 * decide whether to record a milestone.
 */
export async function engineRestore(repoId: string, commitHash: string): Promise<void> {
  const session = requireSession(repoId);
  if (!session.trackedFilePath) {
    throw new GitError('no-file', 'Session has no trackedFilePath');
  }
  const rel = toPosixPath(relative(session.handle.dir, session.trackedFilePath));
  await restoreFileFromCommit({
    handle: session.handle,
    filepath: rel,
    commitHash,
  });
}

/**
 * Promote an autosave commit to a milestone:
 *   1. Read the autosave's blob for the tracked file.
 *   2. Write it to the working tree.
 *   3. Create a milestone commit (which advances both heads and autosaves).
 *
 * The result is a clean milestone with the user-supplied message that
 * captures the same content as the autosave. The intermediate autosave
 * chain is abandoned by the milestone's force-update of the autosave ref.
 */
export async function enginePromote(
  repoId: string,
  autosaveHash: string,
  message: string,
  author: { name: string; email: string },
): Promise<{ hash: string }> {
  const session = requireSession(repoId);
  if (!session.trackedFilePath) {
    throw new GitError('no-file', 'Session has no trackedFilePath');
  }
  const rel = toPosixPath(relative(session.handle.dir, session.trackedFilePath));

  // Step 1+2: read the autosave content and write it to disk.
  const content = await readBlobAtCommit({
    handle: session.handle,
    filepath: rel,
    commitHash: autosaveHash,
  });
  await fsp.writeFile(session.trackedFilePath, content, 'utf-8');

  // Step 3: milestone commit using the engine's normal path.
  return engineCommit(repoId, { kind: 'milestone', message, author });
}

/**
 * List local branches with current-marker and lastCommit metadata.
 * ahead/behind are 0 in Phase 2a (no remote tracking yet).
 */
export async function engineBranchList(repoId: string): Promise<BranchInfo[]> {
  const session = requireSession(repoId);
  const names = await listBranches({ handle: session.handle });
  const current = await getCurrentBranch({ handle: session.handle });

  const out: BranchInfo[] = [];
  for (const name of names) {
    const log = await logForRef({
      handle: session.handle,
      ref: `refs/heads/${name}`,
      depth: 1,
    });
    const tip = log[0];
    out.push({
      name,
      isCurrent: name === current,
      ahead: 0,
      behind: 0,
      lastCommit: tip
        ? {
            hash: tip.hash,
            message: tip.message.trim(),
            timestamp: tip.author.timestamp,
          }
        : null,
    });
  }
  return out;
}

/**
 * Create a new branch under refs/heads/. If `fromCommit` is omitted, the
 * branch is created at the current HEAD.
 */
export async function engineBranchCreate(
  repoId: string,
  opts: { name: string; fromCommit?: string },
): Promise<void> {
  const session = requireSession(repoId);
  await createBranch({
    handle: session.handle,
    name: opts.name,
    fromCommit: opts.fromCommit,
  });
}

/**
 * Switch HEAD to the given branch. Updates the tracked file in the working
 * tree to that branch's tip. Throws 'no-file' if the session has no
 * trackedFilePath bound (the panel UI must bind a file before allowing
 * branch switches).
 */
export async function engineBranchSwitch(repoId: string, name: string): Promise<void> {
  const session = requireSession(repoId);
  if (!session.trackedFilePath) {
    throw new GitError('no-file', 'Session has no trackedFilePath');
  }
  const rel = toPosixPath(relative(session.handle.dir, session.trackedFilePath));
  await switchBranch({ handle: session.handle, name, filepath: rel });
}

/**
 * Delete a non-current branch. The Phase 1b primitive throws 'branch-current'
 * if you try to delete the active branch and 'branch-unmerged' if the
 * branch has commits not reachable from any other ref; Phase 5 adds the
 * `force` flag so the renderer can retry after a confirm dialog.
 */
export async function engineBranchDelete(
  repoId: string,
  name: string,
  opts: { force?: boolean } = {},
): Promise<void> {
  const session = requireSession(repoId);
  await deleteBranch({ handle: session.handle, name, force: opts.force === true });
}

/**
 * Clone a remote repository. For HTTPS URLs we use isomorphic-git's clone
 * with the node http transport. For SSH URLs (or auth.kind === 'ssh') we
 * shell out to system git with GIT_SSH_COMMAND configured to use the
 * specified private key.
 *
 * Per spec line 669, clone NEVER auto-binds a tracked file — the renderer
 * must call bindTrackedFile before commit/restore work, even if the cloned
 * repo happens to contain exactly one .op file. We therefore do NOT call
 * engineOpen (which auto-binds the single-candidate case) and instead
 * register the session manually with trackedFilePath: null.
 */
export async function engineClone(opts: {
  url: string;
  dest: string;
  auth?: AuthCreds;
}): Promise<RepoOpenInfo> {
  const { url, dest } = opts;
  // Resolve auth: explicit > stored-by-host > anonymous.
  const resolvedAuth = await resolveAuthForRemote(url, opts.auth);
  const useSys = shouldUseSys(url, resolvedAuth);

  if (useSys) {
    // SSH path requires system git.
    if (!(await isSystemGitAvailable())) {
      throw new GitError(
        'ssh-not-supported-iso',
        'SSH transport requires system git; install git or use HTTPS',
      );
    }
    let env: Record<string, string> | undefined;
    if (resolvedAuth?.kind === 'ssh') {
      const keyPath = await resolveSshKeyPath(resolvedAuth.keyId);
      env = { GIT_SSH_COMMAND: buildSshCommand(keyPath) };
    }
    await sysClone({ url, dest, env });
  } else {
    // HTTPS path via iso. onAuth uses the pre-resolved creds; iso may invoke
    // it lazily when the server demands authentication.
    try {
      await git.clone({
        fs,
        http: httpNode,
        dir: dest,
        url,
        singleBranch: false,
        depth: undefined, // full history
        onAuth: () => {
          if (resolvedAuth?.kind === 'token') {
            return { username: resolvedAuth.username, password: resolvedAuth.token };
          }
          return undefined; // anonymous
        },
        onAuthFailure: () => {
          // Returning undefined cancels the request → iso throws an error.
          return undefined;
        },
      });
    } catch (err) {
      throw new GitError(mapIsoError(err), `Clone failed: ${url}`, {
        cause: err,
        detail: { url, dest },
      });
    }
  }

  // After a successful clone, register the session manually. We bypass
  // engineOpen on purpose: spec §"User Experience" line 109 says clone
  // ALWAYS lands in needs-tracked-file regardless of how many .op files
  // the repo contains. engineOpen auto-binds the single-candidate case,
  // which would silently violate that contract.
  const detection = await detectRepo(join(dest, '__probe__.op'));
  if (detection.mode !== 'folder') {
    throw new GitError(
      'clone-failed',
      `Cloned repo at ${dest} is not detectable as a folder repository`,
    );
  }
  const handle = await openRepo(detection);
  const candidates = await walkCandidates(handle);
  const session = registerSession({
    handle,
    trackedFilePath: null, // ALWAYS null after clone, per spec
    candidateFiles: candidates,
    engineKind: 'iso',
  });
  return toOpenInfo(session);
}

/**
 * Phase 6a: read the configured `origin` remote from `.git/config` only.
 * No network, no IO outside the gitdir. Returns `{ name: 'origin', url:
 * null, host: null }` when origin is absent.
 */
export async function engineRemoteGet(repoId: string): Promise<RemoteInfo> {
  const session = requireSession(repoId);
  const url = await getRemoteUrl(session.handle, 'origin');
  return {
    name: 'origin',
    url,
    host: url ? parseHost(url) : null,
  };
}

/**
 * Phase 6a: set, update, or remove the single 'origin' remote.
 *
 *   - non-empty `url` → upsert in `.git/config`
 *   - `null`           → remove from `.git/config` (idempotent if absent)
 *
 * Returns the fresh RemoteInfo so the renderer can update its cached state
 * from a single round-trip without a follow-up engineRemoteGet().
 */
export async function engineRemoteSet(repoId: string, url: string | null): Promise<RemoteInfo> {
  const session = requireSession(repoId);
  await writeRemoteOrigin({ handle: session.handle, url });
  // Read back through the same getRemoteUrl path to guarantee the renderer
  // sees exactly what `.git/config` now holds (handles trailing-slash and
  // any other normalization that addRemote may apply).
  const stored = await getRemoteUrl(session.handle, 'origin');
  return {
    name: 'origin',
    url: stored,
    host: stored ? parseHost(stored) : null,
  };
}

/**
 * Fetch from origin, updating remote-tracking refs. Returns ahead/behind
 * for the current branch after the fetch.
 *
 * Dispatch policy: read the actual configured `origin` URL from .git/config
 * (no network) and use shouldUseSys(remoteUrl, resolvedAuth) to decide iso
 * vs sys. This means a repo cloned via SSH always routes to sys for fetch,
 * even when the caller doesn't pass an explicit auth argument.
 */
export async function engineFetch(
  repoId: string,
  auth?: AuthCreds,
): Promise<{ ahead: number; behind: number }> {
  const session = requireSession(repoId);
  const branch = await getCurrentBranch({ handle: session.handle });
  if (!branch) {
    throw new GitError('engine-crash', 'HEAD is detached');
  }

  const remoteUrl = await getRemoteUrl(session.handle);
  const resolvedAuth = await resolveAuthForRemote(remoteUrl, auth);
  const useSys = shouldUseSys(remoteUrl, resolvedAuth);

  if (useSys) {
    if (!(await isSystemGitAvailable())) {
      throw new GitError('ssh-not-supported-iso', 'SSH transport requires system git');
    }
    let env: Record<string, string> | undefined;
    if (resolvedAuth?.kind === 'ssh') {
      const keyPath = await resolveSshKeyPath(resolvedAuth.keyId);
      env = { GIT_SSH_COMMAND: buildSshCommand(keyPath) };
    }
    await sysFetch({ cwd: session.handle.dir, env });
  } else {
    try {
      await git.fetch({
        fs,
        http: httpNode,
        dir: session.handle.dir,
        gitdir: session.handle.gitdir,
        onAuth: () => {
          if (resolvedAuth?.kind === 'token') {
            return { username: resolvedAuth.username, password: resolvedAuth.token };
          }
          return undefined;
        },
        onAuthFailure: () => undefined,
      });
    } catch (err) {
      throw new GitError(mapIsoError(err), `Fetch failed`, { cause: err });
    }
  }

  // Compute ahead/behind. Try sys first (more accurate); fall back to 0/0
  // if sys git isn't available.
  if (await isSystemGitAvailable()) {
    return sysAheadBehind({ cwd: session.handle.dir, branch });
  }
  return { ahead: 0, behind: 0 };
}

/**
 * Pull from origin. Phase 2c rewrites this to use the full merge path:
 * fetch updates refs/remotes/origin/<branch>, then engineBranchMerge runs
 * the in-process pen-core merge against that ref. Conflicts land in session
 * state the same way as a local branchMerge would.
 */
export async function enginePull(
  repoId: string,
  auth?: AuthCreds,
): Promise<{
  result: 'fast-forward' | 'merge' | 'conflict' | 'conflict-non-op';
  conflicts?: ConflictBag;
}> {
  const session = requireSession(repoId);
  const branch = await getCurrentBranch({ handle: session.handle });
  if (!branch) {
    throw new GitError('engine-crash', 'HEAD is detached');
  }

  // Step 1: fetch from origin so refs/remotes/origin/<branch> is current.
  await engineFetch(repoId, auth);

  // Step 2: merge the remote-tracking ref into the current branch using the
  // same code path as a local branch merge.
  const remoteRef = `refs/remotes/origin/${branch}`;
  return engineBranchMerge(repoId, remoteRef);
}

/**
 * Push the current branch to origin. iso for HTTPS, sys for SSH/local. Same
 * dispatch + auth-resolution policy as engineFetch.
 *
 * Phase 2b throws GitError on rejection/auth-fail rather than returning a
 * tagged result. The renderer (Phase 3) will translate the GitError code
 * back to the spec's `result: 'rejected' | 'auth-failed'` shape if needed.
 */
export async function enginePush(repoId: string, auth?: AuthCreds): Promise<{ result: 'ok' }> {
  const session = requireSession(repoId);
  const branch = await getCurrentBranch({ handle: session.handle });
  if (!branch) {
    throw new GitError('engine-crash', 'HEAD is detached');
  }

  const remoteUrl = await getRemoteUrl(session.handle);
  const resolvedAuth = await resolveAuthForRemote(remoteUrl, auth);
  const useSys = shouldUseSys(remoteUrl, resolvedAuth);

  if (useSys) {
    if (!(await isSystemGitAvailable())) {
      throw new GitError('ssh-not-supported-iso', 'SSH transport requires system git');
    }
    let env: Record<string, string> | undefined;
    if (resolvedAuth?.kind === 'ssh') {
      const keyPath = await resolveSshKeyPath(resolvedAuth.keyId);
      env = { GIT_SSH_COMMAND: buildSshCommand(keyPath) };
    }
    await sysPush({ cwd: session.handle.dir, branch, env });
  } else {
    try {
      await git.push({
        fs,
        http: httpNode,
        dir: session.handle.dir,
        gitdir: session.handle.gitdir,
        ref: branch,
        onAuth: () => {
          if (resolvedAuth?.kind === 'token') {
            return { username: resolvedAuth.username, password: resolvedAuth.token };
          }
          return undefined;
        },
        onAuthFailure: () => undefined,
      });
    } catch (err) {
      throw new GitError(mapIsoError(err), `Push failed`, { cause: err });
    }
  }

  return { result: 'ok' };
}

/**
 * Diff two commits' versions of the tracked file. Returns a NodePatch[] plus
 * an aggregated summary. Read-only — does not touch the working tree.
 *
 * For Phase 2c, requires that the session has a trackedFilePath set; the
 * diff is always computed for that single file. (The renderer never asks
 * for cross-file diffs in folder mode.)
 */
export async function engineDiff(
  repoId: string,
  fromCommit: string,
  toCommit: string,
): Promise<{
  summary: {
    framesChanged: number;
    nodesAdded: number;
    nodesRemoved: number;
    nodesModified: number;
  };
  patches: NodePatch[];
}> {
  const session = requireSession(repoId);
  if (!session.trackedFilePath) {
    throw new GitError('no-file', 'Session has no trackedFilePath');
  }
  const rel = toPosixPath(relative(session.handle.dir, session.trackedFilePath));

  const [fromStr, toStr] = await Promise.all([
    readBlobAtCommit({ handle: session.handle, filepath: rel, commitHash: fromCommit }),
    readBlobAtCommit({ handle: session.handle, filepath: rel, commitHash: toCommit }),
  ]);

  let from: PenDocument;
  let to: PenDocument;
  try {
    from = JSON.parse(fromStr) as PenDocument;
    to = JSON.parse(toStr) as PenDocument;
  } catch (err) {
    throw new GitError('engine-crash', 'Failed to parse blobs for diff', {
      cause: err,
      detail: { fromCommit, toCommit },
    });
  }

  const patches = diffDocuments(from, to);

  // Aggregate summary counts. framesChanged is the number of distinct parent
  // ids that appear in any patch (lower bound — we don't deeply track frames
  // separately from other nodes in 2c).
  let nodesAdded = 0;
  let nodesRemoved = 0;
  let nodesModified = 0;
  const framesChanged = new Set<string>();
  for (const p of patches) {
    if (p.op === 'add') nodesAdded++;
    else if (p.op === 'remove') nodesRemoved++;
    else if (p.op === 'modify') nodesModified++;
    else if (p.op === 'move') nodesModified++; // move counts as a modification
    if (p.parentId) framesChanged.add(p.parentId);
  }

  return {
    summary: {
      framesChanged: framesChanged.size,
      nodesAdded,
      nodesRemoved,
      nodesModified,
    },
    patches,
  };
}

/**
 * Folder-mode 3-way merge path. Delegates to system-git worktree helpers
 * in worktree-merge.ts. Returns the same result union as engineBranchMerge.
 *
 * Contract (Phase 7a):
 *   - fast-forward / clean merge → 'merge' (handled by caller before reaching here)
 *   - .op conflict (possibly mixed) → 'conflict' with InflightMerge stashed in session
 *   - non-.op-only conflict → 'conflict-non-op' (no InflightMerge — renderer shows
 *     "unresolved non-design files" message)
 */
async function engineBranchMergeFolderMode(
  repoId: string,
  session: RepoSession,
  branch: string,
  fromBranch: string,
  theirsRef: string,
  oursRef: string,
): Promise<{
  result: 'fast-forward' | 'merge' | 'conflict' | 'conflict-non-op';
  conflicts?: ConflictBag;
}> {
  if (!(await isSystemGitAvailable())) {
    throw new GitError(
      'engine-crash',
      'Folder-mode divergent merge requires system git, which is not available',
    );
  }

  const mergeResult = await sysMergeNoCommit({ cwd: session.handle.dir, ref: theirsRef });

  if (mergeResult.kind === 'clean') {
    // No conflicts — finalize the merge commit right away.
    const hash = await sysFinalizeMerge({
      cwd: session.handle.dir,
      message: `Merge ${fromBranch} into ${branch}`,
      author: { name: 'OpenPencil', email: 'noreply@openpencil' },
    });
    // Sync the isomorphic-git refs to the new HEAD so the engine stays consistent.
    await setRef({ handle: session.handle, ref: oursRef, value: hash });
    await setRef({
      handle: session.handle,
      ref: `refs/openpencil/autosaves/${branch}`,
      value: hash,
    });
    return { result: 'merge' };
  }

  // Conflicts present. Classify them: which files are unresolved?
  const unresolved = await sysListUnresolved({ cwd: session.handle.dir });
  const trackedRel = session.trackedFilePath
    ? toPosixPath(relative(session.handle.dir, session.trackedFilePath))
    : null;

  const opConflicted = trackedRel !== null && unresolved.includes(trackedRel);

  if (!opConflicted) {
    // Only non-.op files have conflicts — return conflict-non-op.
    // We do NOT abort: the renderer must surface the non-.op conflicts so
    // the user can resolve them in a terminal or external tool.
    return { result: 'conflict-non-op' };
  }

  // The tracked .op file is among the conflicts. Read base/ours/theirs from
  // the index stages so the pen-core merge can produce a semantic merge result.
  const [baseBlob, oursBlob, theirsBlob] = await Promise.all([
    sysShowStageBlob({ cwd: session.handle.dir, stage: 1, filepath: trackedRel! }),
    sysShowStageBlob({ cwd: session.handle.dir, stage: 2, filepath: trackedRel! }),
    sysShowStageBlob({ cwd: session.handle.dir, stage: 3, filepath: trackedRel! }),
  ]);

  if (!baseBlob || !oursBlob || !theirsBlob) {
    // One or more index stages are missing for the tracked .op file.
    // This covers rename conflicts (e.g. theirs renamed the file so stage :3: is
    // null) and delete/add conflicts. In all such cases a semantic 3-way merge
    // of the document is impossible, so we surface conflict-non-op and let the
    // user resolve it in a terminal.
    return { result: 'conflict-non-op' };
  }

  let base: PenDocument;
  let ours: PenDocument;
  let theirs: PenDocument;
  try {
    base = JSON.parse(baseBlob);
    ours = JSON.parse(oursBlob);
    theirs = JSON.parse(theirsBlob);
  } catch (err) {
    throw new GitError('engine-crash', 'Failed to parse .op blobs during folder-mode merge', {
      cause: err,
    });
  }

  const opMergeResult = mergeDocuments({ base, ours, theirs });
  const { bag, conflictMap } = buildConflictBag(opMergeResult);

  // Restore the tracked .op file to readable JSON (stage 2 = ours) so the
  // renderer can open the document without seeing conflict markers. MERGE_HEAD
  // and any non-.op unresolved files remain alive.
  await sysRestoreOurs({ cwd: session.handle.dir, filepath: trackedRel! });

  // Read the commit hashes for InflightMerge.
  const oursCommit = await git.resolveRef({ fs, gitdir: session.handle.gitdir, ref: oursRef });
  const theirsCommit = await git.resolveRef({
    fs,
    gitdir: session.handle.gitdir,
    ref: theirsRef,
  });
  const baseCommit = await findMergeBase({
    handle: session.handle,
    oid1: oursCommit,
    oid2: theirsCommit,
  });

  setInflightMerge(repoId, {
    oursCommit,
    theirsCommit,
    baseCommit,
    mergeResult: opMergeResult,
    conflictMap,
    resolutions: new Map(),
    defaultMessage: `Merge ${fromBranch} into ${branch}`,
  });

  return { result: 'conflict', conflicts: bag };
}

/**
 * Merge another branch into the current branch. Single-file mode uses the
 * pen-core merge directly; folder mode uses system-git merge machinery
 * (Phase 7a+).
 *
 * Return shape:
 *   - { result: 'fast-forward' }       — theirs is a descendant of ours, or
 *                                        theirs is an ancestor of ours (up-to-date)
 *   - { result: 'merge' }              — clean merge produced new merge commit
 *   - { result: 'conflict', conflicts } — .op-level conflicts; InflightMerge stashed in session
 *   - { result: 'conflict-non-op' }    — conflicts only in non-.op files
 */
export async function engineBranchMerge(
  repoId: string,
  fromBranch: string,
): Promise<{
  result: 'fast-forward' | 'merge' | 'conflict' | 'conflict-non-op';
  conflicts?: ConflictBag;
}> {
  const session = requireSession(repoId);
  const branch = await getCurrentBranch({ handle: session.handle });
  if (!branch) {
    throw new GitError('engine-crash', 'HEAD is detached');
  }

  // Resolve the two commit oids. We do this BEFORE the mode check so that
  // up-to-date and fast-forward cases work uniformly for both single-file
  // and folder mode (FF is just ref movement + checkout; no merge involved).
  const oursRef = `refs/heads/${branch}`;
  const theirsRef = fromBranch.startsWith('refs/') ? fromBranch : `refs/heads/${fromBranch}`;
  let oursCommit: string;
  let theirsCommit: string;
  try {
    oursCommit = await git.resolveRef({ fs, gitdir: session.handle.gitdir, ref: oursRef });
    theirsCommit = await git.resolveRef({ fs, gitdir: session.handle.gitdir, ref: theirsRef });
  } catch (err) {
    throw new GitError('engine-crash', `Failed to resolve refs for merge`, {
      cause: err,
      detail: { oursRef, theirsRef },
    });
  }

  // Find the merge base. Three shortcut cases before we run the 3-way merge:
  //   1. ours === theirs → trivially the same commit → no-op.
  //   2. base === theirs → theirs is an ancestor of ours → already up to date.
  //   3. base === ours → ours is an ancestor of theirs → fast-forward.
  const baseCommit = await findMergeBase({
    handle: session.handle,
    oid1: oursCommit,
    oid2: theirsCommit,
  });

  if (oursCommit === theirsCommit || baseCommit === theirsCommit) {
    // Already up to date — theirs has nothing ours doesn't already have.
    return { result: 'fast-forward' };
  }

  if (baseCommit === oursCommit) {
    // Fast-forward: move heads + autosaves to theirs and update the working
    // tree. Works for both single-file and folder mode because git.checkout
    // without filepaths updates every changed file from the target ref.
    await setRef({ handle: session.handle, ref: oursRef, value: theirsCommit });
    await setRef({
      handle: session.handle,
      ref: `refs/openpencil/autosaves/${branch}`,
      value: theirsCommit,
    });
    try {
      await git.checkout({
        fs,
        dir: session.handle.dir,
        gitdir: session.handle.gitdir,
        ref: branch,
        force: true,
      });
    } catch (err) {
      throw new GitError('engine-crash', 'Fast-forward checkout failed', { cause: err });
    }
    return { result: 'fast-forward' };
  }

  // From here on we need a true 3-way merge.
  if (session.handle.mode === 'folder') {
    return engineBranchMergeFolderMode(repoId, session, branch, fromBranch, theirsRef, oursRef);
  }

  if (!session.trackedFilePath) {
    throw new GitError('no-file', 'Session has no trackedFilePath');
  }
  const rel = toPosixPath(relative(session.handle.dir, session.trackedFilePath));

  // True 3-way merge: load the three blobs and run pen-core merge.
  const merge = await runMerge({
    handle: session.handle,
    filepath: rel,
    oursCommit,
    theirsCommit,
    baseCommit,
  });

  if (merge.bag.nodeConflicts.length === 0 && merge.bag.docFieldConflicts.length === 0) {
    // Clean merge. Write the merged document to disk and create a merge commit.
    const mergedJson = JSON.stringify(merge.result.merged);
    await fsp.writeFile(session.trackedFilePath, mergedJson, 'utf-8');
    const { hash } = await commitFile({
      handle: session.handle,
      filepath: rel,
      ref: oursRef,
      message: `Merge ${fromBranch} into ${branch}`,
      author: { name: 'OpenPencil', email: 'noreply@openpencil' },
      parents: [oursCommit, theirsCommit],
    });
    await setRef({
      handle: session.handle,
      ref: `refs/openpencil/autosaves/${branch}`,
      value: hash,
    });
    return { result: 'merge' };
  }

  // Conflicts present. Stash the InflightMerge and return the bag.
  setInflightMerge(repoId, {
    oursCommit,
    theirsCommit,
    baseCommit,
    mergeResult: merge.result,
    conflictMap: merge.conflictMap,
    resolutions: new Map(),
    defaultMessage: `Merge ${fromBranch} into ${branch}`,
  });
  return { result: 'conflict', conflicts: merge.bag };
}

/**
 * Record the user's choice for a single conflict. The choice is stored in
 * the session's InflightMerge.resolutions map; applyMerge consumes the map
 * later to produce the final merged document.
 *
 * Throws 'engine-crash' if the conflictId is unknown for the current
 * in-flight merge — that's a programming error since the renderer should
 * only ever pass back ids the engine just emitted.
 */
export async function engineResolveConflict(
  repoId: string,
  conflictId: string,
  choice: ConflictResolution,
): Promise<void> {
  const session = requireSession(repoId);
  if (!session.inflightMerge) {
    throw new GitError('engine-crash', 'No in-flight merge for this repo');
  }
  if (!session.inflightMerge.conflictMap.has(conflictId)) {
    throw new GitError('engine-crash', `Unknown conflict id: ${conflictId}`);
  }
  session.inflightMerge.resolutions.set(conflictId, choice);
}

/**
 * Finalize the in-flight merge. Applies all accumulated resolutions to the
 * merged document, writes the result to disk, creates a merge commit, advances
 * both heads + autosaves refs, and clears the session's inflightMerge.
 *
 * Handles three cases uniformly (Phase 7a):
 *   1. .op conflicts only (single-file or folder mode with no non-.op unresolved)
 *   2. Mixed .op + non-.op conflicts — throws merge-still-conflicted if non-.op
 *      files remain unresolved (the user must fix those in a terminal first)
 *   3. Merge committed externally (inflightMerge === null and no MERGE_HEAD)
 *      → returns { noop: true }
 */
export async function engineApplyMerge(repoId: string): Promise<{ hash: string; noop: boolean }> {
  const session = requireSession(repoId);

  if (!session.inflightMerge) {
    // Check whether an on-disk merge was already committed (e.g. via terminal).
    // If MERGE_HEAD is gone, the merge is done → noop.
    if (session.handle.mode === 'folder') {
      const mergeHead = await readMergeHead(session.handle.gitdir);
      if (!mergeHead) {
        const branch = await getCurrentBranch({ handle: session.handle });
        if (!branch) throw new GitError('engine-crash', 'HEAD is detached');
        const head = await git.resolveRef({
          fs,
          gitdir: session.handle.gitdir,
          ref: `refs/heads/${branch}`,
        });
        return { hash: head, noop: true };
      }
      // MERGE_HEAD present but no inflightMerge in session — conflict was
      // detected in a different session (e.g. after panel reopen) but the
      // user tried to apply before we re-established the InflightMerge.
      throw new GitError(
        'merge-still-conflicted',
        'Merge in progress but no in-flight merge state — call status() first to re-establish',
      );
    }
    // Single-file mode: no inflightMerge → noop.
    const branch = await getCurrentBranch({ handle: session.handle });
    if (!branch) throw new GitError('engine-crash', 'HEAD is detached');
    const head = await git.resolveRef({
      fs,
      gitdir: session.handle.gitdir,
      ref: `refs/heads/${branch}`,
    });
    return { hash: head, noop: true };
  }

  if (!session.trackedFilePath) {
    throw new GitError('no-file', 'Session has no trackedFilePath');
  }

  const branch = await getCurrentBranch({ handle: session.handle });
  if (!branch) throw new GitError('engine-crash', 'HEAD is detached');

  // Verify all .op conflicts have a resolution.
  const { conflictMap, resolutions, mergeResult } = session.inflightMerge;
  const unresolvedConflicts: string[] = [];
  for (const id of conflictMap.keys()) {
    if (!resolutions.has(id)) unresolvedConflicts.push(id);
  }
  if (unresolvedConflicts.length > 0) {
    throw new GitError(
      'merge-still-conflicted',
      `Cannot apply merge: ${unresolvedConflicts.length} unresolved .op conflicts`,
      { detail: { unresolved: unresolvedConflicts } },
    );
  }

  // For folder mode, check whether non-.op files are still unresolved.
  if (session.handle.mode === 'folder') {
    const trackedRel = toPosixPath(relative(session.handle.dir, session.trackedFilePath));
    const onDiskUnresolved = await sysListUnresolved({ cwd: session.handle.dir });
    const nonOpUnresolved = onDiskUnresolved.filter((p) => p !== trackedRel);
    if (nonOpUnresolved.length > 0) {
      throw new GitError(
        'merge-still-conflicted',
        `Cannot apply merge: ${nonOpUnresolved.length} non-.op file(s) still unresolved`,
        { detail: { unresolved: nonOpUnresolved } },
      );
    }
  }

  // Build the final document.
  const finalDoc = applyResolutions({
    merged: mergeResult.merged,
    conflictMap,
    resolutions,
  });

  // Write to disk.
  const rel = toPosixPath(relative(session.handle.dir, session.trackedFilePath));
  await fsp.writeFile(session.trackedFilePath, JSON.stringify(finalDoc), 'utf-8');

  let hash: string;

  if (session.handle.mode === 'folder') {
    // Folder mode: stage the .op file and finalize with system git.
    await sysStageFile({ cwd: session.handle.dir, filepath: rel });
    hash = await sysFinalizeMerge({
      cwd: session.handle.dir,
      message: session.inflightMerge.defaultMessage,
      author: { name: 'OpenPencil', email: 'noreply@openpencil' },
    });
    // Sync isomorphic-git refs to the new HEAD.
    const oursRef = `refs/heads/${branch}`;
    await setRef({ handle: session.handle, ref: oursRef, value: hash });
    await setRef({
      handle: session.handle,
      ref: `refs/openpencil/autosaves/${branch}`,
      value: hash,
    });
  } else {
    // Single-file mode: use isomorphic-git commit (same as before).
    const { oursCommit, theirsCommit } = session.inflightMerge;
    const oursRef = `refs/heads/${branch}`;
    const commitResult = await commitFile({
      handle: session.handle,
      filepath: rel,
      ref: oursRef,
      message: session.inflightMerge.defaultMessage,
      author: { name: 'OpenPencil', email: 'noreply@openpencil' },
      parents: [oursCommit, theirsCommit],
    });
    hash = commitResult.hash;
    await setRef({
      handle: session.handle,
      ref: `refs/openpencil/autosaves/${branch}`,
      value: hash,
    });
  }

  clearInflightMerge(repoId);
  return { hash, noop: false };
}

/**
 * Abort the in-flight merge. Clears both in-memory session state and on-disk
 * merge state (MERGE_HEAD) for folder mode.
 *
 * In single-file mode, the working tree was never modified by the engine
 * during the conflict path, so only session state needs clearing.
 *
 * In folder mode, `git merge --abort` restores the working tree and index.
 */
export async function engineAbortMerge(repoId: string): Promise<void> {
  const session = requireSession(repoId);

  // Clear in-memory state.
  clearInflightMerge(repoId);

  // Abort on-disk merge state for folder mode.
  if (session.handle.mode === 'folder') {
    const mergeHead = await readMergeHead(session.handle.gitdir);
    if (mergeHead) {
      await sysAbortMerge({ cwd: session.handle.dir });
    }
  }
}
