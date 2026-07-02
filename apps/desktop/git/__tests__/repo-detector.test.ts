// apps/desktop/git/__tests__/repo-detector.test.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { detectRepo } from '../repo-detector';
import { mkTempDir, writeOpFile, mkSubdir } from './test-helpers';

describe('detectRepo', () => {
  let temp: { dir: string; dispose: () => Promise<void> };

  beforeEach(async () => {
    temp = await mkTempDir();
  });

  afterEach(async () => {
    await temp.dispose();
  });

  it('returns single-file mode when .op-history/<basename>.git/HEAD exists adjacent', async () => {
    const opFile = await writeOpFile(temp.dir, 'login.op');
    const gitdir = await mkSubdir(temp.dir, '.op-history', 'login.op.git');
    await writeFile(join(gitdir, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');

    const result = await detectRepo(opFile);
    expect(result.mode).toBe('single-file');
    if (result.mode === 'single-file') {
      expect(result.rootPath).toBe(resolve(temp.dir));
      expect(result.gitdir).toBe(resolve(gitdir));
    }
  });

  it('returns folder mode when the file lives inside a directory containing .git/HEAD', async () => {
    const repoRoot = await mkSubdir(temp.dir, 'repo');
    const dotGit = await mkSubdir(repoRoot, '.git');
    await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
    const opFile = await writeOpFile(repoRoot, 'design.op');

    const result = await detectRepo(opFile);
    expect(result.mode).toBe('folder');
    if (result.mode === 'folder') {
      expect(result.rootPath).toBe(resolve(repoRoot));
      expect(result.gitdir).toBe(resolve(dotGit));
    }
  });

  it('returns folder mode when the file is nested several levels inside a parent git repo', async () => {
    const repoRoot = await mkSubdir(temp.dir, 'project');
    const dotGit = await mkSubdir(repoRoot, '.git');
    await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
    const designsDir = await mkSubdir(repoRoot, 'designs', 'login');
    const opFile = await writeOpFile(designsDir, 'login.op');

    const result = await detectRepo(opFile);
    expect(result.mode).toBe('folder');
    if (result.mode === 'folder') {
      expect(result.rootPath).toBe(resolve(repoRoot));
    }
  });

  it('prefers single-file mode when both single-file and parent folder repos exist', async () => {
    // Set up: a parent .git AND a sibling .op-history. Spec says single-file wins.
    const repoRoot = await mkSubdir(temp.dir, 'project');
    const parentGit = await mkSubdir(repoRoot, '.git');
    await writeFile(join(parentGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
    const opFile = await writeOpFile(repoRoot, 'login.op');
    const singleGitdir = await mkSubdir(repoRoot, '.op-history', 'login.op.git');
    await writeFile(join(singleGitdir, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');

    const result = await detectRepo(opFile);
    expect(result.mode).toBe('single-file');
    if (result.mode === 'single-file') {
      expect(result.gitdir).toBe(resolve(singleGitdir));
    }
  });

  it('returns none when no repository is found anywhere up the parent chain', async () => {
    const opFile = await writeOpFile(temp.dir, 'orphan.op');
    const result = await detectRepo(opFile);
    expect(result.mode).toBe('none');
  });

  it('does not blow up when given a file path with non-existent parent directories', async () => {
    // The walk-up should still produce a 'none' result, not throw.
    const fakePath = join(temp.dir, 'does-not-exist', 'nested', 'fake.op');
    const result = await detectRepo(fakePath);
    expect(result.mode).toBe('none');
  });
});
