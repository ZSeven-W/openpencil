// apps/web/src/stores/__tests__/git-store.test.ts
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
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
      getSystemAuthor: vi.fn(),
      remoteGet: vi.fn(),
      remoteSet: vi.fn(),
    },
    isGitApiAvailable: vi.fn(() => true),
  };
});

// Mock the load helper so acknowledgeAutoBindAndOpen can be asserted on
// without actually wiring the file IPC + document store flow.
vi.mock('@/utils/load-op-file', () => ({
  loadOpFileFromPath: vi.fn(async () => true),
}));

// Mock documentEvents so the autosave subscriber tests can fire 'saved'
// events deterministically without wiring the real emitter.
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
        // Snapshot before iterating so a handler that unsubscribes itself
        // mid-iteration does not cause siblings to be skipped (same pattern
        // as the real DocumentEventEmitter, which iterates over a Set).
        const snapshot = Array.from(handlers);
        for (const h of snapshot) h(payload);
      },
      __clear: () => {
        handlers.length = 0;
      },
    },
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
    (mockedDocumentEvents as any).__clear?.();
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
    vi.mocked(gitClient.log).mockResolvedValue([]);
    // Phase 6a: refreshRemote() is invoked from init/open/clone too. Default
    // to a "no remote configured" stub so existing test expectations still
    // hold; individual tests override this.
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
    // Phase 4a: window.electronAPI mock for author identity prefs lookup
    vi.stubGlobal('window', {
      electronAPI: {
        getPreferences: vi.fn(async () => ({})),
        setPreference: vi.fn(async () => {}),
        git: {}, // truthy so step 2 of loadAuthorIdentity proceeds
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
      // Use multiple candidates so openRepo's Phase 4b auto-bind branch
      // does NOT fire — we want this test to exercise the manual
      // bindTrackedFile flow from the needs-tracked-file state.
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

  it('refreshStatus promotes ready → conflict with reopenedMidMerge=true when backend signals degraded panel-reopen state', async () => {
    // I2: when engineStatus returns mergeInProgress=true + reopenedMidMerge=true
    // with no conflicts bag and empty unresolvedFiles (tracked .op filtered out),
    // refreshStatus must still promote to conflict state and pass the flag through.
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

  // ---- Phase 4a: author identity slice ----------------------------------

  it('loadAuthorIdentity hits prefs first when both keys are set', async () => {
    // Stub window.electronAPI.getPreferences to return both git keys.
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
    // The sysGit fallback must NOT have been called when prefs hit.
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

  // ---- Phase 4b: auto-bind banner ---------------------------------------

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
    // Manually seed the flag (no need to go through openRepo here).
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

  // ---- Phase 4c: commit input slice -------------------------------------

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

  // ---- Phase 4c: overflow menu actions ----------------------------------

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

    // The action must REMOVE the keys (not set them to an empty string),
    // otherwise the lookup chain in resolveAuthorIdentity will see blank
    // sentinels on disk instead of absent keys.
    expect(removePrefSpy).toHaveBeenCalledTimes(2);
    expect(removePrefSpy).toHaveBeenCalledWith('git.authorName');
    expect(removePrefSpy).toHaveBeenCalledWith('git.authorEmail');
    expect(useGitStore.getState().authorIdentity).toBeNull();
  });

  // ---- Phase 4c: autosave subscriber ------------------------------------

  it('initAutosaveSubscriber fires commitAutosave on saved event for tracked file', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.commit).mockResolvedValue({ hash: 'abc123' });
    await useGitStore.getState().initRepo('/tmp/login.op');

    useGitStore.getState().initAutosaveSubscriber();
    expect(useGitStore.getState().__autosaveUnsub).not.toBeNull();

    // Fire a saved event for the tracked file.
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

    // Call twice — second call must be a no-op.
    useGitStore.getState().initAutosaveSubscriber();
    const firstUnsub = useGitStore.getState().__autosaveUnsub;
    useGitStore.getState().initAutosaveSubscriber();
    const secondUnsub = useGitStore.getState().__autosaveUnsub;
    expect(secondUnsub).toBe(firstUnsub); // exact reference equality

    // Fire a single saved event — only ONE commit should fire, not two.
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

    // The fix: after gitClient.restore resolves, the store must reload the
    // on-disk .op file into document-store so the in-memory document
    // matches the restored tree. Otherwise the next Cmd+S / autosave
    // silently overwrites the restore with the stale in-memory content.
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

    // Same reasoning as restoreCommit: promote writes a new milestone
    // commit at the autosave's tree; reload the document unconditionally
    // so the in-memory content cannot diverge from disk.
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

    // A branch switch moves HEAD and rewrites the tracked file. Both the
    // in-memory document and the history list must be refreshed — the
    // GitPanelReady log effect keys on state.kind which does NOT change
    // during switch, so the store is the only place that can do this.
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

    // On conflict the store transitions to the conflict state and intentionally
    // skips both loadLog and the document reload:
    //   - loadLog is redundant because GitPanelConflict mounts for the first
    //     time on the transition and runs its own loadLog effect keyed on
    //     state.kind.
    //   - Reloading the document would clobber any merge artifacts the engine
    //     left on disk as part of the conflict bag.
    expect(useGitStore.getState().state.kind).toBe('conflict');
    expect(mockedLoadOpFileFromPath).not.toHaveBeenCalled();
    expect(gitClient.log).not.toHaveBeenCalled();
  });

  it('mergeBranch (conflict-non-op path) calls refreshStatus and does NOT call loadOpFileFromPath', async () => {
    // I3: conflict-non-op result must call refreshStatus() (which promotes
    // ready → conflict) instead of falling through to syncAfterHeadMove
    // (which would reload the .op file and log — semantically wrong during
    // an incomplete merge).
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.branchMerge).mockResolvedValue({ result: 'conflict-non-op' });
    // refreshStatus needs status() to return mergeInProgress so it can
    // promote to conflict state. Provide a minimal status that satisfies
    // the mergeInProgress branch (unresolvedFiles non-empty).
    vi.mocked(gitClient.status)
      .mockResolvedValueOnce(DEFAULT_STATUS) // post-init
      .mockResolvedValueOnce({
        ...DEFAULT_STATUS,
        mergeInProgress: true,
        unresolvedFiles: ['README.md'],
      }); // refreshStatus inside conflict-non-op
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();
    await useGitStore.getState().mergeBranch('feature/x');

    // refreshStatus was called (status IPC was called a second time).
    expect(gitClient.status).toHaveBeenCalledTimes(2);
    // Store must be in conflict state with the non-op files listed.
    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.unresolvedFiles).toEqual(['README.md']);
    }
    // loadOpFileFromPath must NOT have been called — HEAD has not moved.
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
    // First status() call (post-init) returns main; second (post-switch) returns feature/x.
    vi.mocked(gitClient.status)
      .mockResolvedValueOnce(DEFAULT_STATUS)
      .mockResolvedValueOnce({ ...DEFAULT_STATUS, branch: 'feature/x' });
    vi.mocked(gitClient.branchList).mockResolvedValue([]);
    vi.mocked(gitClient.log).mockResolvedValue([]);
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.log).mockClear();
    await useGitStore.getState().switchBranch('feature/x');

    // After switch, refreshStatus updates state.repo.currentBranch to
    // 'feature/x'. loadLog must then be called with ref: 'feature/x', not
    // the previously hardcoded 'main', so the history list follows the
    // actual current branch after the transition.
    expect(gitClient.log).toHaveBeenCalledWith('repo-1', { ref: 'feature/x', limit: 50 });
  });

  // ---- Phase 6a: clone wizard + remote contract -------------------------

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
    // Enter the wizard so cloneRepo's prevWasWizard branch fires.
    useGitStore.getState().enterCloneWizard();

    vi.mocked(gitClient.clone).mockRejectedValue(
      new GitError('auth-failed', 'bad credentials', { recoverable: true }),
    );

    await useGitStore.getState().cloneRepo({
      url: 'https://github.com/foo/bar.git',
      dest: '/tmp/clone',
    });

    // Critical: we stay in wizard-clone (no `initializing` round-trip) so
    // the form component survives and its URL/dest/token inputs keep their
    // values. busy flips back to false so the Submit button re-enables.
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
    // Before the fix, cloneRepo set state.kind = 'initializing' for the
    // duration of the IPC, which unmounted GitPanelCloneForm and wiped the
    // user's form inputs. We now stay in `wizard-clone` with busy=true.
    useGitStore.getState().enterCloneWizard();

    // Resolve with a single candidate so the clone succeeds and leaves
    // the wizard cleanly (ready state).
    vi.mocked(gitClient.clone).mockImplementationOnce(async () => {
      // Snapshot state mid-IPC: it must still be wizard-clone with busy=true.
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

    // On success we exit the wizard entirely — single-candidate auto-bind.
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

    // initRepo already invoked refreshRemote() once with the default null
    // stub. Override and call again to verify the round-trip.
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

    // Renderer state must reflect the new url WITHOUT a follow-up
    // refreshRemote() call. Per the Phase 6a contract, the IPC return
    // value is the source of truth for the immediate update.
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

    // null also passes through unchanged.
    await useGitStore.getState().setRemoteUrl(null);
    expect(gitClient.remoteSet).toHaveBeenLastCalledWith('repo-1', null);
  });

  // ---- Phase 6b: pull / push + syncAfterHeadMove ------------------------

  it('pull (fast-forward) refreshes status/branches, reloads the tracked file, and refreshes the log', async () => {
    vi.mocked(gitClient.init).mockResolvedValue(SAMPLE_REPO);
    vi.mocked(gitClient.pull).mockResolvedValue({ result: 'fast-forward' });
    await useGitStore.getState().initRepo('/tmp/login.op');

    vi.mocked(gitClient.status).mockClear();
    vi.mocked(gitClient.branchList).mockClear();
    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();

    await useGitStore.getState().pull();

    // syncAfterHeadMove fires all four cascades: status, branches, tracked
    // file reload, and log refresh for the active branch. Without this a
    // successful pull would leave the canvas and history list stale.
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

    // After the conflict-non-op result, pull() delegates to refreshStatus(),
    // which calls gitClient.status() once. The mock below reports the
    // in-flight merge with the unresolved file list AND the updated repo
    // meta (branch / ahead / behind / working dirty) — refreshStatus mirrors
    // the full status payload, not just the conflict fields.
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

    // pull must have delegated the state rebuild to refreshStatus — i.e.
    // gitClient.status was consulted exactly once.
    expect(gitClient.status).toHaveBeenCalledTimes(1);

    const s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      // Empty node bag — this is a pure non-op conflict.
      expect(s.conflicts.nodeConflicts.size).toBe(0);
      expect(s.conflicts.docFieldConflicts.size).toBe(0);
      expect(s.unresolvedFiles).toEqual(['src/README.md', 'src/package.json']);
      // Repo-meta fields must reflect the status payload — the pre-fix
      // path skipped this update and the branch/ahead/behind stayed stale.
      expect(s.repo.currentBranch).toBe('feature/merge-target');
      expect(s.repo.ahead).toBe(1);
      expect(s.repo.behind).toBe(3);
      expect(s.repo.workingDirty).toBe(true);
    }
    // Non-op conflict must NOT blow away the in-memory document, since
    // the .op file on disk is still the user's pre-merge tree.
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

    // Renderer stays in ready — the button owns the auth-form retry loop
    // and must not race against the generic error card.
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
    // Push does not move HEAD on our side → no document reload, no log
    // refresh. Only status() needs to re-run so ahead/behind zero out.
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

    // Stays ready so the button can render its "pull first" inline strip.
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

  // ---- Phase 7b: finalizeError, exitTrackedFilePicker, reconciler -------

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

    // Phase 7c: applyMerge calls refreshStatus() after merge-still-conflicted
    // so the unresolved-file list is current. Mock status to return mergeInProgress:
    // true so the refreshStatus call does not demote the state to ready.
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });

    // applyMerge with merge-still-conflicted must NOT throw to the caller —
    // it surfaces the error inline on the banner.
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
    // Phase 7c: applyMerge calls refreshStatus() after merge-still-conflicted.
    // Mock status to keep the merge in progress so refreshStatus doesn't demote to ready.
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
    // Set finalizeError via applyMerge
    await useGitStore.getState().applyMerge();
    let s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).not.toBeNull();
    }

    // Resolving a conflict should clear the finalizeError
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
    // Enter picker from ready (rebind scenario)
    useGitStore.getState().enterTrackedFilePicker();
    expect(useGitStore.getState().state.kind).toBe('needs-tracked-file');
    const s = useGitStore.getState().state;
    // trackedFilePath is set (from SAMPLE_REPO.trackedFilePath)
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
    // Should have called close and returned to no-file
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

  // ---- C1: refreshStatus must not wipe in-memory conflict resolutions ----

  it('refreshStatus preserves in-memory resolutions when already in conflict', async () => {
    // Set up conflict state with one node conflict via mergeBranch.
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

    // User resolves nodeA.
    await useGitStore.getState().resolveConflict('node:_:nodeA', { kind: 'ours' });
    let s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.conflicts.nodeConflicts.get('node:_:nodeA')?.resolution).toEqual({ kind: 'ours' });
    }

    // Polling fires: backend still reports merge in progress with the same bag.
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

    // CRITICAL: the resolution on nodeA must still be present after the poll.
    s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.conflicts.nodeConflicts.get('node:_:nodeA')?.resolution).toEqual({ kind: 'ours' });
      // nodeB is still unresolved.
      expect(s.conflicts.nodeConflicts.get('node:_:nodeB')?.resolution).toBeUndefined();
    }
  });

  it('refreshStatus preserves finalizeError when already in conflict', async () => {
    // Set up conflict state via mergeBranch.
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
    // Phase 7c: set up status to return mergeInProgress:true before applyMerge
    // so the refreshStatus() call inside the merge-still-conflicted handler
    // does not demote the state to ready.
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    // Trigger a finalizeError by attempting applyMerge with unresolved conflicts.
    await useGitStore.getState().applyMerge();
    let s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBe('some conflicts remain unresolved');
    }

    // Polling fires: backend still reports merge in progress.
    vi.mocked(gitClient.status).mockResolvedValue({
      ...DEFAULT_STATUS,
      mergeInProgress: true,
      unresolvedFiles: [],
      conflicts: { nodeConflicts: [], docFieldConflicts: [] },
    });
    await useGitStore.getState().refreshStatus();

    // CRITICAL: finalizeError must still be set after the poll.
    s = useGitStore.getState().state;
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBe('some conflicts remain unresolved');
    }
  });

  it('refreshStatus updates unresolvedFiles mid-session without wiping resolutions', async () => {
    // Set up conflict state with one node conflict and two unresolved files.
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

    // Seed unresolvedFiles manually — branch merge doesn't set them.
    useGitStore.setState((s) => {
      if (s.state.kind !== 'conflict') return s;
      return { state: { ...s.state, unresolvedFiles: ['README.md', 'package.json'] } };
    });

    // User resolves nodeA.
    await useGitStore.getState().resolveConflict('node:_:nodeA', { kind: 'theirs' });

    // Backend now reports only one unresolved file (user resolved README externally).
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
      // unresolvedFiles must reflect the backend update.
      expect(s.unresolvedFiles).toEqual(['package.json']);
      // The nodeA resolution must be preserved.
      expect(s.conflicts.nodeConflicts.get('node:_:nodeA')?.resolution).toEqual({ kind: 'theirs' });
    }
  });

  // ---- Phase 7c: applyMerge reload + noop + merge-still-conflicted -------

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

    // Phase 7c: success must transition to ready AND reload the tracked file
    // and refresh the log so the canvas reflects the merged result and the
    // history list shows the new merge commit.
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
    // noop: true means the backend had nothing to write (all conflicts were trivial)
    vi.mocked(gitClient.applyMerge).mockResolvedValue({ hash: '', noop: true });
    vi.mocked(gitClient.status).mockResolvedValue(DEFAULT_STATUS);
    vi.mocked(gitClient.log).mockResolvedValue([]);
    await useGitStore.getState().initRepo('/tmp/login.op');
    await useGitStore.getState().mergeBranch('feature');

    vi.mocked(mockedLoadOpFileFromPath).mockClear();
    vi.mocked(gitClient.log).mockClear();

    await useGitStore.getState().applyMerge();

    // Even a noop result must transition to ready and run the reload cascade.
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
    // refreshStatus is called after merge-still-conflicted to keep unresolved file list current.
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

    // Must not throw — banner owns the error display.
    await expect(useGitStore.getState().applyMerge()).resolves.toBeUndefined();

    const s = useGitStore.getState().state;
    // Still in conflict with the error recorded.
    expect(s.kind).toBe('conflict');
    if (s.kind === 'conflict') {
      expect(s.finalizeError).toBe('1 conflict remains');
    }
    // Phase 7c: refreshStatus must have been called to update unresolved-file list.
    expect(gitClient.status).toHaveBeenCalledTimes(1);
    // Document must NOT have been reloaded — the merge is not complete.
    expect(mockedLoadOpFileFromPath).not.toHaveBeenCalled();
  });
});
