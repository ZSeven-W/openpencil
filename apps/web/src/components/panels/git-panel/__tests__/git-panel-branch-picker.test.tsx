// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-branch-picker.test.tsx
//
// Phase 5 Task 2: branch picker shell tests. The picker:
//   - renders the current branch on a trigger Button
//   - opens a Popover with a list of all branches (current + others)
//   - refreshes status + branches whenever the popover opens
//   - renders create/merge entry buttons in list mode
//   - becomes non-interactive (disabled trigger) in conflict state
//
// Task 2 is shell-only: select/delete handlers are stubbed no-ops. The
// tests here only assert rendering and refresh plumbing; Tasks 3/4 will
// add the behavior tests.
import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import { TooltipProvider } from '@/components/ui/tooltip';

const mocks = vi.hoisted(() => {
  const readyRepo = {
    repoId: 'r1',
    currentBranch: 'main',
    mode: 'single-file' as const,
    rootPath: '/tmp/repo',
    gitdir: '/tmp/repo/.git',
    engineKind: 'iso' as const,
    trackedFilePath: '/tmp/repo/login.op',
    candidateFiles: [],
    branches: [
      {
        name: 'main',
        isCurrent: true,
        ahead: 0,
        behind: 0,
        lastCommit: null,
      },
    ],
    workingDirty: false,
    otherFilesDirty: 0,
    otherFilesPaths: [],
    ahead: 0,
    behind: 0,
  };
  return {
    readyRepo,
    state: { kind: 'ready' as const, repo: readyRepo } as {
      kind: 'ready' | 'conflict';
      repo: typeof readyRepo;
      conflicts?: {
        nodeConflicts: Map<string, unknown>;
        docFieldConflicts: Map<string, unknown>;
      };
    },
    refreshStatus: vi.fn(async () => {}),
    refreshBranches: vi.fn(async () => {}),
  };
});

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: typeof mocks) => unknown) => selector(mocks),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

import { GitPanelBranchPicker } from '@/components/panels/git-panel/git-panel-branch-picker';

function renderWithProvider(ui: React.ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

describe('GitPanelBranchPicker', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.readyRepo.currentBranch = 'main';
    mocks.readyRepo.branches = [
      {
        name: 'main',
        isCurrent: true,
        ahead: 0,
        behind: 0,
        lastCommit: null,
      },
    ];
    mocks.state = { kind: 'ready', repo: mocks.readyRepo };
  });

  afterEach(() => {
    cleanup();
  });

  it('refreshes status and branches when the picker opens', () => {
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByRole('button', { name: 'main' }));
    expect(mocks.refreshStatus).toHaveBeenCalledTimes(1);
    expect(mocks.refreshBranches).toHaveBeenCalledTimes(1);
  });

  it('renders the current branch and the non-current branch rows', () => {
    mocks.readyRepo.branches = [
      { name: 'main', isCurrent: true, ahead: 0, behind: 0, lastCommit: null },
      { name: 'feature/login', isCurrent: false, ahead: 0, behind: 0, lastCommit: null },
    ];
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByRole('button', { name: 'main' }));
    // 'main' appears in both the trigger and a row; getAllByText covers both.
    expect(screen.getAllByText('main').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('feature/login')).toBeTruthy();
  });

  it('renders create and merge entry buttons in list mode', () => {
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByRole('button', { name: 'main' }));
    expect(screen.getByText('git.branch.createAction')).toBeTruthy();
    expect(screen.getByText('git.branch.mergeAction')).toBeTruthy();
  });

  it('renders the list heading in list mode', () => {
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByRole('button', { name: 'main' }));
    expect(screen.getByText('git.branch.listHeading')).toBeTruthy();
  });

  it('renders a delete button for non-current branches with the localized aria-label', () => {
    mocks.readyRepo.branches = [
      { name: 'main', isCurrent: true, ahead: 0, behind: 0, lastCommit: null },
      { name: 'feature/login', isCurrent: false, ahead: 0, behind: 0, lastCommit: null },
    ];
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByRole('button', { name: 'main' }));
    expect(
      screen.getByRole('button', {
        name: 'git.branch.deleteLabel:{"name":"feature/login"}',
      }),
    ).toBeTruthy();
  });

  it('does NOT render a delete button for the current branch', () => {
    mocks.readyRepo.branches = [
      { name: 'main', isCurrent: true, ahead: 0, behind: 0, lastCommit: null },
    ];
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByRole('button', { name: 'main' }));
    expect(
      screen.queryByRole('button', {
        name: 'git.branch.deleteLabel:{"name":"main"}',
      }),
    ).toBeNull();
  });

  it('disables the picker trigger in conflict state', () => {
    mocks.state = {
      kind: 'conflict',
      repo: mocks.readyRepo,
      conflicts: { nodeConflicts: new Map(), docFieldConflicts: new Map() },
    };
    renderWithProvider(<GitPanelBranchPicker />);
    const trigger = screen.getByRole('button', { name: 'main' }) as HTMLButtonElement;
    expect(trigger.disabled).toBe(true);
  });

  it('does not call refresh actions when the trigger is disabled in conflict state', () => {
    mocks.state = {
      kind: 'conflict',
      repo: mocks.readyRepo,
      conflicts: { nodeConflicts: new Map(), docFieldConflicts: new Map() },
    };
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByRole('button', { name: 'main' }));
    expect(mocks.refreshStatus).not.toHaveBeenCalled();
    expect(mocks.refreshBranches).not.toHaveBeenCalled();
  });
});
