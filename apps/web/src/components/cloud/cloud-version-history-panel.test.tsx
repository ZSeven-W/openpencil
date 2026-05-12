// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const documentState = {
  cloudFileId: 'file-1',
  cloudRevision: 5,
  cloudShareRole: null as 'viewer' | 'editor' | null,
  loadCloudDocument: vi.fn(),
};

const panelState = {
  open: true,
  setOpen: vi.fn((open: boolean) => {
    panelState.open = open;
  }),
};

const cloudFileMocks = vi.hoisted(() => ({
  listCloudFileActivity: vi.fn(),
  listCloudFileVersions: vi.fn(),
  restoreCloudFileVersion: vi.fn(),
  updateCloudFileVersionLabel: vi.fn(),
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: Object.assign(
    (selector: (state: typeof documentState) => unknown) => selector(documentState),
    { getState: () => documentState },
  ),
}));

vi.mock('@/stores/cloud-version-panel-store', () => ({
  useCloudVersionPanelStore: Object.assign(
    (selector: (state: typeof panelState) => unknown) => selector(panelState),
    { getState: () => panelState },
  ),
}));

vi.mock('@/services/cloud/cloud-files', () => cloudFileMocks);

import { CloudVersionHistoryPanel } from './cloud-version-history-panel';

beforeEach(() => {
  vi.clearAllMocks();
  panelState.open = true;
  documentState.cloudFileId = 'file-1';
  documentState.cloudRevision = 5;
  documentState.cloudShareRole = null;
  cloudFileMocks.listCloudFileVersions.mockResolvedValue([
    {
      id: 'version-5',
      fileId: 'file-1',
      revision: 5,
      source: 'manual_save',
      label: 'Manual save',
      actorId: 'user-1',
      sizeBytes: 2048,
      createdAt: '2026-05-12T08:00:00.000Z',
    },
    {
      id: 'version-3',
      fileId: 'file-1',
      revision: 3,
      source: 'autosave',
      label: null,
      actorId: null,
      sizeBytes: 512,
      createdAt: '2026-05-12T07:00:00.000Z',
    },
  ]);
  cloudFileMocks.listCloudFileActivity.mockResolvedValue({
    data: [
      {
        id: 'activity-1',
        fileId: 'file-1',
        generationId: null,
        actorId: 'user-1',
        ownerId: 'user-1',
        type: 'file_saved',
        metadata: { revision: 5 },
        createdAt: '2026-05-12T08:01:00.000Z',
      },
    ],
    nextCursor: '2026-05-12T08:00:00.000Z',
    limit: 20,
  });
  cloudFileMocks.restoreCloudFileVersion.mockResolvedValue({
    id: 'file-1',
    name: 'Cloud File',
    revision: 6,
    document: { version: '1.0.0', pages: [] },
  });
  cloudFileMocks.updateCloudFileVersionLabel.mockResolvedValue({
    id: 'version-3',
    fileId: 'file-1',
    revision: 3,
    source: 'autosave',
    label: 'Checkpoint',
    actorId: 'user-1',
    sizeBytes: 512,
    createdAt: '2026-05-12T07:00:00.000Z',
  });
});

afterEach(() => cleanup());

describe('CloudVersionHistoryPanel', () => {
  it('loads and displays cloud versions for the current file', async () => {
    render(<CloudVersionHistoryPanel />);

    expect(screen.getByRole('complementary', { name: 'Version history' })).toBeTruthy();
    await waitFor(() => expect(screen.getAllByText('rev 5').length).toBeGreaterThanOrEqual(2));
    expect(screen.getByText('Manual save')).toBeTruthy();
    expect(screen.getByText('autosave')).toBeTruthy();
    expect(screen.getByText(/2 KB/)).toBeTruthy();
    expect(await screen.findByText('file saved')).toBeTruthy();
    expect(screen.getByText('Current rev 5')).toBeTruthy();
    expect(cloudFileMocks.listCloudFileVersions).toHaveBeenCalledWith('file-1');
    expect(cloudFileMocks.listCloudFileActivity).toHaveBeenCalledWith('file-1', {
      type: 'all',
      cursor: null,
      limit: 20,
    });
  });

  it('restores a selected version into the document store', async () => {
    render(<CloudVersionHistoryPanel />);

    expect(await screen.findByText('rev 3')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Restore rev 3' }));

    await waitFor(() => {
      expect(cloudFileMocks.restoreCloudFileVersion).toHaveBeenCalledWith({
        fileId: 'file-1',
        versionId: 'version-3',
      });
      expect(documentState.loadCloudDocument).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'file-1', revision: 6 }),
      );
    });
  });

  it('closes through the panel store', () => {
    render(<CloudVersionHistoryPanel />);

    fireEvent.click(screen.getByRole('button', { name: 'Close version history' }));

    expect(panelState.setOpen).toHaveBeenCalledWith(false);
  });

  it('updates a version label from the history panel', async () => {
    render(<CloudVersionHistoryPanel />);

    expect(await screen.findByText('rev 3')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Edit label rev 3' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Label rev 3' }), {
      target: { value: 'Checkpoint' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save label rev 3' }));

    await waitFor(() => {
      expect(cloudFileMocks.updateCloudFileVersionLabel).toHaveBeenCalledWith({
        fileId: 'file-1',
        versionId: 'version-3',
        label: 'Checkpoint',
      });
    });
  });

  it('shows shared viewer history as read-only', async () => {
    documentState.cloudShareRole = 'viewer';

    render(<CloudVersionHistoryPanel />);

    expect(await screen.findByText('View-only shared file')).toBeTruthy();
    expect(screen.getByText('Manual save')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Edit label rev 3' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Restore rev 3' })).toBeNull();
  });

  it('filters activity and loads the next activity page', async () => {
    cloudFileMocks.listCloudFileActivity
      .mockResolvedValueOnce({
        data: [
          {
            id: 'activity-1',
            fileId: 'file-1',
            generationId: null,
            actorId: 'user-1',
            ownerId: 'user-1',
            type: 'file_saved',
            metadata: { revision: 5 },
            createdAt: '2026-05-12T08:01:00.000Z',
          },
        ],
        nextCursor: '2026-05-12T08:00:00.000Z',
        limit: 20,
      })
      .mockResolvedValueOnce({
        data: [
          {
            id: 'activity-2',
            fileId: 'file-1',
            generationId: null,
            actorId: 'user-1',
            ownerId: 'user-1',
            type: 'file_saved',
            metadata: { revision: 4 },
            createdAt: '2026-05-12T07:00:00.000Z',
          },
        ],
        nextCursor: null,
        limit: 20,
      })
      .mockResolvedValueOnce({
        data: [],
        nextCursor: null,
        limit: 20,
      });

    render(<CloudVersionHistoryPanel />);

    expect(await screen.findByText('file saved')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Load more activity' }));

    await waitFor(() => {
      expect(cloudFileMocks.listCloudFileActivity).toHaveBeenCalledWith('file-1', {
        type: 'all',
        cursor: '2026-05-12T08:00:00.000Z',
        limit: 20,
      });
      expect(screen.getByText('rev 4')).toBeTruthy();
    });

    fireEvent.change(screen.getByRole('combobox', { name: 'Filter activity' }), {
      target: { value: 'file_shared' },
    });

    await waitFor(() => {
      expect(cloudFileMocks.listCloudFileActivity).toHaveBeenCalledWith('file-1', {
        type: 'file_shared',
        cursor: null,
        limit: 20,
      });
    });
  });

  it('does not render while closed or without a cloud file', () => {
    panelState.open = false;
    const { container, rerender } = render(<CloudVersionHistoryPanel />);
    expect(container.firstChild).toBeNull();

    panelState.open = true;
    documentState.cloudFileId = null as unknown as string;
    rerender(<CloudVersionHistoryPanel />);
    expect(container.firstChild).toBeNull();
  });
});
