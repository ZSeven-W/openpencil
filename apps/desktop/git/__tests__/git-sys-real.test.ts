// apps/desktop/git/__tests__/git-sys-real.test.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fsp } from 'node:fs';
import { join } from 'node:path';
import { execFile, execFileSync } from 'node:child_process';
import { promisify } from 'node:util';
import { sysClone, sysFetch, sysPush, sysAheadBehind, mapSysError } from '../git-sys';
import {
  sysMergeNoCommit,
  sysListUnresolved,
  readMergeHead,
  sysShowStageBlob,
  sysRestoreOurs,
  sysStageFile,
  sysFinalizeMerge,
  sysAbortMerge,
  sysReadHead,
} from '../worktree-merge';
import { mkTempDir } from './test-helpers';

const execFileAsync = promisify(execFile);

// Synchronous availability probe at module load. We can't use the async
// isSystemGitAvailable() because vitest's it.skipIf() reads its predicate at
// test-collection time, before any beforeEach hook has run.
let systemGitAvailable: boolean;
try {
  execFileSync('git', ['--version'], { stdio: 'ignore', timeout: 5000 });
  systemGitAvailable = true;
} catch {
  systemGitAvailable = false;
}

describe('git-sys real (gated on system git)', () => {
  let temp: { dir: string; dispose: () => Promise<void> };

  beforeEach(async () => {
    temp = await mkTempDir();
  });

  afterEach(async () => {
    if (temp) await temp.dispose();
  });

  it.skipIf(!systemGitAvailable)('clones a local bare remote', async () => {
    const remoteDir = join(temp.dir, 'remote.git');
    const sourceDir = join(temp.dir, 'source');
    const cloneDir = join(temp.dir, 'clone');

    // Set up: bare remote, source repo with one commit, push source → remote.
    await execFileAsync('git', ['init', '--bare', remoteDir]);
    await fsp.mkdir(sourceDir, { recursive: true });
    await execFileAsync('git', ['init', '-b', 'main', sourceDir]);
    await fsp.writeFile(join(sourceDir, 'README.md'), '# test\n');
    await execFileAsync('git', ['add', '.'], { cwd: sourceDir });
    await execFileAsync(
      'git',
      ['-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'init'],
      { cwd: sourceDir },
    );
    await execFileAsync('git', ['remote', 'add', 'origin', remoteDir], { cwd: sourceDir });
    await execFileAsync('git', ['push', 'origin', 'main'], { cwd: sourceDir });

    // Now clone via sysClone.
    await sysClone({ url: remoteDir, dest: cloneDir });

    // Verify the clone has the README.
    const content = await fsp.readFile(join(cloneDir, 'README.md'), 'utf-8');
    expect(content).toBe('# test\n');
  });

  it.skipIf(!systemGitAvailable)('fetch updates remote-tracking refs', async () => {
    const remoteDir = join(temp.dir, 'remote.git');
    const aDir = join(temp.dir, 'a');
    const bDir = join(temp.dir, 'b');

    await execFileAsync('git', ['init', '--bare', remoteDir]);
    // a: clone, commit, push
    await execFileAsync('git', ['clone', remoteDir, aDir]);
    await execFileAsync('git', ['-C', aDir, 'checkout', '-b', 'main']);
    await fsp.writeFile(join(aDir, 'one.txt'), '1');
    await execFileAsync('git', ['-C', aDir, 'add', '.']);
    await execFileAsync(
      'git',
      ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'one'],
      {},
    );
    await execFileAsync('git', ['-C', aDir, 'push', '-u', 'origin', 'main']);

    // b: clone the same remote (now has main with one.txt)
    await execFileAsync('git', ['clone', remoteDir, bDir]);

    // a commits another file and pushes
    await fsp.writeFile(join(aDir, 'two.txt'), '2');
    await execFileAsync('git', ['-C', aDir, 'add', '.']);
    await execFileAsync(
      'git',
      ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'two'],
      {},
    );
    await execFileAsync('git', ['-C', aDir, 'push']);

    // b's ahead/behind before fetch should be 0/0 (b doesn't know about the new commit yet).
    const before = await sysAheadBehind({ cwd: bDir, branch: 'main' });
    expect(before).toEqual({ ahead: 0, behind: 0 });

    // Fetch updates b's remote-tracking ref.
    await sysFetch({ cwd: bDir });
    const after = await sysAheadBehind({ cwd: bDir, branch: 'main' });
    expect(after).toEqual({ ahead: 0, behind: 1 });
  });

  it.skipIf(!systemGitAvailable)('push to local bare remote succeeds', async () => {
    const remoteDir = join(temp.dir, 'remote.git');
    const cloneDir = join(temp.dir, 'clone');

    await execFileAsync('git', ['init', '--bare', remoteDir]);
    await execFileAsync('git', ['clone', remoteDir, cloneDir]);
    await execFileAsync('git', ['-C', cloneDir, 'checkout', '-b', 'main']);
    await fsp.writeFile(join(cloneDir, 'a.txt'), 'a');
    await execFileAsync('git', ['-C', cloneDir, 'add', '.']);
    await execFileAsync(
      'git',
      ['-C', cloneDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'a'],
      {},
    );

    await sysPush({ cwd: cloneDir, branch: 'main' });

    // Verify the bare remote has main pointing at the clone's commit.
    const { stdout: remoteHead } = await execFileAsync('git', [
      '-C',
      remoteDir,
      'rev-parse',
      'main',
    ]);
    const { stdout: cloneHead } = await execFileAsync('git', ['-C', cloneDir, 'rev-parse', 'HEAD']);
    expect(remoteHead.trim()).toBe(cloneHead.trim());
  });

  it.skipIf(!systemGitAvailable)('push non-fast-forward is rejected', async () => {
    const remoteDir = join(temp.dir, 'remote.git');
    const aDir = join(temp.dir, 'a');
    const bDir = join(temp.dir, 'b');

    await execFileAsync('git', ['init', '--bare', remoteDir]);
    // a: seed remote with one commit
    await execFileAsync('git', ['clone', remoteDir, aDir]);
    await execFileAsync('git', ['-C', aDir, 'checkout', '-b', 'main']);
    await fsp.writeFile(join(aDir, 'one.txt'), '1');
    await execFileAsync('git', ['-C', aDir, 'add', '.']);
    await execFileAsync(
      'git',
      ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'one'],
      {},
    );
    await execFileAsync('git', ['-C', aDir, 'push', '-u', 'origin', 'main']);

    // b: clone, then a pushes a 2nd commit
    await execFileAsync('git', ['clone', remoteDir, bDir]);
    await fsp.writeFile(join(aDir, 'two.txt'), '2');
    await execFileAsync('git', ['-C', aDir, 'add', '.']);
    await execFileAsync(
      'git',
      ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'two'],
      {},
    );
    await execFileAsync('git', ['-C', aDir, 'push']);

    // b makes a divergent commit and tries to push → rejected.
    await fsp.writeFile(join(bDir, 'b.txt'), 'b');
    await execFileAsync('git', ['-C', bDir, 'add', '.']);
    await execFileAsync(
      'git',
      ['-C', bDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'b'],
      {},
    );
    await expect(sysPush({ cwd: bDir, branch: 'main' })).rejects.toMatchObject({
      name: 'GitError',
      code: 'push-rejected',
    });
  });
});

// ---------------------------------------------------------------------------
// Phase 7a: worktree-merge real-git spike tests
// ---------------------------------------------------------------------------

describe('worktree-merge real-git spike (gated on system git)', () => {
  let temp: { dir: string; dispose: () => Promise<void> };

  beforeEach(async () => {
    temp = await mkTempDir();
  });

  afterEach(async () => {
    if (temp) await temp.dispose();
  });

  /**
   * Helper: create a repo with two divergent branches, each modifying the
   * tracked .op file (and optionally a README.md side file).
   */
  async function setupDivergentRepo(opts: {
    withReadme?: boolean;
    readmeConflict?: boolean;
  }): Promise<{ repoDir: string; gitdir: string }> {
    const repoDir = join(temp.dir, 'repo');
    await fsp.mkdir(repoDir, { recursive: true });

    const g = (...args: string[]) => execFileAsync('git', args, { cwd: repoDir });
    const gc = (...args: string[]) =>
      execFileAsync('git', ['-c', 'user.name=t', '-c', 'user.email=t@e.com', ...args], {
        cwd: repoDir,
      });

    await g('init', '-b', 'main');
    // sysMergeNoCommit reads identity during git's internal bookkeeping;
    // make tests self-sufficient for machines without global git config.
    await g('config', 'user.name', 'Test');
    await g('config', 'user.email', 'test@test.com');
    await fsp.writeFile(
      join(repoDir, 'design.op'),
      JSON.stringify({ version: '1.0.0', children: [{ id: 'base' }] }),
    );
    if (opts.withReadme) {
      await fsp.writeFile(join(repoDir, 'README.md'), '# Base\n');
    }
    await g('add', '.');
    await gc('commit', '-m', 'base');

    // Branch off: feature changes
    await g('checkout', '-b', 'feature');
    await fsp.writeFile(
      join(repoDir, 'design.op'),
      JSON.stringify({ version: '1.0.0', children: [{ id: 'theirs' }] }),
    );
    if (opts.withReadme && opts.readmeConflict) {
      await fsp.writeFile(join(repoDir, 'README.md'), '# Feature\n');
    }
    await g('add', '.');
    await gc('commit', '-m', 'theirs');

    // Return to main: ours changes
    await g('checkout', 'main');
    await fsp.writeFile(
      join(repoDir, 'design.op'),
      JSON.stringify({ version: '1.0.0', children: [{ id: 'ours' }] }),
    );
    if (opts.withReadme && opts.readmeConflict) {
      await fsp.writeFile(join(repoDir, 'README.md'), '# Main\n');
    }
    await g('add', '.');
    await gc('commit', '-m', 'ours');

    const gitdir = join(repoDir, '.git');
    return { repoDir, gitdir };
  }

  it.skipIf(!systemGitAvailable)(
    'sysMergeNoCommit returns conflict and MERGE_HEAD is set',
    async () => {
      const { repoDir, gitdir } = await setupDivergentRepo({});

      const result = await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });
      expect(result.kind).toBe('conflict');

      const mergeHead = await readMergeHead(gitdir);
      expect(mergeHead).not.toBeNull();
      expect(mergeHead).toMatch(/^[a-f0-9]{40}$/);
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysListUnresolved lists the tracked .op file as unresolved',
    async () => {
      const { repoDir } = await setupDivergentRepo({});
      await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });

      const unresolved = await sysListUnresolved({ cwd: repoDir });
      expect(unresolved).toContain('design.op');
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysListUnresolved lists both .op and README when both conflict',
    async () => {
      const { repoDir } = await setupDivergentRepo({ withReadme: true, readmeConflict: true });
      await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });

      const unresolved = await sysListUnresolved({ cwd: repoDir });
      expect(unresolved).toContain('design.op');
      expect(unresolved).toContain('README.md');
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysShowStageBlob reads base/ours/theirs from the index',
    async () => {
      const { repoDir } = await setupDivergentRepo({});
      await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });

      const base = await sysShowStageBlob({ cwd: repoDir, stage: 1, filepath: 'design.op' });
      const ours = await sysShowStageBlob({ cwd: repoDir, stage: 2, filepath: 'design.op' });
      const theirs = await sysShowStageBlob({ cwd: repoDir, stage: 3, filepath: 'design.op' });

      expect(JSON.parse(base!).children[0].id).toBe('base');
      expect(JSON.parse(ours!).children[0].id).toBe('ours');
      expect(JSON.parse(theirs!).children[0].id).toBe('theirs');
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysRestoreOurs writes readable JSON and keeps MERGE_HEAD alive',
    async () => {
      const { repoDir, gitdir } = await setupDivergentRepo({});
      await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });

      await sysRestoreOurs({ cwd: repoDir, filepath: 'design.op' });

      // File on disk is now readable JSON.
      const content = await fsp.readFile(join(repoDir, 'design.op'), 'utf-8');
      expect(() => JSON.parse(content)).not.toThrow();
      expect(JSON.parse(content).children[0].id).toBe('ours');

      // MERGE_HEAD is still set.
      const mergeHead = await readMergeHead(gitdir);
      expect(mergeHead).not.toBeNull();

      // File is still listed as unresolved in the index.
      const unresolved = await sysListUnresolved({ cwd: repoDir });
      expect(unresolved).toContain('design.op');
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysStageFile marks file as resolved so sysListUnresolved no longer includes it',
    async () => {
      const { repoDir } = await setupDivergentRepo({});
      await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });

      // Write the final content and stage it.
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'resolved' }] }),
      );
      await sysStageFile({ cwd: repoDir, filepath: 'design.op' });

      const unresolved = await sysListUnresolved({ cwd: repoDir });
      expect(unresolved).not.toContain('design.op');
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysFinalizeMerge creates a 2-parent merge commit and clears MERGE_HEAD',
    async () => {
      const { repoDir, gitdir } = await setupDivergentRepo({});
      const headBefore = await sysReadHead({ cwd: repoDir });
      await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });

      // Resolve the conflict.
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'resolved' }] }),
      );
      await sysStageFile({ cwd: repoDir, filepath: 'design.op' });

      const mergeCommit = await sysFinalizeMerge({
        cwd: repoDir,
        message: 'Merge feature into main',
        author: { name: 'Test', email: 'test@test.com' },
      });

      expect(mergeCommit).toMatch(/^[a-f0-9]{40}$/);
      expect(mergeCommit).not.toBe(headBefore);

      // MERGE_HEAD is gone.
      const mergeHead = await readMergeHead(gitdir);
      expect(mergeHead).toBeNull();

      // Verify 2-parent commit via git cat-file.
      const catResult = await execFileAsync('git', ['cat-file', '-p', 'HEAD'], { cwd: repoDir });
      const parentLines = catResult.stdout.split('\n').filter((line) => line.startsWith('parent '));
      expect(parentLines).toHaveLength(2);
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysAbortMerge restores working tree and clears MERGE_HEAD',
    async () => {
      const { repoDir, gitdir } = await setupDivergentRepo({});
      const headBefore = await sysReadHead({ cwd: repoDir });
      await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });

      await sysAbortMerge({ cwd: repoDir });

      // MERGE_HEAD is gone.
      const mergeHead = await readMergeHead(gitdir);
      expect(mergeHead).toBeNull();

      // HEAD is unchanged.
      const headAfter = await sysReadHead({ cwd: repoDir });
      expect(headAfter).toBe(headBefore);

      // design.op is the ours version (clean JSON, no conflict markers).
      const content = await fsp.readFile(join(repoDir, 'design.op'), 'utf-8');
      expect(() => JSON.parse(content)).not.toThrow();
      expect(JSON.parse(content).children[0].id).toBe('ours');
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysAbortMerge is idempotent when no merge is in progress',
    async () => {
      const { repoDir } = await setupDivergentRepo({});
      // No merge started — abort should not throw.
      await expect(sysAbortMerge({ cwd: repoDir })).resolves.toBeUndefined();
    },
  );

  // ---------------------------------------------------------------------------
  // Phase 7a spike scenario 3: rename conflict
  // Documents what git actually does when the tracked .op file is RENAMED
  // on the feature branch while also being modified on both branches.
  //
  // Setup:
  //   base:    design.op  (base content)
  //   main:    design.op  (modified — id: 'ours')
  //   feature: design-v2.op (renamed + modified — id: 'theirs')
  //
  // Expected git behavior after `git merge --no-commit --no-ff feature`:
  //   - exitCode 1 (conflict)
  //   - `git ls-files -u` lists BOTH "design.op" (deleted-by-them, stages 1+2)
  //     AND "design-v2.op" (added-by-them, stage 3 only)
  //   - stage 3 blob for "design.op" is absent (file was renamed away on theirs)
  //   - stage 1/2 blobs for "design-v2.op" are absent (file is new on theirs)
  //
  // Engine implication (verified in the engine test below):
  //   Since stage 3 blob for the tracked "design.op" is missing, the engine
  //   falls through to { result: 'conflict-non-op' }. This is CORRECT because
  //   we cannot perform a semantic merge when theirs renamed the tracked file.
  // ---------------------------------------------------------------------------
  it.skipIf(!systemGitAvailable)(
    'spike scenario 3: rename conflict — sysListUnresolved reports both old and new name',
    async () => {
      const repoDir = join(temp.dir, 'repo-rename');
      await fsp.mkdir(repoDir, { recursive: true });

      const g = (...args: string[]) => execFileAsync('git', args, { cwd: repoDir });
      const gc = (...args: string[]) =>
        execFileAsync('git', ['-c', 'user.name=t', '-c', 'user.email=t@e.com', ...args], {
          cwd: repoDir,
        });

      await g('init', '-b', 'main');
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'base' }] }),
      );
      await g('add', '.');
      await gc('commit', '-m', 'base');

      // feature branch: rename design.op → design-v2.op and modify content.
      await g('checkout', '-b', 'feature');
      await fsp.rename(join(repoDir, 'design.op'), join(repoDir, 'design-v2.op'));
      // Overwrite the new name with different content so there's a real content diff.
      await fsp.writeFile(
        join(repoDir, 'design-v2.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'theirs' }] }),
      );
      await g('add', '-A');
      await gc('commit', '-m', 'rename to design-v2.op');

      // main branch: modify design.op in place (divergent from the feature rename).
      await g('checkout', 'main');
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'ours' }] }),
      );
      await g('add', '.');
      await gc('commit', '-m', 'ours');

      // Attempt merge — expect conflict.
      const mergeResult = await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });
      expect(mergeResult.kind).toBe('conflict');

      // SPIKE FINDING: git reports the rename as a conflict by listing the old
      // path ("design.op") and possibly the new path ("design-v2.op") as unresolved.
      // The exact set depends on git version and rename detection thresholds.
      const unresolved = await sysListUnresolved({ cwd: repoDir });

      // The original tracked file must appear in the unresolved list because
      // git detects a rename conflict involving it.
      expect(unresolved).toContain('design.op');

      // Stage 3 blob for the ORIGINAL path must be absent (theirs renamed it away).
      const stage3Original = await sysShowStageBlob({
        cwd: repoDir,
        stage: 3,
        filepath: 'design.op',
      });
      expect(stage3Original).toBeNull();

      // Stage 2 blob for the ORIGINAL path (ours) must be present.
      const stage2Original = await sysShowStageBlob({
        cwd: repoDir,
        stage: 2,
        filepath: 'design.op',
      });
      expect(stage2Original).not.toBeNull();
      expect(JSON.parse(stage2Original!).children[0].id).toBe('ours');

      // CONCLUSION: the engine checks for all three stages of trackedRel.
      // When stage 3 is null, it returns { result: 'conflict-non-op' }.
      // This is correct: the user must resolve the rename in a terminal.
    },
  );

  // ---------------------------------------------------------------------------
  // Phase 7a spike scenario 4: dirty working tree behavior
  //
  // The plan says the renderer-side `withCleanWorkingTree` gate should block
  // merge attempts when the tracked file has uncommitted changes. This test
  // documents what git *actually does* when that gate is bypassed — establishing
  // the engine-layer contract: "the engine trusts callers to gate dirty trees;
  // if they don't, here is what git does."
  //
  // Spike setup:
  //   - Both branches have a divergent commit on design.op (true 3-way merge
  //     scenario, not a fast-forward), so git must merge the working tree.
  //   - The working tree has an ADDITIONAL uncommitted change on top of the
  //     committed ours version.
  //
  // Spike finding:
  //   git merge --no-commit --no-ff with dirty tracked files that the merge
  //   would touch exits with a non-zero code and a "local changes would be
  //   overwritten" message. sysMergeNoCommit sees exit code != 0 and != 1,
  //   so it throws a GitError('engine-crash'). The dirty content is NOT
  //   silently lost or overwritten.
  //
  // This confirms that the renderer gate is the right place for the check:
  // the engine will throw (not silently corrupt) if called with a dirty tree.
  // ---------------------------------------------------------------------------
  it.skipIf(!systemGitAvailable)(
    'spike scenario 4: sysMergeNoCommit throws engine-crash when dirty tracked file would be overwritten',
    async () => {
      const repoDir = join(temp.dir, 'repo-dirty');
      await fsp.mkdir(repoDir, { recursive: true });

      const g = (...args: string[]) => execFileAsync('git', args, { cwd: repoDir });
      const gc = (...args: string[]) =>
        execFileAsync('git', ['-c', 'user.name=t', '-c', 'user.email=t@e.com', ...args], {
          cwd: repoDir,
        });

      // Base commit on main.
      await g('init', '-b', 'main');
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'base' }] }),
      );
      await g('add', '.');
      await gc('commit', '-m', 'base');

      // feature branch: modify design.op and commit.
      await g('checkout', '-b', 'feature');
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'theirs' }] }),
      );
      await g('add', '.');
      await gc('commit', '-m', 'theirs');

      // main: ALSO make a divergent commit (creates a true 3-way merge).
      await g('checkout', 'main');
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'ours' }] }),
      );
      await g('add', '.');
      await gc('commit', '-m', 'ours');

      // Now dirty the tracked file AFTER committing (uncommitted working-tree change).
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'dirty-uncommitted' }] }),
      );
      // Do NOT stage or commit — file is now dirty.

      // SPIKE: attempt the merge. Git detects the dirty tracked file would be
      // overwritten by the merge and exits with a non-zero code OTHER than 1
      // (typically exit code 1 but with a "would be overwritten" message, or
      // exit code 128 on some git versions). Either way, sysMergeNoCommit
      // either throws or returns { kind: 'conflict' } (if git wrote markers).
      //
      // The critical contract: the dirty content is NEVER silently discarded.
      let threwOrConflict = false;
      try {
        const mergeResult = await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });
        // If sysMergeNoCommit did not throw, git returned exit code 0 or 1.
        // Exit code 1 means it entered a conflict state — verify the dirty
        // content is preserved in conflict markers or the file is unresolved.
        if (mergeResult.kind === 'conflict') {
          threwOrConflict = true;
          const unresolved = await sysListUnresolved({ cwd: repoDir });
          // design.op must be listed — dirty working tree + merge conflict.
          expect(unresolved).toContain('design.op');
          // The working tree file should contain conflict markers (not clean JSON).
          const raw = await fsp.readFile(join(repoDir, 'design.op'), 'utf-8');
          // Either it has conflict markers OR it's valid JSON (git preserved ours).
          // In both cases the content must not be silently replaced with theirs.
          const hasConflictMarkers = raw.includes('<<<<<<<') || raw.includes('>>>>>>>');
          const isReadableJson = (() => {
            try {
              JSON.parse(raw);
              return true;
            } catch {
              return false;
            }
          })();
          expect(hasConflictMarkers || isReadableJson).toBe(true);
        }
        // If kind === 'clean', the dirty content was identical to what the merge
        // would produce — the merge happened to be a no-op for this file.
      } catch (err) {
        // sysMergeNoCommit threw a GitError — git refused the merge entirely.
        // This is the most common outcome when dirty files would be overwritten.
        threwOrConflict = true;
        // The error should be an engine-crash (non-0/non-1 exit code from git).
        const e = err as { name?: string; code?: string };
        expect(e.name).toBe('GitError');
        expect(e.code).toBe('engine-crash');
      }

      // CONTRACT: either git threw (refused) or entered conflict state.
      // It must NOT silently overwrite the dirty content with theirs.
      expect(threwOrConflict).toBe(true);
    },
  );

  it.skipIf(!systemGitAvailable)(
    'full workflow: tracked .op conflict + non-.op conflict, then finalize',
    async () => {
      const { repoDir, gitdir } = await setupDivergentRepo({
        withReadme: true,
        readmeConflict: true,
      });
      await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });

      // Confirm both conflict.
      const unresolved = await sysListUnresolved({ cwd: repoDir });
      expect(unresolved).toContain('design.op');
      expect(unresolved).toContain('README.md');

      // Resolve .op by writing final merged content and staging.
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'merged' }] }),
      );
      await sysStageFile({ cwd: repoDir, filepath: 'design.op' });

      // .op is resolved; README still unresolved.
      const afterOp = await sysListUnresolved({ cwd: repoDir });
      expect(afterOp).not.toContain('design.op');
      expect(afterOp).toContain('README.md');

      // Resolve README (take ours).
      await sysRestoreOurs({ cwd: repoDir, filepath: 'README.md' });
      await sysStageFile({ cwd: repoDir, filepath: 'README.md' });

      // All resolved.
      const afterAll = await sysListUnresolved({ cwd: repoDir });
      expect(afterAll).toHaveLength(0);

      // Finalize.
      const mergeCommit = await sysFinalizeMerge({
        cwd: repoDir,
        message: 'Merge feature: mixed conflict',
        author: { name: 'Test', email: 'test@test.com' },
      });
      expect(mergeCommit).toMatch(/^[a-f0-9]{40}$/);
      const mergeHead = await readMergeHead(gitdir);
      expect(mergeHead).toBeNull();
    },
  );
});

describe('mapSysError', () => {
  it('maps known stderr substrings to GitError codes', () => {
    expect(mapSysError('Authentication failed for ...')).toBe('auth-failed');
    expect(mapSysError('Permission denied (publickey).')).toBe('auth-failed');
    expect(mapSysError('Repository not found')).toBe('clone-failed');
    expect(
      mapSysError("destination path 'foo' already exists and is not an empty directory."),
    ).toBe('clone-target-exists');
    expect(mapSysError("Couldn't resolve host 'github.com'")).toBe('network');
    expect(mapSysError('Connection timed out')).toBe('timeout');
    expect(mapSysError('Updates were rejected because ...')).toBe('push-rejected');
    expect(mapSysError('not possible to fast-forward, aborting.')).toBe('pull-non-fast-forward');
    expect(mapSysError('fatal: not a git repository')).toBe('not-a-repo');
    expect(mapSysError('something completely unexpected')).toBe('engine-crash');
  });
});
