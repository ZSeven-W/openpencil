import { execFile } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { promisify } from 'node:util';
import { mkdtemp } from 'node:fs/promises';
import { beforeAll, describe, expect, it } from 'vitest';

import {
  commitCodegenOutput,
  getCodegenOutputGitStatus,
  pushCodegenOutput,
} from '../codegen-git';

const execFileAsync = promisify(execFile);

async function mkTempDir(prefix = 'op-codegen-git-test-'): Promise<{
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

async function hasSystemGit(): Promise<boolean> {
  try {
    await execFileAsync('git', ['--version'], { timeout: 5000 });
    return true;
  } catch {
    return false;
  }
}

async function git(cwd: string, args: string[]): Promise<string> {
  const { stdout } = await execFileAsync('git', args, { cwd, timeout: 10_000 });
  return stdout;
}

describe('codegen-git', () => {
  let canRunGit = false;

  beforeAll(async () => {
    canRunGit = await hasSystemGit();
  });

  it('returns not-a-repo for directories outside a git worktree', async () => {
    const { dir, dispose } = await mkTempDir();
    try {
      const status = await getCodegenOutputGitStatus({
        rootDir: dir,
        files: [{ path: 'src/App.tsx', content: 'x' }],
      });
      expect(status.mode).toBe('none');
    } finally {
      await dispose();
    }
  });

  it('detects a git worktree and returns diff for written files', async () => {
    if (!canRunGit) return;
    const { dir, dispose } = await mkTempDir();
    try {
      await git(dir, ['init']);
      await git(dir, ['config', 'user.name', 'OpenPencil Test']);
      await git(dir, ['config', 'user.email', 'test@openpencil.local']);
      await mkdir(join(dir, 'src'), { recursive: true });
      await writeFile(join(dir, 'src', 'App.tsx'), 'old content\n', 'utf-8');
      await git(dir, ['add', '.']);
      await git(dir, ['commit', '-m', 'initial']);
      await writeFile(join(dir, 'src', 'App.tsx'), 'new content\n', 'utf-8');
      await writeFile(join(dir, 'src', 'New.tsx'), 'new file\n', 'utf-8');

      const status = await getCodegenOutputGitStatus({
        rootDir: dir,
        files: [
          { path: 'src/App.tsx', content: 'new content\n' },
          { path: 'src/New.tsx', content: 'new file\n' },
        ],
      });

      expect(status.mode).toBe('repo');
      if (status.mode !== 'repo') return;
      expect(status.branch).toBeTruthy();
      expect(status.changedFiles).toEqual(['src/App.tsx', 'src/New.tsx']);
      expect(status.diff).toContain('-old content');
      expect(status.diff).toContain('+new content');
      expect(status.diff).toContain('src/New.tsx');
    } finally {
      await dispose();
    }
  });

  it('commits only generated output files', async () => {
    if (!canRunGit) return;
    const { dir, dispose } = await mkTempDir();
    try {
      await git(dir, ['init']);
      await git(dir, ['config', 'user.name', 'OpenPencil Test']);
      await git(dir, ['config', 'user.email', 'test@openpencil.local']);
      await writeFile(join(dir, 'README.md'), 'keep me dirty\n', 'utf-8');
      await git(dir, ['add', '.']);
      await git(dir, ['commit', '-m', 'initial']);
      await writeFile(join(dir, 'README.md'), 'unrelated dirty change\n', 'utf-8');
      await mkdir(join(dir, 'src'), { recursive: true });
      await writeFile(join(dir, 'src', 'App.tsx'), 'generated\n', 'utf-8');

      const result = await commitCodegenOutput({
        rootDir: dir,
        files: [{ path: 'src/App.tsx', content: 'generated\n' }],
        message: 'add generated code',
        author: { name: 'OpenPencil Test', email: 'test@openpencil.local' },
      });

      expect(result.hash).toMatch(/^[a-f0-9]{40}$/);
      expect(result.changedFiles).toEqual(['src/App.tsx']);
      const committedFiles = await git(dir, ['show', '--name-only', '--format=', 'HEAD']);
      expect(committedFiles.trim()).toBe('src/App.tsx');
      const status = await git(dir, ['status', '--short']);
      expect(status).toContain(' M README.md');
    } finally {
      await dispose();
    }
  });

  it('reports unavailable remote when pushing without an origin', async () => {
    if (!canRunGit) return;
    const { dir, dispose } = await mkTempDir();
    try {
      await git(dir, ['init']);

      await expect(pushCodegenOutput({ rootDir: dir })).rejects.toThrow(/No git remote/);
    } finally {
      await dispose();
    }
  });
});
