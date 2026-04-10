// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-conflict.test.tsx
import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
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
  };
  return {
    state: {
      kind: 'conflict' as const,
      repo: conflictRepo,
      conflicts: {
        nodeConflicts: new Map(),
        docFieldConflicts: new Map(),
      },
    } as {
      kind: 'conflict' | 'ready' | 'no-file';
      repo?: typeof conflictRepo;
      conflicts?: { nodeConflicts: Map<string, unknown>; docFieldConflicts: Map<string, unknown> };
    },
    log: [] as GitCommitMeta[],
    authorIdentity: { name: 'Alice', email: 'a@e.com' } as {
      name: string;
      email: string;
    } | null,
    authorPromptVisible: false,
    autosaveError: null as string | null,
    commitMessage: '',
    loadLog: vi.fn(async () => {}),
    abortMerge: vi.fn(async () => {}),
    restoreCommit: vi.fn(async () => {}),
    promoteAutosave: vi.fn(async () => {}),
    clearAutosaveError: vi.fn(),
    enterTrackedFilePicker: vi.fn(),
    clearAuthorIdentity: vi.fn(async () => {}),
    closeRepo: vi.fn(async () => {}),
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
});
