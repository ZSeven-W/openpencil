// apps/web/src/stores/__tests__/git-store.test.ts
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { GitError } from '@/services/git-error';

// Mock 在导入商店之前使用 git-client，以便商店在模块评估时获取模拟。 All 31 IPC 方法使用 vi.fn()
// 默认值进行存根，因此未存根的路径会大声失败（作为无操作返回未定义），而不是因“不是函数”而崩溃。
vi.mock('@/services/git-client', () => {
  return {
    gitClient: {
      detect: vi.fn(),
      init: vi.fn(),
      open: vi.fn(),
      clone: vi.fn(),
      bindTrackedFile: vi.fn(),
      listCandidates: vi.fn(),
      close: vi.fn(),
      status: vi.fn(),
      log: vi.fn(),
      diff: vi.fn(),
      commit: vi.fn(),
      restore: vi.fn(),
      promote: vi.fn(),
      branchList: vi.fn(),
      branchCreate: vi.fn(),
      branchSwitch: vi.fn(),
      branchDelete: vi.fn(),
      branchMerge: vi.fn(),
      resolveConflict: vi.fn(),
      applyMerge: vi.fn(),
      abortMerge: vi.fn(),
      fetch: vi.fn(),
      pull: vi.fn(),
      push: vi.fn(),
      authStore: vi.fn(),
      authGet: vi.fn(),
      authClear: vi.fn(),
      sshListKeys: vi.fn(),
      sshGenerateKey: vi.fn(),
      sshImportKey: vi.fn(),
      sshDeleteKey: vi.fn(),
      getSystemAuthor: vi.fn(),
      remoteGet: vi.fn(),
      remoteSet: vi.fn(),
    },
    isGitApiAvailable: vi.fn(() => true),
  };
});

// Mock 加载助手，因此无需实际连接文件 IPC + 文档存储流即可断言 acknowledgeAutoBindAndOpen。
vi.mock('@/utils/load-op-file', () => ({
  loadOpFileFromPath: vi.fn(async () => true),
}));

// Mock documentEvents 因此自动保存订阅者测试可以确定性地触发“保存”事件，而无需连接真正的发射器。
vi.mock('@/utils/document-events', () => {
  const handlers: Array<(payload: unknown) => void> = [];
  return {
    documentEvents: {
      on: (_event: string, handler: (payload: unknown) => void) => {
        handlers.push(handler);
        return () => {
          const idx = handlers.indexOf(handler);
          if (idx >= 0) handlers.splice(idx, 1);
        };
      },
      emit: (_event: string, payload: unknown) => {
        // Snapshot 在迭代之前，因此在迭代中取消订阅自身的处理程序不会导致兄弟节点被跳过（与真正的
        // DocumentEventEmitter 相同的模式，它迭代 Set）。
        const snapshot = Array.from(handlers);
        for (const h of snapshot) h(payload);
      },
      __clear: () => {
        handlers.length = 0;
      },
    },
  };
});

// Mock 文档存储，因此 withCleanWorkingTree 可以读取 isDirty，无需
// 拉动完整的文件实施。
//
// `save` 被提升到一个稳定的间谍，因此测试可以 (a) 断言它被调用
// (b) 通过 __setsaveresult 覆盖其返回值。 Without 这个
// 提升机，每个 getState() 调用都会构建一个新的 vi.fn() 和间谍
// 在任何断言看到它之前就会消失。
vi.mock('@/stores/document-store', () => {
  let dirty = false;
  let saveResult: string | null = 'saved-path.op';
  const saveSpy = vi.fn(async () => saveResult);
  return {
    useDocumentStore: {
      getState: () => ({
        isDirty: dirty,
        save: saveSpy,
      }),
      // Test 助手：
      __setDirty: (next: boolean) => {
        dirty = next;
      },
      // Test 助手：覆盖 save() 的返回值。 The 存储的 retrySaveRequired 操作将
      // null 视为“保存失败”并在不清除 saveRequiredFor 的情况下进行保释。
      __setSaveResult: (result: string | null) => {
        saveResult = result;
      },
      // Test 助手：稳定的间谍，因此测试可以断言调用计数。
      __saveSpy: saveSpy,
    },
  };
});

// Now 导入商店（它将获取上面的模拟）。
import { useGitStore, __resetGitStore } from '@/stores/git-store';
import { gitClient } from '@/services/git-client';
// eslint-disable-next-line @typescript-eslint/no-explicit-any
import { useDocumentStore as mockedDocStore } from '@/stores/document-store';
import { loadOpFileFromPath as mockedLoadOpFileFromPath } from '@/utils/load-op-file';
import { documentEvents as mockedDocumentEvents } from '@/utils/document-events';

const SAMPLE_REPO = {
  repoId: 'repo-1',
  mode: 'single-file' as const,
  rootPath: '/tmp/repo',
  gitdir: '/tmp/repo/.op-history/login.op.git',
  engineKind: 'iso' as const,
  trackedFilePath: '/tmp/repo/login.op',
  candidates: [
    {
      path: '/tmp/repo/login.op',
      relativePath: 'login.op',
      milestoneCount: 0,
      autosaveCount: 0,
      lastCommitAt: null,
      lastCommitMessage: null,
    },
  ],
};

// Default GitStatusInfo 用于 refresh-after-init/open/clone/bind 路径。
// Individual 测试可以通过 vi.mocked(gitClient.status).mockResolvedValue 覆盖。
const DEFAULT_STATUS = {
  branch: 'main',
  trackedFilePath: '/tmp/repo/login.op',
  workingDirty: false,
  otherFilesDirty: 0,
  otherFilesPaths: [],
  ahead: 0,
  behind: 0,
  mergeInProgress: false,
  unresolvedFiles: [],
  conflicts: null,
};

describe('git-store state machine', () => {
  beforeEach(() => {
    __resetGitStore();
    vi.clearAllMocks();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocumentEvents as any).__clear?.();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setDirty(false);
    // Reset 提升的保存结果，因此之前测试的 __setSaveResult(null)
    // 不会渗入这个。 vi.clearAllMocks() 不触及闭包
    // 状态，所以我们必须明确地重置变量。
// eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setSaveResult('saved-path.op');
    // Set 刷新路径的安全默认解析值。 Without 这些、initRepo/openRepo/cloneRepo/bindTrackedF
    // ile 会崩溃，因为它们现在自动调用 status() 和 branchList()。
    vi.mocked(gitClient.status).mockResolvedValue(DEFAULT_STATUS);
    vi.mocked(gitClient.branchList).mockResolvedValue([]);
    vi.mocked(gitClient.log).mockResolvedValue([]);
    // Phase 6a：refreshRemote() 也是从 init/open/clone 调用的。 Default
    // 到“无远程配置”存根，因此现有的测试期望仍然有效；个别测试会覆盖这一点。
    vi.mocked(gitClient.remoteGet).mockResolvedValue({
      name: 'origin',
      url: null,
      host: null,
    });
    vi.mocked(gitClient.remoteSet).mockResolvedValue({
      name: 'origin',
      url: null,
      host: null,
    });
    // Phase 4a：用于作者身份首选项查找的 window.electronAPI 模拟
    vi.stubGlobal('window', {
      electronAPI: {
        getPreferences: vi.fn(async () => ({})),
        setPreference: vi.fn(async () => {}),
        git: {}, // 说实话，loadAuthorIdentity 的第 2 步继续进行
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('initial state is no-file with panelOpen=false', () => {
    const s = useGitStore.getState();
    expect(s.state).toEqual({ kind: 'no-file' });
    expect(s.panelOpen).toBe(false);
  });

  it('togglePanel flips panelOpen', () => {
    useGitStore.getState().togglePanel();
    expect(useGitStore.getState().panelOpen).toBe(true);
    useGitStore.getState().togglePanel();
    expect(useGitStore.getState().panelOpen).toBe(false);
  });

  it('detectRepo(none) transitions to no-repo', async () => {
    vi.mocked(gitClient.detect).mockResolvedValue({ mode: 'none' });
    await useGitStore.getState().detectRepo('/tmp/file.op');
    expect(useGitStore.getState().state.kind).toBe('no-repo');
  });

  it('initRepo transitions through initializing to ready', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    const promise = useGitStore.getState().initRepo('/tmp/login.op');
    // Note：vitest 同步运行 Promise 直到第一个等待，因此我们无法在不拆分解析的情况下轻松观察“初始化”中间状态。 Just
    // 断言最终状态。
    await promise;
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('ready');
    if (s.kind === 'ready') {
      expect(s.repo.repoId).toBe('repo-1');
      expect(s.repo.trackedFilePath).toBe('/tmp/repo/login.op');
    }
  });

  it('openRepo with null trackedFilePath lands in needs-tracked-file', async () => {
    vi.mocked(gitClient.open).mockResolvedValue({
      ...SAMPLE_REPO,
      mode: 'folder',
      trackedFilePath: null,
      candidates: [
        { ...SAMPLE_REPO.candidates[0], relativePath: 'a.op', path: '/tmp/repo/a.op' },
        { ...SAMPLE_REPO.candidates[0], relativePath: 'b.op', path: '/tmp/repo/b.op' },
      ],
    });
    await useGitStore.getState().openRepo('/tmp/repo');
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('needs-tracked-file');
    if (s.kind === 'needs-tracked-file') {
      expect(s.repo.candidateFiles).toHaveLength(2);
    }
  });

  it('bindTrackedFile promotes needs-tracked-file → ready', async () => {
    vi.mocked(gitClient.open).mockResolvedValue({
      ...SAMPLE_REPO,
      mode: 'folder',
      trackedFilePath: null,
      // Use 多个候选者，因此 openRepo 的 Phase 4b 自动绑定分支会触发 NOT —
      // 我们希望此测试从需求跟踪文件状态执行手动 bindTrackedFile 流程。
      candidates: [
        { ...SAMPLE_REPO.candidates[0], relativePath: 'a.op', path: '/tmp/repo/a.op' },
        {
          ...SAMPLE_REPO.candidates[0],
          relativePath: 'login.op',
          path: '/tmp/repo/login.op',
        },
      ],
    });
    vi.mocked(gitClient.bindTrackedFile).mockResolvedValue({
      trackedFilePath: '/tmp/repo/login.op',
    });
    await useGitStore.getState().openRepo('/tmp/repo');
    expect(useGitStore.getState().state.kind).toBe('needs-tracked-file');
    await useGitStore.getState().bindTrackedFile('/tmp/repo/login.op');
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('ready');
    if (s.kind === 'ready') {
      expect(s.repo.trackedFilePath).toBe('/tmp/repo/login.op');
    }
  });

  it('commitMilestone with dirty document sets saveRequiredFor and throws save-required', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setDirty(true);

    await expect(
      useGitStore.getState().commitMilestone('first', { name: 't', email: 't@e.com' }),
    ).rejects.toMatchObject({ name: 'GitError', code: 'save-required' });

    const s = useGitStore.getState().state;
    expect(s.kind).toBe('ready');
    if (s.kind === 'ready') {
      expect(s.saveRequiredFor).toBeDefined();
      expect(s.saveRequiredFor?.label).toBe('commit milestone');
    }
    // The 客户端的提交方法应该已被调用。
    expect(gitClient.commit).not.toHaveBeenCalled();
  });

  it('a thrown GitError during initRepo transitions to error state', async () => {
    vi.mocked(gitClient.init).mockRejectedValue(
      new GitError('init-failed', 'permission denied', { recoverable: false }),
    );
    await expect(useGitStore.getState().initRepo('/tmp/login.op')).rejects.toBeInstanceOf(GitError);
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('error');
    if (s.kind === 'error') {
      expect(s.message).toBe('permission denied');
      expect(s.recoverable).toBe(false);
    }
  });

  it('refreshStatus promotes ready → conflict when backend reports mergeInProgress', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    expect(useGitStore.getState().state.kind).toBe('ready');

    // Now 模拟后端报告正在进行的合并与一个冲突。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: ['login.op'],
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:rect-1',
            pageId: null,
            nodeId: 'rect-1',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });

    await useGitStore.getState().refreshStatus();
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.conflicts.nodeConflicts.size).toBe(1);
      expect(s.conflicts.nodeConflicts.get('node:_:rect-1')).toBeDefined();
    }
  });

  it('refreshStatus promotes ready → conflict with reopenedMidMerge=true when backend signals degraded panel-reopen state', async () => {
    // I2：当 engineStatus 返回 mergeInProgress=true + reopenedMidMerge=true 且没有冲突包且空
    // unresolvedFiles（跟踪的 .op 被过滤掉）时，refreshStatus 仍必须提升到冲突状态并传递标志。
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    expect(useGitStore.getState().state.kind).toBe('ready');

    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: null,
      reopenedMidMerge: true,
    });

    await useGitStore.getState().refreshStatus();
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.reopenedMidMerge).toBe(true);
      expect(s.unresolvedFiles).toEqual([]);
      expect(s.conflicts.nodeConflicts.size).toBe(0);
      expect(s.conflicts.docFieldConflicts.size).toBe(0);
    }
  });

  it('refreshStatus demotes conflict → ready when backend says merge is no longer in flight', async () => {
    // Set 通过 mergeBranch 进入冲突状态。
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:rect-1',
            pageId: null,
            nodeId: 'rect-1',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    await useGitStore.getState().mergeBranch('feature');
    expect(useGitStore.getState().state.kind).toBe('conflict');

    // Backend 现在报告合并已在外部完成（例如终端 git）。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: false,
      conflicts: null,
    });
    await useGitStore.getState().refreshStatus();
    expect(useGitStore.getState().state.kind).toBe('ready');
  });

  it('retrySaveRequired clears saveRequiredFor and re-runs the queued action after save succeeds', async () => {
    // Set 进入就绪状态，并在脏文档上等待排队提交。
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.commit).mockResolvedValue({ hash: 'abc123' });
    await useGitStore.getState().initRepo('/tmp/login.op');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setDirty(true);

    // First 调用 saveRequiredFor 中的陷阱。
    await expect(
      useGitStore.getState().commitMilestone('first', { name: 't', email: 't@e.com' }),
    ).rejects.toMatchObject({ name: 'GitError', code: 'save-required' });

    // The 用户单击面板中的“保存”。 Simulate 脏旗翻转
    // 返回到 false （就像保存后真实的文档存储一样）。
    // saveResult 已经是 beforeEach 重置后的“saved-path.op”。
// eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setDirty(false);

    await useGitStore.getState().retrySaveRequired();

    // The save 间谍仅被调用一次，原始提交 IPC 是
    // 使用排队参数仅调用一次，并且 saveRequiredFor 是
    // 现在清除了。
// eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((mockedDocStore as any).__saveSpy).toHaveBeenCalledTimes(1);
    expect(gitClient.commit).toHaveBeenCalledTimes(1);
    expect(gitClient.commit).toHaveBeenCalledWith('repo-1', {
      kind: 'milestone',
      message: 'first',
      author: { name: 't', email: 't@e.com' },
    });
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('ready');
    if (s.kind === 'ready') {
      expect(s.saveRequiredFor).toBeUndefined();
    }
  });

  it('retrySaveRequired bails without clearing saveRequiredFor when save returns null', async () => {
    // Set 进入就绪状态，并在脏文档上等待排队提交。
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setDirty(true);

    await expect(
      useGitStore.getState().commitMilestone('first', { name: 't', email: 't@e.com' }),
    ).rejects.toMatchObject({ name: 'GitError', code: 'save-required' });

    // Simulate save() 失败（返回 null）。 Do NOT 清除 isDirty —
    // 在现实生活中保存失败后，文档仍然很脏。
// eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setSaveResult(null);

    await useGitStore.getState().retrySaveRequired();

    // The save 间谍被调用一次，但是提交 IPC 被调用了 NOT 并且
    // saveRequiredFor 仍处于设置状态，因此用户可以重试。
// eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((mockedDocStore as any).__saveSpy).toHaveBeenCalledTimes(1);
    expect(gitClient.commit).not.toHaveBeenCalled();
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('ready');
    if (s.kind === 'ready') {
      expect(s.saveRequiredFor).toBeDefined();
      expect(s.saveRequiredFor?.label).toBe('commit milestone');
    }
  });

  it('closeRepo swallows gitClient.close failures and still resets state to no-file', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    expect(useGitStore.getState().state.kind).toBe('ready');

    // Backend 近距离投掷 — 例如，会话已经清理完毕。
    vi.mocked(gitClient.close).mockRejectedValue(new Error('session not found'));

    // closeRepo 不能抛出——无论如何它都会吞下并重置状态。
    await expect(useGitStore.getState().closeRepo()).resolves.toBeUndefined();

    const s = useGitStore.getState();
    expect(s.state).toEqual({ kind: 'no-file' });
    expect(s.log).toEqual([]);
    // The close IPC 只尝试过一次。
    expect(gitClient.close).toHaveBeenCalledTimes(1);
    expect(gitClient.close).toHaveBeenCalledWith('repo-1');
  });

  it('refreshStatus Step 1 copies basic status fields onto the active repo', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    expect(useGitStore.getState().state.kind).toBe('ready');

    // Override 状态为每个 Step 1 字段返回新值。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      branch: 'feature/login-redesign',
      workingDirty: true,
      otherFilesDirty: 2,
      otherFilesPaths: ['README.md', 'src/index.ts'],
      ahead: 3,
      behind: 1,
    });

    await useGitStore.getState().refreshStatus();

    const s = useGitStore.getState().state;
    expect(s.kind).toBe('ready');
    if (s.kind === 'ready') {
      expect(s.repo.currentBranch).toBe('feature/login-redesign');
      expect(s.repo.workingDirty).toBe(true);
      expect(s.repo.otherFilesDirty).toBe(2);
      expect(s.repo.otherFilesPaths).toEqual(['README.md', 'src/index.ts']);
      expect(s.repo.ahead).toBe(3);
      expect(s.repo.behind).toBe(1);
    }
  });

  it('requireRepoId throws GitError(no-file) when called from a non-repo state', async () => {
    // Initial 状态为无文件（由 beforeEach 中的 __resetgitstore
    // 设置）。如果没有首先转换到存储库状态，则调用 requireRepoId 的 Any 操作必须以
    // GitError('no-file') 拒绝。
    expect(useGitStore.getState().state.kind).toBe('no-file');

    await expect(useGitStore.getState().refreshStatus()).rejects.toMatchObject({
      name: 'GitError',
      code: 'no-file',
    });
  });

  // ---- Phase 4a：作者身份切片 ----------------------------------

  it('loadAuthorIdentity hits prefs first when both keys are set', async () => {
    // Stub window.electronAPI.getPreferences 返回两个 git 键。
    vi.stubGlobal('window', {
      electronAPI: {
        getPreferences: vi.fn(async () => ({
          'git.authorName': 'Alice',
          'git.authorEmail': 'alice@example.com',
        })),
        setPreference: vi.fn(async () => {}),
        git: {},
      },
    });

    await useGitStore.getState().loadAuthorIdentity();

    const id = useGitStore.getState().authorIdentity;
    expect(id).toEqual({ name: 'Alice', email: 'alice@example.com' });
    // 当首选项命中时，必须调用 The sysGit 后备 NOT。
    expect(gitClient.getSystemAuthor).not.toHaveBeenCalled();
  });

  it('loadAuthorIdentity falls through to sysGit when prefs are missing', async () => {
    vi.stubGlobal('window', {
      electronAPI: {
        getPreferences: vi.fn(async () => ({})),
        setPreference: vi.fn(async () => {}),
        git: {},
      },
    });
    vi.mocked(gitClient.getSystemAuthor).mockResolvedValue({
      name: 'Bob',
      email: 'bob@local',
    });

    await useGitStore.getState().loadAuthorIdentity();

    const id = useGitStore.getState().authorIdentity;
    expect(id).toEqual({ name: 'Bob', email: 'bob@local' });
    expect(gitClient.getSystemAuthor).toHaveBeenCalledTimes(1);
  });

  it('loadAuthorIdentity leaves identity null when both prefs and sysGit are empty', async () => {
    vi.stubGlobal('window', {
      electronAPI: {
        getPreferences: vi.fn(async () => ({})),
        setPreference: vi.fn(async () => {}),
        git: {},
      },
    });
    vi.mocked(gitClient.getSystemAuthor).mockResolvedValue(null);

    await useGitStore.getState().loadAuthorIdentity();

    expect(useGitStore.getState().authorIdentity).toBeNull();
    expect(gitClient.getSystemAuthor).toHaveBeenCalledTimes(1);
  });

  it('setAuthorIdentity persists to prefs and updates the in-memory cache', async () => {
    const setPrefSpy = vi.fn(async () => {});
    vi.stubGlobal('window', {
      electronAPI: {
        getPreferences: vi.fn(async () => ({})),
        setPreference: setPrefSpy,
        git: {},
      },
    });

    await useGitStore.getState().setAuthorIdentity('Charlie', 'charlie@example.com');

    expect(setPrefSpy).toHaveBeenCalledTimes(2);
    expect(setPrefSpy).toHaveBeenCalledWith('git.authorName', 'Charlie');
    expect(setPrefSpy).toHaveBeenCalledWith('git.authorEmail', 'charlie@example.com');
    expect(useGitStore.getState().authorIdentity).toEqual({
      name: 'Charlie',
      email: 'charlie@example.com',
    });
  });

  // ---- Phase 4b：自动绑定横幅 ---------------------------------------

  it('openRepo auto-binds the single candidate and sets lastAutoBindedPath', async () => {
    vi.mocked(gitClient.open).mockResolvedValue({
      ...SAMPLE_REPO,
      mode: 'folder',
      trackedFilePath: null,
      candidates: [
        {
          path: '/tmp/repo/login.op',
          relativePath: 'login.op',
          milestoneCount: 5,
          autosaveCount: 12,
          lastCommitAt: 1700000000,
          lastCommitMessage: 'init',
        },
      ],
    });
    vi.mocked(gitClient.bindTrackedFile).mockResolvedValue({
      trackedFilePath: '/tmp/repo/login.op',
    });

    await useGitStore.getState().openRepo('/tmp/repo');

    const s = useGitStore.getState();
    expect(s.state.kind).toBe('ready');
    if (s.state.kind === 'ready') {
      expect(s.state.repo.trackedFilePath).toBe('/tmp/repo/login.op');
    }
    expect(s.lastAutoBindedPath).toBe('/tmp/repo/login.op');
    expect(gitClient.bindTrackedFile).toHaveBeenCalledWith('repo-1', '/tmp/repo/login.op');
  });

  it('cloneRepo auto-binds the single candidate and sets lastAutoBindedPath', async () => {
    vi.mocked(gitClient.clone).mockResolvedValue({
      ...SAMPLE_REPO,
      mode: 'folder',
      trackedFilePath: null,
      candidates: [
        {
          path: '/tmp/cloned/main.op',
          relativePath: 'main.op',
          milestoneCount: 0,
          autosaveCount: 0,
          lastCommitAt: null,
          lastCommitMessage: null,
        },
      ],
    });
    vi.mocked(gitClient.bindTrackedFile).mockResolvedValue({
      trackedFilePath: '/tmp/cloned/main.op',
    });

    await useGitStore.getState().cloneRepo({
      url: 'https://example.com/repo.git',
      dest: '/tmp/cloned',
    });

    const s = useGitStore.getState();
    expect(s.state.kind).toBe('ready');
    expect(s.lastAutoBindedPath).toBe('/tmp/cloned/main.op');
    expect(gitClient.bindTrackedFile).toHaveBeenCalledWith('repo-1', '/tmp/cloned/main.op');
  });

  it('acknowledgeAutoBind clears lastAutoBindedPath', () => {
    // Manually 为标志播种（此处无需经过 openRepo）。
    useGitStore.setState({ lastAutoBindedPath: '/tmp/repo/login.op' });
    expect(useGitStore.getState().lastAutoBindedPath).toBe('/tmp/repo/login.op');

    useGitStore.getState().acknowledgeAutoBind();

    expect(useGitStore.getState().lastAutoBindedPath).toBeNull();
  });

  it('acknowledgeAutoBindAndOpen calls loadOpFileFromPath and clears the flag', async () => {
    useGitStore.setState({ lastAutoBindedPath: '/tmp/repo/login.op' });

    await useGitStore.getState().acknowledgeAutoBindAndOpen();

    expect(mockedLoadOpFileFromPath).toHaveBeenCalledTimes(1);
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/login.op');
    expect(useGitStore.getState().lastAutoBindedPath).toBeNull();
  });

  // ---- Phase 4c：提交输入切片 -------------------------------------

  it('setCommitMessage + clearCommitMessage round-trip the draft', () => {
    useGitStore.getState().setCommitMessage('first milestone');
    expect(useGitStore.getState().commitMessage).toBe('first milestone');
    useGitStore.getState().clearCommitMessage();
    expect(useGitStore.getState().commitMessage).toBe('');
  });

  it('cancelSaveRequired clears the flag without retrying', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setDirty(true);

    await expect(
      useGitStore.getState().commitMilestone('test', { name: 't', email: 't@e.com' }),
    ).rejects.toMatchObject({ name: 'GitError', code: 'save-required' });

    const before = useGitStore.getState().state;
    expect(before.kind).toBe('ready');
    if (before.kind === 'ready') {
      expect(before.saveRequiredFor).toBeDefined();
    }

    useGitStore.getState().cancelSaveRequired();

    const after = useGitStore.getState().state;
    expect(after.kind).toBe('ready');
    if (after.kind === 'ready') {
      expect(after.saveRequiredFor).toBeUndefined();
    }
  });

  // ---- Phase 4c：溢出菜单操作 ----------------------------------

  it('enterTrackedFilePicker flips ready → needs-tracked-file with the same repo', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');

    const before = useGitStore.getState().state;
    expect(before.kind).toBe('ready');

    useGitStore.getState().enterTrackedFilePicker();

    const after = useGitStore.getState().state;
    expect(after.kind).toBe('needs-tracked-file');
    if (after.kind === 'needs-tracked-file' && before.kind === 'ready') {
      expect(after.repo.repoId).toBe(before.repo.repoId);
    }
  });

  it('clearAuthorIdentity removes prefs keys and clears in-memory cache', async () => {
    const removePrefSpy = vi.fn(async () => {});
    vi.stubGlobal('window', {
      electronAPI: {
        getPreferences: vi.fn(async () => ({})),
        setPreference: vi.fn(async () => {}),
        removePreference: removePrefSpy,
        git: {},
      },
    });
    useGitStore.setState({
      authorIdentity: { name: 'Alice', email: 'alice@example.com' },
    });

    await useGitStore.getState().clearAuthorIdentity();

    // The 操作必须 REMOVE 键（不要将它们设置为空字符串），否则 resolveAuthorIdentity
    // 中的查找链将在磁盘上看到空白标记而不是缺失的键。
    expect(removePrefSpy).toHaveBeenCalledTimes(2);
    expect(removePrefSpy).toHaveBeenCalledWith('git.authorName');
    expect(removePrefSpy).toHaveBeenCalledWith('git.authorEmail');
    expect(useGitStore.getState().authorIdentity).toBeNull();
  });

  // ---- Phase 4c：自动保存订阅者 ------------------------------------

  it('initAutosaveSubscriber fires commitAutosave on saved event for tracked file', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.commit).mockResolvedValue({ hash: 'abc123' });
    await useGitStore.getState().initRepo('/tmp/login.op');

    useGitStore.getState().initAutosaveSubscriber();
    expect(useGitStore.getState().__autosaveUnsub).not.toBeNull();

    // Fire 跟踪文件的已保存事件。
// eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocumentEvents as any).emit('saved', {
      filePath: '/tmp/repo/login.op',
      fileName: 'login.op',
      document: {},
    });

    await new Promise((r) => setTimeout(r, 0));

    expect(gitClient.commit).toHaveBeenCalledTimes(1);
    expect(gitClient.commit).toHaveBeenCalledWith(
      'repo-1',
      expect.objectContaining({ kind: 'autosave' }),
    );
    expect(useGitStore.getState().autosaveError).toBeNull();

    useGitStore.getState().disposeAutosaveSubscriber();
  });

  it('initAutosaveSubscriber ignores saved event for a different file', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    useGitStore.getState().initAutosaveSubscriber();

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocumentEvents as any).emit('saved', {
      filePath: '/tmp/repo/other.op',
      fileName: 'other.op',
      document: {},
    });
    await new Promise((r) => setTimeout(r, 0));

    expect(gitClient.commit).not.toHaveBeenCalled();

    useGitStore.getState().disposeAutosaveSubscriber();
  });

  it('initAutosaveSubscriber ignores saved event when state is not ready', async () => {
    useGitStore.getState().initAutosaveSubscriber();

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocumentEvents as any).emit('saved', {
      filePath: '/tmp/repo/login.op',
      fileName: 'login.op',
      document: {},
    });
    await new Promise((r) => setTimeout(r, 0));

    expect(gitClient.commit).not.toHaveBeenCalled();

    useGitStore.getState().disposeAutosaveSubscriber();
  });

  it('initAutosaveSubscriber is idempotent when called multiple times', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.commit).mockResolvedValue({ hash: 'abc123' });
    await useGitStore.getState().initRepo('/tmp/login.op');

    // Call 两次 — 第二次调用必须是无操作。
    useGitStore.getState().initAutosaveSubscriber();
    const firstUnsub = useGitStore.getState().__autosaveUnsub;
    useGitStore.getState().initAutosaveSubscriber();
    const secondUnsub = useGitStore.getState().__autosaveUnsub;
    expect(secondUnsub).toBe(firstUnsub); // 精确引用相等

    // Fire 一个保存的事件 - 只有 ONE 提交应该触发，而不是两个。
// eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocumentEvents as any).emit('saved', {
      filePath: '/tmp/repo/login.op',
      fileName: 'login.op',
      document: {},
    });
    await new Promise((r) => setTimeout(r, 0));

    expect(gitClient.commit).toHaveBeenCalledTimes(1);

    useGitStore.getState().disposeAutosaveSubscriber();
  });

  it('autosave error is captured without throwing', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.commit).mockRejectedValue(
      new GitError('engine-crash', 'disk write failed'),
    );
    await useGitStore.getState().initRepo('/tmp/login.op');
    useGitStore.getState().initAutosaveSubscriber();

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocumentEvents as any).emit('saved', {
      filePath: '/tmp/repo/login.op',
      fileName: 'login.op',
      document: {},
    });
    await new Promise((r) => setTimeout(r, 0));

    expect(useGitStore.getState().autosaveError).toBe('disk write failed');

    useGitStore.getState().disposeAutosaveSubscriber();
  });

  it('restoreCommit reloads the tracked file into document-store after IPC', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.restore).mockResolvedValue(undefined);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    await useGitStore.getState().restoreCommit('abc123');

    // The 修复：在 gitClient.restore 解析后，存储必须将磁盘上的 .op
    // 文件重新加载到文档存储中，以便内存中的文档与恢复的树相匹配。 Otherwise 下一个 Cmd+s /
    // 自动保存会默默地用过时的内存内容覆盖恢复。
    expect(gitClient.restore).toHaveBeenCalledWith('repo-1', 'abc123');
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledTimes(1);
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/login.op');
  });

  it('promoteAutosave reloads the tracked file into document-store after IPC', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.promote).mockResolvedValue({ hash: 'new-milestone-hash' });
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    await useGitStore
      .getState()
      .promoteAutosave('autosave-hash', 'promote to milestone', { name: 't', email: 't@e.com' });

    // Same 推理为 restoreCommit：promote 在自动保存树中写入新的里程碑提交；无条件地重新加载文档，以便内存
    // 中的内容不会偏离磁盘。
    expect(gitClient.promote).toHaveBeenCalledWith(
      'repo-1',
      'autosave-hash',
      'promote to milestone',
      {
        name: 't',
        email: 't@e.com',
      },
    );
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledTimes(1);
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/login.op');
  });

  it('switchBranch refreshes the log and reloads the document after the IPC', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchSwitch).mockResolvedValue(undefined);
    vi.mocked(gitClient.status).mockResolvedValue(DEFAULT_STATUS);
    vi.mocked(gitClient.branchList).mockResolvedValue([]);
    vi.mocked(gitClient.log).mockResolvedValue([]);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();
    await useGitStore.getState().switchBranch('feature/x');

    // 分支开关移动 HEAD 并重写跟踪文件。 Both 内存中的文档和历史列表必须刷新 - state.kind 上的
    // GitPanelReady 日志效果键在切换期间会更改 NOT，因此存储是唯一可以执行此操作的地方。
    expect(gitClient.branchSwitch).toHaveBeenCalledWith('repo-1', 'feature/x');
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/login.op');
    expect(gitClient.log).toHaveBeenCalledWith('repo-1', { ref: 'main', limit: 50 });
  });

  it('mergeBranch (fast-forward / clean path) refreshes the log and reloads the document', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({ result: 'fast-forward' });
    vi.mocked(gitClient.status).mockResolvedValue(DEFAULT_STATUS);
    vi.mocked(gitClient.branchList).mockResolvedValue([]);
    vi.mocked(gitClient.log).mockResolvedValue([]);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();
    await useGitStore.getState().mergeBranch('feature/x');

    expect(gitClient.branchMerge).toHaveBeenCalledWith('repo-1', 'feature/x');
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/login.op');
    expect(gitClient.log).toHaveBeenCalledWith('repo-1', { ref: 'main', limit: 50 });
  });

  it('mergeBranch (conflict path) does NOT refresh log or reload document', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();
    await useGitStore.getState().mergeBranch('feature/x');

    // On 冲突 存储转换到冲突状态，并有意跳过 loadLog 和文档重新加载： - loadLog 是多余的，因为 GitPanelConflict
    // 在转换时首次安装并运行其自己的在 state.kind 上键入的 loadLog
    // 效果。 - Reloading 该文档将破坏引擎作为冲突包的一部分留在磁盘上的任何合并工件。
    expect(useGitStore.getState().state.kind).toBe('conflict');
    expect(mockedLoadOpFileFromPath).not.toHaveBeenCalled();
    expect(gitClient.log).not.toHaveBeenCalled();
  });

  it('mergeBranch (conflict-non-op path) calls refreshStatus and does NOT call loadOpFileFromPath', async () => {
    // I3：冲突非操作结果必须调用 refreshStatus() （这会促进就绪 → 冲突），而不是落到 syncAfterHeadMove
    // （这会重新加载 .op 文件和日志 — 在不完整合并期间语义错误）。
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({ result: 'conflict-non-op' });
    // refreshStatus 需要 status() 返回 mergeInProgress，这样它才能升级到冲突状态。
    // Provide 满足 mergeInProgress 分支的最小状态（unresolvedFiles 非空）。
    vi.mocked(gitClient.status)
      .mockResolvedValueOnce(DEFAULT_STATUS) // 初始化后
      .mockResolvedValueOnce({
        ...DEFAULT_STATUS,
        mergeInProgress: true,
        unresolvedFiles: ['README.md'],
      }); // refreshStatus 内部冲突非操作
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();
    await useGitStore.getState().mergeBranch('feature/x');

    // refreshStatus 被调用（状态 IPC 被第二次调用）。
    expect(gitClient.status).toHaveBeenCalledTimes(2);
    // Store 必须与列出的非操作文件处于冲突状态。
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.unresolvedFiles).toEqual(['README.md']);
    }
    // loadOpFileFromPath 必须已调用 NOT — HEAD 尚未移动。
    expect(mockedLoadOpFileFromPath).not.toHaveBeenCalled();
  });

  it('deleteBranch forwards the optional force flag to gitClient', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchDelete).mockResolvedValue(undefined);
    await useGitStore.getState().initRepo('/tmp/login.op');

    await useGitStore.getState().deleteBranch('feature-x', { force: true });
    expect(gitClient.branchDelete).toHaveBeenCalledWith('repo-1', 'feature-x', { force: true });

    await useGitStore.getState().deleteBranch('feature-y');
    expect(gitClient.branchDelete).toHaveBeenLastCalledWith('repo-1', 'feature-y', undefined);
  });

  it('switchBranch refreshes the log for the current branch instead of hardcoded main', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchSwitch).mockResolvedValue(undefined);
    // First status() 调用（初始化后）返回 main；第二个（切换后）返回 feature/x。
    vi.mocked(gitClient.status)
      .mockResolvedValueOnce(DEFAULT_STATUS)
      .mockResolvedValueOnce({ ...DEFAULT_STATUS, branch: 'feature/x' });
    vi.mocked(gitClient.branchList).mockResolvedValue([]);
    vi.mocked(gitClient.log).mockResolvedValue([]);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.log).mockClear();
    await useGitStore.getState().switchBranch('feature/x');

    // After 开关，refreshStatus 将 state.repo.currentBranch
    // 更新为“feature/x”。然后必须使用 ref: 'feature/x' 调用 loadLog，而不是之前硬编码的
    // 'main'，因此历史列表在转换后遵循实际的当前分支。
    expect(gitClient.log).toHaveBeenCalledWith('repo-1', { ref: 'feature/x', limit: 50 });
  });

  // ---- Phase 6a：克隆向导 + 远程合约 -------------------------

  it('enterCloneWizard transitions to wizard-clone with busy=false and no inline error', () => {
    useGitStore.getState().enterCloneWizard();
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('wizard-clone');
    if (s.kind === 'wizard-clone') {
      expect(s.error).toBeNull();
      expect(s.busy).toBe(false);
    }
  });

  it('cancelCloneWizard always transitions to no-file', () => {
    useGitStore.getState().enterCloneWizard();
    expect(useGitStore.getState().state.kind).toBe('wizard-clone');
    useGitStore.getState().cancelCloneWizard();
    expect(useGitStore.getState().state).toEqual({ kind: 'no-file' });
  });

  it('cloneRepo with a recoverable error keeps the wizard mounted with state.error set', async () => {
    // Enter 向导将触发 cloneRepo 的 prevWasWizard 分支。
    useGitStore.getState().enterCloneWizard();

    vi.mocked(gitClient.clone).mockRejectedValue(
      new GitError('auth-failed', 'bad credentials', { recoverable: true }),
    );

    await useGitStore.getState().cloneRepo({
      url: 'https://github.com/foo/bar.git',
      dest: '/tmp/clone',
    });

    // Critical：我们停留在向导克隆中（没有 `initializing` 往返），因此表单组件可以生存，并且其
    // URL/dest/token 输入保留其值。 busy 翻转回 false，因此 Submit 按钮重新启用。
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('wizard-clone');
    if (s.kind === 'wizard-clone') {
      expect(s.busy).toBe(false);
      expect(s.error).not.toBeNull();
      expect(s.error?.code).toBe('auth-failed');
      expect(s.error?.message).toBe('bad credentials');
    }
  });

  it('cloneRepo launched from wizard never transitions through initializing', async () => {
    // Before 修复，cloneRepo 在 IPC 的持续时间内设置 state.kind =
    // 'initializing'，从而卸载 GitPanelCloneForm 并擦除用户的表单输入。 We 现在停留在
    // `wizard-clone` 且 busy=true。
    useGitStore.getState().enterCloneWizard();

    // Resolve 具有单个候选者，以便克隆成功并干净地离开向导（就绪状态）。
    vi.mocked(gitClient.clone).mockImplementationOnce(async () => {
      // Snapshot 状态 mid-IPC：它仍必须是使用 busy=true 的向导克隆。
      const mid = useGitStore.getState().state;
      expect(mid.kind).toBe('wizard-clone');
      if (mid.kind === 'wizard-clone') {
        expect(mid.busy).toBe(true);
        expect(mid.error).toBeNull();
      }
      return {
        ...SAMPLE_REPO,
        mode: 'folder',
        trackedFilePath: null,
        candidates: [
          {
            path: '/tmp/cloned/main.op',
            relativePath: 'main.op',
            milestoneCount: 0,
            autosaveCount: 0,
            lastCommitAt: null,
            lastCommitMessage: null,
          },
        ],
      };
    });
    vi.mocked(gitClient.bindTrackedFile).mockResolvedValue({
      trackedFilePath: '/tmp/cloned/main.op',
    });

    await useGitStore.getState().cloneRepo({
      url: 'https://example.com/repo.git',
      dest: '/tmp/cloned',
    });

    // On 成功后，我们完全退出向导 — 单候选自动绑定。
    expect(useGitStore.getState().state.kind).toBe('ready');
  });

  it('cloneRepo with a non-recoverable error transitions to the generic error state', async () => {
    useGitStore.getState().enterCloneWizard();

    vi.mocked(gitClient.clone).mockRejectedValue(
      new GitError('engine-crash', 'disk full', { recoverable: false }),
    );

    await expect(
      useGitStore.getState().cloneRepo({
        url: 'https://github.com/foo/bar.git',
        dest: '/tmp/clone',
      }),
    ).rejects.toBeInstanceOf(GitError);

    const s = useGitStore.getState().state;
    expect(s.kind).toBe('error');
    if (s.kind === 'error') {
      expect(s.message).toBe('disk full');
      expect(s.recoverable).toBe(false);
    }
  });

  it('refreshRemote pulls origin metadata into state.repo.remote', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');

    // initRepo 已经使用默认的空存根调用了 refreshRemote() 一次。 Override 并再次调用以验证往返行程。
    vi.mocked(gitClient.remoteGet).mockResolvedValue({
      name: 'origin',
      url: 'https://github.com/foo/bar.git',
      host: 'github.com',
    });
    await useGitStore.getState().refreshRemote();

    const s = useGitStore.getState().state;
    expect(s.kind).toBe('ready');
    if (s.kind === 'ready') {
      expect(s.repo.remote).toEqual({
        name: 'origin',
        url: 'https://github.com/foo/bar.git',
        host: 'github.com',
      });
    }
    expect(gitClient.remoteGet).toHaveBeenCalledWith('repo-1');
  });

  it('setRemoteUrl updates state.repo.remote immediately from the IPC return value', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.remoteSet).mockResolvedValue({
      name: 'origin',
      url: 'https://github.com/new/repo.git',
      host: 'github.com',
    });

    await useGitStore.getState().setRemoteUrl('https://github.com/new/repo.git');

    // Renderer 状态必须反映后续 refreshRemote() 调用的新 url WITHOUT。 Per
    // Phase 6a 合约，IPC 返回值是立即更新的真实来源。
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('ready');
    if (s.kind === 'ready') {
      expect(s.repo.remote).toEqual({
        name: 'origin',
        url: 'https://github.com/new/repo.git',
        host: 'github.com',
      });
    }
    expect(gitClient.remoteSet).toHaveBeenCalledWith('repo-1', 'https://github.com/new/repo.git');
  });

  it('setRemoteUrl normalizes whitespace-only input to null before sending to IPC', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.remoteSet).mockResolvedValue({
      name: 'origin',
      url: null,
      host: null,
    });

    await useGitStore.getState().setRemoteUrl('   ');
    expect(gitClient.remoteSet).toHaveBeenLastCalledWith('repo-1', null);

    // null 也不变地通过。
    await useGitStore.getState().setRemoteUrl(null);
    expect(gitClient.remoteSet).toHaveBeenLastCalledWith('repo-1', null);
  });

  // ---- Phase 6b：拉/推 + syncAfterHeadMove ------------------------

  it('pull (fast-forward) refreshes status/branches, reloads the tracked file, and refreshes the log', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.pull).mockResolvedValue({ result: 'fast-forward' });
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.status).mockClear();
    vi.mocked(gitClient.branchList).mockClear();
    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();

    await useGitStore.getState().pull();

    // syncAfterHeadMove 触发所有四个级联：状态、分支、跟踪文件重新加载和活动分支的日志刷新。 Without
    // 这次成功的拉动将使画布和历史列表变得陈旧。
    expect(gitClient.pull).toHaveBeenCalledWith('repo-1', undefined);
    expect(gitClient.status).toHaveBeenCalledTimes(1);
    expect(gitClient.branchList).toHaveBeenCalledTimes(1);
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/login.op');
    expect(gitClient.log).toHaveBeenCalledWith('repo-1', { ref: 'main', limit: 50 });
  });

  it('pull (merge) runs the same head-move cascade as fast-forward', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.pull).mockResolvedValue({ result: 'merge' });
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();
    await useGitStore.getState().pull();

    expect(mockedLoadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/login.op');
    expect(gitClient.log).toHaveBeenCalledWith('repo-1', { ref: 'main', limit: 50 });
  });

  it('pull (conflict) transitions into conflict state without reloading the document', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.pull).mockResolvedValue({
      result: 'conflict',
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:rect-1',
            pageId: null,
            nodeId: 'rect-1',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    await useGitStore.getState().pull();

    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.conflicts.nodeConflicts.size).toBe(1);
      expect(s.unresolvedFiles).toEqual([]);
    }
    expect(mockedLoadOpFileFromPath).not.toHaveBeenCalled();
  });

  it('pull (conflict-non-op) threads unresolvedFiles into conflict state without refreshing the document', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.pull).mockResolvedValue({ result: 'conflict-non-op' });
    await useGitStore.getState().initRepo('/tmp/login.op');

    // After 冲突非操作结果，pull() 委托给 refreshStatus()，它调用 gitClient.status() 一次。下面的
    // The 模拟报告与未解析的文件列表的进行中合并 AND 更新的存储库元（分支/前面/后面/工作脏） -
    // refreshStatus 镜像完整的状态负载，而不仅仅是冲突字段。
    vi.mocked(gitClient.status).mockClear();
    vi.mocked(gitClient.status).mockResolvedValueOnce({
      ...DEFAULT_STATUS,
      branch: 'feature/merge-target',
      ahead: 1,
      behind: 3,
      workingDirty: true,
      mergeInProgress: true,
      unresolvedFiles: ['src/README.md', 'src/package.json'],
      conflicts: null,
    });

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    await useGitStore.getState().pull();

    // pull 必须将状态重建委托给 refreshStatus — 即 gitClient.status 被查询了一次。
    expect(gitClient.status).toHaveBeenCalledTimes(1);

    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      // Empty 节点包 — 这是纯粹的非操作冲突。
      expect(s.conflicts.nodeConflicts.size).toBe(0);
      expect(s.conflicts.docFieldConflicts.size).toBe(0);
      expect(s.unresolvedFiles).toEqual(['src/README.md', 'src/package.json']);
      // Repo-meta 字段必须反映状态有效负载 - 前缀路径跳过了此更新，并且 branch/ahead/behind
      // 保持陈旧状态。
      expect(s.repo.currentBranch).toBe('feature/merge-target');
      expect(s.repo.ahead).toBe(1);
      expect(s.repo.behind).toBe(3);
      expect(s.repo.workingDirty).toBe(true);
    }
    // Non-op 冲突必须 NOT 清除内存中的文档，因为磁盘上的 .op 文件仍然是用户的预合并树。
    expect(mockedLoadOpFileFromPath).not.toHaveBeenCalled();
  });

  it('pull surfaces auth-required as a GitError the button can catch (no error state transition)', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.pull).mockRejectedValue(
      new GitError('auth-required', 'HTTP 401', { recoverable: true }),
    );

    await expect(useGitStore.getState().pull()).rejects.toMatchObject({
      name: 'GitError',
      code: 'auth-required',
    });

    // Renderer 保持就绪状态 — 该按钮拥有 auth-form 重试循环，并且不得与通用错误卡竞争。
    expect(useGitStore.getState().state.kind).toBe('ready');
  });

  it('push success refreshes status without firing the head-move cascade', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.push).mockResolvedValue({ result: 'ok' });
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.status).mockClear();
    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();

    await useGitStore.getState().push();

    expect(gitClient.push).toHaveBeenCalledWith('repo-1', undefined);
    expect(gitClient.status).toHaveBeenCalledTimes(1);
    // Push 不会移动我们这边的 HEAD → 没有文档重新加载，没有日志刷新。 Only status() 需要重新运行，以便
    // ahead/behind 归零。
    expect(mockedLoadOpFileFromPath).not.toHaveBeenCalled();
    expect(gitClient.log).not.toHaveBeenCalled();
  });

  it('push surfaces push-rejected as a GitError the button can catch', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.push).mockRejectedValue(
      new GitError('push-rejected', 'non-fast-forward', { recoverable: true }),
    );

    await expect(useGitStore.getState().push()).rejects.toMatchObject({
      name: 'GitError',
      code: 'push-rejected',
    });

    // Stays 已准备就绪，因此按钮可以呈现其“先拉”内嵌条。
    expect(useGitStore.getState().state.kind).toBe('ready');
  });

  it('push surfaces auth-failed as a GitError the button can catch', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.push).mockRejectedValue(
      new GitError('auth-failed', 'HTTP 403', { recoverable: true }),
    );

    await expect(useGitStore.getState().push()).rejects.toMatchObject({
      name: 'GitError',
      code: 'auth-failed',
    });

    expect(useGitStore.getState().state.kind).toBe('ready');
  });

  // ---- Phase 7b：finalizeError、exitTrackedFilePicker、调节器 -------

  it('conflict state includes finalizeError: null by default when entering via mergeBranch', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBeNull();
    }
  });

  it('applyMerge sets finalizeError when backend throws merge-still-conflicted', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    vi.mocked(gitClient.applyMerge).mockRejectedValue(
      new GitError('merge-still-conflicted', 'some conflicts remain unresolved'),
    );
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');
    expect(useGitStore.getState().state.kind).toBe('conflict');

    // Phase 7c：applyMerge 在合并仍然冲突后调用 refreshStatus()，因此未解析的文件列表是最新的。 Mock
    // 状态返回 mergeInProgress: true，因此 refreshStatus 调用不会将状态降级为就绪。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });

    // 具有 merge-still-conflicted 的 applyMerge 必须将 NOT 抛出给调用者 — 它会在横幅上内联显示错误。
    await expect(useGitStore.getState().applyMerge()).resolves.toBeUndefined();

    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBe('some conflicts remain unresolved');
    }
  });

  it('applyMerge clears finalizeError and transitions to ready on success', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    vi.mocked(gitClient.applyMerge).mockResolvedValue({ hash: 'merge-hash', noop: false });
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');

    await useGitStore.getState().applyMerge();

    expect(useGitStore.getState().state.kind).toBe('ready');
  });

  it('applyMerge rethrows non-merge-still-conflicted errors (e.g. engine-crash)', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    vi.mocked(gitClient.applyMerge).mockRejectedValue(
      new GitError('engine-crash', 'disk full', { recoverable: false }),
    );
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');

    await expect(useGitStore.getState().applyMerge()).rejects.toMatchObject({
      name: 'GitError',
      code: 'engine-crash',
    });
  });

  it('resolveConflict clears finalizeError when the user resolves a conflict', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:rect-1',
            pageId: null,
            nodeId: 'rect-1',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    vi.mocked(gitClient.applyMerge).mockRejectedValue(
      new GitError('merge-still-conflicted', 'still conflicted'),
    );
    vi.mocked(gitClient.resolveConflict).mockResolvedValue(undefined);
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');
    // Phase 7c：applyMerge 在合并后仍然冲突，调用 refreshStatus()。 Mock 状态以保持合并正在进行，因此
    // refreshStatus 不会降级为就绪状态。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:rect-1',
            pageId: null,
            nodeId: 'rect-1',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    // Set finalizeError 通过 applyMerge
    await useGitStore.getState().applyMerge();
    let s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).not.toBeNull();
    }

    // Resolving 冲突应清除 finalizeError
    await useGitStore.getState().resolveConflict('node:_:rect-1', { kind: 'ours' });
    s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBeNull();
    }
  });

  it('refreshStatus promotes ready → conflict with finalizeError: null', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:rect-1',
            pageId: null,
            nodeId: 'rect-1',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    await useGitStore.getState().refreshStatus();
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBeNull();
    }
  });

  it('refreshStatus mergeInProgress=true with unresolvedFiles but conflicts=null → conflict with empty maps', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: ['README.md'],
      conflicts: null,
    });
    await useGitStore.getState().refreshStatus();
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.conflicts.nodeConflicts.size).toBe(0);
      expect(s.conflicts.docFieldConflicts.size).toBe(0);
      expect(s.unresolvedFiles).toEqual(['README.md']);
      expect(s.finalizeError).toBeNull();
    }
  });

  it('exitTrackedFilePicker from rebind (trackedFilePath non-null) returns to ready', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    // Enter 就绪选择器（重新绑定场景）
    useGitStore.getState().enterTrackedFilePicker();
    expect(useGitStore.getState().state.kind).toBe('needs-tracked-file');
    const s = useGitStore.getState().state;
    // trackedFilePath 已设置（来自 SAMPLE_REPO.trackedFilePath）
    if (s.kind === 'needs-tracked-file') {
      expect(s.repo.trackedFilePath).toBe('/tmp/repo/login.op');
    }
    await useGitStore.getState().exitTrackedFilePicker();
    expect(useGitStore.getState().state.kind).toBe('ready');
  });

  it('exitTrackedFilePicker from first-open (trackedFilePath null) closes repo and returns to no-file', async () => {
    vi.mocked(gitClient.open).mockResolvedValue({
      ...SAMPLE_REPO,
      mode: 'folder',
      trackedFilePath: null,
      candidates: [
        { ...SAMPLE_REPO.candidates[0], relativePath: 'a.op', path: '/tmp/repo/a.op' },
        { ...SAMPLE_REPO.candidates[0], relativePath: 'b.op', path: '/tmp/repo/b.op' },
      ],
    });
    vi.mocked(gitClient.close).mockResolvedValue(undefined);
    await useGitStore.getState().openRepo('/tmp/repo');
    expect(useGitStore.getState().state.kind).toBe('needs-tracked-file');
    const s = useGitStore.getState().state;
    if (s.kind === 'needs-tracked-file') {
      expect(s.repo.trackedFilePath).toBeNull();
    }
    await useGitStore.getState().exitTrackedFilePicker();
    // Should 已调用 close 并返回无文件
    expect(gitClient.close).toHaveBeenCalledWith('repo-1');
    expect(useGitStore.getState().state.kind).toBe('no-file');
  });

  it('exitTrackedFilePicker swallows close errors and still resets to no-file', async () => {
    vi.mocked(gitClient.open).mockResolvedValue({
      ...SAMPLE_REPO,
      mode: 'folder',
      trackedFilePath: null,
      candidates: [
        { ...SAMPLE_REPO.candidates[0], relativePath: 'a.op', path: '/tmp/repo/a.op' },
        { ...SAMPLE_REPO.candidates[0], relativePath: 'b.op', path: '/tmp/repo/b.op' },
      ],
    });
    vi.mocked(gitClient.close).mockRejectedValue(new Error('session gone'));
    await useGitStore.getState().openRepo('/tmp/repo');
    expect(useGitStore.getState().state.kind).toBe('needs-tracked-file');
    await expect(useGitStore.getState().exitTrackedFilePicker()).resolves.toBeUndefined();
    expect(useGitStore.getState().state.kind).toBe('no-file');
  });

  // ---- C1：refreshStatus 不得擦除内存中冲突解决方案 ----

  it('refreshStatus preserves in-memory resolutions when already in conflict', async () => {
    // Set up 冲突状态，通过 mergeBranch 与一个节点发生冲突。
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:nodeA',
            pageId: null,
            nodeId: 'nodeA',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
          {
            id: 'node:_:nodeB',
            pageId: null,
            nodeId: 'nodeB',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    vi.mocked(gitClient.resolveConflict).mockResolvedValue(undefined);
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');
    expect(useGitStore.getState().state.kind).toBe('conflict');

    // User 解析 nodeA。
    await useGitStore.getState().resolveConflict('node:_:nodeA', { kind: 'ours' });
    let s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.conflicts.nodeConflicts.get('node:_:nodeA')?.resolution).toEqual({ kind: 'ours' });
    }

    // Polling 触发：后端仍然报告同一包正在进行合并。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:nodeA',
            pageId: null,
            nodeId: 'nodeA',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
          {
            id: 'node:_:nodeB',
            pageId: null,
            nodeId: 'nodeB',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    await useGitStore.getState().refreshStatus();

    // CRITICAL：关于 nodeA 的决议在轮询后必须仍然存在。
    s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.conflicts.nodeConflicts.get('node:_:nodeA')?.resolution).toEqual({ kind: 'ours' });
      // nodeB 仍未解决。
      expect(s.conflicts.nodeConflicts.get('node:_:nodeB')?.resolution).toBeUndefined();
    }
  });

  it('refreshStatus preserves finalizeError when already in conflict', async () => {
    // Set 通过 mergeBranch 启动冲突状态。
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    vi.mocked(gitClient.applyMerge).mockRejectedValue(
      new GitError('merge-still-conflicted', 'some conflicts remain unresolved'),
    );
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');
    // Phase 7c：设置状态以在 applyMerge 之前返回 mergeInProgress:true，以便合并仍然冲突处理程序内的
    // refreshStatus() 调用不会将状态降级为就绪。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    // Trigger 通过尝试 applyMerge 与未解决的冲突来实现 finalizeError。
    await useGitStore.getState().applyMerge();
    let s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBe('some conflicts remain unresolved');
    }

    // Polling 触发：后端仍然报告合并正在进行中。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    await useGitStore.getState().refreshStatus();

    // CRITICAL：在轮询后仍必须设置 finalizeError。
    s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBe('some conflicts remain unresolved');
    }
  });

  it('refreshStatus updates unresolvedFiles mid-session without wiping resolutions', async () => {
    // Set up 冲突状态，有一个节点冲突和两个未解决的文件。
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:nodeA',
            pageId: null,
            nodeId: 'nodeA',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    vi.mocked(gitClient.resolveConflict).mockResolvedValue(undefined);
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');

    // Seed unresolvedFiles 手动 - 分支合并不会设置它们。
    useGitStore.setState((s) => {
      if (s.state.kind !== 'conflict') return s;
      return { state: { ...s.state, unresolvedFiles: ['README.md', 'package.json'] } };
    });

    // User 解析 nodeA。
    await useGitStore.getState().resolveConflict('node:_:nodeA', { kind: 'theirs' });

    // Backend 现在仅报告一个未解析的文件（用户在外部解析了 README）。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: ['package.json'],
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:nodeA',
            pageId: null,
            nodeId: 'nodeA',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    await useGitStore.getState().refreshStatus();

    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      // unresolvedFiles 必须反映后端更新。
      expect(s.unresolvedFiles).toEqual(['package.json']);
      // 必须保留 The nodeA 分辨率。
      expect(s.conflicts.nodeConflicts.get('node:_:nodeA')?.resolution).toEqual({ kind: 'theirs' });
    }
  });

  // ---- Phase 7c：applyMerge 重新加载 + noop + 合并仍然冲突的 -------

  it('applyMerge (success) reloads the tracked file and refreshes the log', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    vi.mocked(gitClient.applyMerge).mockResolvedValue({ hash: 'merge-commit-hash', noop: false });
    vi.mocked(gitClient.status).mockResolvedValue(DEFAULT_STATUS);
    vi.mocked(gitClient.log).mockResolvedValue([]);
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');
    expect(useGitStore.getState().state.kind).toBe('conflict');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();

    await useGitStore.getState().applyMerge();

    // Phase 7c：成功必须转换为就绪 AND 重新加载跟踪文件并刷新日志，以便画布反映合并结果，历史列表显示新的合并提交。
    expect(useGitStore.getState().state.kind).toBe('ready');
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/login.op');
    expect(gitClient.log).toHaveBeenCalledTimes(1);
  });

  it('applyMerge (noop: true) transitions to ready and reloads tracked file', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    // noop: true 意味着后端没有什么可写的（所有冲突都是微不足道的）
    vi.mocked(gitClient.applyMerge).mockResolvedValue({ hash: '', noop: true });
    vi.mocked(gitClient.status).mockResolvedValue(DEFAULT_STATUS);
    vi.mocked(gitClient.log).mockResolvedValue([]);
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();

    await useGitStore.getState().applyMerge();

    // Even noop 结果必须转换为就绪状态并运行重新加载级联。
    expect(useGitStore.getState().state.kind).toBe('ready');
    expect(mockedLoadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/login.op');
    expect(gitClient.log).toHaveBeenCalledTimes(1);
  });

  it('applyMerge (merge-still-conflicted) stays in conflict and calls refreshStatus', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({
      result: 'conflict',
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:rect-1',
            pageId: null,
            nodeId: 'rect-1',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    vi.mocked(gitClient.applyMerge).mockRejectedValue(
      new GitError('merge-still-conflicted', '1 conflict remains'),
    );
    // refreshStatus 在 merge-still-conflicted 之后调用，以保持未解析的文件列表为最新。
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: {
        nodeConflicts: [
          {
            id: 'node:_:rect-1',
            pageId: null,
            nodeId: 'rect-1',
            reason: 'both-modified-same-field',
            base: null,
            ours: null,
            theirs: null,
          },
        ],
        docFieldConflicts: [],
      },
    });
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');
    expect(useGitStore.getState().state.kind).toBe('conflict');

    vi.mocked(gitClient.status).mockClear();
    vi.mocked(mockedLoadOpFileFromPath).mockClear();

    // Must 不抛出 — 横幅拥有错误显示。
    await expect(useGitStore.getState().applyMerge()).resolves.toBeUndefined();

    const s = useGitStore.getState().state;
    // Still 与记录的错误冲突。
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBe('1 conflict remains');
    }
    // Phase 7c：必须调用 refreshStatus 来更新未解析的文件列表。
    expect(gitClient.status).toHaveBeenCalledTimes(1);
    // Document 必须已重新加载 NOT — 合并未完成。
    expect(mockedLoadOpFileFromPath).not.toHaveBeenCalled();
  });
});
