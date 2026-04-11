// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-conflict-banner.test.tsx
//
// Phase 7b: tests for the upgraded conflict banner — progress display,
// non-.op file list summary, dynamic apply/continue label, abort,
// and inline finalizeError from merge-still-conflicted.

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

type NodeConflict = {
  id: string;
  pageId: null;
  nodeId: string;
  reason: 'both-modified-same-field';
  base: null;
  ours: null;
  theirs: null;
  resolution?: { kind: 'ours' } | { kind: 'theirs' };
};

type MockRepo = {
  repoId: string;
  currentBranch: string;
  mode: 'single-file';
  rootPath: string;
  gitdir: string;
  engineKind: 'iso';
  trackedFilePath: string;
  candidateFiles: never[];
  branches: never[];
  workingDirty: boolean;
  otherFilesDirty: number;
  otherFilesPaths: never[];
  ahead: number;
  behind: number;
  remote: null;
};

type MockState = {
  kind: 'conflict';
  repo: MockRepo;
  conflicts: {
    nodeConflicts: Map<string, NodeConflict>;
    docFieldConflicts: Map<string, never>;
  };
  unresolvedFiles: string[];
  finalizeError: string | null;
  reopenedMidMerge: boolean;
};

const mocks = vi.hoisted(() => ({
  state: {
    kind: 'conflict' as const,
    repo: {
      repoId: 'r1',
      currentBranch: 'main',
      mode: 'single-file' as const,
      rootPath: '/tmp/repo',
      gitdir: '/tmp/repo/.git',
      engineKind: 'iso' as const,
      trackedFilePath: '/tmp/repo/login.op',
      candidateFiles: [] as never[],
      branches: [] as never[],
      workingDirty: false,
      otherFilesDirty: 0,
      otherFilesPaths: [] as never[],
      ahead: 0,
      behind: 0,
      remote: null as null,
    },
    conflicts: {
      nodeConflicts: new Map<string, NodeConflict>(),
      docFieldConflicts: new Map<string, never>(),
    },
    unresolvedFiles: [] as string[],
    finalizeError: null as string | null,
    reopenedMidMerge: false,
  } as MockState,
  abortMerge: vi.fn(async () => {}),
  applyMerge: vi.fn(async () => {}),
}));

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: typeof mocks) => unknown) => selector(mocks),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

import { GitPanelConflictBanner } from '@/components/panels/git-panel/git-panel-conflict-banner';

function makeNodeConflict(id: string, resolved: boolean = false): NodeConflict {
  return {
    id,
    pageId: null,
    nodeId: id.replace('node:_:', ''),
    reason: 'both-modified-same-field',
    base: null,
    ours: null,
    theirs: null,
    ...(resolved ? { resolution: { kind: 'ours' as const } } : {}),
  };
}

describe('GitPanelConflictBanner', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.state = {
      kind: 'conflict',
      repo: mocks.state.repo,
      conflicts: {
        nodeConflicts: new Map<string, NodeConflict>(),
        docFieldConflicts: new Map<string, never>(),
      },
      unresolvedFiles: [],
      finalizeError: null,
      reopenedMidMerge: false,
    };
  });

  afterEach(() => {
    cleanup();
  });

  it('renders title, description, and abort button when no conflicts', () => {
    render(<GitPanelConflictBanner />);
    expect(screen.getByText('git.conflict.title')).toBeTruthy();
    expect(screen.getByText('git.conflict.description')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'git.conflict.abort' })).toBeTruthy();
  });

  it('renders with role="alert" so screen readers announce the conflict', () => {
    const { container } = render(<GitPanelConflictBanner />);
    expect(container.querySelector('[role="alert"]')).toBeTruthy();
  });

  it('clicking the abort button calls abortMerge', () => {
    render(<GitPanelConflictBanner />);
    fireEvent.click(screen.getByRole('button', { name: 'git.conflict.abort' }));
    expect(mocks.abortMerge).toHaveBeenCalledTimes(1);
  });

  it('shows progress counter when there are .op conflicts', () => {
    mocks.state.conflicts.nodeConflicts.set(
      'node:_:rect-1',
      makeNodeConflict('node:_:rect-1', false),
    );
    mocks.state.conflicts.nodeConflicts.set(
      'node:_:rect-2',
      makeNodeConflict('node:_:rect-2', true),
    );
    render(<GitPanelConflictBanner />);
    // 1 of 2 resolved: progress shows resolved=1 total=2
    expect(screen.getByText('git.conflict.banner.progress:{"resolved":1,"total":2}')).toBeTruthy();
  });

  it('shows "Apply merge" button when .op conflicts are unresolved', () => {
    mocks.state.conflicts.nodeConflicts.set(
      'node:_:rect-1',
      makeNodeConflict('node:_:rect-1', false),
    );
    render(<GitPanelConflictBanner />);
    expect(screen.getByRole('button', { name: 'git.conflict.banner.apply' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'git.conflict.banner.continue' })).toBeNull();
  });

  it('clicking apply button calls applyMerge', () => {
    mocks.state.conflicts.nodeConflicts.set(
      'node:_:rect-1',
      makeNodeConflict('node:_:rect-1', false),
    );
    render(<GitPanelConflictBanner />);
    fireEvent.click(screen.getByRole('button', { name: 'git.conflict.banner.apply' }));
    expect(mocks.applyMerge).toHaveBeenCalledTimes(1);
  });

  it('shows "Continue" button when all .op conflicts resolved and non-.op files remain', () => {
    mocks.state.conflicts.nodeConflicts.set(
      'node:_:rect-1',
      makeNodeConflict('node:_:rect-1', true),
    );
    mocks.state.unresolvedFiles = ['src/README.md'];
    render(<GitPanelConflictBanner />);
    expect(screen.getByRole('button', { name: 'git.conflict.banner.continue' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'git.conflict.banner.apply' })).toBeNull();
  });

  it('renders the non-.op unresolved file list when unresolvedFiles is non-empty', () => {
    mocks.state.unresolvedFiles = ['src/README.md', 'package.json'];
    render(<GitPanelConflictBanner />);
    expect(screen.getByText('git.conflict.nonOp.title')).toBeTruthy();
    expect(screen.getByText('src/README.md')).toBeTruthy();
    expect(screen.getByText('package.json')).toBeTruthy();
  });

  it('renders finalizeError inline when set', () => {
    mocks.state.finalizeError = 'some conflicts remain unresolved';
    render(<GitPanelConflictBanner />);
    expect(screen.getByText(/git\.conflict\.banner\.finalizeError/)).toBeTruthy();
    expect(screen.getByText(/some conflicts remain unresolved/)).toBeTruthy();
  });

  it('does NOT render finalizeError block when finalizeError is null', () => {
    mocks.state.finalizeError = null;
    render(<GitPanelConflictBanner />);
    expect(screen.queryByText(/git\.conflict\.banner\.finalizeError/)).toBeNull();
  });

  // ── I2: panel-reopen degraded mode ────────────────────────────────────

  it('shows reopenMessage and hides primary button when reopenedMidMerge=true', () => {
    mocks.state.reopenedMidMerge = true;
    render(<GitPanelConflictBanner />);
    // Abort button must still be present.
    expect(screen.getByRole('button', { name: 'git.conflict.abort' })).toBeTruthy();
    // Primary action button must NOT be present.
    expect(screen.queryByRole('button', { name: 'git.conflict.banner.apply' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'git.conflict.banner.continue' })).toBeNull();
    // Reopen warning message must be shown.
    expect(screen.getByText('git.conflict.banner.reopenMessage')).toBeTruthy();
  });

  it('shows primary button normally when reopenedMidMerge=false (default)', () => {
    mocks.state.reopenedMidMerge = false;
    render(<GitPanelConflictBanner />);
    // In the zero-conflict state (totalCount=0, no non-op files), the apply
    // button should still be visible (normal "Apply merge" path).
    expect(screen.getByRole('button', { name: 'git.conflict.banner.apply' })).toBeTruthy();
    // No reopen message.
    expect(screen.queryByText('git.conflict.banner.reopenMessage')).toBeNull();
  });

  it('does not show primary button for reopened state even when unresolvedFiles is non-empty', () => {
    // If somehow unresolvedFiles is non-empty AND reopenedMidMerge=true, the
    // primary button must still be suppressed (abort-only is unconditional
    // in reopen state).
    mocks.state.reopenedMidMerge = true;
    mocks.state.unresolvedFiles = ['README.md'];
    render(<GitPanelConflictBanner />);
    expect(screen.queryByRole('button', { name: 'git.conflict.banner.continue' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'git.conflict.banner.apply' })).toBeNull();
    expect(screen.getByRole('button', { name: 'git.conflict.abort' })).toBeTruthy();
    expect(screen.getByText('git.conflict.banner.reopenMessage')).toBeTruthy();
  });
});
