// apps/desktop/git/__tests__/git-engine.test.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join, resolve } from 'node:path';
import { writeFile } from 'node:fs/promises';
import { promises as fsp } from 'node:fs';
import { execFile, execFileSync } from 'node:child_process';
import { promisify } from 'node:util';
import {
  engineDetect,
  engineInit,
  engineOpen,
  engineBindTrackedFile,
  engineListCandidates,
  engineClose,
  engineStatus,
  engineLog,
  engineCommit,
  engineRestore,
  enginePromote,
  engineBranchList,
  engineBranchCreate,
  engineBranchSwitch,
  engineBranchDelete,
  engineFetch,
  enginePull,
  enginePush,
  engineDiff,
  engineBranchMerge,
  engineResolveConflict,
  engineApplyMerge,
  engineAbortMerge,
  engineRemoteGet,
  engineRemoteSet,
} from '../git-engine';
import { clearAllSessions, sessionCount } from '../repo-session';
import { mkTempDir, writeOpFile, mkSubdir } from './test-helpers';

const execFileAsync = promisify(execFile);

// Synchronous availability probe at module load — same pattern as
// git-sys-real.test.ts. vitest's it.skipIf() reads its predicate at
// test-collection time, before any beforeEach hook has run.
let systemGitAvailable: boolean;
try {
  execFileSync('git', ['--version'], { stdio: 'ignore', timeout: 5000 });
  systemGitAvailable = true;
} catch {
  systemGitAvailable = false;
}

describe('git-engine', () => {
  let temp: { dir: string; dispose: () => Promise<void> };

  beforeEach(async () => {
    temp = await mkTempDir();
    clearAllSessions();
  });

  afterEach(async () => {
    clearAllSessions();
    await temp.dispose();
  });

  describe('engineDetect', () => {
    it("returns { mode: 'none' } for an .op file with no surrounding repo", async () => {
      const opFile = await writeOpFile(temp.dir, 'orphan.op');
      const result = await engineDetect(opFile);
      expect(result.mode).toBe('none');
      expect(sessionCount()).toBe(0);
    });

    it('registers a session and auto-binds the file when a single-file repo exists', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      // Use engineInit to create the repo first.
      const initResult = await engineInit(opFile);
      // Drop the init session so detect re-allocates a fresh one.
      clearAllSessions();

      const result = await engineDetect(opFile);
      expect(result.mode).toBe('single-file');
      if (result.mode === 'none') throw new Error('unreachable');
      expect(result.trackedFilePath).toBe(resolve(opFile));
      expect(result.engineKind).toBe('iso');
      expect(result.gitdir).toBe(initResult.gitdir);
      expect(sessionCount()).toBe(1);
    });

    it('detects a folder-mode repo when the .op file lives inside a parent .git', async () => {
      const repoRoot = await mkSubdir(temp.dir, 'project');
      const dotGit = await mkSubdir(repoRoot, '.git');
      await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
      const opFile = await writeOpFile(repoRoot, 'design.op');

      const result = await engineDetect(opFile);
      expect(result.mode).toBe('folder');
      if (result.mode === 'none') throw new Error('unreachable');
      expect(result.rootPath).toBe(resolve(repoRoot));
      expect(result.trackedFilePath).toBe(resolve(opFile));
    });
  });

  describe('engineInit', () => {
    it('initializes a single-file repo, auto-binds, and returns a single candidate', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const result = await engineInit(opFile);

      expect(result.mode).toBe('single-file');
      expect(result.rootPath).toBe(temp.dir);
      expect(result.trackedFilePath).toBe(resolve(opFile));
      expect(result.candidates).toHaveLength(1);
      expect(result.candidates[0].relativePath).toBe('login.op');
      expect(result.candidates[0].milestoneCount).toBe(0);
      expect(result.candidates[0].autosaveCount).toBe(0);
      expect(sessionCount()).toBe(1);
    });
  });

  describe('engineOpen + walk + bind', () => {
    it('engineOpen on a folder-mode repo discovers all .op files and sets candidates', async () => {
      const repoRoot = await mkSubdir(temp.dir, 'project');
      const dotGit = await mkSubdir(repoRoot, '.git');
      await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
      await writeOpFile(repoRoot, 'a.op');
      await writeOpFile(repoRoot, 'b.op');
      await mkSubdir(repoRoot, 'subdir');
      await writeOpFile(join(repoRoot, 'subdir'), 'c.op');

      const result = await engineOpen(repoRoot);
      expect(result.mode).toBe('folder');
      expect(result.candidates).toHaveLength(3);
      const rels = result.candidates.map((c) => c.relativePath).sort();
      expect(rels).toEqual(['a.op', 'b.op', join('subdir', 'c.op')]);
    });

    it('engineOpen auto-binds when currentFilePath is inside the repo', async () => {
      const repoRoot = await mkSubdir(temp.dir, 'project');
      const dotGit = await mkSubdir(repoRoot, '.git');
      await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
      const opFile = await writeOpFile(repoRoot, 'design.op');

      const result = await engineOpen(repoRoot, opFile);
      expect(result.trackedFilePath).toBe(resolve(opFile));
    });

    it('engineOpen auto-binds the only candidate when currentFilePath is omitted', async () => {
      const repoRoot = await mkSubdir(temp.dir, 'project');
      const dotGit = await mkSubdir(repoRoot, '.git');
      await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
      const opFile = await writeOpFile(repoRoot, 'design.op');

      const result = await engineOpen(repoRoot);
      expect(result.trackedFilePath).toBe(resolve(opFile));
    });

    it('engineOpen leaves trackedFilePath null when multiple candidates exist and no currentFilePath', async () => {
      const repoRoot = await mkSubdir(temp.dir, 'project');
      const dotGit = await mkSubdir(repoRoot, '.git');
      await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
      await writeOpFile(repoRoot, 'a.op');
      await writeOpFile(repoRoot, 'b.op');

      const result = await engineOpen(repoRoot);
      expect(result.trackedFilePath).toBeNull();
    });

    it('engineBindTrackedFile updates the session and rejects paths outside the repo', async () => {
      const repoRoot = await mkSubdir(temp.dir, 'project');
      const dotGit = await mkSubdir(repoRoot, '.git');
      await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
      await writeOpFile(repoRoot, 'a.op');
      await writeOpFile(repoRoot, 'b.op');
      const result = await engineOpen(repoRoot);

      const a = join(repoRoot, 'a.op');
      const bound = await engineBindTrackedFile(result.repoId, a);
      expect(bound.trackedFilePath).toBe(resolve(a));

      // Outside path is rejected.
      await expect(
        engineBindTrackedFile(result.repoId, join(temp.dir, 'outside.op')),
      ).rejects.toMatchObject({ name: 'GitError', code: 'open-failed' });
    });

    it('engineListCandidates re-walks the worktree and includes newly-added files', async () => {
      const repoRoot = await mkSubdir(temp.dir, 'project');
      const dotGit = await mkSubdir(repoRoot, '.git');
      await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
      await writeOpFile(repoRoot, 'a.op');
      const result = await engineOpen(repoRoot);
      expect(result.candidates).toHaveLength(1);

      // Add a new file outside OpenPencil and refresh.
      await writeOpFile(repoRoot, 'b.op');
      const fresh = await engineListCandidates(result.repoId);
      expect(fresh).toHaveLength(2);
    });

    it('engineClose removes the session and subsequent operations throw no-file', async () => {
      const repoRoot = await mkSubdir(temp.dir, 'project');
      const dotGit = await mkSubdir(repoRoot, '.git');
      await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
      await writeOpFile(repoRoot, 'a.op');
      const result = await engineOpen(repoRoot);

      engineClose(result.repoId);
      await expect(engineListCandidates(result.repoId)).rejects.toMatchObject({
        name: 'GitError',
        code: 'no-file',
      });
    });
  });

  describe('engineStatus', () => {
    it('reports branch=main and workingDirty=true on a fresh single-file repo', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const result = await engineInit(opFile);
      const status = await engineStatus(result.repoId);
      expect(status.trackedFilePath).toBe(resolve(opFile));
      // currentBranch reads HEAD's symbolic ref which initSingleFile sets to
      // refs/heads/main, so branch='main' even before any commit exists.
      expect(status.branch).toBe('main');
      expect(status.workingDirty).toBe(true);
      expect(status.otherFilesDirty).toBe(0);
      expect(status.mergeInProgress).toBe(false);
      expect(status.ahead).toBe(0);
      expect(status.behind).toBe(0);
    });

    it('after a milestone commit, workingDirty=false; after a disk edit, workingDirty=true', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const { commitFile, setRef } = await import('../git-iso');

      const { hash } = await commitFile({
        handle: session.handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      // Force the autosave ref to the same commit (this is what engineCommit
      // milestone path will do in Task 8).
      await setRef({
        handle: session.handle,
        ref: 'refs/openpencil/autosaves/main',
        value: hash,
      });

      const cleanStatus = await engineStatus(result.repoId);
      expect(cleanStatus.branch).toBe('main');
      expect(cleanStatus.workingDirty).toBe(false);

      // Mutate the file on disk.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      const dirtyStatus = await engineStatus(result.repoId);
      expect(dirtyStatus.workingDirty).toBe(true);
    });

    it('folder mode: a modified tracked .gitignore counts toward otherFilesDirty', async () => {
      // Build a folder-mode repo with two tracked files: design.op and .gitignore.
      const repoRoot = temp.dir; // use temp.dir directly so the gitdir is .git here
      const opFile = await writeOpFile(repoRoot, 'design.op');
      const gitignorePath = join(repoRoot, '.gitignore');
      await fsp.writeFile(gitignorePath, 'node_modules\n', 'utf-8');

      // Init a folder-mode repo via isomorphic-git directly (engineInit only
      // does single-file mode).
      const isoGit = await import('isomorphic-git');
      const fsMod = await import('node:fs');
      await isoGit.init({
        fs: fsMod,
        dir: repoRoot,
        defaultBranch: 'main',
      });
      // Stage and commit BOTH files in one commit so both are in the heads tree.
      await isoGit.add({ fs: fsMod, dir: repoRoot, filepath: 'design.op' });
      await isoGit.add({ fs: fsMod, dir: repoRoot, filepath: '.gitignore' });
      await isoGit.commit({
        fs: fsMod,
        dir: repoRoot,
        message: 'initial',
        author: { name: 't', email: 't@example.com' },
      });

      // Open via the engine and bind design.op as the tracked file.
      const result = await engineOpen(repoRoot, opFile);
      expect(result.trackedFilePath).toBe(resolve(opFile));

      // Clean state: both files match the tree → otherFilesDirty=0.
      const cleanStatus = await engineStatus(result.repoId);
      expect(cleanStatus.otherFilesDirty).toBe(0);
      expect(cleanStatus.otherFilesPaths).toEqual([]);

      // Modify .gitignore on disk and re-check.
      await fsp.writeFile(gitignorePath, 'node_modules\n.DS_Store\n', 'utf-8');
      const dirtyStatus = await engineStatus(result.repoId);
      expect(dirtyStatus.otherFilesDirty).toBe(1);
      expect(dirtyStatus.otherFilesPaths).toEqual(['.gitignore']);
    });

    it('folder mode: a tracked file deleted from disk counts toward otherFilesDirty', async () => {
      const repoRoot = temp.dir;
      const opFile = await writeOpFile(repoRoot, 'design.op');
      const notesPath = join(repoRoot, 'notes.md');
      await fsp.writeFile(notesPath, '# notes\n', 'utf-8');

      const isoGit = await import('isomorphic-git');
      const fsMod = await import('node:fs');
      await isoGit.init({ fs: fsMod, dir: repoRoot, defaultBranch: 'main' });
      await isoGit.add({ fs: fsMod, dir: repoRoot, filepath: 'design.op' });
      await isoGit.add({ fs: fsMod, dir: repoRoot, filepath: 'notes.md' });
      await isoGit.commit({
        fs: fsMod,
        dir: repoRoot,
        message: 'initial',
        author: { name: 't', email: 't@example.com' },
      });

      const result = await engineOpen(repoRoot, opFile);

      // Delete notes.md from disk; the heads tree still has it.
      await fsp.unlink(notesPath);

      const status = await engineStatus(result.repoId);
      expect(status.otherFilesDirty).toBe(1);
      expect(status.otherFilesPaths).toEqual(['notes.md']);
    });
  });

  describe('engineLog', () => {
    it("returns milestone-kind entries when querying ref: 'main'", async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const { commitFile, setRef } = await import('../git-iso');

      const { hash: h1 } = await commitFile({
        handle: session.handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await setRef({
        handle: session.handle,
        ref: 'refs/openpencil/autosaves/main',
        value: h1,
      });

      const log = await engineLog(result.repoId, { ref: 'main', limit: 10 });
      expect(log).toHaveLength(1);
      expect(log[0].kind).toBe('milestone');
      expect(log[0].hash).toBe(h1);
    });

    it("autosave commits are kind='autosave' when queried via ref: 'autosaves'", async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const { commitFile, setRef } = await import('../git-iso');

      // Milestone first.
      const { hash: m1 } = await commitFile({
        handle: session.handle,
        filepath: 'login.op',
        ref: 'refs/heads/main',
        message: 'milestone',
        author: { name: 't', email: 't@example.com' },
      });
      await setRef({
        handle: session.handle,
        ref: 'refs/openpencil/autosaves/main',
        value: m1,
      });
      // Then an autosave on top.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      const { hash: a1 } = await commitFile({
        handle: session.handle,
        filepath: 'login.op',
        ref: 'refs/openpencil/autosaves/main',
        message: 'auto',
        author: { name: 't', email: 't@example.com' },
      });

      const log = await engineLog(result.repoId, { ref: 'autosaves', limit: 10 });
      // log[0] = a1 (autosave), log[1] = m1 (milestone, reachable as parent)
      expect(log[0].hash).toBe(a1);
      expect(log[0].kind).toBe('autosave');
      expect(log[1].hash).toBe(m1);
      expect(log[1].kind).toBe('milestone');
    });

    it('respects the limit parameter', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const { commitFile, setRef } = await import('../git-iso');

      // Make 3 milestones.
      for (let i = 0; i < 3; i++) {
        await writeOpFile(temp.dir, 'login.op', {
          version: '1.0.0',
          children: [{ id: `r${i}` }],
        });
        const { hash } = await commitFile({
          handle: session.handle,
          filepath: 'login.op',
          ref: 'refs/heads/main',
          message: `m${i}`,
          author: { name: 't', email: 't@example.com' },
        });
        await setRef({
          handle: session.handle,
          ref: 'refs/openpencil/autosaves/main',
          value: hash,
        });
      }

      const log = await engineLog(result.repoId, { ref: 'main', limit: 2 });
      expect(log).toHaveLength(2);
    });
  });

  describe('engineCommit', () => {
    it('milestone commit advances both heads and autosaves to the same hash', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      const { hash } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first milestone',
        author: { name: 't', email: 't@example.com' },
      });

      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const isoGit = await import('isomorphic-git');
      const fsMod = await import('node:fs');
      const headsTip = await isoGit.resolveRef({
        fs: fsMod,
        gitdir: session.handle.gitdir,
        ref: 'refs/heads/main',
      });
      const autoTip = await isoGit.resolveRef({
        fs: fsMod,
        gitdir: session.handle.gitdir,
        ref: 'refs/openpencil/autosaves/main',
      });
      expect(headsTip).toBe(hash);
      expect(autoTip).toBe(hash);
    });

    it('autosave commit advances only the autosaves ref, leaving heads behind', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      // First a milestone.
      const { hash: m1 } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'milestone',
        author: { name: 't', email: 't@example.com' },
      });
      // Mutate and autosave.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      const { hash: a1 } = await engineCommit(result.repoId, {
        kind: 'autosave',
        message: 'auto',
        author: { name: 't', email: 't@example.com' },
      });
      expect(a1).not.toBe(m1);

      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const isoGit = await import('isomorphic-git');
      const fsMod = await import('node:fs');
      const headsTip = await isoGit.resolveRef({
        fs: fsMod,
        gitdir: session.handle.gitdir,
        ref: 'refs/heads/main',
      });
      const autoTip = await isoGit.resolveRef({
        fs: fsMod,
        gitdir: session.handle.gitdir,
        ref: 'refs/openpencil/autosaves/main',
      });
      expect(headsTip).toBe(m1); // unchanged
      expect(autoTip).toBe(a1); // advanced
    });

    it('a second milestone after autosaves abandons the autosave chain', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);

      // milestone, autosave, autosave
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'm1',
        author: { name: 't', email: 't@example.com' },
      });
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r2' }] });
      await engineCommit(result.repoId, {
        kind: 'autosave',
        message: 'a1',
        author: { name: 't', email: 't@example.com' },
      });
      await writeOpFile(temp.dir, 'login.op', { version: '1.0.0', children: [{ id: 'r3' }] });
      await engineCommit(result.repoId, {
        kind: 'autosave',
        message: 'a2',
        author: { name: 't', email: 't@example.com' },
      });

      // Now a second milestone.
      const { hash: m2 } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'm2',
        author: { name: 't', email: 't@example.com' },
      });

      // The heads ref should now show 2 milestones (m2 → m1).
      const headsLog = await engineLog(result.repoId, { ref: 'main', limit: 10 });
      expect(headsLog).toHaveLength(2);
      expect(headsLog[0].hash).toBe(m2);
      expect(headsLog[0].kind).toBe('milestone');
      expect(headsLog[1].kind).toBe('milestone');

      // The autosaves ref now points at m2 (force-updated), so its log walks
      // m2 → m1. The intermediate autosaves a1, a2 are unreachable.
      const autoLog = await engineLog(result.repoId, { ref: 'autosaves', limit: 10 });
      expect(autoLog).toHaveLength(2);
      expect(autoLog[0].hash).toBe(m2);
    });

    it("throws 'commit-empty' when committing the same content twice", async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const result = await engineInit(opFile);
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await expect(
        engineCommit(result.repoId, {
          kind: 'milestone',
          message: 'second',
          author: { name: 't', email: 't@example.com' },
        }),
      ).rejects.toMatchObject({ name: 'GitError', code: 'commit-empty' });
    });

    it('engineCommit autosave is a no-op when disk content matches tip blob', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      // Create an initial milestone so there is a headsRef parent.
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'initial',
        author: { name: 't', email: 't@example.com' },
      });
      // Mutate the file and autosave — creates the autosave tip.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      const { hash: first } = await engineCommit(result.repoId, {
        kind: 'autosave',
        message: 'auto-1',
        author: { name: 't', email: 't@example.com' },
      });
      // Second autosave with identical disk content — must be a no-op.
      const { hash: second } = await engineCommit(result.repoId, {
        kind: 'autosave',
        message: 'auto-2',
        author: { name: 't', email: 't@example.com' },
      });
      expect(second).toBe(first);
    });

    it('engineCommit autosave creates a new commit when disk content changed', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      // Initial milestone.
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'initial',
        author: { name: 't', email: 't@example.com' },
      });
      // First autosave with mutated content.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      const { hash: first } = await engineCommit(result.repoId, {
        kind: 'autosave',
        message: 'auto-1',
        author: { name: 't', email: 't@example.com' },
      });
      // Mutate again, then autosave — content differs, so a new commit is expected.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r3' }],
      });
      const { hash: second } = await engineCommit(result.repoId, {
        kind: 'autosave',
        message: 'auto-2',
        author: { name: 't', email: 't@example.com' },
      });
      expect(second).not.toBe(first);
    });
  });

  describe('engineRestore + enginePromote', () => {
    it('engineRestore writes a previous commit to disk without recording a new commit', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      const { hash: m1 } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      // Mutate.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'second',
        author: { name: 't', email: 't@example.com' },
      });
      const logBefore = await engineLog(result.repoId, { ref: 'main', limit: 10 });

      // Restore to first milestone.
      await engineRestore(result.repoId, m1);
      const restored = await fsp.readFile(opFile, 'utf-8');
      expect(JSON.parse(restored).children[0].id).toBe('r1');

      // History unchanged (restore doesn't commit).
      const logAfter = await engineLog(result.repoId, { ref: 'main', limit: 10 });
      expect(logAfter).toHaveLength(logBefore.length);
    });

    it('enginePromote reads an autosave, writes it, and records a milestone', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      // Initial milestone.
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'm1',
        author: { name: 't', email: 't@example.com' },
      });
      // Autosave with new content.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      const { hash: a1 } = await engineCommit(result.repoId, {
        kind: 'autosave',
        message: 'auto',
        author: { name: 't', email: 't@example.com' },
      });

      // The user keeps editing and ends up at r3 in memory, but on disk is
      // currently r2 (last autosave). Promote a1 to a milestone.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r3' }],
      });
      const { hash: m2 } = await enginePromote(result.repoId, a1, 'promoted to milestone', {
        name: 't',
        email: 't@example.com',
      });
      expect(m2).not.toBe(a1);

      // Working tree should now hold r2 (the promoted autosave content), not r3.
      const onDisk = await fsp.readFile(opFile, 'utf-8');
      expect(JSON.parse(onDisk).children[0].id).toBe('r2');

      // Heads ref now has 2 milestones (m1, m2).
      const headsLog = await engineLog(result.repoId, { ref: 'main', limit: 10 });
      expect(headsLog).toHaveLength(2);
      expect(headsLog[0].hash).toBe(m2);
      expect(headsLog[0].message.trim()).toBe('promoted to milestone');
    });

    it('engineRestore throws no-file when session has no trackedFilePath', async () => {
      const repoRoot = await mkSubdir(temp.dir, 'project');
      const dotGit = await mkSubdir(repoRoot, '.git');
      await writeFile(join(dotGit, 'HEAD'), 'ref: refs/heads/main\n', 'utf-8');
      await writeOpFile(repoRoot, 'a.op');
      await writeOpFile(repoRoot, 'b.op');
      const result = await engineOpen(repoRoot); // multiple candidates → no auto-bind

      await expect(engineRestore(result.repoId, 'deadbeef')).rejects.toMatchObject({
        name: 'GitError',
        code: 'no-file',
      });
    });
  });

  describe('branch operations', () => {
    it('engineBranchList returns the current branch decorated with lastCommit', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });

      const list = await engineBranchList(result.repoId);
      expect(list).toHaveLength(1);
      expect(list[0].name).toBe('main');
      expect(list[0].isCurrent).toBe(true);
      expect(list[0].lastCommit?.message).toBe('first');
    });

    it('engineBranchCreate adds a new branch from current HEAD', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await engineBranchCreate(result.repoId, { name: 'feature-x' });

      const list = await engineBranchList(result.repoId);
      const names = list.map((b) => b.name).sort();
      expect(names).toEqual(['feature-x', 'main']);
    });

    it('engineBranchSwitch updates the working tree to the target branch tip', async () => {
      // Mirrors Phase 1b's switchBranch test structure: commit r1 on main,
      // branch off, commit r2 on feature-x (working tree currently r2), then
      // switch to main and verify disk reverts to r1. This is the cleanest
      // observable test because filepaths-scoped checkout reads the target
      // ref's tree and writes it to disk.
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await engineBranchCreate(result.repoId, { name: 'feature-x' });

      // Mutate the working tree, then commit r2 onto feature-x via the
      // underlying primitive (the engine commits to current HEAD only).
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const { commitFile } = await import('../git-iso');
      await commitFile({
        handle: session.handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature-x',
        message: 'feature change',
        author: { name: 't', email: 't@example.com' },
      });

      // Working tree currently has r2 (from the writeOpFile above).
      // Switch to main and verify the file reverts to r1.
      await engineBranchSwitch(result.repoId, 'main');
      const onDisk = await fsp.readFile(opFile, 'utf-8');
      expect(JSON.parse(onDisk).children[0].id).toBe('r1');
    });

    it('engineBranchDelete removes a non-current branch', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await engineBranchCreate(result.repoId, { name: 'feature-x' });
      await engineBranchDelete(result.repoId, 'feature-x');

      const list = await engineBranchList(result.repoId);
      expect(list.map((b) => b.name)).toEqual(['main']);
    });

    it('engineBranchDelete refuses to delete the current branch', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      await expect(engineBranchDelete(result.repoId, 'main')).rejects.toMatchObject({
        name: 'GitError',
        code: 'branch-current',
      });
    });

    it('engineBranchDelete refuses an unmerged branch without force and succeeds with force=true', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r1' }],
      });
      const result = await engineInit(opFile);
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });
      await engineBranchCreate(result.repoId, { name: 'feature-x' });

      // Switch to the new branch, modify the tracked file, and commit so
      // feature-x has a commit main does not see.
      await engineBranchSwitch(result.repoId, 'feature-x');
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        children: [{ id: 'r2' }],
      });
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'feature only',
        author: { name: 't', email: 't@example.com' },
      });
      await engineBranchSwitch(result.repoId, 'main');

      await expect(engineBranchDelete(result.repoId, 'feature-x')).rejects.toMatchObject({
        name: 'GitError',
        code: 'branch-unmerged',
      });

      await expect(
        engineBranchDelete(result.repoId, 'feature-x', { force: true }),
      ).resolves.toBeUndefined();

      const list = await engineBranchList(result.repoId);
      expect(list.map((b) => b.name)).toEqual(['main']);
    });
  });

  describe('shouldUseSys dispatch', () => {
    it('routes git@ and ssh:// URLs to sys', async () => {
      const { shouldUseSys } = await import('../git-engine');
      expect(shouldUseSys('git@github.com:user/repo.git', undefined)).toBe(true);
      expect(shouldUseSys('ssh://git@github.com/user/repo.git', undefined)).toBe(true);
    });

    it('routes https URLs to iso', async () => {
      const { shouldUseSys } = await import('../git-engine');
      expect(shouldUseSys('https://github.com/user/repo.git', undefined)).toBe(false);
    });

    it('routes auth.kind=ssh to sys regardless of URL', async () => {
      const { shouldUseSys } = await import('../git-engine');
      expect(shouldUseSys('https://github.com/user/repo.git', { kind: 'ssh', keyId: 'k1' })).toBe(
        true,
      );
    });
  });

  describe('parseHost', () => {
    it('extracts the host from https/http/ssh URLs', async () => {
      const { parseHost } = await import('../git-engine');
      expect(parseHost('https://github.com/user/repo.git')).toBe('github.com');
      expect(parseHost('http://gitlab.example.com:8080/u/r')).toBe('gitlab.example.com');
      expect(parseHost('ssh://git@github.com:22/user/repo.git')).toBe('github.com');
    });

    it('extracts the host from SCP-style git URLs', async () => {
      const { parseHost } = await import('../git-engine');
      expect(parseHost('git@github.com:user/repo.git')).toBe('github.com');
      expect(parseHost('user@gitlab.com:org/project.git')).toBe('gitlab.com');
    });

    it('returns null for unparseable inputs', async () => {
      const { parseHost } = await import('../git-engine');
      expect(parseHost('/local/path/to/bare.git')).toBeNull();
      expect(parseHost('not a url at all')).toBeNull();
    });
  });

  describe('resolveAuthForRemote', () => {
    it('returns the explicit auth argument unchanged when provided', async () => {
      const { resolveAuthForRemote, setAuthStore } = await import('../git-engine');
      // No store needed; explicit auth wins.
      setAuthStore(null);
      const explicit = { kind: 'token' as const, username: 'a', token: 'b' };
      const got = await resolveAuthForRemote('https://github.com/u/r.git', explicit);
      expect(got).toBe(explicit);
    });

    it('falls back to the stored credential when explicit auth is omitted', async () => {
      const { resolveAuthForRemote, setAuthStore } = await import('../git-engine');
      const { createAuthStore, createInMemoryBackend } = await import('../auth-store');
      const filePath = join(temp.dir, 'auth.bin');
      const store = createAuthStore({ filePath, backend: createInMemoryBackend() });
      await store.set('github.com', { kind: 'token', username: 'kay', token: 'stored-pat' });
      setAuthStore(store);

      const got = await resolveAuthForRemote('https://github.com/u/r.git', undefined);
      expect(got).toEqual({ kind: 'token', username: 'kay', token: 'stored-pat' });

      // Cleanup so the singleton doesn't leak into other tests.
      setAuthStore(null);
    });

    it('returns undefined when no explicit auth, no store, and no host match', async () => {
      const { resolveAuthForRemote, setAuthStore } = await import('../git-engine');
      setAuthStore(null);
      expect(await resolveAuthForRemote('https://unknown.com/u/r.git', undefined)).toBeUndefined();
      expect(await resolveAuthForRemote(null, undefined)).toBeUndefined();
    });
  });

  describe('engineFetch + enginePull (system git gated)', () => {
    async function setupClonePair() {
      const remoteDir = join(temp.dir, 'remote.git');
      const aDir = join(temp.dir, 'a');
      const bDir = join(temp.dir, 'b');

      await execFileAsync('git', ['init', '--bare', remoteDir]);
      await execFileAsync('git', ['clone', remoteDir, aDir]);
      await execFileAsync('git', ['-C', aDir, 'checkout', '-b', 'main']);
      await fsp.writeFile(join(aDir, 'design.op'), '{"version":"1.0.0","children":[{"id":"r1"}]}');
      await execFileAsync('git', ['-C', aDir, 'add', '.']);
      await execFileAsync(
        'git',
        ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'one'],
        {},
      );
      await execFileAsync('git', ['-C', aDir, 'push', '-u', 'origin', 'main']);
      await execFileAsync('git', ['clone', remoteDir, bDir]);
      return { remoteDir, aDir, bDir };
    }

    it.skipIf(!systemGitAvailable)(
      'engineFetch reports behind=1 after upstream advances',
      async () => {
        const { aDir, bDir } = await setupClonePair();
        // a commits and pushes a new commit
        await fsp.writeFile(
          join(aDir, 'design.op'),
          '{"version":"1.0.0","children":[{"id":"r2"}]}',
        );
        await execFileAsync('git', ['-C', aDir, 'add', '.']);
        await execFileAsync(
          'git',
          ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'two'],
          {},
        );
        await execFileAsync('git', ['-C', aDir, 'push']);

        // Open b via the engine and fetch.
        const result = await engineOpen(bDir, join(bDir, 'design.op'));
        const fetchResult = await engineFetch(result.repoId);
        expect(fetchResult).toEqual({ ahead: 0, behind: 1 });
      },
    );

    it.skipIf(!systemGitAvailable)('enginePull fast-forwards a clean clone', async () => {
      const { aDir, bDir } = await setupClonePair();
      // a pushes a new commit
      await fsp.writeFile(join(aDir, 'design.op'), '{"version":"1.0.0","children":[{"id":"r2"}]}');
      await execFileAsync('git', ['-C', aDir, 'add', '.']);
      await execFileAsync(
        'git',
        ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'two'],
        {},
      );
      await execFileAsync('git', ['-C', aDir, 'push']);

      const result = await engineOpen(bDir, join(bDir, 'design.op'));
      const pullResult = await enginePull(result.repoId);
      expect(pullResult.result).toBe('fast-forward');

      // Verify b's design.op now has r2.
      const content = await fsp.readFile(join(bDir, 'design.op'), 'utf-8');
      expect(JSON.parse(content).children[0].id).toBe('r2');
    });

    it.skipIf(!systemGitAvailable)(
      'enginePull in folder mode enters merge workflow when histories diverge (Phase 7a)',
      async () => {
        const { aDir, bDir } = await setupClonePair();
        // Both a and b commit divergently with conflicting fill values on node 'r1'.
        await fsp.writeFile(
          join(aDir, 'design.op'),
          JSON.stringify({
            version: '1.0.0',
            children: [
              {
                id: 'r1',
                type: 'rectangle',
                x: 0,
                y: 0,
                width: 1,
                height: 1,
                fill: [{ type: 'solid', color: '#0000ff' }],
              },
            ],
          }),
        );
        await execFileAsync('git', ['-C', aDir, 'add', '.']);
        await execFileAsync(
          'git',
          ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'a: blue'],
          {},
        );
        await execFileAsync('git', ['-C', aDir, 'push']);

        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({
            version: '1.0.0',
            children: [
              {
                id: 'r1',
                type: 'rectangle',
                x: 0,
                y: 0,
                width: 1,
                height: 1,
                fill: [{ type: 'solid', color: '#00ff00' }],
              },
            ],
          }),
        );
        await execFileAsync('git', ['-C', bDir, 'add', '.']);
        await execFileAsync(
          'git',
          ['-C', bDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'b: green'],
          {},
        );

        const result = await engineOpen(bDir, join(bDir, 'design.op'));
        // Phase 7a: folder-mode divergent pull should now enter the merge workflow,
        // returning 'conflict' (or 'merge' if pen-core finds no semantic conflicts).
        const pullResult = await enginePull(result.repoId);
        expect(['conflict', 'merge', 'fast-forward']).toContain(pullResult.result);

        // design.op on disk must be readable JSON (not conflict markers).
        const onDisk = await fsp.readFile(join(bDir, 'design.op'), 'utf-8');
        expect(() => JSON.parse(onDisk)).not.toThrow();
      },
    );
  });

  describe('enginePush (system git gated)', () => {
    it.skipIf(!systemGitAvailable)(
      'enginePush succeeds on a clean local clone with new commits',
      async () => {
        const remoteDir = join(temp.dir, 'remote.git');
        const cloneDir = join(temp.dir, 'clone');

        await execFileAsync('git', ['init', '--bare', remoteDir]);
        await execFileAsync('git', ['clone', remoteDir, cloneDir]);
        await execFileAsync('git', ['-C', cloneDir, 'checkout', '-b', 'main']);
        await fsp.writeFile(
          join(cloneDir, 'design.op'),
          '{"version":"1.0.0","children":[{"id":"r1"}]}',
        );
        await execFileAsync('git', ['-C', cloneDir, 'add', '.']);
        await execFileAsync(
          'git',
          [
            '-C',
            cloneDir,
            '-c',
            'user.name=t',
            '-c',
            'user.email=t@e.com',
            'commit',
            '-m',
            'first',
          ],
          {},
        );
        // First push needs --set-upstream; we use the underlying execFile for it,
        // then enginePush handles subsequent pushes.
        await execFileAsync('git', ['-C', cloneDir, 'push', '-u', 'origin', 'main']);

        // Make another commit, then push via enginePush.
        await fsp.writeFile(
          join(cloneDir, 'design.op'),
          '{"version":"1.0.0","children":[{"id":"r2"}]}',
        );
        await execFileAsync('git', ['-C', cloneDir, 'add', '.']);
        await execFileAsync(
          'git',
          [
            '-C',
            cloneDir,
            '-c',
            'user.name=t',
            '-c',
            'user.email=t@e.com',
            'commit',
            '-m',
            'second',
          ],
          {},
        );

        const result = await engineOpen(cloneDir, join(cloneDir, 'design.op'));
        const pushResult = await enginePush(result.repoId);
        expect(pushResult.result).toBe('ok');

        // Verify the bare remote saw the second commit.
        const { stdout } = await execFileAsync('git', [
          '-C',
          remoteDir,
          'log',
          '--oneline',
          'main',
        ]);
        expect(stdout).toContain('second');
      },
    );
  });

  describe('engineDiff', () => {
    it('returns 0/0/0/0 summary for two identical commits', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [{ id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 }],
      });
      const result = await engineInit(opFile);
      const { hash: h1 } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });

      const diff = await engineDiff(result.repoId, h1, h1);
      expect(diff.summary).toEqual({
        framesChanged: 0,
        nodesAdded: 0,
        nodesRemoved: 0,
        nodesModified: 0,
      });
      expect(diff.patches).toEqual([]);
    });

    it('reports nodes added/removed between two commits', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [{ id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 }],
      });
      const result = await engineInit(opFile);
      const { hash: h1 } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });

      // Add a node and commit again.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          { id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 },
          { id: 'b', type: 'rectangle', x: 5, y: 5, width: 1, height: 1 },
        ],
      });
      const { hash: h2 } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'second',
        author: { name: 't', email: 't@example.com' },
      });

      const diff = await engineDiff(result.repoId, h1, h2);
      expect(diff.summary.nodesAdded).toBe(1);
      expect(diff.summary.nodesRemoved).toBe(0);
      expect(diff.patches.some((p) => p.op === 'add' && p.nodeId === 'b')).toBe(true);
    });
  });

  describe('engineBranchMerge', () => {
    it('clean merge of disjoint changes produces a merge commit', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [{ id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 }],
      });
      const result = await engineInit(opFile);
      // Base commit on main.
      const { hash: baseHash } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });
      // Branch off, commit different content on feature.
      await engineBranchCreate(result.repoId, { name: 'feature' });
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const { commitFile: cf, setRef: sr } = await import('../git-iso');
      // Theirs adds node 'b'.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          { id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 },
          { id: 'b', type: 'rectangle', x: 5, y: 5, width: 1, height: 1 },
        ],
      });
      const { hash: theirs } = await cf({
        handle: session.handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature',
        message: 'add b',
        author: { name: 't', email: 't@example.com' },
        parents: [baseHash],
      });
      await sr({
        handle: session.handle,
        ref: 'refs/openpencil/autosaves/feature',
        value: theirs,
      });
      // Restore main's content.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [{ id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 }],
      });
      // Ours adds node 'c'.
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          { id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 },
          { id: 'c', type: 'rectangle', x: 9, y: 9, width: 1, height: 1 },
        ],
      });
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'add c',
        author: { name: 't', email: 't@example.com' },
      });

      // Now merge feature into main.
      const mergeResult = await engineBranchMerge(result.repoId, 'feature');
      expect(mergeResult.result).toBe('merge');
      expect(mergeResult.conflicts).toBeUndefined();

      // Working tree should now contain both b and c.
      const onDisk = JSON.parse(await fsp.readFile(opFile, 'utf-8'));
      const ids = (onDisk.children as Array<{ id: string }>).map((n) => n.id).sort();
      expect(ids).toEqual(['a', 'b', 'c']);
    });

    it('fast-forward when ours is an ancestor of theirs', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [{ id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 }],
      });
      const result = await engineInit(opFile);
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });
      await engineBranchCreate(result.repoId, { name: 'feature' });
      // Switch to feature and add a commit.
      await engineBranchSwitch(result.repoId, 'feature');
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          { id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 },
          { id: 'b', type: 'rectangle', x: 5, y: 5, width: 1, height: 1 },
        ],
      });
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'add b on feature',
        author: { name: 't', email: 't@example.com' },
      });
      // Switch back to main and merge feature.
      await engineBranchSwitch(result.repoId, 'main');
      const mergeResult = await engineBranchMerge(result.repoId, 'feature');
      expect(mergeResult.result).toBe('fast-forward');
    });

    it('conflict path stashes InflightMerge and returns the bag', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#ff0000' }],
          },
        ],
      });
      const result = await engineInit(opFile);
      const { hash: baseHash } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });

      // feature: change to blue
      await engineBranchCreate(result.repoId, { name: 'feature' });
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const { commitFile: cf, setRef: sr } = await import('../git-iso');
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#0000ff' }],
          },
        ],
      });
      const { hash: theirs } = await cf({
        handle: session.handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature',
        message: 'blue',
        author: { name: 't', email: 't@example.com' },
        parents: [baseHash],
      });
      await sr({
        handle: session.handle,
        ref: 'refs/openpencil/autosaves/feature',
        value: theirs,
      });

      // main: change to green (after restoring main's content first)
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#ff0000' }],
          },
        ],
      });
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#00ff00' }],
          },
        ],
      });
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'green',
        author: { name: 't', email: 't@example.com' },
      });

      const mergeResult = await engineBranchMerge(result.repoId, 'feature');
      expect(mergeResult.result).toBe('conflict');
      expect(mergeResult.conflicts).toBeDefined();
      expect(mergeResult.conflicts!.nodeConflicts.length).toBeGreaterThan(0);

      // The session should now hold the InflightMerge.
      const refreshed = (await import('../repo-session')).getSession(result.repoId)!;
      expect(refreshed.inflightMerge).not.toBeNull();
      expect(refreshed.inflightMerge!.conflictMap.size).toBeGreaterThan(0);
    });
  });

  describe('engineResolveConflict + engineApplyMerge + engineAbortMerge', () => {
    // Helper: build a repo with a known conflict in flight.
    async function setupConflict() {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#ff0000' }],
          },
        ],
      });
      const result = await engineInit(opFile);
      const { hash: baseHash } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });

      await engineBranchCreate(result.repoId, { name: 'feature' });
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const { commitFile: cf, setRef: sr } = await import('../git-iso');

      // theirs: blue
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#0000ff' }],
          },
        ],
      });
      const { hash: theirsHash } = await cf({
        handle: session.handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature',
        message: 'blue',
        author: { name: 't', email: 't@example.com' },
        parents: [baseHash],
      });
      await sr({
        handle: session.handle,
        ref: 'refs/openpencil/autosaves/feature',
        value: theirsHash,
      });

      // ours: green
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#ff0000' }],
          },
        ],
      });
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#00ff00' }],
          },
        ],
      });
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'green',
        author: { name: 't', email: 't@example.com' },
      });

      const mergeResult = await engineBranchMerge(result.repoId, 'feature');
      if (mergeResult.result !== 'conflict') {
        throw new Error('expected conflict result');
      }
      return { result, opFile, conflictId: mergeResult.conflicts!.nodeConflicts[0].id };
    }

    it('engineResolveConflict records the choice in session state', async () => {
      const { result, conflictId } = await setupConflict();
      await engineResolveConflict(result.repoId, conflictId, { kind: 'theirs' });
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      expect(session.inflightMerge!.resolutions.get(conflictId)).toEqual({ kind: 'theirs' });
    });

    it('engineResolveConflict throws on unknown conflict id', async () => {
      const { result } = await setupConflict();
      await expect(
        engineResolveConflict(result.repoId, 'node:_:nope', { kind: 'ours' }),
      ).rejects.toMatchObject({ name: 'GitError', code: 'engine-crash' });
    });

    it('engineApplyMerge throws merge-still-conflicted when resolutions are missing', async () => {
      const { result } = await setupConflict();
      await expect(engineApplyMerge(result.repoId)).rejects.toMatchObject({
        name: 'GitError',
        code: 'merge-still-conflicted',
      });
    });

    it('engineApplyMerge writes the merged doc and creates the merge commit', async () => {
      const { result, opFile, conflictId } = await setupConflict();
      await engineResolveConflict(result.repoId, conflictId, { kind: 'theirs' });
      const applied = await engineApplyMerge(result.repoId);
      expect(applied.noop).toBe(false);
      expect(applied.hash).toMatch(/^[a-f0-9]{40}$/);

      // The working tree should now have theirs's blue rectangle.
      const onDisk = JSON.parse(await fsp.readFile(opFile, 'utf-8'));
      expect(onDisk.children[0].fill[0].color).toBe('#0000ff');

      // Session inflightMerge should be cleared.
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      expect(session.inflightMerge).toBeNull();

      // The merge commit should have two parents.
      const isoGit = await import('isomorphic-git');
      const fsMod = await import('node:fs');
      const commit = await isoGit.readCommit({
        fs: fsMod,
        gitdir: session.handle.gitdir,
        oid: applied.hash,
      });
      expect(commit.commit.parent.length).toBe(2);
    });

    it('engineApplyMerge with no in-flight merge returns noop=true', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [],
      });
      const result = await engineInit(opFile);
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'first',
        author: { name: 't', email: 't@example.com' },
      });
      const applied = await engineApplyMerge(result.repoId);
      expect(applied.noop).toBe(true);
    });

    it('engineAbortMerge clears the in-flight merge state', async () => {
      const { result } = await setupConflict();
      await engineAbortMerge(result.repoId);
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      expect(session.inflightMerge).toBeNull();
    });
  });

  describe('engineStatus with in-flight merge', () => {
    it('mergeInProgress=false on a fresh repo', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [],
      });
      const result = await engineInit(opFile);
      const status = await engineStatus(result.repoId);
      expect(status.mergeInProgress).toBe(false);
      expect(status.unresolvedFiles).toEqual([]);
      expect(status.conflicts).toBeNull();
    });

    it('mergeInProgress=true and conflicts populated when a merge is in flight', async () => {
      // Build a conflict using the same pattern as the Task 8 setupConflict.
      const opFile = await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#ff0000' }],
          },
        ],
      });
      const result = await engineInit(opFile);
      const { hash: baseHash } = await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'base',
        author: { name: 't', email: 't@example.com' },
      });
      await engineBranchCreate(result.repoId, { name: 'feature' });
      const session = (await import('../repo-session')).getSession(result.repoId)!;
      const { commitFile: cf, setRef: sr } = await import('../git-iso');
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#0000ff' }],
          },
        ],
      });
      const { hash: theirs } = await cf({
        handle: session.handle,
        filepath: 'login.op',
        ref: 'refs/heads/feature',
        message: 'blue',
        author: { name: 't', email: 't@example.com' },
        parents: [baseHash],
      });
      await sr({
        handle: session.handle,
        ref: 'refs/openpencil/autosaves/feature',
        value: theirs,
      });
      await writeOpFile(temp.dir, 'login.op', {
        version: '1.0.0',
        name: 'd',
        children: [
          {
            id: 'a',
            type: 'rectangle',
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            fill: [{ type: 'solid', color: '#00ff00' }],
          },
        ],
      });
      await engineCommit(result.repoId, {
        kind: 'milestone',
        message: 'green',
        author: { name: 't', email: 't@example.com' },
      });
      const mergeResult = await engineBranchMerge(result.repoId, 'feature');
      expect(mergeResult.result).toBe('conflict');

      const status = await engineStatus(result.repoId);
      expect(status.mergeInProgress).toBe(true);
      expect(status.conflicts).not.toBeNull();
      expect(status.conflicts!.nodeConflicts.length).toBeGreaterThan(0);
      expect(status.unresolvedFiles).toEqual(['login.op']);
    });
  });

  describe('engineRemoteGet + engineRemoteSet (Phase 6a)', () => {
    it('returns { url: null, host: null } when origin is absent', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const init = await engineInit(opFile);
      const info = await engineRemoteGet(init.repoId);
      expect(info).toEqual({ name: 'origin', url: null, host: null });
    });

    it('engineRemoteSet adds origin when it does not exist and parses the host', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const init = await engineInit(opFile);

      const result = await engineRemoteSet(init.repoId, 'https://github.com/foo/bar.git');
      expect(result).toEqual({
        name: 'origin',
        url: 'https://github.com/foo/bar.git',
        host: 'github.com',
      });

      // engineRemoteGet now returns the same thing.
      const got = await engineRemoteGet(init.repoId);
      expect(got).toEqual(result);
    });

    it('engineRemoteSet updates an existing origin in place', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const init = await engineInit(opFile);

      await engineRemoteSet(init.repoId, 'https://github.com/foo/bar.git');
      const updated = await engineRemoteSet(init.repoId, 'https://gitlab.com/foo/bar.git');
      expect(updated).toEqual({
        name: 'origin',
        url: 'https://gitlab.com/foo/bar.git',
        host: 'gitlab.com',
      });

      // Read back to confirm there is exactly one origin and it is the new url.
      const got = await engineRemoteGet(init.repoId);
      expect(got.url).toBe('https://gitlab.com/foo/bar.git');
    });

    it('engineRemoteSet(null) removes origin and is idempotent', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const init = await engineInit(opFile);

      // Add then remove.
      await engineRemoteSet(init.repoId, 'https://github.com/foo/bar.git');
      const removed = await engineRemoteSet(init.repoId, null);
      expect(removed).toEqual({ name: 'origin', url: null, host: null });

      // Removing again must not throw. Note: writeRemoteOrigin() does NOT
      // wrap `git.deleteRemote` in a try/catch — this test proves the call
      // is naturally idempotent because isomorphic-git implements it as a
      // filter over parsed config entries that tolerates an absent section.
      // Wrapping the call would silently swallow real I/O errors from
      // GitConfigManager.save (EACCES, ENOSPC, etc.), so the lack of a
      // catch is load-bearing.
      const removedAgain = await engineRemoteSet(init.repoId, null);
      expect(removedAgain).toEqual({ name: 'origin', url: null, host: null });

      // engineRemoteGet confirms origin is gone.
      const got = await engineRemoteGet(init.repoId);
      expect(got.url).toBeNull();
    });

    it('parses SCP-style ssh URLs into the host field', async () => {
      const opFile = await writeOpFile(temp.dir, 'login.op');
      const init = await engineInit(opFile);

      const result = await engineRemoteSet(init.repoId, 'git@github.com:foo/bar.git');
      expect(result.host).toBe('github.com');
      expect(result.url).toBe('git@github.com:foo/bar.git');
    });
  });

  // ---------------------------------------------------------------------------
  // Phase 7a: folder-mode divergent merge (system git gated)
  // ---------------------------------------------------------------------------

  /**
   * Helper: set up two local folder-mode repos sharing a bare remote.
   * Both have an initial commit with design.op. Returns paths for both clones.
   */
  async function setupFolderClonePair(): Promise<{ aDir: string; bDir: string }> {
    const remoteDir = join(temp.dir, 'remote.git');
    const aDir = join(temp.dir, 'a');
    const bDir = join(temp.dir, 'b');

    await execFileAsync('git', ['init', '--bare', remoteDir]);
    await execFileAsync('git', ['clone', remoteDir, aDir]);
    await execFileAsync('git', ['-C', aDir, 'checkout', '-b', 'main']);
    await fsp.writeFile(
      join(aDir, 'design.op'),
      JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#ff0000' }] }),
    );
    await fsp.writeFile(join(aDir, 'README.md'), '# Base\n');
    await execFileAsync('git', ['-C', aDir, 'add', '.']);
    await execFileAsync('git', [
      '-C',
      aDir,
      '-c',
      'user.name=t',
      '-c',
      'user.email=t@e.com',
      'commit',
      '-m',
      'base',
    ]);
    await execFileAsync('git', ['-C', aDir, 'push', '-u', 'origin', 'main']);
    await execFileAsync('git', ['clone', remoteDir, bDir]);
    return { aDir, bDir };
  }

  describe('Phase 7a: folder-mode divergent merge (system git gated)', () => {
    it.skipIf(!systemGitAvailable)(
      'folder-mode divergent pull returns conflict when .op file conflicts',
      async () => {
        const { aDir, bDir } = await setupFolderClonePair();

        // a makes a change to design.op and pushes.
        await fsp.writeFile(
          join(aDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#0000ff' }] }),
        );
        await execFileAsync('git', ['-C', aDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          aDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'a: blue',
        ]);
        await execFileAsync('git', ['-C', aDir, 'push']);

        // b makes a divergent change to design.op.
        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#00ff00' }] }),
        );
        await execFileAsync('git', ['-C', bDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          bDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'b: green',
        ]);

        const bResult = await engineOpen(bDir, join(bDir, 'design.op'));

        // Phase 7a: folder-mode divergent pull should now return conflict (not throw).
        const pullResult = await enginePull(bResult.repoId);
        expect(pullResult.result).toBe('conflict');
        expect(pullResult.conflicts).toBeDefined();
        expect(pullResult.conflicts!.nodeConflicts.length).toBeGreaterThan(0);

        // design.op on disk must be readable JSON (not conflict markers).
        const onDisk = await fsp.readFile(join(bDir, 'design.op'), 'utf-8');
        expect(() => JSON.parse(onDisk)).not.toThrow();
      },
    );

    it.skipIf(!systemGitAvailable)(
      'folder-mode divergent pull returns conflict-non-op when only non-.op files conflict',
      async () => {
        const { aDir, bDir } = await setupFolderClonePair();

        // a changes only README.md and pushes.
        await fsp.writeFile(join(aDir, 'README.md'), '# From A\n');
        await execFileAsync('git', ['-C', aDir, 'add', 'README.md']);
        await execFileAsync('git', [
          '-C',
          aDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'a: readme',
        ]);
        await execFileAsync('git', ['-C', aDir, 'push']);

        // b also changes README.md (divergently) but leaves design.op alone.
        await fsp.writeFile(join(bDir, 'README.md'), '# From B\n');
        await execFileAsync('git', ['-C', bDir, 'add', 'README.md']);
        await execFileAsync('git', [
          '-C',
          bDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'b: readme',
        ]);

        const bResult = await engineOpen(bDir, join(bDir, 'design.op'));

        const pullResult = await enginePull(bResult.repoId);
        expect(pullResult.result).toBe('conflict-non-op');
      },
    );

    it.skipIf(!systemGitAvailable)(
      'engineStatus reports mergeInProgress from on-disk MERGE_HEAD after session close/reopen',
      async () => {
        const { aDir, bDir } = await setupFolderClonePair();

        // Create divergent commits in a and b.
        await fsp.writeFile(
          join(aDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#0000ff' }] }),
        );
        await execFileAsync('git', ['-C', aDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          aDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'a',
        ]);
        await execFileAsync('git', ['-C', aDir, 'push']);

        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#00ff00' }] }),
        );
        await execFileAsync('git', ['-C', bDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          bDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'b',
        ]);

        // First session: open and pull (enters conflict).
        const session1 = await engineOpen(bDir, join(bDir, 'design.op'));
        await enginePull(session1.repoId);
        await engineClose(session1.repoId);

        // Clear in-memory sessions (simulate panel close/reopen).
        clearAllSessions();

        // Second session: reopen — must detect on-disk merge state.
        const session2 = await engineOpen(bDir, join(bDir, 'design.op'));
        const status = await engineStatus(session2.repoId);
        expect(status.mergeInProgress).toBe(true);
      },
    );

    it.skipIf(!systemGitAvailable)(
      'engineApplyMerge with on-disk merge state resolves .op conflicts and creates merge commit',
      async () => {
        const { aDir, bDir } = await setupFolderClonePair();

        await fsp.writeFile(
          join(aDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#0000ff' }] }),
        );
        await execFileAsync('git', ['-C', aDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          aDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'a',
        ]);
        await execFileAsync('git', ['-C', aDir, 'push']);

        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#00ff00' }] }),
        );
        await execFileAsync('git', ['-C', bDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          bDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'b',
        ]);

        const bResult = await engineOpen(bDir, join(bDir, 'design.op'));
        const pullResult = await enginePull(bResult.repoId);

        expect(pullResult.result).toBe('conflict');
        const conflictId = pullResult.conflicts!.nodeConflicts[0].id;
        await engineResolveConflict(bResult.repoId, conflictId, { kind: 'theirs' });

        const applied = await engineApplyMerge(bResult.repoId);
        expect(applied.noop).toBe(false);
        expect(applied.hash).toMatch(/^[a-f0-9]{40}$/);

        // design.op must be readable JSON with theirs's fill.
        const onDisk = JSON.parse(await fsp.readFile(join(bDir, 'design.op'), 'utf-8'));
        expect(onDisk.children[0].fill).toBe('#0000ff');

        // Session inflightMerge cleared.
        const { getSession: gs } = await import('../repo-session');
        const sess = gs(bResult.repoId)!;
        expect(sess.inflightMerge).toBeNull();
      },
    );

    it.skipIf(!systemGitAvailable)(
      'engineApplyMerge throws merge-still-conflicted when non-.op files remain unresolved',
      async () => {
        const { aDir, bDir } = await setupFolderClonePair();

        // Both change design.op and README.md.
        await fsp.writeFile(
          join(aDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#0000ff' }] }),
        );
        await fsp.writeFile(join(aDir, 'README.md'), '# A\n');
        await execFileAsync('git', ['-C', aDir, 'add', '.']);
        await execFileAsync('git', [
          '-C',
          aDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'a',
        ]);
        await execFileAsync('git', ['-C', aDir, 'push']);

        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#00ff00' }] }),
        );
        await fsp.writeFile(join(bDir, 'README.md'), '# B\n');
        await execFileAsync('git', ['-C', bDir, 'add', '.']);
        await execFileAsync('git', [
          '-C',
          bDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'b',
        ]);

        const bResult = await engineOpen(bDir, join(bDir, 'design.op'));
        const pullResult = await enginePull(bResult.repoId);
        expect(pullResult.result).toBe('conflict');

        // Resolve the .op conflict.
        const conflictId = pullResult.conflicts!.nodeConflicts[0].id;
        await engineResolveConflict(bResult.repoId, conflictId, { kind: 'ours' });

        // README.md is still unresolved → must throw merge-still-conflicted.
        await expect(engineApplyMerge(bResult.repoId)).rejects.toMatchObject({
          name: 'GitError',
          code: 'merge-still-conflicted',
        });
      },
    );

    it.skipIf(!systemGitAvailable)(
      'engineAbortMerge in folder mode aborts on-disk merge state',
      async () => {
        const { aDir, bDir } = await setupFolderClonePair();

        await fsp.writeFile(
          join(aDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#0000ff' }] }),
        );
        await execFileAsync('git', ['-C', aDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          aDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'a',
        ]);
        await execFileAsync('git', ['-C', aDir, 'push']);

        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#00ff00' }] }),
        );
        await execFileAsync('git', ['-C', bDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          bDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'b',
        ]);

        const bResult = await engineOpen(bDir, join(bDir, 'design.op'));
        await enginePull(bResult.repoId);

        await engineAbortMerge(bResult.repoId);

        // Session inflightMerge cleared.
        const { getSession: gs } = await import('../repo-session');
        const sess = gs(bResult.repoId)!;
        expect(sess.inflightMerge).toBeNull();

        // On-disk MERGE_HEAD gone.
        const { readMergeHead: rmh } = await import('../worktree-merge');
        const mergeHead = await rmh(join(bDir, '.git'));
        expect(mergeHead).toBeNull();

        // design.op is clean JSON.
        const onDisk = await fsp.readFile(join(bDir, 'design.op'), 'utf-8');
        expect(() => JSON.parse(onDisk)).not.toThrow();
      },
    );

    // -------------------------------------------------------------------------
    // Issue 1 (engine assertion): rename conflict is classified as conflict-non-op
    //
    // When the feature branch renames the tracked .op file, git places it in
    // conflict state as "deleted-by-them" (stage 3 missing for the original
    // path). The engine detects that stage 3 blob is absent for the tracked
    // file and classifies the merge as { result: 'conflict-non-op' } — the user
    // must resolve the rename in a terminal.
    // -------------------------------------------------------------------------
    it.skipIf(!systemGitAvailable)(
      'engineBranchMerge returns conflict-non-op when theirs renames the tracked .op file',
      async () => {
        const repoDir = join(temp.dir, 'repo-rename-engine');
        await fsp.mkdir(repoDir, { recursive: true });

        const g = (...args: string[]) => execFileAsync('git', args, { cwd: repoDir });
        const gc = (...args: string[]) =>
          execFileAsync('git', ['-c', 'user.name=t', '-c', 'user.email=t@e.com', ...args], {
            cwd: repoDir,
          });

        // Base: design.op on main.
        await g('init', '-b', 'main');
        await fsp.writeFile(
          join(repoDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base' }] }),
        );
        await g('add', '.');
        await gc('commit', '-m', 'base');

        // feature branch: rename design.op → design-v2.op AND modify content.
        await g('checkout', '-b', 'feature');
        await fsp.rename(join(repoDir, 'design.op'), join(repoDir, 'design-v2.op'));
        await fsp.writeFile(
          join(repoDir, 'design-v2.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'theirs' }] }),
        );
        await g('add', '-A');
        await gc('commit', '-m', 'rename to design-v2.op');

        // main: make a divergent change on the ORIGINAL design.op.
        await g('checkout', 'main');
        await fsp.writeFile(
          join(repoDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'ours' }] }),
        );
        await g('add', '.');
        await gc('commit', '-m', 'ours');

        // Restore design.op (it was renamed away on feature; main's checkout
        // puts it back with 'ours' content).
        // Open via engine with design.op as the tracked file.
        const result = await engineOpen(repoDir, join(repoDir, 'design.op'));
        expect(result.trackedFilePath).toBe(resolve(join(repoDir, 'design.op')));

        // Also manually set the autosaves ref so the engine can resolve commits.
        const isoGit = await import('isomorphic-git');
        const fsMod = await import('node:fs');
        const mainHash = await isoGit.resolveRef({
          fs: fsMod,
          dir: repoDir,
          ref: 'refs/heads/main',
        });
        const { setRef } = await import('../git-iso');
        const session = (await import('../repo-session')).getSession(result.repoId)!;
        await setRef({
          handle: session.handle,
          ref: 'refs/openpencil/autosaves/main',
          value: mainHash,
        });
        const featureHash = await isoGit.resolveRef({
          fs: fsMod,
          dir: repoDir,
          ref: 'refs/heads/feature',
        });
        await setRef({
          handle: session.handle,
          ref: 'refs/openpencil/autosaves/feature',
          value: featureHash,
        });

        // Merge feature into main.
        const mergeResult = await engineBranchMerge(result.repoId, 'feature');

        // ASSERTION: rename conflict → stage 3 blob missing for tracked file
        // → classified as conflict-non-op (user must resolve rename in terminal).
        expect(mergeResult.result).toBe('conflict-non-op');

        // Cleanup: abort the merge so the temp directory can be removed cleanly.
        await engineAbortMerge(result.repoId);
      },
    );

    // -------------------------------------------------------------------------
    // Issue 2: engineApplyMerge folder-mode noop path
    // The `noop: true` branch is reached when inflightMerge is null AND
    // MERGE_HEAD is absent (merge was committed externally, e.g. via terminal).
    // -------------------------------------------------------------------------
    it.skipIf(!systemGitAvailable)(
      'engineApplyMerge returns noop=true when merge was already committed externally',
      async () => {
        const { aDir, bDir } = await setupFolderClonePair();

        // Create a conflict between a and b.
        await fsp.writeFile(
          join(aDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#0000ff' }] }),
        );
        await execFileAsync('git', ['-C', aDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          aDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'a',
        ]);
        await execFileAsync('git', ['-C', aDir, 'push']);

        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#00ff00' }] }),
        );
        await execFileAsync('git', ['-C', bDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          bDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'b',
        ]);

        // Open b, pull to enter conflict state.
        const bResult = await engineOpen(bDir, join(bDir, 'design.op'));
        const pullResult = await enginePull(bResult.repoId);
        expect(pullResult.result).toBe('conflict');

        // Externally finalize the merge via sysFinalizeMerge (simulating a user
        // resolving the conflict in a terminal). First write a resolved file.
        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#ff00ff' }] }),
        );
        const { sysStageFile: sf, sysFinalizeMerge: sfm } = await import('../worktree-merge');
        await sf({ cwd: bDir, filepath: 'design.op' });
        await sfm({
          cwd: bDir,
          message: 'merge resolved externally',
          author: { name: 'External', email: 'ext@test.com' },
        });

        // Now clear the in-memory inflightMerge to simulate what the engine
        // session holds after the external commit (inflightMerge still set from
        // pullResult, but MERGE_HEAD is now gone).
        // We simulate this by clearing the session's inflightMerge manually.
        const { clearInflightMerge } = await import('../repo-session');
        clearInflightMerge(bResult.repoId);

        // engineApplyMerge should detect: no inflightMerge, no MERGE_HEAD →
        // return { noop: true } with the current HEAD hash.
        const applied = await engineApplyMerge(bResult.repoId);
        expect(applied.noop).toBe(true);
        expect(applied.hash).toMatch(/^[a-f0-9]{40}$/);
      },
    );

    // -------------------------------------------------------------------------
    // Issue 3: panel reopen without calling status() first (Option A contract)
    //
    // When MERGE_HEAD is present on disk but session.inflightMerge is null
    // (e.g. panel closed and reopened mid-conflict), engineApplyMerge must
    // throw a clear, caller-actionable error directing the caller to call
    // status() first. This is the deliberate Option A contract.
    //
    // The Phase 7b renderer always calls refreshStatus() on conflict-panel
    // entry, so this path is only hit by direct callers (CLI, test harnesses)
    // that skip the status() call.
    // -------------------------------------------------------------------------
    it.skipIf(!systemGitAvailable)(
      'engineStatus panel-reopen: returns reopenedMidMerge=true and tracked .op excluded from unresolvedFiles',
      async () => {
        const { aDir, bDir } = await setupFolderClonePair();

        // Create a conflict between a and b on design.op.
        await fsp.writeFile(
          join(aDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#0000ff' }] }),
        );
        await execFileAsync('git', ['-C', aDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          aDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'a',
        ]);
        await execFileAsync('git', ['-C', aDir, 'push']);

        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#00ff00' }] }),
        );
        await execFileAsync('git', ['-C', bDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          bDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'b',
        ]);

        // First session: open and pull (enters conflict).
        const session1 = await engineOpen(bDir, join(bDir, 'design.op'));
        await enginePull(session1.repoId);
        await engineClose(session1.repoId);

        // Simulate panel close/reopen: clear all in-memory sessions.
        clearAllSessions();

        // Second session: reopen — must detect on-disk merge state.
        const session2 = await engineOpen(bDir, join(bDir, 'design.op'));
        const status = await engineStatus(session2.repoId);

        // I2: must signal degraded panel-reopen mode.
        expect(status.reopenedMidMerge).toBe(true);

        // I2: tracked .op file must NOT appear in unresolvedFiles.
        // (It is in the git index with stages 1/2/3, but filtering prevents
        // the renderer from showing it as a misleading "non-op file".)
        const trackedRel = 'design.op';
        expect(status.unresolvedFiles).not.toContain(trackedRel);
      },
    );

    it.skipIf(!systemGitAvailable)(
      'engineApplyMerge throws merge-still-conflicted with actionable message when called without status() after panel reopen',
      async () => {
        const { aDir, bDir } = await setupFolderClonePair();

        // Create a conflict in b.
        await fsp.writeFile(
          join(aDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#0000ff' }] }),
        );
        await execFileAsync('git', ['-C', aDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          aDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'a',
        ]);
        await execFileAsync('git', ['-C', aDir, 'push']);

        await fsp.writeFile(
          join(bDir, 'design.op'),
          JSON.stringify({ version: '1.0.0', children: [{ id: 'base', fill: '#00ff00' }] }),
        );
        await execFileAsync('git', ['-C', bDir, 'add', 'design.op']);
        await execFileAsync('git', [
          '-C',
          bDir,
          '-c',
          'user.name=t',
          '-c',
          'user.email=t@e.com',
          'commit',
          '-m',
          'b',
        ]);

        // Session 1: open and pull (enters conflict — MERGE_HEAD on disk).
        const session1 = await engineOpen(bDir, join(bDir, 'design.op'));
        await enginePull(session1.repoId);
        await engineClose(session1.repoId);

        // Simulate panel close/reopen: clear all in-memory sessions.
        clearAllSessions();

        // Session 2: reopen the same repo. inflightMerge is null (new session),
        // but MERGE_HEAD is still on disk.
        const session2 = await engineOpen(bDir, join(bDir, 'design.op'));

        // Verify MERGE_HEAD is still on disk (the conflict is still active).
        const { readMergeHead: rmh } = await import('../worktree-merge');
        const mergeHead = await rmh(join(bDir, '.git'));
        expect(mergeHead).not.toBeNull();

        // Call engineApplyMerge WITHOUT calling status() first.
        // Must throw merge-still-conflicted with a message that guides the caller.
        await expect(engineApplyMerge(session2.repoId)).rejects.toMatchObject({
          name: 'GitError',
          code: 'merge-still-conflicted',
          message: expect.stringMatching(/call status\(\) first/i),
        });
      },
    );
  });
});
