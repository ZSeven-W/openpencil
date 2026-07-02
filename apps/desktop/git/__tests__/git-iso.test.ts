// apps/desktop/git/__tests__/git-iso.test.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
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
} from '../git-iso';
import { detectRepo } from '../repo-detector';
import { mkTempDir, writeOpFile } from './test-helpers';

describe('git-iso', () => {
  let temp: { dir: string; dispose: () => Promise<void> };

  beforeEach(async () => {
    temp = await mkTempDir();
  });

  afterEach(async () => {
    await temp.dispose();
  });

  describe('initSingleFile', () => {
    it('creates .op-history/<basename>.git/ with HEAD pointing at refs/heads/main', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const handle = await initSingleFile({ filePath: opFile });

      expect(handle.mode).toBe('single-file');
      expect(handle.dir).toBe(temp.dir);
      expect(handle.gitdir).toBe(join(temp.dir, '.op-history', 'login.op.git'));
      expect(existsSync(join(handle.gitdir, 'HEAD'))).toBe(true);
    });

    it('is idempotent: re-running init on an existing repo returns the same handle without erroring', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const first = await initSingleFile({ filePath: opFile });
      const second = await initSingleFile({ filePath: opFile });
      expect(second.gitdir).toBe(first.gitdir);
      expect(second.dir).toBe(first.dir);
      expect(existsSync(join(second.gitdir, 'HEAD'))).toBe(true);
    });

    it('respects the defaultBranch option', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const handle = await initSingleFile({ filePath: opFile, defaultBranch: 'trunk' });
      // isomorphic-git writes HEAD as a symbolic ref string
      const fs = await import('node:fs/promises');
      const head = await fs.readFile(join(handle.gitdir, 'HEAD'), 'utf-8');
      expect(head.trim()).toBe('ref: refs/heads/trunk');
    });

    it('writes core.worktree = ../.. so terminal git can inspect the repo', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const handle = await initSingleFile({ filePath: opFile });
      // Read the on-disk config and verify both core.worktree and core.bare.
      const fsp = await import('node:fs/promises');
      const config = await fsp.readFile(join(handle.gitdir, 'config'), 'utf-8');
      expect(config).toMatch(/worktree\s*=\s*\.\.\/\.\./);
      expect(config).toMatch(/bare\s*=\s*false/);
    });
  });

  describe('openRepo', () => {
    it('opens an existing single-file repo via a successful detection', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      await initSingleFile({ filePath: opFile });

      const detection = await detectRepo(opFile);
      expect(detection.mode).toBe('single-file');
      if (detection.mode !== 'single-file') throw new Error('detection failed');

      const handle = await openRepo(detection);
      expect(handle.mode).toBe('single-file');
      expect(handle.dir).toBe(temp.dir);
    });
  });

  describe('commitFile + readBlobAtCommit', () => {
    it('creates a commit on the given ref and the blob round-trips', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });

      const { hash } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'initial',
        author: { name: 'tester', email: 'tester@example.com' },
      });
      expect(hash).toMatch(/^[a-f0-9]{40}$/);

      const content = await readBlobAtCommit({
        handle,
        filepath: 'login.op',
        commitHash: hash,
      });
      const parsed = JSON.parse(content);
      expect(parsed.children[0].id).toBe('r1');
    });

    it('throws commit-empty when committing the same content twice', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await expect(
        commitFile({
          handle,
          filepath: 'login.op',
          ref: 'refs/heads/main',
          message: 'second',
          author: { name: 't', email: 't@example.com' },
        }),
      ).rejects.toMatchObject({ name: 'GitError', code: 'commit-empty' });
    });

    it('chains commits: each commit has the previous as parent', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });

      const { hash: h1 } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });

      // Mutate the file then commit again.
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r2' }] });
      const { hash: h2 } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'second',
        author: { name: 't', email: 't@example.com' },
      });

      expect(h2).not.toBe(h1);
      // Verify h1 content is recoverable
      const c1 = await readBlobAtCommit({ handle, filepath: 'login.op', commitHash: h1 });
      expect(JSON.parse(c1).children[0].id).toBe('r1');
      // Verify h2 content
      const c2 = await readBlobAtCommit({ handle, filepath: 'login.op', commitHash: h2 });
      expect(JSON.parse(c2).children[0].id).toBe('r2');
    });
  });

  describe('logForRef', () => {
    it('returns commits in reverse chronological order (newest first)', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });

      const { hash: h1 } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r2' }] });
      const { hash: h2 } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'second',
        author: { name: 't', email: 't@example.com' },
      });

      const log = await logForRef({ handle, ref: 'refs/heads/main', depth: 10 });
      expect(log).toHaveLength(2);
      expect(log[0].hash).toBe(h2);
      expect(log[0].message).toBe('second\n');
      expect(log[1].hash).toBe(h1);
      expect(log[1].message).toBe('first\n');
    });

    it('returns an empty array for a ref that does not exist', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const handle = await initSingleFile({ filePath: opFile });
      const log = await logForRef({
        handle,
        ref: 'refs/openpencil/autosaves/main',
        depth: 10,
      });
      expect(log).toEqual([]);
    });

    it('respects the depth parameter', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      // Make 3 commits.
      for (let i = 0; i < 3; i++) {
        await writeOpFile(temp.dir, 'login.op', {
          version: '1.0.0',
          children: [{ id: `r${i}` }],
        });
        await commitFile({
          handle,
          filepath: 'login.op',
          ref: 'refs/heads/main',
          message: `commit-${i}`,
          author: { name: 't', email: 't@example.com' },
        });
      }
      const log = await logForRef({ handle, ref: 'refs/heads/main', depth: 2 });
      expect(log).toHaveLength(2);
    });
  });

  describe('restoreFileFromCommit', () => {
    it("writes a previous commit's content to the working tree", async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      const { hash: h1 } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      // Mutate
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r2' }] });
      // Restore
      await restoreFileFromCommit({ handle, filepath: 'login.op', commitHash: h1 });
      // Verify the working tree file was overwritten
      const fsp = await import('node:fs/promises');
      const restored = await fsp.readFile(opFile, 'utf-8');
      expect(JSON.parse(restored).children[0].id).toBe('r1');
    });

    it('does NOT create a new commit (the caller is responsible)', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      const { hash: h1 } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r2' }] });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'second',
        author: { name: 't', email: 't@example.com' },
      });
      const logBefore = await logForRef({ handle, ref: 'refs/heads/main', depth: 10 });

      await restoreFileFromCommit({ handle, filepath: 'login.op', commitHash: h1 });
      const logAfter = await logForRef({ handle, ref: 'refs/heads/main', depth: 10 });
      expect(logAfter).toHaveLength(logBefore.length); // unchanged
    });
  });

  describe('branch operations', () => {
    it('listBranches returns empty for a fresh repo with no commits', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const handle = await initSingleFile({ filePath: opFile });
      const branches = await listBranches({ handle });
      expect(branches).toEqual([]); // no commits = no branches
    });

    it('listBranches returns the branch after a commit creates it', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      const branches = await listBranches({ handle });
      expect(branches).toContain('main');
    });

    it('createBranch from current HEAD adds a new branch', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await createBranch({ handle, name: 'feature-x' });
      const branches = await listBranches({ handle });
      expect(branches).toContain('feature-x');
      expect(branches).toContain('main');
    });

    it('createBranch throws branch-exists when the name is taken', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await createBranch({ handle, name: 'feature-x' });
      await expect(createBranch({ handle, name: 'feature-x' })).rejects.toMatchObject({
        name: 'GitError',
        code: 'branch-exists',
      });
    });

    it('deleteBranch throws branch-current when deleting the active branch', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await expect(deleteBranch({ handle, name: 'main' })).rejects.toMatchObject({
        name: 'GitError',
        code: 'branch-current',
      });
    });

    it('deleteBranch removes a non-active branch', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await createBranch({ handle, name: 'feature-x' });
      await deleteBranch({ handle, name: 'feature-x' });
      const branches = await listBranches({ handle });
      expect(branches).not.toContain('feature-x');
      expect(branches).toContain('main');
    });

    it('deleteBranch throws branch-unmerged for an unmerged branch without force', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });
      await createBranch({ handle, name: 'feature-x' });
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature-x',
        message: 'feature only',
        author: { name: 't', email: 't@example.com' },
      });
      await expect(deleteBranch({ handle, name: 'feature-x' })).rejects.toMatchObject({
        name: 'GitError',
        code: 'branch-unmerged',
      });
    });

    it('deleteBranch removes an unmerged branch when force=true', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });
      await createBranch({ handle, name: 'feature-x' });
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature-x',
        message: 'feature only',
        author: { name: 't', email: 't@example.com' },
      });
      await deleteBranch({ handle, name: 'feature-x', force: true });
      const branches = await listBranches({ handle });
      expect(branches).toEqual(['main']);
    });

    it('deleteBranch treats a fast-forward-merged branch as merged (equal OID tips)', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });
      // Branch off main with no additional commits: feature-ff tip === main tip.
      await createBranch({ handle, name: 'feature-ff' });
      // Without the equal-OID short-circuit in isBranchMergedAnywhere this would
      // throw branch-unmerged because isomorphic-git's isDescendent returns
      // false when oid === ancestor.
      await deleteBranch({ handle, name: 'feature-ff' });
      const branches = await listBranches({ handle });
      expect(branches).toEqual(['main']);
    });

    it('getCurrentBranch returns the active branch name', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      const current = await getCurrentBranch({ handle });
      expect(current).toBe('main');
    });

    it('switchBranch updates the working tree file to the target branch tip', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      // Branch off, then commit something different on the new branch.
      await createBranch({ handle, name: 'feature-x' });
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r2' }] });
      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature-x',
        message: 'feature change',
        author: { name: 't', email: 't@example.com' },
      });
      // Switch back to main and verify the file content reverts.
      await switchBranch({ handle, name: 'main', filepath: 'login.op' });
      const fsp = await import('node:fs/promises');
      const content = await fsp.readFile(opFile, 'utf-8');
      expect(JSON.parse(content).children[0].id).toBe('r1');
    });
  });

  describe('setRef + readBlobOidAt', () => {
    it('setRef creates a new ref pointing at the given commit and force-overwrites an existing one', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });
      const { hash: h1 } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });

      // Create a brand-new ref pointing at h1.
      await setRef({ handle, ref: 'refs/openpencil/autosaves/main', value: h1 });
      const log1 = await logForRef({
        handle,
        ref: 'refs/openpencil/autosaves/main',
        depth: 10,
      });
      expect(log1).toHaveLength(1);
      expect(log1[0].hash).toBe(h1);

      // Make a second milestone, then force the autosave ref to it (overwrite).
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r2' }] });
      const { hash: h2 } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'second',
        author: { name: 't', email: 't@example.com' },
      });
      await setRef({ handle, ref: 'refs/openpencil/autosaves/main', value: h2 });
      const log2 = await logForRef({
        handle,
        ref: 'refs/openpencil/autosaves/main',
        depth: 10,
      });
      // The autosave ref now jumps to h2 — its log walks h2 → h1 (parent chain).
      expect(log2[0].hash).toBe(h2);
    });

    it('readBlobOidAt returns the blob OID at the ref tip and null for missing ref/file', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });

      // Before any commit, the ref doesn't exist → null.
      const before = await readBlobOidAt({
        handle,
        ref: 'refs/heads/main',
        filepath: 'login.op',
      });
      expect(before).toBeNull();

      await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });

      // After commit, the OID is a 40-char hex string.
      const after = await readBlobOidAt({
        handle,
        ref: 'refs/heads/main',
        filepath: 'login.op',
      });
      expect(after).toMatch(/^[a-f0-9]{40}$/);

      // Asking for a file that's not in the commit's tree → null.
      const missing = await readBlobOidAt({
        handle,
        ref: 'refs/heads/main',
        filepath: 'does-not-exist.op',
      });
      expect(missing).toBeNull();
    });
  });

  describe('findMergeBase', () => {
    it('returns the common ancestor of two divergent branches', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const handle = await initSingleFile({ filePath: opFile });

      const { hash: base } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });

      // Branch off and commit on each branch.
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r2' }] });
      const { hash: ours } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature-a',
        message: 'ours',
        author: { name: 't', email: 't@example.com' },
        parents: [base],
      });
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r3' }] });
      const { hash: theirs } = await commitFile({
        handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature-b',
        message: 'theirs',
        author: { name: 't', email: 't@example.com' },
        parents: [base],
      });

      const mergeBase = await findMergeBase({ handle, oid1: ours, oid2: theirs });
      expect(mergeBase).toBe(base);
    });
  });
});
