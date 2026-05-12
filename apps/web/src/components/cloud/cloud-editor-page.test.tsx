// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CloudFileRecord } from '@/types/cloud';

const authState = vi.hoisted(() => ({
  status: 'authenticated',
  initialized: true,
  initialize: vi.fn(async () => undefined),
}));

const documentMocks = vi.hoisted(() => ({
  loadCloudDocument: vi.fn(),
}));

const cloudFileMocks = vi.hoisted(() => ({
  getCloudFile: vi.fn(),
}));

vi.mock('@/components/editor/editor-layout', () => ({
  default: () => <div>Editor Ready</div>,
}));

vi.mock('@/hooks/use-keyboard-shortcuts', () => ({
  useKeyboardShortcuts: vi.fn(),
}));

vi.mock('@/hooks/use-before-unload', () => ({
  useBeforeUnload: vi.fn(),
}));

vi.mock('@/stores/cloud-auth-store', () => ({
  useCloudAuthStore: (selector: (state: typeof authState) => unknown) => selector(authState),
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => documentMocks,
  },
}));

vi.mock('@/services/cloud/cloud-files', () => cloudFileMocks);

import { CloudEditorPage } from './cloud-editor-page';

const cloudFile: CloudFileRecord = {
  id: 'file-1',
  projectId: 'project-1',
  folderId: null,
  name: 'Cloud Design',
  thumbnailPath: null,
  revision: 5,
  metadata: {},
  starred: false,
  lastOpenedAt: null,
  deletedAt: null,
  createdAt: '2026-05-11T08:00:00.000Z',
  updatedAt: '2026-05-12T08:00:00.000Z',
  document: {
    version: '1.0.0',
    pages: [{ id: 'page-1', name: 'Page 1', children: [] }],
    children: [],
  },
};

const navigate = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  authState.status = 'authenticated';
  authState.initialized = true;
  cloudFileMocks.getCloudFile.mockResolvedValue(cloudFile);
});

afterEach(() => {
  cleanup();
});

describe('CloudEditorPage', () => {
  it('loads a cloud file into the document store before showing the editor', async () => {
    render(<CloudEditorPage fileId="file-1" navigate={navigate} />);

    expect(screen.getByText('Loading file...')).toBeTruthy();

    await waitFor(() => {
      expect(cloudFileMocks.getCloudFile).toHaveBeenCalledWith('file-1');
      expect(documentMocks.loadCloudDocument).toHaveBeenCalledWith(cloudFile);
      expect(screen.getByText('Editor Ready')).toBeTruthy();
    });
  });

  it('redirects anonymous users back to the entry page', async () => {
    authState.status = 'anonymous';

    render(<CloudEditorPage fileId="file-1" navigate={navigate} />);

    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith({ to: '/' });
    });
    expect(cloudFileMocks.getCloudFile).not.toHaveBeenCalled();
  });

  it('offers a direct return to the cloud library when loading fails', async () => {
    cloudFileMocks.getCloudFile.mockRejectedValueOnce(new Error('missing file'));

    render(<CloudEditorPage fileId="file-1" navigate={navigate} />);

    expect(await screen.findByText('missing file')).toBeTruthy();
    screen.getByRole('button', { name: /Back to files/i }).click();

    expect(navigate).toHaveBeenCalledWith({ to: '/cloud' });
  });
});
