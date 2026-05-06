// apps/desktop/git/worktree-merge.ts
//
// System-git 文件夹模式合并操作的助手。 These 是唯一的
// git 层中的函数，将 shell 输出到系统 git 二进制文件
// 合并状态管理——其他一切都使用 isomorphic-git。
//
// DESIGN NOTE（Phase 7a 尖峰）：
// We 使用系统 git 的合并机制，因为 isomorphic-git 没有
// 相当于--no-commit --no-ff 合并，不能写三阶段
// 冲突检测所需的索引条目。 The 确切的命令序列
// 在根据实时存储库验证每个形状后选择：
//
//   1. `git merge --no-commit --no-ff <ref>` — 进入合并状态；出口 1
// 发生冲突时，在干净合并时退出 0（但仍然是 --no-commit，所以我们
// 可以在提交之前写入跟踪文件）。
//   2. `git ls-files -u` — 列出所有未解析的路径（所有冲突类型，
// 不仅仅是“都修改了”），以及阶段编号 1/2/3。
//   3. `git show :1:<path>`、`:2:<path>`、`:3:<path>` — 读取 base/ours/
// 他们的 blob 从索引中取出，而不触及工作树。
//   4. `git checkout --ours -- <file>` — 将我们的版本写入磁盘
// 被跟踪的.op 文件是可读的 JSON；文件保持“未解决”状态
// 索引，因此 MERGE_HEAD 和其他未解析的文件仍然存在。
//   5. `git add <file>` — 标记在索引中解析的文件。
//   6. `git commit -m <message>` — 当 MERGE_HEAD 存在时，git
// 自动创建 2-parent 合并提交。
//   7. `git merge --abort` — 自动恢复工作树和索引。

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
 * Run `git
 * <args>`。 Unlike git-sys.ts 中的私有 runGit，此版本容忍非零退出并返回退出代码，以便调用者可以区分“冲突”和“
 * 错误” - `git merge` 在冲突时退出 1，但从调用者的角度来看这不是错误。
 *
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
    // exitCode 是子进程的数字退出代码；如果它被信号杀死（我们将其映射为-1），则未定义。
    const exitCode = typeof e.code === 'number' ? e.code : -1;
    return {
      stdout: e.stdout ?? '',
      stderr: e.stderr ?? '',
      exitCode,
    };
  }
}

// ---------------------------------------------------------------------------
// Public 帮助者
// ---------------------------------------------------------------------------

/**
 * Attempt 将
 * `ref` 合并到当前分支而不自动提交。 Uses `--no-ff` 始终生成合并提交，即使对于快进也是如此。 Returns: - {
 *
 * kind: 'clean' } — 合并成功，没有冲突；索引已暂存但未提交（已设置 MERGE_HEAD）。 - { kind:
 * 'conflict' }
 * — 一个或多个冲突；设置
 * MERGE_HEAD 后，未解析的文件在冲突阶段保留在索引中。 - 抛出 GitError — 引擎级故障（不可用、未知引用等） NOTE：某些 git
 * 版本在合并簿记期间读取用户身份，即使使用 --no-commit。
 * Callers 必须确保存储库配置了 user.name/user.email（或通过 opts.env 注入它们）——没有全局 git
 *
 * 配置的机器将会失败。
 *
 *
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

  // `git merge` 中的 Exit 代码 1 表示冲突。 Any 其他代码是错误的。
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
