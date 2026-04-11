// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-tracked-picker.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';

type CandidateFile = {
  path: string;
  relativePath: string;
  milestoneCount: number;
  autosaveCount: number;
  lastCommitAt: number | null;
  lastCommitMessage: string | null;
};

type MockedState = {
  kind: 'needs-tracked-file' | 'no-file';
  repo?: {
    candidateFiles: CandidateFile[];
    trackedFilePath?: string | null;
  };
};

// vi.hoisted ensures the refs object is created before vi.mock factories
// run (vi.mock is hoisted to the top of the file). The test body then
// mutates `mocks.mockedState` etc. to drive each scenario.
const mocks = vi.hoisted(() => {
  return {
    bindTrackedFile: vi.fn(async (_: string) => {}),
    closePanel: vi.fn(),
    closeRepo: vi.fn(async () => {}),
    // Phase 7b: exitTrackedFilePicker drives back/cancel navigation
    exitTrackedFilePicker: vi.fn(async () => {}),
    loadOpFileFromPath: vi.fn(async (_: string) => true),
    mockedState: {
      kind: 'needs-tracked-file',
      repo: { candidateFiles: [], trackedFilePath: null },
    } as MockedState,
  };
});

vi.mock('@/stores/git-store', () => ({
  useGitStore: (
    selector: (s: {
      state: MockedState;
      bindTrackedFile: typeof mocks.bindTrackedFile;
      closePanel: typeof mocks.closePanel;
      closeRepo: typeof mocks.closeRepo;
      exitTrackedFilePicker: typeof mocks.exitTrackedFilePicker;
    }) => unknown,
  ) =>
    selector({
      state: mocks.mockedState,
      bindTrackedFile: mocks.bindTrackedFile,
      closePanel: mocks.closePanel,
      closeRepo: mocks.closeRepo,
      exitTrackedFilePicker: mocks.exitTrackedFilePicker,
    }),
}));

vi.mock('@/utils/load-op-file', () => ({
  loadOpFileFromPath: mocks.loadOpFileFromPath,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

import { GitPanelTrackedPicker } from '@/components/panels/git-panel/git-panel-tracked-picker';

const SAMPLE_CANDIDATES = [
  {
    path: '/tmp/repo/login.op',
    relativePath: 'login.op',
    milestoneCount: 32,
    autosaveCount: 12,
    lastCommitAt: 1700000000,
    lastCommitMessage: 'tweak login button color',
  },
  {
    path: '/tmp/repo/home.op',
    relativePath: 'home.op',
    milestoneCount: 8,
    autosaveCount: 0,
    lastCommitAt: 1750000000, // newer than login.op
    lastCommitMessage: 'home redesign',
  },
  {
    path: '/tmp/repo/profile.op',
    relativePath: 'profile.op',
    milestoneCount: 1,
    autosaveCount: 0,
    lastCommitAt: 1690000000, // oldest
    lastCommitMessage: 'init',
  },
];

describe('GitPanelTrackedPicker', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.mockedState = {
      kind: 'needs-tracked-file',
      repo: { candidateFiles: SAMPLE_CANDIDATES, trackedFilePath: null },
    };
  });

  afterEach(() => {
    cleanup();
  });

  it('renders all candidate rows sorted by lastCommitAt desc', () => {
    render(<GitPanelTrackedPicker />);

    // All three relative paths should appear.
    expect(screen.getByText('login.op')).toBeTruthy();
    expect(screen.getByText('home.op')).toBeTruthy();
    expect(screen.getByText('profile.op')).toBeTruthy();

    // The buttons exist for each row — get all <button> elements and
    // verify the FIRST candidate row (after the heading) is home.op
    // (newest by lastCommitAt 1_750_000_000). Row buttons contain a
    // relativePath ending in .op followed by milestone/last-commit
    // sub-elements, so match on ".op" substring rather than suffix.
    const candidateButtons = screen
      .getAllByRole('button')
      .filter((b) => /\.op/.test(b.textContent ?? ''));
    expect(candidateButtons).toHaveLength(3);
    expect(candidateButtons[0].textContent).toContain('home.op');
    expect(candidateButtons[1].textContent).toContain('login.op');
    expect(candidateButtons[2].textContent).toContain('profile.op');
  });

  it('action buttons are disabled until a row is selected', () => {
    render(<GitPanelTrackedPicker />);

    const trackButton = screen.getByText('git.picker.bindButton').closest('button');
    const trackOpenButton = screen.getByText('git.picker.bindAndOpenButton').closest('button');
    expect(trackButton).toBeTruthy();
    expect(trackOpenButton).toBeTruthy();
    expect(trackButton?.hasAttribute('disabled')).toBe(true);
    expect(trackOpenButton?.hasAttribute('disabled')).toBe(true);

    // Select the first row
    const homeRow = screen.getByText('home.op').closest('button');
    fireEvent.click(homeRow!);

    expect(trackButton?.hasAttribute('disabled')).toBe(false);
    expect(trackOpenButton?.hasAttribute('disabled')).toBe(false);
  });

  it('clicking 跟踪此文件 calls bindTrackedFile but NOT loadOpFileFromPath', async () => {
    render(<GitPanelTrackedPicker />);

    fireEvent.click(screen.getByText('login.op').closest('button')!);
    fireEvent.click(screen.getByText('git.picker.bindButton').closest('button')!);
    await Promise.resolve();
    await Promise.resolve();

    expect(mocks.bindTrackedFile).toHaveBeenCalledTimes(1);
    expect(mocks.bindTrackedFile).toHaveBeenCalledWith('/tmp/repo/login.op');
    expect(mocks.loadOpFileFromPath).not.toHaveBeenCalled();
  });

  it('clicking 跟踪并打开 calls bindTrackedFile AND loadOpFileFromPath', async () => {
    render(<GitPanelTrackedPicker />);

    fireEvent.click(screen.getByText('profile.op').closest('button')!);
    fireEvent.click(screen.getByText('git.picker.bindAndOpenButton').closest('button')!);
    await Promise.resolve();
    await Promise.resolve();

    expect(mocks.bindTrackedFile).toHaveBeenCalledTimes(1);
    expect(mocks.bindTrackedFile).toHaveBeenCalledWith('/tmp/repo/profile.op');
    expect(mocks.loadOpFileFromPath).toHaveBeenCalledTimes(1);
    expect(mocks.loadOpFileFromPath).toHaveBeenCalledWith('/tmp/repo/profile.op');
  });

  it('zero candidates renders the empty card with closeRepo + closePanel chain', async () => {
    mocks.mockedState = {
      kind: 'needs-tracked-file',
      repo: { candidateFiles: [], trackedFilePath: null },
    };
    render(<GitPanelTrackedPicker />);

    expect(screen.getByText('git.picker.empty.heading')).toBeTruthy();
    expect(screen.getByText('git.picker.empty.body')).toBeTruthy();

    fireEvent.click(screen.getByText('git.picker.empty.close').closest('button')!);
    await Promise.resolve();
    await Promise.resolve();

    expect(mocks.closeRepo).toHaveBeenCalledTimes(1);
    expect(mocks.closePanel).toHaveBeenCalledTimes(1);
  });

  // ---- Phase 7b: back button / exitTrackedFilePicker --------------------

  it('back button shows "Back" label when rebinding (trackedFilePath set)', async () => {
    mocks.mockedState = {
      kind: 'needs-tracked-file',
      repo: { candidateFiles: SAMPLE_CANDIDATES, trackedFilePath: '/tmp/repo/login.op' },
    };
    render(<GitPanelTrackedPicker />);
    // The back button should show the "Back" label (git.picker.back)
    expect(screen.getByRole('button', { name: 'git.picker.back' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'git.picker.backClose' })).toBeNull();
  });

  it('back button shows "Cancel" label when first opened (trackedFilePath null)', async () => {
    // Default state from beforeEach: trackedFilePath: null
    render(<GitPanelTrackedPicker />);
    expect(screen.getByRole('button', { name: 'git.picker.backClose' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'git.picker.back' })).toBeNull();
  });

  it('clicking the back button calls exitTrackedFilePicker', async () => {
    render(<GitPanelTrackedPicker />);
    fireEvent.click(screen.getByRole('button', { name: 'git.picker.backClose' }));
    await Promise.resolve();
    await Promise.resolve();
    expect(mocks.exitTrackedFilePicker).toHaveBeenCalledTimes(1);
  });

  it('clicking back in rebind mode calls exitTrackedFilePicker (not closeRepo)', async () => {
    mocks.mockedState = {
      kind: 'needs-tracked-file',
      repo: { candidateFiles: SAMPLE_CANDIDATES, trackedFilePath: '/tmp/repo/login.op' },
    };
    render(<GitPanelTrackedPicker />);
    fireEvent.click(screen.getByRole('button', { name: 'git.picker.back' }));
    await Promise.resolve();
    await Promise.resolve();
    expect(mocks.exitTrackedFilePicker).toHaveBeenCalledTimes(1);
    // closeRepo must NOT be called — the store's exitTrackedFilePicker handles
    // the navigation logic without closing the repo session.
    expect(mocks.closeRepo).not.toHaveBeenCalled();
  });
});
