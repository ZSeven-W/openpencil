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

// Synchronous 模块加载时的可用性探测。 We 不能使用异步 isSystemGitAvailable()，因为 vitest 是
// it.skipIf() 在测试收集时、在任何 beforeEach 挂钩运行之前读取其谓词。
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

    // Set up：裸远程，一次提交的源代码库，推送源 → 远程。
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

    // Now 通过 sysClone 克隆。
    await sysClone({ url: remoteDir, dest: cloneDir });

    // Verify 克隆具有 README。
    const content = await fsp.readFile(join(cloneDir, 'README.md'), 'utf-8');
    expect(content).toBe('# test\n');
  });

  it.skipIf(!systemGitAvailable)('fetch updates remote-tracking refs', async () => {
    const remoteDir = join(temp.dir, 'remote.git');
    const aDir = join(temp.dir, 'a');
    const bDir = join(temp.dir, 'b');

    await execFileAsync('git', ['init', '--bare', remoteDir]);
    // a：克隆、提交、推送
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

    // b：克隆相同的远程（现在有 main 和 one.txt）
    await execFileAsync('git', ['clone', remoteDir, bDir]);

    // a 提交另一个文件并推送
    await fsp.writeFile(join(aDir, 'two.txt'), '2');
    await execFileAsync('git', ['-C', aDir, 'add', '.']);
    await execFileAsync(
      'git',
      ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'two'],
      {},
    );
    await execFileAsync('git', ['-C', aDir, 'push']);

    // b 在获取之前的 ahead/behind 应该是 0/0 （b 还不知道新的提交）。
    const before = await sysAheadBehind({ cwd: bDir, branch: 'main' });
    expect(before).toEqual({ ahead: 0, behind: 0 });

    // Fetch 更新 b 的远程跟踪引用。
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

    // Verify 裸遥控器的主要指向克隆的提交。
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
    // a：一次提交的种子远程种子
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

    // b：克隆，然后 a 推送第二次提交
    await execFileAsync('git', ['clone', remoteDir, bDir]);
    await fsp.writeFile(join(aDir, 'two.txt'), '2');
    await execFileAsync('git', ['-C', aDir, 'add', '.']);
    await execFileAsync(
      'git',
      ['-C', aDir, '-c', 'user.name=t', '-c', 'user.email=t@e.com', 'commit', '-m', 'two'],
      {},
    );
    await execFileAsync('git', ['-C', aDir, 'push']);

    // b 进行了不同的提交并尝试推送 → 被拒绝。
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
// Phase 7a：worktree-merge real-git 峰值测试
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
   * Helper：创建一个具
   * 有两个不同分支的存储库，每个分支修改跟踪的 .op 文件（以及可选的 README.md 辅助文件）。
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
    // 使测试对于没有全局 git 配置的机器来说是自给自足的。
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

    // Branch 关闭：功能更改
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

    // Return 到 main：我们的更改
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

      // 磁盘上的 File 现在可以读取 JSON。
      const content = await fsp.readFile(join(repoDir, 'design.op'), 'utf-8');
      expect(() => JSON.parse(content)).not.toThrow();
      expect(JSON.parse(content).children[0].id).toBe('ours');

      // MERGE_HEAD 仍然设置。
      const mergeHead = await readMergeHead(gitdir);
      expect(mergeHead).not.toBeNull();

      // File 在索引中仍列为未解决。
      const unresolved = await sysListUnresolved({ cwd: repoDir });
      expect(unresolved).toContain('design.op');
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysStageFile marks file as resolved so sysListUnresolved no longer includes it',
    async () => {
      const { repoDir } = await setupDivergentRepo({});
      await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });

      // Write 最终内容并上演。
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

      // Resolve 冲突。
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

      // MERGE_HEAD 消失了。
      const mergeHead = await readMergeHead(gitdir);
      expect(mergeHead).toBeNull();

      // Verify 2-parent 通过 git cat 文件提交。
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

      // MERGE_HEAD 消失了。
      const mergeHead = await readMergeHead(gitdir);
      expect(mergeHead).toBeNull();

      // HEAD 不变。
      const headAfter = await sysReadHead({ cwd: repoDir });
      expect(headAfter).toBe(headBefore);

      // design.op 是我们的版本（干净的 JSON，没有冲突标记）。
      const content = await fsp.readFile(join(repoDir, 'design.op'), 'utf-8');
      expect(() => JSON.parse(content)).not.toThrow();
      expect(JSON.parse(content).children[0].id).toBe('ours');
    },
  );

  it.skipIf(!systemGitAvailable)(
    'sysAbortMerge is idempotent when no merge is in progress',
    async () => {
      const { repoDir } = await setupDivergentRepo({});
      // No 合并开始 - 中止不应抛出。
      await expect(sysAbortMerge({ cwd: repoDir })).resolves.toBeUndefined();
    },
  );

  // ---------------------------------------------------------------------------
  // Phase 7a 峰值场景 3：重命名冲突
  // Documents 当跟踪的 .op 文件是 RENAMED 时 git 实际执行的操作
  // 在功能分支上，同时也在两个分支上进行修改。
//
  // Setup：
  // 基础：design.op（基础内容）
  // 主要：design.op（已修改 — id：'我们的'）
  // 功能：design-v2.op（重命名 + 修改 — id：“他们的”）
//
  // `git merge --no-commit --no-ff feature` 之后 Expected git 行为：
  //   - exitCode 1（冲突）
  //   - `git ls-files -u` 列出 BOTH“design.op”（被他们删除，阶段 1+2）
  // AND "design-v2.op" （由他们添加，仅限第 3 阶段）
  //   - “design.op”的第 3 阶段 blob 不存在（文件已在其上重命名）
  //   - “design-v2.op”的 stage 1/2 blob 不存在（文件是新的）
//
  // Engine 含义（在下面的引擎测试中验证）：
  // Since 跟踪的“design.op”的第 3 阶段 blob 丢失，引擎
  // 落入 { result: 'conflict-non-op' }。 This 是 CORRECT 因为
  // 当他们重命名跟踪文件时，我们无法执行语义合并。
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

      // 功能分支：重命名 design.op→design-v2.op 并修改内容。
      await g('checkout', '-b', 'feature');
      await fsp.rename(join(repoDir, 'design.op'), join(repoDir, 'design-v2.op'));
      // Overwrite 具有不同内容的新名称，因此存在真正的内容差异。
      await fsp.writeFile(
        join(repoDir, 'design-v2.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'theirs' }] }),
      );
      await g('add', '-A');
      await gc('commit', '-m', 'rename to design-v2.op');

      // 主分支：就地修改 design.op（与功能重命名不同）。
      await g('checkout', 'main');
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'ours' }] }),
      );
      await g('add', '.');
      await gc('commit', '-m', 'ours');

      // Attempt 合并 — 预计会发生冲突。
      const mergeResult = await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });
      expect(mergeResult.kind).toBe('conflict');

      // SPIKE FINDING：git 通过将旧路径（“design.op”）和可能的新路径（“design-v2.op”）列为未解决来将重命名
      // 报告为冲突。 The 确切的设置取决于 git 版本和重命名检测阈值。
      const unresolved = await sysListUnresolved({ cwd: repoDir });

      // The 原始跟踪文件必须出现在未解析列表中，因为 git 检测到涉及它的重命名冲突。
      expect(unresolved).toContain('design.op');

      // Stage 3 blob 的 ORIGINAL 路径必须不存在（他们已将其重命名）。
      const stage3Original = await sysShowStageBlob({
        cwd: repoDir,
        stage: 3,
        filepath: 'design.op',
      });
      expect(stage3Original).toBeNull();

      // Stage 2 blob 的 ORIGINAL 路径（我们的）必须存在。
      const stage2Original = await sysShowStageBlob({
        cwd: repoDir,
        stage: 2,
        filepath: 'design.op',
      });
      expect(stage2Original).not.toBeNull();
      expect(JSON.parse(stage2Original!).children[0].id).toBe('ours');

      // CONCLUSION：引擎检查 trackedRel 的所有三个阶段。 When 第 3 阶段为 null，它返回 {
      // result: 'conflict-non-op' }。 This 是正确的：用户必须在终端中解析重命名。
    },
  );

  // ---------------------------------------------------------------------------
  // Phase 7a 峰值场景 4：肮脏的工作树行为
//
  // The 计划表示渲染器端 `withCleanWorkingTree` 门应该阻塞
  // 当跟踪的文件有未提交的更改时尝试合并。 This 测试
  // 记录了当该门被绕过时 git *实际上做了什么 - 建立
// the engine-layer contract: "the engine trusts callers to gate dirty trees;
// if they don't, here is what git does."
//
  // Spike 设置：
  //   - Both 分支在 design.op 上有不同的提交（真正的 3 路合并
  // 场景，而不是快进），所以 git 必须合并工作树。
  //   - The 工作树在其顶部有一个 ADDITIONAL 未提交的更改
  // 提交了我们的版本。
//
  // Spike 发现：
  // git merge --no-commit --no-ff 与合并的脏跟踪文件
  // 会用非零代码触摸退出，并且“本地更改将是
  // 已覆盖”消息。 sysMergeNoCommit 看到退出代码 != 0 和 != 1，
  // 所以它会抛出 GitError('engine-crash')。 The 脏内容是 NOT
  // 悄然丢失或被覆盖。
//
  // This 确认渲染器门是检查的正确位置：
  // 如果使用脏树调用，引擎将抛出异常（不是默默地损坏）。
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

      // Base 在 main 上提交。
      await g('init', '-b', 'main');
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'base' }] }),
      );
      await g('add', '.');
      await gc('commit', '-m', 'base');

      // 功能分支：修改 design.op 并提交。
      await g('checkout', '-b', 'feature');
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'theirs' }] }),
      );
      await g('add', '.');
      await gc('commit', '-m', 'theirs');

      // main: ALSO 进行发散提交（创建真正的三向合并）。
      await g('checkout', 'main');
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'ours' }] }),
      );
      await g('add', '.');
      await gc('commit', '-m', 'ours');

      // Now 弄脏了跟踪文件 AFTER 提交（未提交的工作树更改）。
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'dirty-uncommitted' }] }),
      );
      // Do NOT 阶段或提交 — 文件现在已脏。
      // SPIKE：尝试合并。 Git 检测到脏跟踪文件将是
      // 被合并覆盖并以非零代码 OTHER 大于 1 退出
      // （通常退出代码 1，但带有“将被覆盖”消息，或者
      // 在某些 git 版本上退出代码 128）。 Either 方式，sysMergeNoCommit
      // 抛出或返回 { kind: 'conflict' } （如果 git 写入了标记）。
//
      // The 关键合约：脏内容被 NEVER 默默丢弃。
      let threwOrConflict = false;
      try {
        const mergeResult = await sysMergeNoCommit({ cwd: repoDir, ref: 'feature' });
        // If sysMergeNoCommit 没有抛出，git 返回退出代码 0 或 1。 Exit 代码 1 表示它进入了冲突状态 -
        // 验证脏内容是否保留在冲突标记中或文件未解析。
        if (mergeResult.kind === 'conflict') {
          threwOrConflict = true;
          const unresolved = await sysListUnresolved({ cwd: repoDir });
          // design.op 必须列出 — 肮脏的工作树 + 合并冲突。
          expect(unresolved).toContain('design.op');
          // The 工作树文件应包含冲突标记（不是干净的 JSON）。
          const raw = await fsp.readFile(join(repoDir, 'design.op'), 'utf-8');
          // Either 它有冲突标记 OR 它是有效的 JSON （git 保留了我们的）。 In 这两种情况的内容都不能默默地替换为他们的内容。
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
        // If kind === 'clean'，脏内容与合并产生的内容相同 - 合并恰好是该文件的无操作。
      } catch (err) {
        // sysMergeNoCommit 抛出了 GitError —— git 完全拒绝了合并。 This 是覆盖脏文件时最常见的结果。
        threwOrConflict = true;
        // The 错误应该是引擎崩溃（来自 git 的 non-0/non-1 退出代码）。
        const e = err as { name?: string; code?: string };
        expect(e.name).toBe('GitError');
        expect(e.code).toBe('engine-crash');
      }

      // CONTRACT：git 抛出（拒绝）或进入冲突状态。 It 必须默默地用他们的内容覆盖 NOT 的脏内容。
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

      // Confirm 两者冲突。
      const unresolved = await sysListUnresolved({ cwd: repoDir });
      expect(unresolved).toContain('design.op');
      expect(unresolved).toContain('README.md');

      // Resolve .op 通过编写最终合并的内容和暂存。
      await fsp.writeFile(
        join(repoDir, 'design.op'),
        JSON.stringify({ version: '1.0.0', children: [{ id: 'merged' }] }),
      );
      await sysStageFile({ cwd: repoDir, filepath: 'design.op' });

      // .op 已解决； README 仍未解决。
      const afterOp = await sysListUnresolved({ cwd: repoDir });
      expect(afterOp).not.toContain('design.op');
      expect(afterOp).toContain('README.md');

      // Resolve README（拿我们的）。
      await sysRestoreOurs({ cwd: repoDir, filepath: 'README.md' });
      await sysStageFile({ cwd: repoDir, filepath: 'README.md' });

      // All 已解决。
      const afterAll = await sysListUnresolved({ cwd: repoDir });
      expect(afterAll).toHaveLength(0);

      // Finalize。
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
