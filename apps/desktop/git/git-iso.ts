// apps/desktop/git/git-iso.ts
//
// Local git operations using isomorphic-git. This module is namespace-agnostic:
// callers pass ref names (e.g. 'refs/heads/main') as parameters. The engine
// in Phase 2 wraps these primitives with the actual ref naming convention
// for milestones (refs/heads/<branch>) and autosaves (refs/openpencil/autosaves/<branch>).

import * as fs from 'node:fs';
import { dirname, basename, resolve, join } from 'node:path';
import { mkdir } from 'node:fs/promises';
import * as git from 'isomorphic-git';
import { GitError } from './error';
import type { RepoDetectionFound } from './repo-detector';

export interface IsoRepoHandle {
  /** worktree (parent dir of .op file in single-file mode; repo root in folder mode) */
  dir: string;
  /** absolute path to the gitdir */
  gitdir: string;
  mode: 'single-file' | 'folder';
}

export interface CommitMetaIso {
  hash: string;
  parentHashes: string[];
  message: string;
  author: { name: string; email: string; timestamp: number };
}

export interface InitOptions {
  /** absolute path to the .op file (must already exist on disk) */
  filePath: string;
  /** branch name to initialize HEAD with; default 'main' */
  defaultBranch?: string;
  /** author for the initial empty commit; defaults to 'OpenPencil <noreply@openpencil>' */
  authorName?: string;
  authorEmail?: string;
}

/**
 * Initialize a single-file repo at .op-history/<basename>.git next to the file.
 * Idempotent: if the gitdir already exists with a HEAD, returns the existing
 * handle without re-initializing.
 *
 * The new repo has:
 *   - HEAD pointing at refs/heads/<defaultBranch>
 *   - `core.worktree = ../..` written into the gitdir's config file so that
 *     a user running `git -C <gitdir> status` from the terminal sees the
 *     correct working tree (the parent dir of the .op file). isomorphic-git
 *     itself doesn't need this — it accepts `dir` explicitly on every call —
 *     but the spec documents the on-disk shape with this setting and the
 *     CLI inspection use case demands it.
 *   - No initial commit (the engine will create one with the file's current
 *     content via commitFile).
 */
export async function initSingleFile(opts: InitOptions): Promise<IsoRepoHandle> {
  const absFile = resolve(opts.filePath);
  const dir = dirname(absFile);
  const baseName = basename(absFile);
  const gitdir = resolve(dir, '.op-history', `${baseName}.git`);
  const defaultBranch = opts.defaultBranch ?? 'main';

  try {
    // If a HEAD already exists, this is a re-init. Return the handle as-is.
    if (fs.existsSync(join(gitdir, 'HEAD'))) {
      return { dir, gitdir, mode: 'single-file' };
    }

    // Create the parent .op-history/ if needed.
    await mkdir(gitdir, { recursive: true });

    // isomorphic-git's init creates the gitdir layout. We pass `dir` and
    // `gitdir` separately so the worktree is the file's parent dir, not
    // a sibling of gitdir.
    await git.init({
      fs,
      dir,
      gitdir,
      defaultBranch,
      bare: false,
    });

    // Explicitly write core.worktree so terminal `git -C <gitdir>` sees the
    // correct working tree. The path is relative to the gitdir: from
    // <parent>/.op-history/<basename>.git → ../.. brings us back to <parent>.
    // We also force core.bare = false because some isomorphic-git versions
    // default it to true when dir != dirname(gitdir).
    await git.setConfig({
      fs,
      gitdir,
      path: 'core.worktree',
      value: '../..',
    });
    await git.setConfig({
      fs,
      gitdir,
      path: 'core.bare',
      value: false,
    });

    return { dir, gitdir, mode: 'single-file' };
  } catch (err) {
    throw new GitError(
      'init-failed',
      `Failed to initialize single-file repo for ${opts.filePath}`,
      {
        cause: err,
        detail: { filePath: opts.filePath, gitdir },
      },
    );
  }
}

/**
 * Open an existing repository given a successful detection result.
 * This is a thin wrapper that just packages the detection into a handle and
 * verifies the gitdir is readable.
 */
export async function openRepo(detection: RepoDetectionFound): Promise<IsoRepoHandle> {
  try {
    // Verify the gitdir is readable by reading HEAD.
    const head = join(detection.gitdir, 'HEAD');
    if (!fs.existsSync(head)) {
      throw new GitError('open-failed', `gitdir has no HEAD: ${detection.gitdir}`);
    }
    return {
      dir: detection.rootPath,
      gitdir: detection.gitdir,
      mode: detection.mode,
    };
  } catch (err) {
    if (err instanceof GitError) throw err;
    throw new GitError('open-failed', `Failed to open repository at ${detection.rootPath}`, {
      cause: err,
    });
  }
}

/**
 * Stage the given file and create a commit on the specified ref. The ref
 * doesn't have to exist beforehand — if it doesn't, it's created at this
 * commit (this is how the engine seeds an empty branch with its first
 * autosave).
 *
 * Throws GitError 'commit-empty' if the working tree state for `filepath`
 * is identical to the ref's current tip (no changes to commit).
 *
 * Implementation note: isomorphic-git's `commit()` accepts a `ref` argument
 * and updates that ref directly (instead of HEAD). It also accepts an
 * explicit `parent` array. So we don't need any low-level tree-writing
 * primitives — just stage, detect-empty, then commit.
 */
export async function commitFile(opts: {
  handle: IsoRepoHandle;
  filepath: string;
  ref: string;
  message: string;
  author: { name: string; email: string };
  parents?: string[];
}): Promise<{ hash: string }> {
  const { handle, filepath, ref, message, author } = opts;
  try {
    // Determine parent commit(s). If the caller passed parents explicitly,
    // use those. Otherwise look up the current ref tip; if the ref doesn't
    // exist, the new commit is a root commit (no parents).
    let parents = opts.parents;
    if (!parents) {
      try {
        const tip = await git.resolveRef({ fs, gitdir: handle.gitdir, ref });
        parents = [tip];
      } catch {
        parents = [];
      }
    }

    // Detect "no changes" by comparing the working tree blob hash for
    // `filepath` to the blob hash recorded at that path in the parent
    // commit's tree. Only valid for linear commits (exactly one parent).
    //
    // For root commits (no parents) the check doesn't apply. For merge
    // commits (2+ parents) the new tree may legitimately match one parent
    // but not the others — a merge that was fully resolved in favor of one
    // side still records history and must not be rejected. The engine layer
    // in Phase 2 owns the higher-level "is this merge meaningful" decision.
    if (parents.length === 1) {
      const fileBytes = await fs.promises.readFile(resolve(handle.dir, filepath));
      const { oid: workOid } = await git.hashBlob({ object: fileBytes });
      let parentBlobOid: string | undefined;
      try {
        const { oid } = await git.readBlob({
          fs,
          gitdir: handle.gitdir,
          oid: parents[0],
          filepath,
        });
        parentBlobOid = oid;
      } catch {
        // file didn't exist in parent → not empty, fall through to commit
      }
      if (parentBlobOid && parentBlobOid === workOid) {
        throw new GitError('commit-empty', `No changes to commit for ${filepath} on ${ref}`, {
          recoverable: true,
        });
      }
    }

    // Stage the file. isomorphic-git's add reads the working tree file
    // and writes a blob into objects/, plus updates the index.
    await git.add({ fs, dir: handle.dir, gitdir: handle.gitdir, filepath });

    // Create the commit. Passing `ref` makes isomorphic-git update that
    // ref instead of HEAD. Passing `parent` overrides the default
    // (which would be HEAD's tip). Together this lets us write to e.g.
    // refs/openpencil/autosaves/main without touching HEAD or refs/heads/main.
    const ts = Math.floor(Date.now() / 1000);
    const hash = await git.commit({
      fs,
      dir: handle.dir,
      gitdir: handle.gitdir,
      message,
      ref,
      parent: parents,
      author: {
        name: author.name,
        email: author.email,
        timestamp: ts,
        timezoneOffset: 0,
      },
      committer: {
        name: author.name,
        email: author.email,
        timestamp: ts,
        timezoneOffset: 0,
      },
    });

    return { hash };
  } catch (err) {
    if (err instanceof GitError) throw err;
    throw new GitError('engine-crash', `commitFile failed for ${filepath} on ${ref}`, {
      cause: err,
      detail: { filepath, ref },
    });
  }
}

/**
 * Read the contents of a file at a specific commit. Returns the blob as
 * a UTF-8 string. Used by `restoreFileFromCommit` and by the engine's
 * "promote autosave" flow which needs to read the autosave's tree blob.
 */
export async function readBlobAtCommit(opts: {
  handle: IsoRepoHandle;
  filepath: string;
  commitHash: string;
}): Promise<string> {
  const { handle, filepath, commitHash } = opts;
  try {
    const { blob } = await git.readBlob({
      fs,
      gitdir: handle.gitdir,
      oid: commitHash,
      filepath,
    });
    return new TextDecoder('utf-8').decode(blob);
  } catch (err) {
    throw new GitError('engine-crash', `readBlobAtCommit failed for ${filepath} at ${commitHash}`, {
      cause: err,
      detail: { filepath, commitHash },
    });
  }
}

/**
 * Walk commits from a ref tip in reverse chronological order. Returns up to
 * `depth` commits, oldest-last. If the ref doesn't exist, returns an empty
 * array (rather than throwing) — this is convenient for the engine which
 * may query autosave refs that haven't been created yet.
 */
export async function logForRef(opts: {
  handle: IsoRepoHandle;
  ref: string;
  depth: number;
}): Promise<CommitMetaIso[]> {
  const { handle, ref, depth } = opts;
  try {
    // Check existence first; missing ref → empty log.
    try {
      await git.resolveRef({ fs, gitdir: handle.gitdir, ref });
    } catch {
      return [];
    }

    const commits = await git.log({
      fs,
      gitdir: handle.gitdir,
      ref,
      depth,
    });

    return commits.map((c) => ({
      hash: c.oid,
      parentHashes: c.commit.parent,
      message: c.commit.message,
      author: {
        name: c.commit.author.name,
        email: c.commit.author.email,
        timestamp: c.commit.author.timestamp,
      },
    }));
  } catch (err) {
    throw new GitError('engine-crash', `logForRef failed for ${ref}`, {
      cause: err,
      detail: { ref },
    });
  }
}

/**
 * Read a file's contents at a commit and write them to the working tree.
 * Does NOT create a new commit — that's the caller's responsibility (the
 * engine's restore flow runs commitFile afterward to record the restore).
 *
 * The file is written via Node fs.writeFile, NOT via git checkout, because
 * checkout would also update the index and we want the working tree to be
 * dirty after a restore so the user sees pending changes.
 */
export async function restoreFileFromCommit(opts: {
  handle: IsoRepoHandle;
  filepath: string;
  commitHash: string;
}): Promise<void> {
  const { handle, filepath, commitHash } = opts;
  try {
    const content = await readBlobAtCommit({ handle, filepath, commitHash });
    const absPath = resolve(handle.dir, filepath);
    await fs.promises.writeFile(absPath, content, 'utf-8');
  } catch (err) {
    if (err instanceof GitError) throw err;
    throw new GitError(
      'engine-crash',
      `restoreFileFromCommit failed for ${filepath} at ${commitHash}`,
      {
        cause: err,
        detail: { filepath, commitHash },
      },
    );
  }
}

/**
 * List branches under a ref prefix. The prefix is RELATIVE to refs/, so
 * 'heads' lists refs/heads/*, 'openpencil/autosaves' lists
 * refs/openpencil/autosaves/*. Returns just the branch names (last segment),
 * not the full ref paths.
 */
export async function listBranches(opts: {
  handle: IsoRepoHandle;
  prefix?: string;
}): Promise<string[]> {
  const { handle } = opts;
  const prefix = opts.prefix ?? 'heads';
  try {
    if (prefix === 'heads') {
      // Use isomorphic-git's high-level API for the common case.
      return await git.listBranches({ fs, gitdir: handle.gitdir });
    }
    // Custom prefix: walk the gitdir's refs/<prefix>/ directory directly.
    const refsRoot = join(handle.gitdir, 'refs', prefix);
    try {
      const entries = await fs.promises.readdir(refsRoot, { withFileTypes: true });
      return entries.filter((e) => e.isFile()).map((e) => e.name);
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') return [];
      throw err;
    }
  } catch (err) {
    if (err instanceof GitError) throw err;
    throw new GitError('engine-crash', `listBranches failed for prefix ${prefix}`, {
      cause: err,
      detail: { prefix },
    });
  }
}

/**
 * Create a new branch under refs/heads/. If `fromCommit` is omitted, the
 * new branch points at the current HEAD. Throws GitError 'branch-exists'
 * if the name is already taken.
 */
export async function createBranch(opts: {
  handle: IsoRepoHandle;
  name: string;
  fromCommit?: string;
}): Promise<void> {
  const { handle, name, fromCommit } = opts;
  try {
    // Check existence first.
    const existing = await git.listBranches({ fs, gitdir: handle.gitdir });
    if (existing.includes(name)) {
      throw new GitError('branch-exists', `Branch ${name} already exists`);
    }

    if (fromCommit) {
      // Write the ref directly to the specified commit.
      await git.writeRef({
        fs,
        gitdir: handle.gitdir,
        ref: `refs/heads/${name}`,
        value: fromCommit,
        force: false,
      });
    } else {
      // Branch from current HEAD via the high-level API.
      await git.branch({ fs, gitdir: handle.gitdir, ref: name, checkout: false });
    }
  } catch (err) {
    if (err instanceof GitError) throw err;
    throw new GitError('engine-crash', `createBranch failed for ${name}`, {
      cause: err,
      detail: { name, fromCommit },
    });
  }
}

/**
 * Return true if the branch `name` is reachable from the tip of any OTHER
 * branch in refs/heads. "Reachable" means either:
 *   - another branch's tip OID equals `name`'s tip (fast-forward case), OR
 *   - another branch descends from `name`'s tip (merged-via-merge-commit).
 *
 * The equal-tip short-circuit is load-bearing: isomorphic-git's
 * `git.isDescendent` explicitly returns false when `oid === ancestor`
 * (see node_modules/isomorphic-git/index.cjs:12094), so without this
 * shortcut a branch that was just fast-forward merged (or a branch that was
 * just created from HEAD and never advanced) would be incorrectly flagged
 * as unmerged, blocking a legitimate delete.
 */
async function isBranchMergedAnywhere(opts: {
  handle: IsoRepoHandle;
  name: string;
}): Promise<boolean> {
  const { handle, name } = opts;
  const targetOid = await git.resolveRef({ fs, gitdir: handle.gitdir, ref: name });
  const branches = await git.listBranches({ fs, gitdir: handle.gitdir });

  for (const candidate of branches) {
    if (candidate === name) continue;
    const candidateOid = await git.resolveRef({ fs, gitdir: handle.gitdir, ref: candidate });
    if (candidateOid === targetOid) return true; // equal tips = merged (fast-forward case)
    const merged = await git.isDescendent({
      fs,
      dir: handle.dir,
      gitdir: handle.gitdir,
      oid: candidateOid,
      ancestor: targetOid,
      depth: -1,
    });
    if (merged) return true;
  }

  return false;
}

/**
 * Delete a branch under refs/heads/. Throws GitError 'branch-current' if
 * the branch is the active HEAD. Throws GitError 'branch-unmerged' if the
 * branch's tip is not reachable from any other branch, unless `force` is
 * set — in which case the branch is deleted unconditionally.
 *
 * Note: `isomorphic-git`'s `deleteBranch` has no `force` option of its own,
 * so we implement the mergedness check ourselves above the low-level call.
 */
export async function deleteBranch(opts: {
  handle: IsoRepoHandle;
  name: string;
  force?: boolean;
}): Promise<void> {
  const { handle, name, force = false } = opts;
  try {
    const current = await getCurrentBranch({ handle });
    if (current === name) {
      throw new GitError('branch-current', `Cannot delete the current branch ${name}`);
    }
    if (!force) {
      const merged = await isBranchMergedAnywhere({ handle, name });
      if (!merged) {
        throw new GitError('branch-unmerged', `Branch ${name} has unmerged commits`, {
          detail: { name },
        });
      }
    }
    await git.deleteBranch({ fs, gitdir: handle.gitdir, ref: name });
  } catch (err) {
    if (err instanceof GitError) throw err;
    throw new GitError('engine-crash', `deleteBranch failed for ${name}`, {
      cause: err,
      detail: { name, force },
    });
  }
}

/**
 * Switch HEAD to a different branch and update the working tree's tracked
 * file to that branch's tip. Uses filepaths-scoped checkout so other files
 * in the worktree are untouched.
 */
export async function switchBranch(opts: {
  handle: IsoRepoHandle;
  name: string;
  filepath: string;
}): Promise<void> {
  const { handle, name, filepath } = opts;
  try {
    await git.checkout({
      fs,
      dir: handle.dir,
      gitdir: handle.gitdir,
      ref: name,
      filepaths: [filepath],
      force: true,
    });
  } catch (err) {
    throw new GitError('engine-crash', `switchBranch failed for ${name}`, {
      cause: err,
      detail: { name, filepath },
    });
  }
}

/**
 * Return the current branch name (without the 'refs/heads/' prefix), or
 * null if HEAD is detached.
 */
export async function getCurrentBranch(opts: { handle: IsoRepoHandle }): Promise<string | null> {
  const { handle } = opts;
  try {
    const branch = await git.currentBranch({ fs, gitdir: handle.gitdir, fullname: false });
    return branch ?? null;
  } catch (err) {
    throw new GitError('engine-crash', `getCurrentBranch failed`, { cause: err });
  }
}

/**
 * Force a ref to point at the given commit hash. Creates the ref if it does
 * not exist. Used by the engine's milestone commit flow to jump the
 * `refs/openpencil/autosaves/<branch>` ref to a freshly written milestone,
 * abandoning the intermediate autosave chain.
 */
export async function setRef(opts: {
  handle: IsoRepoHandle;
  ref: string;
  value: string;
}): Promise<void> {
  const { handle, ref, value } = opts;
  try {
    await git.writeRef({
      fs,
      gitdir: handle.gitdir,
      ref,
      value,
      force: true,
    });
  } catch (err) {
    throw new GitError('engine-crash', `setRef failed for ${ref}`, {
      cause: err,
      detail: { ref, value },
    });
  }
}

/**
 * Look up the blob OID for `filepath` at the tip of `ref`. Returns null if
 * the ref doesn't exist OR the file isn't present in that commit's tree.
 * Never throws — used by workingDirty detection where missing refs/files are
 * a normal case (fresh repo with no commits).
 */
export async function readBlobOidAt(opts: {
  handle: IsoRepoHandle;
  ref: string;
  filepath: string;
}): Promise<string | null> {
  const { handle, ref, filepath } = opts;
  let tip: string;
  try {
    tip = await git.resolveRef({ fs, gitdir: handle.gitdir, ref });
  } catch {
    return null;
  }
  try {
    const { oid } = await git.readBlob({
      fs,
      gitdir: handle.gitdir,
      oid: tip,
      filepath,
    });
    return oid;
  } catch {
    return null;
  }
}

/**
 * Phase 6a: write the single 'origin' remote in `.git/config`.
 *
 *   - non-empty `url` → `git.addRemote({ remote: 'origin', url, force: true })`
 *     The `force: true` flag makes this an upsert (add OR update). isomorphic-git
 *     itself rejects an existing remote without `force`.
 *   - `null` url     → `git.deleteRemote({ remote: 'origin' })`. Naturally
 *     idempotent: isomorphic-git's `deleteRemote` is implemented as
 *     `GitConfigManager.deleteSection('remote', 'origin')`, which is a
 *     filter over parsed config entries and never throws when the section
 *     is absent. We therefore do NOT wrap it in a try/catch — the only
 *     failure modes are real I/O errors (EACCES, ENOSPC, etc.) from
 *     `GitConfigManager.save`, and those must propagate so the UI can
 *     surface them instead of silently claiming "origin removed" while
 *     `.git/config` still holds the old url.
 *
 * This is a LOCAL config mutation only — never opens a network socket. Lives
 * in git-iso.ts (not git-sys.ts) per the Phase 6a plan: there is no transport
 * decision to make here, so the dispatch helper in git-engine.ts isn't needed.
 */
export async function writeRemoteOrigin(opts: {
  handle: IsoRepoHandle;
  url: string | null;
}): Promise<void> {
  const { handle, url } = opts;
  try {
    if (url === null) {
      await git.deleteRemote({ fs, gitdir: handle.gitdir, remote: 'origin' });
      return;
    }
    await git.addRemote({
      fs,
      gitdir: handle.gitdir,
      remote: 'origin',
      url,
      force: true, // upsert
    });
  } catch (err) {
    throw new GitError('engine-crash', `writeRemoteOrigin failed`, {
      cause: err,
      detail: { url },
    });
  }
}

/**
 * Find the merge base (common ancestor) of two commits. Used by the merge
 * orchestrator to load the `base` document for 3-way merge.
 *
 * Throws GitError 'engine-crash' if no merge base exists (unrelated
 * histories) — Phase 2c does not support merging unrelated histories.
 */
export async function findMergeBase(opts: {
  handle: IsoRepoHandle;
  oid1: string;
  oid2: string;
}): Promise<string> {
  const { handle, oid1, oid2 } = opts;
  try {
    const oids = await git.findMergeBase({
      fs,
      gitdir: handle.gitdir,
      oids: [oid1, oid2],
    });
    if (!oids || oids.length === 0) {
      throw new GitError(
        'engine-crash',
        `No merge base for ${oid1} and ${oid2} (unrelated histories)`,
      );
    }
    return oids[0];
  } catch (err) {
    if (err instanceof GitError) throw err;
    throw new GitError('engine-crash', `findMergeBase failed`, {
      cause: err,
      detail: { oid1, oid2 },
    });
  }
}
