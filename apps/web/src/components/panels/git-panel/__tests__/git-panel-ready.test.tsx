// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-ready.test.tsx
import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
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
    branches: [],
    workingDirty: false,
    otherFilesDirty: 0,
    otherFilesPaths: [],
    ahead: 0,
    behind: 0,
    remote: null as { name: 'origin'; url: string | null; host: string | null } | null,
  };
  return {
    state: { kind: 'ready' as const, repo: readyRepo } as {
      kind: 'ready' | 'no-file';
      repo?: typeof readyRepo;
      saveRequiredFor?: { label: string };
    },
    log: [] as Array<unknown>,
    sshKeys: [] as Array<unknown>,
    authorIdentity: { name: 'Alice', email: 'a@e.com' } as { name: string; email: string } | null,
    authorPromptVisible: false,
    autosaveError: null as string | null,
    commitMessage: '',
    loadLog: vi.fn(async () => {}),
    setCommitMessage: vi.fn(),
    showAuthorPrompt: vi.fn(),
    hideAuthorPrompt: vi.fn(),
    setAuthorIdentity: vi.fn(async () => {}),
    commitMilestone: vi.fn(async () => {}),
    retrySaveRequired: vi.fn(async () => {}),
    cancelSaveRequired: vi.fn(),
    enterTrackedFilePicker: vi.fn(),
    clearAuthorIdentity: vi.fn(async () => {}),
    closeRepo: vi.fn(async () => {}),
    clearAutosaveError: vi.fn(),
    restoreCommit: vi.fn(async () => {}),
    promoteAutosave: vi.fn(async () => {}),
    pull: vi.fn(async () => {}),
    push: vi.fn(async () => {}),
    getAuth: vi.fn(async () => null),
    storeAuth: vi.fn(async () => {}),
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

import { GitPanelReady } from '@/components/panels/git-panel/git-panel-ready';

function renderWithProvider(ui: React.ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

describe('GitPanelReady', () => {
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
    mocks.log = [];
    mocks.authorIdentity = { name: 'Alice', email: 'a@e.com' };
    mocks.autosaveError = null;
    mocks.commitMessage = '';
    mocks.authorPromptVisible = false;
  });

  afterEach(() => {
    cleanup();
  });

  it('calls loadLog on first mount with { ref: main, limit: 50 }', () => {
    renderWithProvider(<GitPanelReady />);
    expect(mocks.loadLog).toHaveBeenCalledTimes(1);
    expect(mocks.loadLog).toHaveBeenCalledWith({ ref: 'main', limit: 50 });
  });

  it('renders the header, commit input, and history list when state is ready', () => {
    renderWithProvider(<GitPanelReady />);
    // Header shows the branch name
    expect(screen.getByText('main')).toBeTruthy();
    // Commit input shows its placeholder
    expect(screen.getByPlaceholderText('git.commit.placeholder')).toBeTruthy();
    // History list shows the empty state
    expect(screen.getByText('git.history.empty')).toBeTruthy();
  });

  it('does NOT call loadLog when state is not ready', () => {
    mocks.state = { kind: 'no-file' };
    renderWithProvider(<GitPanelReady />);
    expect(mocks.loadLog).not.toHaveBeenCalled();
    // Header returns null in non-ready/conflict state, so 'main' is not rendered.
    expect(screen.queryByText('main')).toBeNull();
  });

  it('fires loadLog when state transitions from non-ready to ready', () => {
    // Start in a non-ready state — loadLog should not fire.
    mocks.state = { kind: 'no-file' };
    const { rerender } = renderWithProvider(<GitPanelReady />);
    expect(mocks.loadLog).not.toHaveBeenCalled();

    // Transition to ready. The effect's [state.kind, loadLog] deps should
    // re-fire loadLog exactly once, proving the dep array is load-bearing
    // (not dead code that could be replaced with []).
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
    rerender(
      <TooltipProvider>
        <GitPanelReady />
      </TooltipProvider>,
    );
    expect(mocks.loadLog).toHaveBeenCalledTimes(1);
    expect(mocks.loadLog).toHaveBeenCalledWith({ ref: 'main', limit: 50 });
  });
});
