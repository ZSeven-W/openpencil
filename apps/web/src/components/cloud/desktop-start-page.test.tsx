// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

const navigateMock = vi.fn();
const newDocumentMock = vi.fn();
const loadDocumentMock = vi.fn();
const parseAndPrepareImportedDocumentMock = vi.fn();

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => navigateMock,
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({
      newDocument: newDocumentMock,
      loadDocument: loadDocumentMock,
    }),
  },
}));

vi.mock('@/utils/import-pen-document', () => ({
  parseAndPrepareImportedDocument: (...args: unknown[]) => parseAndPrepareImportedDocumentMock(...args),
}));

import { DesktopStartPage } from './desktop-start-page';

afterEach(() => {
  cleanup();
  navigateMock.mockReset();
  newDocumentMock.mockReset();
  loadDocumentMock.mockReset();
  parseAndPrepareImportedDocumentMock.mockReset();
  delete (window as unknown as Record<string, unknown>).electronAPI;
});

describe('DesktopStartPage', () => {
  it('routes to the cloud file library', () => {
    render(<DesktopStartPage />);

    fireEvent.click(screen.getByRole('button', { name: /Open Cloud Files/i }));

    expect(navigateMock).toHaveBeenCalledWith({ to: '/cloud' });
  });

  it('creates a local untitled document and opens the local editor', () => {
    render(<DesktopStartPage />);

    fireEvent.click(screen.getByRole('button', { name: /New Local File/i }));

    expect(newDocumentMock).toHaveBeenCalled();
    expect(navigateMock).toHaveBeenCalledWith({ to: '/editor/local' });
  });

  it('opens a selected local .op file without importing it to cloud', async () => {
    const doc = { version: '1.0.0', pages: [{ id: 'page-1', name: 'Page 1', children: [] }] };
    parseAndPrepareImportedDocumentMock.mockReturnValue({ doc });
    (window as unknown as { electronAPI: Partial<ElectronAPI> }).electronAPI = {
      isElectron: true,
      openFile: vi.fn(async () => ({
        filePath: '/tmp/local-design.op',
        content: JSON.stringify(doc),
      })),
    };

    render(<DesktopStartPage />);

    fireEvent.click(screen.getByRole('button', { name: /Open Local File/i }));

    await waitFor(() => {
      expect(loadDocumentMock).toHaveBeenCalledWith(doc, 'local-design.op', null, '/tmp/local-design.op');
      expect(navigateMock).toHaveBeenCalledWith({ to: '/editor/local' });
    });
  });
});
