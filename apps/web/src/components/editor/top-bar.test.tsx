// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TooltipProvider } from '@/components/ui/tooltip';
import '@/i18n';

const navigateMock = vi.fn();
const getCloudFileMock = vi.hoisted(() => vi.fn());
const createCloudFileMock = vi.hoisted(() => vi.fn());
const versionPanelStoreMock = vi.hoisted(() => ({
  setOpen: vi.fn(),
}));

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => navigateMock,
}));

const canvasState = {
  layerPanelOpen: true,
  toggleLayerPanel: vi.fn(),
  setFigmaImportDialogOpen: vi.fn(),
  setExportDialogOpen: vi.fn(),
};

vi.mock('@/stores/canvas-store', () => ({
  useCanvasStore: Object.assign(
    (selector: (state: typeof canvasState) => unknown) => selector(canvasState),
    {
      getState: () => canvasState,
    },
  ),
}));

const documentState = {
  fileName: 'Cloud File',
  filePath: null as string | null,
  isDirty: false,
  cloudFileId: 'file-1' as string | null,
  cloudSaveState: 'idle',
  cloudSaveError: null as string | null,
  cloudSaveConflict: null as {
    code: 'revision_conflict';
    fileId: string;
    expectedRevision: number;
    serverRevision: number | null;
  } | null,
  document: {
    version: '1.0.0',
    name: 'Untitled',
    pages: [{ id: 'page-1', name: 'Page 1', children: [] }],
  },
  save: vi.fn(async () => 'Cloud File'),
  exportOp: vi.fn(async () => 'Cloud File.op'),
  loadCloudDocument: vi.fn(),
};

vi.mock('@/stores/document-store', () => ({
  createEmptyDocument: () => ({
    version: '1.0.0',
    name: 'Untitled',
    pages: [{ id: 'page-1', name: 'Page 1', children: [] }],
  }),
  useDocumentStore: Object.assign(
    (selector: (state: typeof documentState) => unknown) => selector(documentState),
    {
      getState: () => documentState,
    },
  ),
}));

vi.mock('@/stores/agent-settings-store', () => ({
  useAgentSettingsStore: Object.assign(
    (selector: (state: unknown) => unknown) =>
      selector({
        providers: {
          anthropic: { isConnected: false },
          openai: { isConnected: false },
          opencode: { isConnected: false },
          copilot: { isConnected: false },
          gemini: { isConnected: false },
        },
        mcpIntegrations: [],
        setDialogOpen: vi.fn(),
      }),
    {
      getState: () => ({ setDialogOpen: vi.fn() }),
    },
  ),
}));

vi.mock('@/stores/cloud-version-panel-store', () => ({
  useCloudVersionPanelStore: Object.assign(
    (selector: (state: typeof versionPanelStoreMock) => unknown) =>
      selector(versionPanelStoreMock),
    { getState: () => versionPanelStoreMock },
  ),
}));

vi.mock('@/components/shared/file-menu', () => ({
  default: () => null,
}));

vi.mock('@/components/shared/language-selector', () => ({
  default: () => <div data-testid="language-selector" />,
}));

vi.mock('@/components/editor/git-button', () => ({
  GitButton: () => null,
}));

vi.mock('@/components/icons/claude-logo', () => ({ default: () => null }));
vi.mock('@/components/icons/openai-logo', () => ({ default: () => null }));
vi.mock('@/components/icons/opencode-logo', () => ({ default: () => null }));
vi.mock('@/components/icons/copilot-logo', () => ({ default: () => null }));
vi.mock('@/components/icons/gemini-logo', () => ({ default: () => null }));
vi.mock('@/components/icons/figma-logo', () => ({ default: () => null }));

vi.mock('@/canvas/skia-engine-ref', () => ({
  syncCanvasPositionsToStore: vi.fn(),
  zoomToFitContent: vi.fn(),
}));

vi.mock('@/utils/recent-files', () => ({
  addRecentFile: vi.fn(),
}));

vi.mock('@/utils/file-operations', () => ({
  isElectron: () => false,
}));

vi.mock('@/services/cloud/cloud-files', () => ({
  createCloudFile: createCloudFileMock,
  getCloudFile: getCloudFileMock,
}));

vi.mock('@/utils/import-pen-document', () => ({
  parseAndPrepareImportedDocument: vi.fn(),
}));

import TopBar from './top-bar';

function renderTopBar() {
  return render(
    <TooltipProvider>
      <TopBar />
    </TooltipProvider>,
  );
}

afterEach(() => {
  cleanup();
  navigateMock.mockClear();
  documentState.isDirty = false;
  documentState.cloudFileId = 'file-1';
  documentState.cloudSaveState = 'idle';
  documentState.cloudSaveError = null;
  documentState.cloudSaveConflict = null;
  documentState.fileName = 'Cloud File';
  documentState.save.mockClear();
  documentState.save.mockResolvedValue('Cloud File');
  documentState.loadCloudDocument.mockClear();
  getCloudFileMock.mockReset();
  createCloudFileMock.mockReset();
  versionPanelStoreMock.setOpen.mockClear();
  delete (window as unknown as { __showUnsavedDialog?: unknown }).__showUnsavedDialog;
});

describe('TopBar back to files', () => {
  it('navigates to the cloud file library when the document is clean', async () => {
    renderTopBar();

    fireEvent.click(screen.getByRole('button', { name: /Back to files/i }));

    await waitFor(() => {
      expect(navigateMock).toHaveBeenCalledWith({ to: '/cloud' });
    });
  });

  it('stays in the editor when unsaved confirmation is cancelled', async () => {
    documentState.isDirty = true;
    const showUnsavedDialog = vi.fn<() => Promise<'cancel'>>(async () => 'cancel');
    (window as unknown as { __showUnsavedDialog: typeof showUnsavedDialog }).__showUnsavedDialog =
      showUnsavedDialog;

    renderTopBar();

    fireEvent.click(screen.getByRole('button', { name: /Back to files/i }));

    await waitFor(() => {
      expect(
        (window as unknown as { __showUnsavedDialog: () => Promise<'cancel'> })
          .__showUnsavedDialog,
      ).toHaveBeenCalledWith('Cloud File');
    });
    expect(navigateMock).not.toHaveBeenCalled();
  });

  it('saves dirty work before navigating back to the cloud file library', async () => {
    documentState.isDirty = true;
    const showUnsavedDialog = vi.fn<() => Promise<'save'>>(async () => 'save');
    (window as unknown as { __showUnsavedDialog: typeof showUnsavedDialog }).__showUnsavedDialog =
      showUnsavedDialog;

    renderTopBar();

    fireEvent.click(screen.getByRole('button', { name: /Back to files/i }));

    await waitFor(() => {
      expect(documentState.save).toHaveBeenCalled();
      expect(navigateMock).toHaveBeenCalledWith({ to: '/cloud' });
    });
  });

  it('opens cloud version history for cloud files', () => {
    renderTopBar();

    fireEvent.click(screen.getByRole('button', { name: 'Version history' }));

    expect(versionPanelStoreMock.setOpen).toHaveBeenCalledWith(true);
  });

  it('shows cloud revision conflicts and can reload the remote version', async () => {
    const remoteFile = {
      id: 'file-1',
      name: 'Cloud File',
      revision: 9,
      document: documentState.document,
    };
    documentState.cloudSaveState = 'conflict';
    documentState.cloudSaveError = 'Cloud file has a newer revision';
    documentState.cloudSaveConflict = {
      code: 'revision_conflict',
      fileId: 'file-1',
      expectedRevision: 7,
      serverRevision: 9,
    };
    getCloudFileMock.mockResolvedValue(remoteFile);

    renderTopBar();

    expect(screen.getByText(/remote rev 9/i)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Reload' }));

    await waitFor(() => {
      expect(getCloudFileMock).toHaveBeenCalledWith('file-1');
      expect(documentState.loadCloudDocument).toHaveBeenCalledWith(remoteFile);
    });
  });

  it('saves conflicted local work as a new cloud copy', async () => {
    documentState.cloudSaveState = 'conflict';
    documentState.cloudSaveError = 'Cloud file has a newer revision';
    documentState.cloudSaveConflict = {
      code: 'revision_conflict',
      fileId: 'file-1',
      expectedRevision: 7,
      serverRevision: 9,
    };
    createCloudFileMock.mockResolvedValue({ id: 'copy-file-1' });

    renderTopBar();

    fireEvent.click(screen.getByRole('button', { name: 'Save copy' }));

    await waitFor(() => {
      expect(createCloudFileMock).toHaveBeenCalledWith({
        name: 'Cloud File copy',
        document: documentState.document,
        source: 'manual_save',
      });
      expect(navigateMock).toHaveBeenCalledWith({
        to: '/editor/$fileId',
        params: { fileId: 'copy-file-1' },
      });
    });
  });
});
