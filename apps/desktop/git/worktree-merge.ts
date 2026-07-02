// apps/desktop/git/worktree-merge.ts
//
// System-git helpers for folder-mode merge operations. These are the only
// functions in the git layer that shell out to the system git binary for
// merge state management — everything else uses isomorphic-git.
//
// DESIGN NOTE (Phase 7a spike):
//   We use system git's merge machinery because isomorphic-git has no
//   equivalent of --no-commit --no-ff merges and cannot write the three-stage
//   index entries needed for conflict detection. The exact command sequence
//   was chosen after verifying each shape against a live repo:
//
//   1. `git merge --no-commit --no-ff <ref>`  — enters merge state; exits 1
//      on conflicts, exits 0 on clean merge (but still --no-commit so we
//      can write the tracked file before committing).
//   2. `git ls-files -u`  — lists all unresolved paths (all conflict types,
//      not just "both modified"), along with stage numbers 1/2/3.
//   3. `git show :1:<path>`, `:2:<path>`, `:3:<path>`  — reads base/ours/
//      theirs blobs from the index without touching the working tree.
//   4. `git checkout --ours -- <file>`  — writes the ours version to disk so
//      the tracked .op file is readable JSON; file stays "unresolved" in the
//      index so MERGE_HEAD and other unresolved files survive.
//   5. `git add <file>`  — marks a file resolved in the index.
//   6. `git commit -m <message>`  — when MERGE_HEAD is present, git
//      automatically creates a 2-parent merge commit.
//   7. `git merge --abort`  — atomically restores the working tree and index.

import { execFile } from 'node:child_process';
import { promises as fsp } from 'node:fs';
import { join } from 'node:path';
import { promisify } from 'node:util';
import { GitError } from './error';

const execFileAsync = promisify(execFile);

const DEFAULT_TIMEOUT_MS = 60_000;

interface RunOpts {
  cwd: string;
  env?: Record<string, string>;
  timeoutMs?: number;
}

interface RunResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

/**
 * Run `git <args>`. Unlike the private runGit in git-sys.ts, this version
 * tolerates non-zero exits and returns the exit code so callers can
 * distinguish "conflict" from "error" — `git merge` exits 1 on conflicts
 * but that is not an error from the caller's perspective.
 */
async function runGitTolerant(args: string[], opts: RunOpts): Promise<RunResult> {
  const env = { ...process.env, ...opts.env };
  try {
    const { stdout, stderr } = await execFileAsync('git', args, {
      cwd: opts.cwd,
      env,
      timeout: opts.timeoutMs ?? DEFAULT_TIMEOUT_MS,
      maxBuffer: 32 * 1024 * 1024,
    });
    return { stdout, stderr, exitCode: 0 };
  } catch (err) {
    const e = err as NodeJS.ErrnoException & { stderr?: string; stdout?: string; code?: number };
    // exitCode is the numeric exit code from the child process; undefined if it
    // was killed by a signal (which we map to -1).
    const exitCode = typeof e.code === 'number' ? e.code : -1;
    return {
      stdout: e.stdout ?? '',
      stderr: e.stderr ?? '',
      exitCode,
    };
  }
}

// ---------------------------------------------------------------------------
// Public helpers
// ---------------------------------------------------------------------------

/**
 * Attempt to merge `ref` into the current branch without auto-committing.
 * Uses `--no-ff` to always produce a merge commit even for fast-forwards.
 *
 * Returns:
 *   - { kind: 'clean' }     — merge succeeded with no conflicts; index is staged
 *                             but not committed (MERGE_HEAD is set).
 *   - { kind: 'conflict' }  — one or more conflicts; MERGE_HEAD is set, unresolved
 *                             files remain in the index at conflict stages.
 *   - throws GitError       — on engine-level failures (not available, unknown ref, etc.)
 *
 * NOTE: some git versions read user identity during merge bookkeeping even with
 * --no-commit. Callers must ensure the repo has user.name/user.email configured
 * (or inject them via opts.env) — machines without a global git config will fail.
 */
export async function sysMergeNoCommit(opts: {
  cwd: string;
  ref: string;
  env?: Record<string, string>;
}): Promise<{ kind: 'clean' | 'conflict' }> {
  const result = await runGitTolerant(['merge', '--no-commit', '--no-ff', opts.ref], {
    cwd: opts.cwd,
    env: opts.env,
  });

  if (result.exitCode === 0) return { kind: 'clean' };

  // Exit code 1 from `git merge` means conflicts. Any other code is an error.
  if (result.exitCode === 1) return { kind: 'conflict' };

  throw new GitError(
    'engine-crash',
    `git merge --no-commit failed: ${result.stderr.trim() || result.stdout.trim()}`,
    { detail: { ref: opts.ref, exitCode: result.exitCode } },
  );
}

/**
 * List all unresolved file paths in the current merge state.
 * Uses `git ls-files -u` which reports ALL conflict types (both-modified,
 * deleted-by-them, etc.), not just `--diff-filter=U` which only reports
 * "both modified". Returns deduplicated, sorted paths.
 *
 * MINIMUM GIT VERSION: `--format=%(path)` requires git ≥ 2.35 (Feb 2022).
 * No version check or fallback is provided here — callers must ensure the
 * system git is new enough. Document this floor in deployment requirements.
 */
export async function sysListUnresolved(opts: { cwd: string }): Promise<string[]> {
  const result = await runGitTolerant(['ls-files', '-u', '--format=%(path)'], {
    cwd: opts.cwd,
  });

  if (result.exitCode !== 0) {
    throw new GitError('engine-crash', `git ls-files -u failed: ${result.stderr.trim()}`, {
      detail: { exitCode: result.exitCode },
    });
  }

  const paths = result.stdout
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);

  // Deduplicate: each unresolved path appears 2-3 times (one per stage).
  return [...new Set(paths)].sort();
}

/**
 * Detect whether a merge is in progress by checking for MERGE_HEAD in the
 * gitdir. Does NOT run git — pure filesystem check. Returns the theirs
 * commit hash if in progress, null otherwise.
 */
export async function readMergeHead(gitdir: string): Promise<string | null> {
  const mergeHeadPath = join(gitdir, 'MERGE_HEAD');
  try {
    const content = await fsp.readFile(mergeHeadPath, 'utf-8');
    const hash = content.trim();
    if (hash.length === 40) return hash;
    return null;
  } catch {
    return null;
  }
}

/**
 * Read the content of a tracked file from the index at a specific stage:
 *   stage 1 = base (merge-base ancestor)
 *   stage 2 = ours (HEAD)
 *   stage 3 = theirs (MERGE_HEAD)
 *
 * Returns null if the file is not present at that stage (e.g. deleted-by-them
 * conflict has no stage 3, only stages 1 and 2).
 */
export async function sysShowStageBlob(opts: {
  cwd: string;
  stage: 1 | 2 | 3;
  filepath: string;
}): Promise<string | null> {
  const stageRef = `:${opts.stage}:${opts.filepath}`;
  const result = await runGitTolerant(['show', stageRef], { cwd: opts.cwd });

  if (result.exitCode === 0) return result.stdout;

  // Non-zero exit means the file doesn't exist at this stage — that is not
  // an error, it's a normal state (e.g. deleted-by-them has no :3:).
  return null;
}

/**
 * Restore the working-tree content of a tracked file to the "ours" version
 * (stage 2) so the renderer can read readable JSON instead of conflict
 * markers. The file stays "unresolved" in the index — MERGE_HEAD survives.
 *
 * The exact behaviour was verified in the Phase 7a spike:
 *   `git checkout --ours -- <file>` writes stage 2 to disk and leaves the
 *   index at conflict stages (1/2/3). `git diff --name-only --diff-filter=U`
 *   still reports the file as unresolved after this call.
 */
export async function sysRestoreOurs(opts: { cwd: string; filepath: string }): Promise<void> {
  const result = await runGitTolerant(['checkout', '--ours', '--', opts.filepath], {
    cwd: opts.cwd,
  });

  if (result.exitCode !== 0) {
    throw new GitError(
      'engine-crash',
      `git checkout --ours failed for ${opts.filepath}: ${result.stderr.trim()}`,
      { detail: { filepath: opts.filepath, exitCode: result.exitCode } },
    );
  }
}

/**
 * Stage a file, marking it as resolved in the index. Used after the tracked
 * .op file has been written with the final merged document so git accepts the
 * merge commit.
 */
export async function sysStageFile(opts: { cwd: string; filepath: string }): Promise<void> {
  const result = await runGitTolerant(['add', '--', opts.filepath], { cwd: opts.cwd });

  if (result.exitCode !== 0) {
    throw new GitError(
      'engine-crash',
      `git add failed for ${opts.filepath}: ${result.stderr.trim()}`,
      { detail: { filepath: opts.filepath, exitCode: result.exitCode } },
    );
  }
}

/**
 * Finalize the merge by creating the merge commit. MERGE_HEAD must be set.
 * When MERGE_HEAD is present, git automatically records both parents.
 *
 * Returns the new merge commit hash.
 */
export async function sysFinalizeMerge(opts: {
  cwd: string;
  message: string;
  author: { name: string; email: string };
  env?: Record<string, string>;
}): Promise<string> {
  const env: Record<string, string> = {
    ...opts.env,
    GIT_AUTHOR_NAME: opts.author.name,
    GIT_AUTHOR_EMAIL: opts.author.email,
    GIT_COMMITTER_NAME: opts.author.name,
    GIT_COMMITTER_EMAIL: opts.author.email,
  };

  const result = await runGitTolerant(['commit', '-m', opts.message], {
    cwd: opts.cwd,
    env,
  });

  if (result.exitCode !== 0) {
    throw new GitError(
      'engine-crash',
      `git commit (merge finalize) failed: ${result.stderr.trim()}`,
      { detail: { exitCode: result.exitCode } },
    );
  }

  // Parse the new commit hash from `git rev-parse HEAD`.
  const headResult = await runGitTolerant(['rev-parse', 'HEAD'], { cwd: opts.cwd });
  if (headResult.exitCode !== 0 || !headResult.stdout.trim()) {
    throw new GitError('engine-crash', 'Failed to read HEAD after merge commit');
  }
  return headResult.stdout.trim();
}

/**
 * Abort an in-progress merge. Restores the working tree and index to pre-merge
 * state. Idempotent: safe to call even if no merge is in progress (git merge
 * --abort exits 0 with a warning in that case on modern git versions).
 */
export async function sysAbortMerge(opts: { cwd: string }): Promise<void> {
  const result = await runGitTolerant(['merge', '--abort'], { cwd: opts.cwd });

  // Exit code 0 = success. Exit code 128 with "MERGE_HEAD missing" means there
  // was no merge in progress — treat that as idempotent success.
  if (result.exitCode === 0) return;

  const msg = (result.stderr + result.stdout).toLowerCase();
  if (msg.includes('merge_head') || msg.includes('no merge in progress')) {
    return; // Nothing to abort — already clean.
  }

  throw new GitError('merge-abort-failed', `git merge --abort failed: ${result.stderr.trim()}`, {
    detail: { exitCode: result.exitCode },
  });
}

/**
 * Read the current HEAD commit hash. Throws if HEAD cannot be resolved
 * (e.g. repo has no commits).
 */
export async function sysReadHead(opts: { cwd: string }): Promise<string> {
  const result = await runGitTolerant(['rev-parse', 'HEAD'], { cwd: opts.cwd });
  if (result.exitCode !== 0 || !result.stdout.trim()) {
    throw new GitError('engine-crash', `git rev-parse HEAD failed: ${result.stderr.trim()}`, {
      detail: { exitCode: result.exitCode },
    });
  }
  return result.stdout.trim();
}
