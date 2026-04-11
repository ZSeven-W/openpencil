// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel.test.tsx
//
// Phase 4a.1: GitPanel is now a thin body component inside a Popover.
// The "renders null when closed/minimized" tests are gone — the Popover
// ancestor controls visibility, so GitPanel always renders when visible.
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import { TooltipProvider } from '@/components/ui/tooltip';

// Mock the store before importing the component.
vi.mock('@/stores/git-store', () => {
  const baseState = {
    state: { kind: 'no-file' as const },
    panelOpen: true, // always true by contract — popover controls visibility
    log: [],
    sshKeys: [],
    authorIdentity: null,
    authorPromptVisible: false,
    lastAutoBindedPath: null as string | null,
    commitMessage: '',
    autosaveError: null as string | null,
    __autosaveUnsub: null as (() => void) | null,
    loadAuthorIdentity: vi.fn(async () => {}),
    detectRepo: vi.fn(async () => {}),
    closeRepo: vi.fn(async () => {}),
    initRepo: vi.fn(async () => {}),
    openRepo: vi.fn(async () => {}),
    acknowledgeAutoBind: vi.fn(),
    acknowledgeAutoBindAndOpen: vi.fn(async () => {}),
    loadLog: vi.fn(async () => {}),
    setCommitMessage: vi.fn(),
    clearCommitMessage: vi.fn(),
    cancelSaveRequired: vi.fn(),
    enterTrackedFilePicker: vi.fn(),
    clearAuthorIdentity: vi.fn(async () => {}),
    initAutosaveSubscriber: vi.fn(),
    disposeAutosaveSubscriber: vi.fn(),
    clearAutosaveError: vi.fn(),
    restoreCommit: vi.fn(async () => {}),
    promoteAutosave: vi.fn(async () => {}),
    showAuthorPrompt: vi.fn(),
    hideAuthorPrompt: vi.fn(),
    setAuthorIdentity: vi.fn(async () => {}),
    commitMilestone: vi.fn(async () => {}),
    retrySaveRequired: vi.fn(async () => {}),
    // Phase 6b + 6c stubs: the ready-state header pulls these via selectors.
    pull: vi.fn(async () => {}),
    push: vi.fn(async () => {}),
    getAuth: vi.fn(async () => null),
    storeAuth: vi.fn(async () => {}),
    clearAuth: vi.fn(async () => {}),
    refreshRemote: vi.fn(async () => {}),
    setRemoteUrl: vi.fn(async () => {}),
    fetchRemote: vi.fn(async () => {}),
    refreshSshKeys: vi.fn(async () => {}),
    generateSshKey: vi.fn(async () => ({
      id: 'k',
      host: 'github.com',
      publicKey: 'ssh',
      fingerprint: 'SHA256:a',
      comment: 'k',
    })),
    importSshKey: vi.fn(async () => ({
      id: 'k',
      host: 'github.com',
      publicKey: 'ssh',
      fingerprint: 'SHA256:a',
      comment: 'k',
    })),
    deleteSshKey: vi.fn(async () => {}),
    refreshStatus: vi.fn(async () => {}),
    refreshBranches: vi.fn(async () => {}),
    createBranch: vi.fn(async () => {}),
    switchBranch: vi.fn(async () => {}),
    deleteBranch: vi.fn(async () => {}),
    mergeBranch: vi.fn(async () => {}),
  };
  let current = { ...baseState };
  const useGitStore = (selector: (s: typeof current) => unknown) => selector(current);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (useGitStore as any).getState = () => current;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (useGitStore as any).__set = (partial: Partial<typeof current>) => {
    current = { ...current, ...partial };
  };
  return { useGitStore };
});

vi.mock('@/stores/document-store', () => {
  const docState = {
    filePath: '/tmp/test.op' as string | null,
    fileName: 'test.op' as string | null,
    isDirty: false,
  };
  const useDocumentStore = (selector: (s: typeof docState) => unknown) => selector(docState);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (useDocumentStore as any).getState = () => docState;
  return { useDocumentStore };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

import { GitPanel } from '@/components/panels/git-panel/git-panel';
import { useGitStore } from '@/stores/git-store';

function renderWithProvider(ui: React.ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

describe('GitPanel (dropdown body)', () => {
  beforeEach(() => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (useGitStore as any).__set({
      state: { kind: 'no-file' },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('renders the empty state body for no-file state', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (useGitStore as any).__set({ state: { kind: 'no-file' } });
    renderWithProvider(<GitPanel />);
    // GitPanelEmptyState uses the 'git.empty.heading' i18n key.
    expect(screen.getByText('git.empty.heading')).toBeTruthy();
  });

  it('renders the empty state body for no-repo state', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (useGitStore as any).__set({ state: { kind: 'no-repo' } });
    renderWithProvider(<GitPanel />);
    expect(screen.getByText('git.empty.heading')).toBeTruthy();
  });

  it('renders the initializing spinner', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (useGitStore as any).__set({ state: { kind: 'initializing' } });
    renderWithProvider(<GitPanel />);
    expect(screen.getByText('git.initializing')).toBeTruthy();
  });

  it('renders the error card for error state', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (useGitStore as any).__set({
      state: {
        kind: 'error',
        message: 'init-failed: permission denied',
        recoverable: false,
      },
    });
    renderWithProvider(<GitPanel />);
    // Error card renders the title key and the message.
    expect(screen.getByText('git.error.title')).toBeTruthy();
    expect(screen.getByText('init-failed: permission denied')).toBeTruthy();
  });

  it('renders the ready state body when state is ready', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (useGitStore as any).__set({
      state: {
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
      },
    });
    renderWithProvider(<GitPanel />);
    // Header renders the current branch name
    expect(screen.getByText('main')).toBeTruthy();
    // Commit input renders
    expect(screen.getByPlaceholderText('git.commit.placeholder')).toBeTruthy();
  });

  // ---- Phase 6c smoke: overflow -> remote-settings / ssh-keys ----

  it('overflow menu can enter and exit the remote-settings subview', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (useGitStore as any).__set({
      state: {
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
          ahead: 1,
          behind: 2,
          remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
        },
      },
    });
    renderWithProvider(<GitPanel />);
    // Open overflow popover
    fireEvent.click(screen.getByLabelText('git.header.overflowMoreActions'));
    // Enter the remote settings subview
    fireEvent.click(screen.getByTestId('overflow-open-remote-settings'));
    // Subview heading renders
    expect(screen.getByText('git.remote.settingsHeading')).toBeTruthy();
    // Back → menu (entry buttons reappear)
    fireEvent.click(screen.getByLabelText('git.remote.back'));
    expect(screen.getByTestId('overflow-open-remote-settings')).toBeTruthy();
  });

  it('overflow menu can enter and exit the ssh-keys subview', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (useGitStore as any).__set({
      state: {
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
      },
    });
    renderWithProvider(<GitPanel />);
    fireEvent.click(screen.getByLabelText('git.header.overflowMoreActions'));
    fireEvent.click(screen.getByTestId('overflow-open-ssh-keys'));
    expect(screen.getByText('git.ssh.heading')).toBeTruthy();
    fireEvent.click(screen.getByLabelText('git.ssh.back'));
    expect(screen.getByTestId('overflow-open-ssh-keys')).toBeTruthy();
  });
});
