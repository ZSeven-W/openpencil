// apps/web/src/stores/__tests__/git-store.test.ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { GitError } from '@/services/git-error';

// Mock the git-client before importing the store so the store picks up
// the mock at module evaluation time. All 31 IPC methods are stubbed with
// vi.fn() defaults so unstubbed paths fail loudly (as a no-op returning
// undefined) rather than crashing with "is not a function".
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
    },
    isGitApiAvailable: vi.fn(() => true),
  };
});

// Mock document-store so withCleanWorkingTree can read isDirty without
// pulling in the full document implementation.
//
// `save` is hoisted to a stable spy so tests can (a) assert it was called
// and (b) override its return value via __setSaveResult. Without this
// hoist, every getState() call would build a fresh vi.fn() and the spy
// would disappear before any assertion could see it.
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
      // Test helper:
      __setDirty: (next: boolean) => {
        dirty = next;
      },
      // Test helper: override save()'s return value. The store's
      // retrySaveRequired action treats null as "save failed" and bails
      // without clearing saveRequiredFor.
      __setSaveResult: (result: string | null) => {
        saveResult = result;
      },
      // Test helper: stable spy so tests can assert call counts.
      __saveSpy: saveSpy,
    },
  };
});

// Now import the store (it'll pick up the mocks above).
import { useGitStore, __resetGitStore } from '@/stores/git-store';
import { gitClient } from '@/services/git-client';
// eslint-disable-next-line @typescript-eslint/no-explicit-any
import { useDocumentStore as mockedDocStore } from '@/stores/document-store';

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

// Default GitStatusInfo for the refresh-after-init/open/clone/bind paths.
// Individual tests can override via vi.mocked(gitClient.status).mockResolvedValue.
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
    (mockedDocStore as any).__setDirty(false);
    // Reset the hoisted save result so a previous test's __setSaveResult(null)
    // doesn't bleed into this one. vi.clearAllMocks() doesn't touch closure
    // state, so we have to reset the variable explicitly.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setSaveResult('saved-path.op');
    // Set safe default resolved values for the refresh path. Without these,
    // initRepo/openRepo/cloneRepo/bindTrackedFile would crash because they
    // now invoke status() and branchList() automatically.
    vi.mocked(gitClient.status).mockResolvedValue(DEFAULT_STATUS);
    vi.mocked(gitClient.branchList).mockResolvedValue([]);
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
    // Note: vitest runs the promise synchronously up to the first await, so
    // we can't easily observe the 'initializing' intermediate state without
    // splitting the resolve. Just assert the final state.
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
    // The client's commit method should NOT have been called.
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

    // Now simulate the backend reporting an in-flight merge with one conflict.
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

  it('refreshStatus demotes conflict → ready when backend says merge is no longer in flight', async () => {
    // Set up a conflict state via mergeBranch.
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

    // Backend now reports the merge was finalized externally (e.g. terminal git).
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: false,
      conflicts: null,
    });
    await useGitStore.getState().refreshStatus();
    expect(useGitStore.getState().state.kind).toBe('ready');
  });

  it('retrySaveRequired clears saveRequiredFor and re-runs the queued action after save succeeds', async () => {
    // Set up a ready state with a queued commit waiting on a dirty doc.
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.commit).mockResolvedValue({ hash: 'abc123' });
    await useGitStore.getState().initRepo('/tmp/login.op');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setDirty(true);

    // First call traps in saveRequiredFor.
    await expect(
      useGitStore.getState().commitMilestone('first', { name: 't', email: 't@e.com' }),
    ).rejects.toMatchObject({ name: 'GitError', code: 'save-required' });

    // The user clicks save in the panel. Simulate the dirty flag flipping
    // back to false (as the real document-store would after a save).
    // saveResult is already 'saved-path.op' from the beforeEach reset.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setDirty(false);

    await useGitStore.getState().retrySaveRequired();

    // The save spy was called exactly once, the original commit IPC was
    // invoked exactly once with the queued args, and saveRequiredFor is
    // now cleared.
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
    // Set up a ready state with a queued commit waiting on a dirty doc.
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setDirty(true);

    await expect(
      useGitStore.getState().commitMilestone('first', { name: 't', email: 't@e.com' }),
    ).rejects.toMatchObject({ name: 'GitError', code: 'save-required' });

    // Simulate save() failing (returning null). Do NOT clear isDirty —
    // the doc is still dirty after a failed save in real life.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedDocStore as any).__setSaveResult(null);

    await useGitStore.getState().retrySaveRequired();

    // The save spy was called once, but the commit IPC was NOT called and
    // saveRequiredFor is still set so the user can retry.
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

    // Backend close throws — e.g., the session was already cleaned up.
    vi.mocked(gitClient.close).mockRejectedValue(new Error('session not found'));

    // closeRepo must not throw — it swallows and resets state regardless.
    await expect(useGitStore.getState().closeRepo()).resolves.toBeUndefined();

    const s = useGitStore.getState();
    expect(s.state).toEqual({ kind: 'no-file' });
    expect(s.log).toEqual([]);
    // The close IPC was attempted exactly once.
    expect(gitClient.close).toHaveBeenCalledTimes(1);
    expect(gitClient.close).toHaveBeenCalledWith('repo-1');
  });

  it('refreshStatus Step 1 copies basic status fields onto the active repo', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    await useGitStore.getState().initRepo('/tmp/login.op');
    expect(useGitStore.getState().state.kind).toBe('ready');

    // Override status to return new values for every Step 1 field.
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
    // Initial state is no-file (set by __resetGitStore in beforeEach). Any
    // action that calls requireRepoId without first transitioning to a
    // repo-bearing state must reject with GitError('no-file').
    expect(useGitStore.getState().state.kind).toBe('no-file');

    await expect(useGitStore.getState().refreshStatus()).rejects.toMatchObject({
      name: 'GitError',
      code: 'no-file',
    });
  });
});
