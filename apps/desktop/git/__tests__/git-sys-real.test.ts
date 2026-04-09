// apps/desktop/git/__tests__/git-sys-real.test.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fsp } from 'node:fs';
import { join } from 'node:path';
import { execFile, execFileSync } from 'node:child_process';
import { promisify } from 'node:util';
import { sysClone, sysFetch, sysPush, sysAheadBehind, mapSysError } from '../git-sys';
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
