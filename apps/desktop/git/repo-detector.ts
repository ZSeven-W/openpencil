// apps/desktop/git/repo-detector.ts
//
// Discover whether a .op file lives inside a git repository, and if so,
// in which mode (single-file or folder). Returns a discriminated union
// 引擎无需后续文件系统检查即可进行匹配。

import { dirname, basename, resolve } from 'node:path';
import { stat } from 'node:fs/promises';

/**
 * Result of a
 * successful detection.两种模式的 The 形状相同，因此无论模式如何，引擎都可以将其直接传递给 `openRepo`。
 */
export interface RepoDetectionFound {
  mode: 'single-file' | 'folder';
  /** worktree root (parent of the .op file in single-file mode; repo root in folder mode) */
  rootPath: string;
  /** gitdir 的绝对路径 */
  gitdir: string;
}

export type RepoDetection = RepoDetectionFound | { mode: 'none' };

/**
 * Walk up from
 *
 * the given .op file looking for a tracked repository.检查
 * Order（每个规范单文
 * 件获胜）： 1. <dirname(filePath)>/.op-history/<ba
 * sename(filePath)>.git/HEAD 存在 → 单文件模式 2. Walk 向上父目录查找任何 /.git/HEAD →
 * 文件夹模式 3. Otherwise
 *
 * → 无 The 函数永远不会抛出丢失的文件；仅针对表明更深层次错误（权限、损坏的符号链接）的文件系统错误。 Those
 * 作为标准 Node 错误传播，并且
 * NOT 包装在
 * GitError 中 - 引擎层负责翻译。
 */
export async function detectRepo(filePath: string): Promise<RepoDetection> {
  const absFile = resolve(filePath);
  const parentDir = dirname(absFile);
  const baseName = basename(absFile);

  // 1. Single-file mode check.
  const singleGitdir = resolve(parentDir, '.op-history', `${baseName}.git`);
  if (await pathExists(resolve(singleGitdir, 'HEAD'))) {
    return {
      mode: 'single-file',
      rootPath: parentDir,
      gitdir: singleGitdir,
    };
  }

  // 2. Walk up parents looking for a .git directory.
  let current = parentDir;
  while (true) {
    const candidate = resolve(current, '.git');
    if (await pathExists(resolve(candidate, 'HEAD'))) {
      return {
        mode: 'folder',
        rootPath: current,
        gitdir: candidate,
      };
    }
    const parent = dirname(current);
    if (parent === current) {
      // Reached filesystem root.
      break;
    }
    current = parent;
  }

  // 3. No repo found.
  return { mode: 'none' };
}

/**
 * Returns true if `path` exists (file or directory). Returns false on ENOENT.
 * Re-throws other errors (permission denied, etc.) so we don't silently
 * misbehave.
 */
async function pathExists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return false;
    throw err;
  }
}
