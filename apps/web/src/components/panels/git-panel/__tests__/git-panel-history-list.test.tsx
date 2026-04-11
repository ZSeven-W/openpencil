// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-history-list.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';

const mocks = vi.hoisted(() => ({
  log: [] as Array<{
    hash: string;
    parentHashes: string[];
    message: string;
    author: { name: string; email: string; timestamp: number };
    kind: 'milestone' | 'autosave';
  }>,
  restoreCommit: vi.fn(async () => {}),
  promoteAutosave: vi.fn(async () => {}),
  authorIdentity: { name: 'Test Author', email: 't@e.com' } as {
    name: string;
    email: string;
  } | null,
  // Phase 7b: computeDiff is used by GitPanelHistoryDiff inside expanded rows.
  computeDiff: vi.fn(async () => ({
    summary: { framesChanged: 0, nodesAdded: 0, nodesRemoved: 0, nodesModified: 0 },
    patches: [],
  })),
}));

vi.mock('@/stores/git-store', () => ({
  useGitStore: (
    selector: (s: {
      log: typeof mocks.log;
      restoreCommit: typeof mocks.restoreCommit;
      promoteAutosave: typeof mocks.promoteAutosave;
      authorIdentity: typeof mocks.authorIdentity;
      computeDiff: typeof mocks.computeDiff;
    }) => unknown,
  ) =>
    selector({
      log: mocks.log,
      restoreCommit: mocks.restoreCommit,
      promoteAutosave: mocks.promoteAutosave,
      authorIdentity: mocks.authorIdentity,
      computeDiff: mocks.computeDiff,
    }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

import { GitPanelHistoryList } from '@/components/panels/git-panel/git-panel-history-list';

function makeCommit(
  hash: string,
  kind: 'milestone' | 'autosave',
  message: string,
  secondsAgo: number,
): (typeof mocks.log)[number] {
  const timestamp = Math.floor(Date.now() / 1000) - secondsAgo;
  return {
    hash,
    parentHashes: [],
    message,
    author: { name: 'Test Author', email: 't@e.com', timestamp },
    kind,
  };
}

describe('GitPanelHistoryList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.log = [];
    mocks.authorIdentity = { name: 'Test Author', email: 't@e.com' };
  });

  afterEach(() => {
    cleanup();
  });

  it('renders the empty state when log is empty', () => {
    mocks.log = [];
    render(<GitPanelHistoryList />);
    expect(screen.getByText('git.history.empty')).toBeTruthy();
  });

  it('renders milestone rows with message and author info', () => {
    mocks.log = [
      makeCommit('m1', 'milestone', 'first milestone', 60),
      makeCommit('m2', 'milestone', 'second milestone', 3600),
    ];
    render(<GitPanelHistoryList />);
    expect(screen.getByText('first milestone')).toBeTruthy();
    expect(screen.getByText('second milestone')).toBeTruthy();
  });

  it('groups 3+ consecutive autosaves into a collapsible group header', () => {
    mocks.log = [
      makeCommit('a1', 'autosave', 'auto: 15:42', 60),
      makeCommit('a2', 'autosave', 'auto: 15:41', 120),
      makeCommit('a3', 'autosave', 'auto: 15:40', 180),
      makeCommit('a4', 'autosave', 'auto: 15:39', 240),
      makeCommit('m1', 'milestone', 'before autosaves', 3600),
    ];
    render(<GitPanelHistoryList />);

    // The group header renders the pluralized key. Our i18n mock returns
    // `key:{"count":4}` for interpolated keys.
    expect(screen.getByText((text) => text.startsWith('git.history.autosaveGroup'))).toBeTruthy();
    // Individual autosave rows are NOT rendered until expansion
    expect(screen.queryByText('git.history.autosaveLabel:{"time":"15:42"}')).toBeNull();
  });

  it('expanding an autosave group reveals individual rows', () => {
    mocks.log = [
      makeCommit('a1', 'autosave', 'auto: 15:42', 60),
      makeCommit('a2', 'autosave', 'auto: 15:41', 120),
      makeCommit('a3', 'autosave', 'auto: 15:40', 180),
    ];
    render(<GitPanelHistoryList />);

    // Find and click the group toggle
    const groupToggle = screen
      .getByText((text) => text.startsWith('git.history.autosaveGroup'))
      .closest('button');
    expect(groupToggle).toBeTruthy();
    fireEvent.click(groupToggle!);

    // Now the individual autosave labels should be present
    expect(screen.getByText('git.history.autosaveLabel:{"time":"15:42"}')).toBeTruthy();
    expect(screen.getByText('git.history.autosaveLabel:{"time":"15:41"}')).toBeTruthy();
    expect(screen.getByText('git.history.autosaveLabel:{"time":"15:40"}')).toBeTruthy();
  });

  it('clicking a milestone row expands the detail card with restore button', () => {
    mocks.log = [makeCommit('m1', 'milestone', 'first milestone', 60)];
    render(<GitPanelHistoryList />);

    // Detail card should not be visible initially
    expect(screen.queryByText('git.history.milestoneDetailTitle')).toBeNull();

    // Click the milestone row
    fireEvent.click(screen.getByText('first milestone').closest('button')!);

    // Detail card is now visible
    expect(screen.getByText('git.history.milestoneDetailTitle')).toBeTruthy();
    // Phase 7b: diff block renders (loading state since computeDiff is async;
    // no parent hash → initial-commit state for this makeCommit default).
    expect(screen.getByText('git.history.diff.initialCommit')).toBeTruthy();

    // Click restore
    fireEvent.click(screen.getByText('git.history.restoreButton').closest('button')!);

    expect(mocks.restoreCommit).toHaveBeenCalledTimes(1);
    expect(mocks.restoreCommit).toHaveBeenCalledWith('m1');
  });

  it('clicking an autosave row expands its detail card with restore + promote buttons and calls restoreCommit', () => {
    // A single autosave (below the group threshold) renders as an
    // individual HistoryAutosaveRow, not inside a group.
    mocks.log = [
      makeCommit('a1', 'autosave', 'auto: 15:42', 60),
      makeCommit('m1', 'milestone', 'before autosave', 3600),
    ];
    render(<GitPanelHistoryList />);

    // Detail card should not be visible initially
    expect(screen.queryByText('git.history.restoreButton')).toBeNull();
    expect(screen.queryByText('git.history.promoteButton')).toBeNull();

    // Click the autosave row (find the row button by its label text)
    fireEvent.click(
      screen.getByText('git.history.autosaveLabel:{"time":"15:42"}').closest('button')!,
    );

    // Both action buttons are now visible
    expect(screen.getByText('git.history.restoreButton')).toBeTruthy();
    expect(screen.getByText('git.history.promoteButton')).toBeTruthy();

    // Restore on the autosave detail card calls restoreCommit with the
    // autosave's hash.
    fireEvent.click(screen.getByText('git.history.restoreButton').closest('button')!);
    expect(mocks.restoreCommit).toHaveBeenCalledTimes(1);
    expect(mocks.restoreCommit).toHaveBeenCalledWith('a1');
  });

  it('clicking promote on an autosave detail card calls promoteAutosave with the current identity', () => {
    mocks.log = [makeCommit('a1', 'autosave', 'auto: 15:42', 60)];
    render(<GitPanelHistoryList />);

    // Expand the autosave detail card
    fireEvent.click(
      screen.getByText('git.history.autosaveLabel:{"time":"15:42"}').closest('button')!,
    );

    // Click the promote button
    fireEvent.click(screen.getByText('git.history.promoteButton').closest('button')!);

    expect(mocks.promoteAutosave).toHaveBeenCalledTimes(1);
    // Phase 4c reuses the autosave's own message as the milestone message
    // and the current authorIdentity (falling back to the sentinel when
    // null — see the row implementation).
    expect(mocks.promoteAutosave).toHaveBeenCalledWith('a1', 'auto: 15:42', {
      name: 'Test Author',
      email: 't@e.com',
    });
  });
});
