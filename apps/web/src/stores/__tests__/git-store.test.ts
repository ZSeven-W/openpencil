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
vi.mock('@/stores/document-store', () => {
  let dirty = false;
  return {
    useDocumentStore: {
      getState: () => ({
        isDirty: dirty,
        save: vi.fn(async () => 'saved-path.op'),
      }),
      // Test helper:
      __setDirty: (next: boolean) => {
        dirty = next;
      },
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
});
