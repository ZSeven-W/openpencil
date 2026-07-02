// apps/desktop/git/__tests__/test-helpers.ts
//
// Shared test utilities for the desktop git layer tests. Each test creates
// a fresh temp dir, runs its operation, and cleans up via the returned
// disposer. This keeps tests isolated and parallel-safe.

import { mkdtemp, rm, writeFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

/**
 * Create a fresh temp directory under the OS temp path. Returns the path
 * and a disposer that recursively removes it. Always pair the call with
 * `try { ... } finally { await dispose(); }`.
 */
export async function mkTempDir(prefix = 'op-git-test-'): Promise<{
  dir: string;
  dispose: () => Promise<void>;
}> {
  const dir = await mkdtemp(join(tmpdir(), prefix));
  return {
    dir,
    dispose: async () => {
      await rm(dir, { recursive: true, force: true });
    },
  };
}

/**
 * Write a stub `.op` file (a tiny PenDocument JSON) into a directory.
 * Returns the absolute file path.
 */
export async function writeOpFile(
  dir: string,
  name: string,
  content: object = { version: '1.0.0', children: [] },
): Promise<string> {
  const path = join(dir, name);
  await writeFile(path, JSON.stringify(content), 'utf-8');
  return path;
}

/**
 * Create a nested directory structure under a temp root.
 * Useful for setting up "file inside parent git repo" scenarios.
 */
export async function mkSubdir(root: string, ...segments: string[]): Promise<string> {
  const dir = join(root, ...segments);
  await mkdir(dir, { recursive: true });
  return dir;
}
