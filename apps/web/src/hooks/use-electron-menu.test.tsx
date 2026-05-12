// @vitest-environment jsdom

import { cleanup, render, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

let menuCallback: ((action: string) => void) | null = null;
const loadDocumentMock = vi.fn();
const newDocumentMock = vi.fn();
const parseAndPrepareImportedDocumentMock = vi.fn();

vi.mock('@/i18n', () => ({
  default: { t: (key: string) => key },
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({
      isDirty: false,
      fileName: null,
      loadDocument: loadDocumentMock,
      newDocument: newDocumentMock,
      save: vi.fn(async () => 'saved.op'),
    }),
  },
}));

vi.mock('@/stores/canvas-store', () => ({
  useCanvasStore: {
    getState: () => ({
      setFigmaImportDialogOpen: vi.fn(),
      setExportDialogOpen: vi.fn(),
      clearSelection: vi.fn(),
    }),
  },
}));

vi.mock('@/stores/history-store', () => ({
  useHistoryStore: {
    getState: () => ({
      undo: vi.fn(),
      redo: vi.fn(),
    }),
  },
}));

vi.mock('@/canvas/skia-engine-ref', () => ({
  syncCanvasPositionsToStore: vi.fn(),
  zoomToFitContent: vi.fn(),
}));

vi.mock('@/utils/import-pen-document', () => ({
  parseAndPrepareImportedDocument: (...args: unknown[]) => parseAndPrepareImportedDocumentMock(...args),
}));

vi.mock('@/utils/recent-files', () => ({
  addRecentFile: vi.fn(),
  clearRecentFiles: vi.fn(),
}));

import { useElectronMenu } from './use-electron-menu';

function Harness() {
  useElectronMenu();
  return null;
}

afterEach(() => {
  cleanup();
  menuCallback = null;
  loadDocumentMock.mockReset();
  newDocumentMock.mockReset();
  parseAndPrepareImportedDocumentMock.mockReset();
  delete (window as unknown as Record<string, unknown>).electronAPI;
  window.history.replaceState({}, '', '/');
});

describe('useElectronMenu local file actions', () => {
  it('creates a new local document from the native menu', async () => {
    (window as unknown as { electronAPI: Partial<ElectronAPI> }).electronAPI = {
      isElectron: true,
      onMenuAction: (callback) => {
        menuCallback = callback;
        return vi.fn();
      },
      getPendingFile: vi.fn(async () => null),
      onOpenFile: vi.fn(),
    };
    render(<Harness />);

    menuCallback?.('new');

    await waitFor(() => {
      expect(newDocumentMock).toHaveBeenCalled();
      expect(window.location.pathname).toBe('/editor/local');
    });
  });

  it('opens a local .op file from the native menu without cloud import', async () => {
    const doc = { version: '1.0.0', pages: [{ id: 'page-1', name: 'Page 1', children: [] }] };
    parseAndPrepareImportedDocumentMock.mockReturnValue({ doc });
    (window as unknown as { electronAPI: Partial<ElectronAPI> }).electronAPI = {
      isElectron: true,
      onMenuAction: (callback) => {
        menuCallback = callback;
        return vi.fn();
      },
      getPendingFile: vi.fn(async () => null),
      onOpenFile: vi.fn(),
      openFile: vi.fn(async () => ({
        filePath: '/tmp/local.op',
        content: JSON.stringify(doc),
      })),
    };
    render(<Harness />);

    menuCallback?.('open');

    await waitFor(() => {
      expect(loadDocumentMock).toHaveBeenCalledWith(doc, 'local.op', null, '/tmp/local.op');
      expect(window.location.pathname).toBe('/editor/local');
    });
  });
});
