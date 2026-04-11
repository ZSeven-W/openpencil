// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-conflict.test.tsx
import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent, act } from '@testing-library/react';
import { TooltipProvider } from '@/components/ui/tooltip';
import type { GitCommitMeta } from '@/services/git-types';

const mocks = vi.hoisted(() => {
  const conflictRepo = {
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
    state: {
      kind: 'conflict' as const,
      repo: conflictRepo,
      conflicts: {
        nodeConflicts: new Map(),
        docFieldConflicts: new Map(),
      },
      unresolvedFiles: [] as string[],
      finalizeError: null as string | null,
    } as {
      kind: 'conflict' | 'ready' | 'no-file';
      repo?: typeof conflictRepo;
      conflicts?: { nodeConflicts: Map<string, unknown>; docFieldConflicts: Map<string, unknown> };
      unresolvedFiles?: string[];
      finalizeError?: string | null;
    },
    log: [] as GitCommitMeta[],
    sshKeys: [] as Array<unknown>,
    authorIdentity: { name: 'Alice', email: 'a@e.com' } as {
      name: string;
      email: string;
    } | null,
    authorPromptVisible: false,
    autosaveError: null as string | null,
    commitMessage: '',
    loadLog: vi.fn(async () => {}),
    abortMerge: vi.fn(async () => {}),
    applyMerge: vi.fn(async () => {}),
    pull: vi.fn(async () => {}),
    push: vi.fn(async () => {}),
    getAuth: vi.fn(async () => null),
    storeAuth: vi.fn(async () => {}),
    restoreCommit: vi.fn(async () => {}),
    promoteAutosave: vi.fn(async () => {}),
    clearAutosaveError: vi.fn(),
    enterTrackedFilePicker: vi.fn(),
    clearAuthorIdentity: vi.fn(async () => {}),
    closeRepo: vi.fn(async () => {}),
    refreshStatus: vi.fn(async () => {}),
    // Phase 7b: computeDiff needed by GitPanelHistoryDiff inside expanded rows
    computeDiff: vi.fn(async () => ({
      summary: { framesChanged: 0, nodesAdded: 0, nodesRemoved: 0, nodesModified: 0 },
      patches: [],
    })),
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

import { GitPanelConflict } from '@/components/panels/git-panel/git-panel-conflict';

function renderWithProvider(ui: React.ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

describe('GitPanelConflict', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.state = {
      kind: 'conflict',
      repo: mocks.state.repo,
      conflicts: { nodeConflicts: new Map(), docFieldConflicts: new Map() },
      unresolvedFiles: [],
      finalizeError: null,
    };
  });

  afterEach(() => {
    cleanup();
  });

  it('renders the header, the conflict banner, and the history list', () => {
    renderWithProvider(<GitPanelConflict />);
    // Header shows the branch name
    expect(screen.getByText('main')).toBeTruthy();
    // Conflict banner renders title + description + abort button
    expect(screen.getByText('git.conflict.title')).toBeTruthy();
    expect(screen.getByText('git.conflict.description')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'git.conflict.abort' })).toBeTruthy();
    // Read-only history list shows the empty state (no commit input)
    expect(screen.getByText('git.history.empty')).toBeTruthy();
    // There is NO commit input during a conflict
    expect(screen.queryByPlaceholderText('git.commit.placeholder')).toBeNull();
  });

  it('loads the log on mount while in conflict state', () => {
    renderWithProvider(<GitPanelConflict />);
    expect(mocks.loadLog).toHaveBeenCalledTimes(1);
    expect(mocks.loadLog).toHaveBeenCalledWith({ ref: 'main', limit: 50 });
  });

  it('renders the history list in read-only mode (no restore / promote buttons)', () => {
    // Populate the log with one milestone and one standalone autosave so
    // that expanding each row exposes its detail card. Under normal (non-
    // read-only) mode both cards would render Restore buttons and the
    // autosave card would also render a Promote button.
    mocks.log = [
      {
        hash: 'mile-1',
        parentHashes: [],
        message: 'first milestone',
        author: { name: 'Alice', email: 'a@e.com', timestamp: Math.floor(Date.now() / 1000) },
        kind: 'milestone',
      },
      {
        hash: 'auto-1',
        parentHashes: ['mile-1'],
        message: 'auto: 12:34',
        author: { name: 'Alice', email: 'a@e.com', timestamp: Math.floor(Date.now() / 1000) },
        kind: 'autosave',
      },
    ];

    renderWithProvider(<GitPanelConflict />);

    // Expand the milestone row (click the row button that shows the commit message)
    fireEvent.click(screen.getByText('first milestone'));
    expect(screen.queryByRole('button', { name: 'git.history.restoreButton' })).toBeNull();
    // Copy hash is not a mutation and should still be available
    expect(screen.getByRole('button', { name: 'git.history.copyHashButton' })).toBeTruthy();

    // Expand the autosave row — autosave rows show the time label
    fireEvent.click(screen.getByText('git.history.autosaveLabel:{"time":"12:34"}'));
    expect(screen.queryByRole('button', { name: 'git.history.restoreButton' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'git.history.promoteButton' })).toBeNull();

    // Reset log for the beforeEach in the next test (though cleanup also runs)
    mocks.log = [];
  });

  it('renders the non-op unresolved files strip when unresolvedFiles is non-empty', () => {
    mocks.state = {
      kind: 'conflict',
      repo: mocks.state.repo,
      conflicts: { nodeConflicts: new Map(), docFieldConflicts: new Map() },
      unresolvedFiles: ['src/README.md', 'src/package.json'],
      finalizeError: null,
    };

    renderWithProvider(<GitPanelConflict />);

    // Non-op banner title/description replaces the plain merge-conflict copy
    expect(screen.getByText('git.conflict.nonOp.title')).toBeTruthy();
    expect(screen.getByText('git.conflict.nonOp.description')).toBeTruthy();

    // Unresolved file paths render as a monospace list
    expect(screen.getByText('src/README.md')).toBeTruthy();
    expect(screen.getByText('src/package.json')).toBeTruthy();

    // Phase 7b: button labels changed to banner.* keys for unified UI.
    // Continue + abort are both present.
    expect(screen.getByRole('button', { name: 'git.conflict.banner.continue' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'git.conflict.abort' })).toBeTruthy();
  });

  it('clicking the continue button in the non-op strip calls applyMerge', () => {
    mocks.state = {
      kind: 'conflict',
      repo: mocks.state.repo,
      conflicts: { nodeConflicts: new Map(), docFieldConflicts: new Map() },
      unresolvedFiles: ['src/README.md'],
      finalizeError: null,
    };
    renderWithProvider(<GitPanelConflict />);
    fireEvent.click(screen.getByRole('button', { name: 'git.conflict.banner.continue' }));
    expect(mocks.applyMerge).toHaveBeenCalledTimes(1);
  });

  it('clicking abort in the non-op strip calls abortMerge', () => {
    mocks.state = {
      kind: 'conflict',
      repo: mocks.state.repo,
      conflicts: { nodeConflicts: new Map(), docFieldConflicts: new Map() },
      unresolvedFiles: ['src/README.md'],
      finalizeError: null,
    };
    renderWithProvider(<GitPanelConflict />);
    fireEvent.click(screen.getByRole('button', { name: 'git.conflict.abort' }));
    expect(mocks.abortMerge).toHaveBeenCalledTimes(1);
  });

  it('falls back to the plain abort-only banner when unresolvedFiles is empty', () => {
    renderWithProvider(<GitPanelConflict />);
    // Default mock state has unresolvedFiles: [] — plain banner
    expect(screen.getByText('git.conflict.title')).toBeTruthy();
    expect(screen.queryByText('git.conflict.nonOp.title')).toBeNull();
    expect(screen.queryByRole('button', { name: 'git.conflict.banner.continue' })).toBeNull();
  });

  describe('polling error surface', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('displays the poll error inline and stops polling after a single failure', async () => {
      mocks.state = {
        kind: 'conflict',
        repo: mocks.state.repo,
        conflicts: { nodeConflicts: new Map(), docFieldConflicts: new Map() },
        unresolvedFiles: ['src/README.md'],
        finalizeError: null,
      };
      mocks.refreshStatus.mockRejectedValueOnce(new Error('git: network timeout'));

      renderWithProvider(<GitPanelConflict />);

      // Advance to trigger the first poll (3 s) and flush React state updates
      await act(async () => {
        await vi.advanceTimersByTimeAsync(3000);
      });

      // The error message should now be visible in the UI
      expect(
        screen.getByText('git.conflict.banner.pollError:{"message":"git: network timeout"}'),
      ).toBeTruthy();

      // refreshStatus was called exactly once (the failed call)
      expect(mocks.refreshStatus).toHaveBeenCalledTimes(1);

      // Advance another 3 s — polling has stopped so refreshStatus is NOT called again
      await act(async () => {
        await vi.advanceTimersByTimeAsync(3000);
      });
      expect(mocks.refreshStatus).toHaveBeenCalledTimes(1);
    });
  });
});
