// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-branch-picker.test.tsx
//
// Phase 5 Task 2 + Task 3: branch picker tests.
//
// Task 2 shell: rendering + refresh plumbing + conflict disable.
// Task 3: inline create + branch switching + save-required close behavior.
//
// Selector gotcha: once the popover is open, each <GitPanelBranchRow>
// renders its own button with aria-label={branch.name}. That collides
// with the trigger's aria-label (which matches the current branch name).
// To avoid "found multiple elements" errors we click the trigger via
// its data-testid ("branch-picker-trigger") — the tests below follow
// that convention consistently.
import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import { TooltipProvider } from '@/components/ui/tooltip';
import { GitError } from '@/services/git-error';

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
    createBranch: vi.fn(async (_opts: { name: string }) => {}),
    switchBranch: vi.fn(async (_name: string) => {}),
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
    mocks.createBranch.mockImplementation(async () => {});
    mocks.switchBranch.mockImplementation(async () => {});
  });

  afterEach(() => {
    cleanup();
  });

  it('refreshes status and branches when the picker opens', () => {
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
    expect(mocks.refreshStatus).toHaveBeenCalledTimes(1);
    expect(mocks.refreshBranches).toHaveBeenCalledTimes(1);
  });

  it('renders the current branch and the non-current branch rows', () => {
    mocks.readyRepo.branches = [
      { name: 'main', isCurrent: true, ahead: 0, behind: 0, lastCommit: null },
      { name: 'feature/login', isCurrent: false, ahead: 0, behind: 0, lastCommit: null },
    ];
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
    // 'main' appears in both the trigger and a row; getAllByText covers both.
    expect(screen.getAllByText('main').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('feature/login')).toBeTruthy();
  });

  it('renders create and merge entry buttons in list mode', () => {
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
    expect(screen.getByText('git.branch.createAction')).toBeTruthy();
    expect(screen.getByText('git.branch.mergeAction')).toBeTruthy();
  });

  it('renders the list heading in list mode', () => {
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
    expect(screen.getByText('git.branch.listHeading')).toBeTruthy();
  });

  it('renders a delete button for non-current branches with the localized aria-label', () => {
    mocks.readyRepo.branches = [
      { name: 'main', isCurrent: true, ahead: 0, behind: 0, lastCommit: null },
      { name: 'feature/login', isCurrent: false, ahead: 0, behind: 0, lastCommit: null },
    ];
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
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
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
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

  // --- Task 3: create + switch behavior -------------------------------

  it('creates a branch from the inline create view', async () => {
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
    fireEvent.click(screen.getByText('git.branch.createAction'));
    fireEvent.change(screen.getByPlaceholderText('git.branch.createPlaceholder'), {
      target: { value: 'feature/login' },
    });
    fireEvent.click(screen.getByText('git.branch.createSubmit'));
    expect(mocks.createBranch).toHaveBeenCalledWith({ name: 'feature/login' });
  });

  it('switches branches when a non-current branch row is clicked', async () => {
    mocks.readyRepo.branches = [
      { name: 'main', isCurrent: true, ahead: 0, behind: 0, lastCommit: null },
      { name: 'feature/login', isCurrent: false, ahead: 0, behind: 0, lastCommit: null },
    ];
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
    // after the popover opens, feature/login's row has aria-label="feature/login"
    fireEvent.click(screen.getByRole('button', { name: 'feature/login' }));
    expect(mocks.switchBranch).toHaveBeenCalledWith('feature/login');
  });

  it('keeps the picker open and shows an inline error when create validation fails', () => {
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
    fireEvent.click(screen.getByText('git.branch.createAction'));
    fireEvent.click(screen.getByText('git.branch.createSubmit'));
    // Empty branch name → createEmpty error shown.
    expect(screen.getByText('git.branch.createEmpty')).toBeTruthy();
    expect(mocks.createBranch).not.toHaveBeenCalled();
  });

  it('closes the popover when switch throws save-required so the panel alert takes over', async () => {
    mocks.readyRepo.branches = [
      { name: 'main', isCurrent: true, ahead: 0, behind: 0, lastCommit: null },
      { name: 'feature/login', isCurrent: false, ahead: 0, behind: 0, lastCommit: null },
    ];
    mocks.switchBranch.mockRejectedValueOnce(
      new GitError('save-required', 'Document has unsaved changes'),
    );
    renderWithProvider(<GitPanelBranchPicker />);
    fireEvent.click(screen.getByTestId('branch-picker-trigger'));
    fireEvent.click(screen.getByRole('button', { name: 'feature/login' }));
    // Give the async handler a microtask to settle.
    await new Promise((r) => setTimeout(r, 0));
    // Popover should have closed — the list-mode heading is no longer visible.
    expect(screen.queryByText('git.branch.listHeading')).toBeNull();
  });
});
