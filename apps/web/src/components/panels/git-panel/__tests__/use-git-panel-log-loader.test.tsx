// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/use-git-panel-log-loader.test.tsx
//
// Phase 7b: verifies that useGitPanelLogLoader reads state.repo.currentBranch
// instead of a hardcoded 'main' so branch switches and conflict → ready
// transitions both see the correct log.

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, cleanup } from '@testing-library/react';

type MockRepo = {
  repoId: string;
  currentBranch: string;
  mode: 'single-file';
  rootPath: string;
  gitdir: string;
  engineKind: 'iso';
  trackedFilePath: string | null;
  candidateFiles: never[];
  branches: never[];
  workingDirty: boolean;
  otherFilesDirty: number;
  otherFilesPaths: never[];
  ahead: number;
  behind: number;
  remote: null;
};

const mocks = vi.hoisted(() => ({
  state: {
    kind: 'ready' as string,
    repo: {
      repoId: 'r1',
      currentBranch: 'main',
      mode: 'single-file',
      rootPath: '/tmp/repo',
      gitdir: '/tmp/repo/.git',
      engineKind: 'iso',
      trackedFilePath: '/tmp/repo/login.op',
      candidateFiles: [],
      branches: [],
      workingDirty: false,
      otherFilesDirty: 0,
      otherFilesPaths: [],
      ahead: 0,
      behind: 0,
      remote: null,
    } as MockRepo,
  },
  loadLog: vi.fn(async () => {}),
}));

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: typeof mocks) => unknown) => selector(mocks),
}));

import { useGitPanelLogLoader } from '@/components/panels/git-panel/use-git-panel-log-loader';

/** Minimal test harness component that exercises the hook. */
function Harness({ kinds }: { kinds: ReadonlyArray<string> }) {
  useGitPanelLogLoader(kinds);
  return null;
}

describe('useGitPanelLogLoader', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.state = {
      kind: 'ready',
      repo: {
        repoId: 'r1',
        currentBranch: 'main',
        mode: 'single-file',
        rootPath: '/tmp/repo',
        gitdir: '/tmp/repo/.git',
        engineKind: 'iso',
        trackedFilePath: '/tmp/repo/login.op',
        candidateFiles: [],
        branches: [],
        workingDirty: false,
        otherFilesDirty: 0,
        otherFilesPaths: [],
        ahead: 0,
        behind: 0,
        remote: null,
      },
    };
  });

  afterEach(() => {
    cleanup();
  });

  it('loads the log for the current branch (not hardcoded main)', () => {
    mocks.state.repo.currentBranch = 'feature/login-redesign';
    render(<Harness kinds={['ready']} />);
    expect(mocks.loadLog).toHaveBeenCalledTimes(1);
    expect(mocks.loadLog).toHaveBeenCalledWith({ ref: 'feature/login-redesign', limit: 50 });
  });

  it('loads the log for the current branch in conflict state', () => {
    mocks.state = {
      kind: 'conflict',
      repo: {
        ...mocks.state.repo,
        currentBranch: 'dev',
      },
    };
    render(<Harness kinds={['conflict']} />);
    expect(mocks.loadLog).toHaveBeenCalledTimes(1);
    expect(mocks.loadLog).toHaveBeenCalledWith({ ref: 'dev', limit: 50 });
  });

  it('does NOT load log when state.kind is not in the allowed kinds', () => {
    // @ts-expect-error mock state shape
    mocks.state = { kind: 'no-file' };
    render(<Harness kinds={['ready']} />);
    expect(mocks.loadLog).not.toHaveBeenCalled();
  });

  it('re-fires when currentBranch changes', () => {
    mocks.state.repo.currentBranch = 'main';
    const { rerender } = render(<Harness kinds={['ready']} />);
    expect(mocks.loadLog).toHaveBeenCalledTimes(1);
    expect(mocks.loadLog).toHaveBeenLastCalledWith({ ref: 'main', limit: 50 });

    // Simulate a branch switch (same state kind, different currentBranch)
    mocks.state.repo.currentBranch = 'feature/x';
    rerender(<Harness kinds={['ready']} />);
    expect(mocks.loadLog).toHaveBeenCalledTimes(2);
    expect(mocks.loadLog).toHaveBeenLastCalledWith({ ref: 'feature/x', limit: 50 });
  });

  it('re-fires when state transitions from no-file to ready', () => {
    // @ts-expect-error mock state shape
    mocks.state = { kind: 'no-file' };
    const { rerender } = render(<Harness kinds={['ready']} />);
    expect(mocks.loadLog).not.toHaveBeenCalled();

    mocks.state = {
      kind: 'ready',
      repo: {
        repoId: 'r1',
        currentBranch: 'main',
        mode: 'single-file',
        rootPath: '/tmp/repo',
        gitdir: '/tmp/repo/.git',
        engineKind: 'iso',
        trackedFilePath: '/tmp/repo/login.op',
        candidateFiles: [],
        branches: [],
        workingDirty: false,
        otherFilesDirty: 0,
        otherFilesPaths: [],
        ahead: 0,
        behind: 0,
        remote: null,
      },
    };
    rerender(<Harness kinds={['ready']} />);
    expect(mocks.loadLog).toHaveBeenCalledTimes(1);
    expect(mocks.loadLog).toHaveBeenCalledWith({ ref: 'main', limit: 50 });
  });
});
