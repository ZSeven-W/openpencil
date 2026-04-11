// apps/desktop/git/repo-detector.ts
//
// Discover whether a .op file lives inside a git repository, and if so,
// in which mode (single-file or folder). Returns a discriminated union
// the engine can match against without follow-up filesystem checks.

import { dirname, basename, resolve } from 'node:path';
import { stat } from 'node:fs/promises';

/**
 * Result of a successful detection. The shape is the same for both modes
 * so the engine can pass it directly to `openRepo` regardless of mode.
 */
export interface RepoDetectionFound {
  mode: 'single-file' | 'folder';
  /** worktree root (parent of the .op file in single-file mode; repo root in folder mode) */
  rootPath: string;
  /** absolute path to the gitdir */
  gitdir: string;
}

export type RepoDetection = RepoDetectionFound | { mode: 'none' };

/**
 * Walk up from the given .op file looking for a tracked repository.
 *
 * Order of checks (single-file wins per spec):
 *   1. <dirname(filePath)>/.op-history/<basename(filePath)>.git/HEAD exists
 *      → single-file mode
 *   2. Walk up parent dirs looking for any /.git/HEAD → folder mode
 *   3. Otherwise → none
 *
 * The function never throws on missing files; only on filesystem errors that
 * indicate something deeper is wrong (permissions, broken symlinks). Those
 * propagate as standard Node errors and are NOT wrapped in GitError — the
 * engine layer is responsible for translation.
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
