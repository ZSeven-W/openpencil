// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const documentState = {
  cloudFileId: 'file-1',
  cloudSaveState: 'conflict',
  cloudSaveError: 'Cloud file has a newer revision',
  cloudSaveConflict: {
    code: 'revision_conflict' as const,
    fileId: 'file-1',
    expectedRevision: 7,
    serverRevision: 9,
  },
  fileName: 'Cloud File',
  document: { version: '1.0.0', pages: [] },
  saveCloud: vi.fn(),
  loadCloudDocument: vi.fn(),
  clearCloudError: vi.fn(),
};

const cloudFileMocks = vi.hoisted(() => ({
  createCloudFile: vi.fn(),
  getCloudFile: vi.fn(),
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: Object.assign(
    (selector: (state: typeof documentState) => unknown) => selector(documentState),
    { getState: () => documentState },
  ),
}));

vi.mock('@/services/cloud/cloud-files', () => cloudFileMocks);

import { CloudSaveConflictDialog } from './cloud-save-conflict-dialog';

beforeEach(() => {
  vi.clearAllMocks();
  documentState.cloudFileId = 'file-1';
  documentState.cloudSaveState = 'conflict';
  documentState.cloudSaveError = 'Cloud file has a newer revision';
  documentState.cloudSaveConflict = {
    code: 'revision_conflict',
    fileId: 'file-1',
    expectedRevision: 7,
    serverRevision: 9,
  };
  cloudFileMocks.getCloudFile.mockResolvedValue({
    id: 'file-1',
    name: 'Cloud File',
    revision: 9,
    document: { version: '1.0.0', pages: [] },
  });
  cloudFileMocks.createCloudFile.mockResolvedValue({ id: 'copy-file-1' });
  documentState.saveCloud.mockResolvedValue('Cloud File');
});

afterEach(() => cleanup());

describe('CloudSaveConflictDialog', () => {
  it('shows revision conflict details and reloads the remote document', async () => {
    render(<CloudSaveConflictDialog />);

    expect(screen.getByRole('dialog', { name: 'Save conflict' })).toBeTruthy();
    expect(screen.getByText('Local rev 7')).toBeTruthy();
    expect(screen.getByText('Remote rev 9')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Reload remote' }));

    await waitFor(() => {
      expect(cloudFileMocks.getCloudFile).toHaveBeenCalledWith('file-1');
      expect(documentState.loadCloudDocument).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'file-1', revision: 9 }),
      );
    });
  });

  it('saves the local conflicted document as a new copy', async () => {
    const onSavedCopy = vi.fn();
    render(<CloudSaveConflictDialog onSavedCopy={onSavedCopy} />);

    fireEvent.click(screen.getByRole('button', { name: 'Save as copy' }));

    await waitFor(() => {
      expect(cloudFileMocks.createCloudFile).toHaveBeenCalledWith({
        name: 'Cloud File copy',
        document: documentState.document,
        source: 'manual_save',
      });
      expect(documentState.clearCloudError).toHaveBeenCalled();
      expect(onSavedCopy).toHaveBeenCalledWith('copy-file-1');
    });
  });

  it('can force overwrite the remote cloud file', async () => {
    render(<CloudSaveConflictDialog />);

    fireEvent.click(screen.getByRole('button', { name: 'Overwrite remote' }));

    await waitFor(() => {
      expect(documentState.saveCloud).toHaveBeenCalledWith(
        'manual_save',
        'Forced overwrite',
        true,
        { force: true },
      );
    });
  });

  it('does not render when there is no active conflict', () => {
    documentState.cloudSaveState = 'idle';
    const { container } = render(<CloudSaveConflictDialog />);

    expect(container.firstChild).toBeNull();
  });
});
