// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-history-diff.test.tsx
//
// Phase 7b: verifies the inline diff block loads on expand, handles initial
// commit, surfaces compute errors inline, and renders the diff summary.

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, act } from '@testing-library/react';
import type { GitCommitMeta } from '@/services/git-types';

const mocks = vi.hoisted(() => ({
  computeDiff: vi.fn(async () => ({
    summary: { framesChanged: 0, nodesAdded: 0, nodesRemoved: 0, nodesModified: 0 },
    patches: [] as unknown[],
  })),
}));

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: { computeDiff: typeof mocks.computeDiff }) => unknown) =>
    selector({ computeDiff: mocks.computeDiff }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

import { GitPanelHistoryDiff } from '@/components/panels/git-panel/git-panel-history-diff';

function makeCommit(
  hash: string,
  parentHashes: string[],
  overrides: Partial<GitCommitMeta> = {},
): GitCommitMeta {
  return {
    hash,
    parentHashes,
    message: 'test commit',
    author: { name: 'Test', email: 't@e.com', timestamp: Math.floor(Date.now() / 1000) },
    kind: 'milestone',
    ...overrides,
  };
}

describe('GitPanelHistoryDiff', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.computeDiff.mockResolvedValue({
      summary: { framesChanged: 0, nodesAdded: 0, nodesRemoved: 0, nodesModified: 0 },
      patches: [],
    });
  });

  afterEach(() => {
    cleanup();
  });

  it('shows initial-commit state when commit has no parent', () => {
    const commit = makeCommit('abc', []);
    render(<GitPanelHistoryDiff commit={commit} />);
    expect(screen.getByText('git.history.diff.initialCommit')).toBeTruthy();
    // computeDiff must NOT have been called for the initial-commit case.
    expect(mocks.computeDiff).not.toHaveBeenCalled();
  });

  it('shows loading state while computeDiff is in flight', () => {
    // Keep the promise pending indefinitely so loading state persists.
    mocks.computeDiff.mockImplementation(() => new Promise(() => {}));
    const commit = makeCommit('abc', ['parent-1']);
    render(<GitPanelHistoryDiff commit={commit} />);
    expect(screen.getByText('git.history.diff.loading')).toBeTruthy();
  });

  it('calls computeDiff(parentHash, commitHash) on mount', async () => {
    const commit = makeCommit('abc', ['parent-1']);
    await act(async () => {
      render(<GitPanelHistoryDiff commit={commit} />);
    });
    expect(mocks.computeDiff).toHaveBeenCalledTimes(1);
    expect(mocks.computeDiff).toHaveBeenCalledWith('parent-1', 'abc');
  });

  it('renders diff summary when computeDiff resolves', async () => {
    mocks.computeDiff.mockResolvedValue({
      summary: { framesChanged: 1, nodesAdded: 3, nodesRemoved: 2, nodesModified: 0 },
      patches: [],
    });
    const commit = makeCommit('abc', ['parent-1']);
    await act(async () => {
      render(<GitPanelHistoryDiff commit={commit} />);
    });
    // The t mock returns `key:{"count":N}` for interpolated calls.
    // framesChanged renders with count=1
    expect(screen.getByText('git.history.diff.framesChanged:{"count":1}')).toBeTruthy();
    // nodesAdded with + prefix, count=3
    expect(screen.getByText(/git\.history\.diff\.nodesAdded/)).toBeTruthy();
    // nodesRemoved with - prefix, count=2
    expect(screen.getByText(/git\.history\.diff\.nodesRemoved/)).toBeTruthy();
  });

  it('renders no-changes when all summary fields are zero', async () => {
    mocks.computeDiff.mockResolvedValue({
      summary: { framesChanged: 0, nodesAdded: 0, nodesRemoved: 0, nodesModified: 0 },
      patches: [],
    });
    const commit = makeCommit('abc', ['parent-1']);
    await act(async () => {
      render(<GitPanelHistoryDiff commit={commit} />);
    });
    expect(screen.getByText('git.history.diff.noChanges')).toBeTruthy();
  });

  it('surfaces computeDiff errors inline without throwing', async () => {
    mocks.computeDiff.mockRejectedValue(new Error('connection refused'));
    const commit = makeCommit('abc', ['parent-1']);
    await act(async () => {
      render(<GitPanelHistoryDiff commit={commit} />);
    });
    expect(screen.getByText(/git\.history\.diff\.error/)).toBeTruthy();
    expect(screen.getByText(/connection refused/)).toBeTruthy();
  });

  it('renders patch list with op and nodeId when patches are non-empty', async () => {
    mocks.computeDiff.mockResolvedValue({
      summary: { framesChanged: 0, nodesAdded: 1, nodesRemoved: 0, nodesModified: 0 },
      patches: [
        { op: 'add', nodeId: 'rect-1' },
        { op: 'remove', nodeId: 'circle-2' },
      ],
    });
    const commit = makeCommit('abc', ['parent-1']);
    await act(async () => {
      render(<GitPanelHistoryDiff commit={commit} />);
    });
    expect(screen.getByText('add')).toBeTruthy();
    expect(screen.getByText('rect-1')).toBeTruthy();
    expect(screen.getByText('remove')).toBeTruthy();
    expect(screen.getByText('circle-2')).toBeTruthy();
  });
});
