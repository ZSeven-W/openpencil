// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-empty-state.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';

const initRepoMock = vi.fn(async () => {});
const openRepoMock = vi.fn(async () => {});
const enterCloneWizardMock = vi.fn(() => {});

vi.mock('@/stores/git-store', () => {
  return {
    useGitStore: (
      selector: (s: {
        initRepo: typeof initRepoMock;
        openRepo: typeof openRepoMock;
        enterCloneWizard: typeof enterCloneWizardMock;
      }) => unknown,
    ) =>
      selector({
        initRepo: initRepoMock,
        openRepo: openRepoMock,
        enterCloneWizard: enterCloneWizardMock,
      }),
  };
});

let mockedFilePath: string | null = '/tmp/test.op';
vi.mock('@/stores/document-store', () => {
  // Hook form: useDocumentStore((s) => s.filePath) — selector pattern.
  // Each call reads the current mockedFilePath via closure so individual
  // tests can mutate it before render.
  const useDocumentStore = (selector: (s: { filePath: string | null }) => unknown) =>
    selector({ filePath: mockedFilePath });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (useDocumentStore as any).getState = () => ({ filePath: mockedFilePath });
  return { useDocumentStore };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k }),
}));

import { GitPanelEmptyState } from '@/components/panels/git-panel/git-panel-empty-state';
import { TooltipProvider } from '@/components/ui/tooltip';

const renderWithProvider = (ui: React.ReactElement) =>
  render(<TooltipProvider>{ui}</TooltipProvider>);

describe('GitPanelEmptyState', () => {
  beforeEach(() => {
    mockedFilePath = '/tmp/test.op';
    vi.clearAllMocks();
    // Attach electronAPI to the existing jsdom window — replacing the
    // entire window object via vi.stubGlobal would clobber clearTimeout
    // and other browser APIs that Radix Tooltip depends on.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any).electronAPI = {
      openDirectory: vi.fn(async () => '/tmp/repo'),
    };
  });

  afterEach(() => {
    cleanup();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (window as any).electronAPI;
  });

  it('disables the new card when currentFilePath is null', () => {
    mockedFilePath = null;
    renderWithProvider(<GitPanelEmptyState />);
    // The new card button is disabled — find by accessible name (label key).
    const newButton = screen.getByText('git.empty.newCard').closest('button');
    expect(newButton).toBeTruthy();
    expect(newButton?.hasAttribute('disabled')).toBe(true);
  });

  it('clicking the new card calls initRepo with the current file path', async () => {
    renderWithProvider(<GitPanelEmptyState />);
    const newButton = screen.getByText('git.empty.newCard').closest('button');
    expect(newButton).not.toBeNull();
    fireEvent.click(newButton!);
    // Wait a tick for the async handler.
    await Promise.resolve();
    expect(initRepoMock).toHaveBeenCalledTimes(1);
    expect(initRepoMock).toHaveBeenCalledWith('/tmp/test.op');
  });

  it('clicking the open card calls electronAPI.openDirectory then openRepo', async () => {
    renderWithProvider(<GitPanelEmptyState />);
    const openButton = screen.getByText('git.empty.openCard').closest('button');
    expect(openButton).not.toBeNull();
    fireEvent.click(openButton!);
    // Wait for the async chain.
    await Promise.resolve();
    await Promise.resolve();
    expect(openRepoMock).toHaveBeenCalledTimes(1);
    // openRepo gets the picked directory + the current file path.
    expect(openRepoMock).toHaveBeenCalledWith('/tmp/repo', '/tmp/test.op');
  });

  it('clicking the clone card opens the clone wizard (Phase 6a)', () => {
    renderWithProvider(<GitPanelEmptyState />);
    const cloneButton = screen.getByText('git.empty.cloneCard').closest('button');
    expect(cloneButton).toBeTruthy();
    expect(cloneButton?.hasAttribute('disabled')).toBe(false);
    fireEvent.click(cloneButton!);
    expect(enterCloneWizardMock).toHaveBeenCalledTimes(1);
  });
});
