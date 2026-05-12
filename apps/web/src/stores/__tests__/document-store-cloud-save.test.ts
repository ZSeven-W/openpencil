import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useDocumentStore, createEmptyDocument } from '@/stores/document-store';
import { CloudApiError } from '@/services/cloud/cloud-fetch';
import { createCloudFile, saveCloudFile } from '@/services/cloud/cloud-files';
import type { CloudFileRecord } from '@/types/cloud';
import type { PenDocument } from '@/types/pen';

vi.mock('@/services/cloud/cloud-files', () => ({
  createCloudFile: vi.fn(),
  saveCloudFile: vi.fn(),
}));

vi.mock('@/stores/cloud-auth-store', () => ({
  useCloudAuthStore: {
    getState: () => ({
      status: 'authenticated',
    }),
  },
}));

describe('useDocumentStore.saveCloud()', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDocumentStore.setState({
      document: createEmptyDocument(),
      fileName: 'Design',
      fileHandle: null,
      filePath: null,
      isDirty: true,
      cloudFileId: 'file-1',
      cloudRevision: 7,
      cloudShareRole: null,
      cloudSaveState: 'idle',
      cloudSaveError: null,
      cloudSaveConflict: null,
    });
  });

  it('creates a cloud file when saving an authenticated document without cloud metadata', async () => {
    const document = createEmptyDocument();
    vi.mocked(createCloudFile).mockResolvedValueOnce({
      id: 'file-2',
      projectId: 'project-1',
      folderId: null,
      name: 'Untitled',
      thumbnailPath: null,
      revision: 1,
      metadata: {},
      starred: false,
      lastOpenedAt: null,
      deletedAt: null,
      createdAt: '2026-01-01T00:00:00.000Z',
      updatedAt: '2026-01-01T00:00:00.000Z',
      document,
    });
    useDocumentStore.setState({
      document,
      fileName: null,
      fileHandle: null,
      filePath: null,
      isDirty: true,
      cloudFileId: null,
      cloudRevision: null,
      cloudShareRole: null,
      cloudSaveState: 'idle',
      cloudSaveError: null,
      cloudSaveConflict: null,
    });

    await expect(useDocumentStore.getState().save()).resolves.toBe('Untitled');

    expect(createCloudFile).toHaveBeenCalledWith({
      name: 'Untitled',
      document,
      source: 'manual_save',
    });
    expect(useDocumentStore.getState()).toMatchObject({
      fileName: 'Untitled',
      fileHandle: null,
      filePath: null,
      isDirty: false,
      cloudFileId: 'file-2',
      cloudRevision: 1,
      cloudShareRole: null,
      cloudSaveState: 'idle',
      cloudSaveError: null,
    });
  });

  it('ignores stale conflict responses from an older in-flight save after a newer save succeeds', async () => {
    let rejectFirst!: (error: Error) => void;
    let resolveSecond!: (file: CloudFileRecord) => void;
    vi.mocked(saveCloudFile)
      .mockReturnValueOnce(
        new Promise((_, reject) => {
          rejectFirst = reject;
        }),
      )
      .mockReturnValueOnce(
        new Promise((resolve) => {
          resolveSecond = resolve;
        }),
      );

    const first = useDocumentStore.getState().saveCloud('autosave', undefined, false);
    const second = useDocumentStore.getState().saveCloud('manual_save', 'Manual save', true);

    resolveSecond({
      id: 'file-1',
      projectId: 'project-1',
      folderId: null,
      name: 'Design',
      thumbnailPath: null,
      revision: 8,
      shareRole: 'editor',
      metadata: {},
      starred: false,
      lastOpenedAt: null,
      deletedAt: null,
      createdAt: '2026-01-01T00:00:00.000Z',
      updatedAt: '2026-01-01T00:00:00.000Z',
      document: createEmptyDocument(),
    });
    await expect(second).resolves.toBe('Design');

    rejectFirst(
      new CloudApiError(409, 'revision_conflict', 'Cloud file has a newer revision', {
        serverRevision: 8,
      }),
    );
    await expect(first).resolves.toBeNull();

    const state = useDocumentStore.getState();
    expect(state.cloudRevision).toBe(8);
    expect(state.cloudShareRole).toBe('editor');
    expect(state.cloudSaveState).toBe('saved');
    expect(state.cloudSaveError).toBeNull();
    expect(state.cloudSaveConflict).toBeNull();
  });

  it('surfaces a real revision conflict when no newer local save has succeeded', async () => {
    vi.mocked(saveCloudFile).mockRejectedValueOnce(
      new CloudApiError(409, 'revision_conflict', 'Cloud file has a newer revision', {
        fileId: 'file-1',
        expectedRevision: 7,
        serverRevision: 9,
      }),
    );

    await expect(
      useDocumentStore.getState().saveCloud('manual_save', 'Manual save', true),
    ).resolves.toBeNull();

    const state = useDocumentStore.getState();
    expect(state.cloudRevision).toBe(7);
    expect(state.cloudSaveState).toBe('conflict');
    expect(state.cloudSaveError).toBe('Cloud file has a newer revision');
    expect(state.cloudSaveConflict).toEqual({
      code: 'revision_conflict',
      fileId: 'file-1',
      expectedRevision: 7,
      serverRevision: 9,
    });
  });

  it('blocks saving view-only shared cloud files before calling the API', async () => {
    const document = createEmptyDocument();
    useDocumentStore.getState().loadCloudDocument({
      id: 'file-1',
      projectId: 'project-1',
      folderId: null,
      name: 'Shared Design',
      thumbnailPath: null,
      revision: 7,
      metadata: {},
      starred: false,
      lastOpenedAt: null,
      deletedAt: null,
      createdAt: '2026-01-01T00:00:00.000Z',
      updatedAt: '2026-01-01T00:00:00.000Z',
      shareRole: 'viewer',
      document,
    });

    await expect(
      useDocumentStore.getState().saveCloud('manual_save', 'Manual save', true),
    ).resolves.toBeNull();

    expect(saveCloudFile).not.toHaveBeenCalled();
    expect(useDocumentStore.getState()).toMatchObject({
      cloudShareRole: 'viewer',
      cloudSaveState: 'error',
      cloudSaveError: 'View-only shared files cannot be saved',
    });
  });

  it('does not overwrite newer local edits when an older save response returns', async () => {
    let resolveSave!: (file: CloudFileRecord) => void;
    const submitted = createEmptyDocument();
    submitted.name = 'Submitted';
    const edited = createEmptyDocument();
    edited.name = 'Edited after save started';
    useDocumentStore.setState({ document: submitted, isDirty: true });
    vi.mocked(saveCloudFile).mockReturnValueOnce(
      new Promise((resolve) => {
        resolveSave = resolve;
      }),
    );

    const save = useDocumentStore.getState().saveCloud('manual_save', 'Manual save', true);
    useDocumentStore.setState({ document: edited, isDirty: true });

    resolveSave({
      id: 'file-1',
      projectId: 'project-1',
      folderId: null,
      name: 'Design',
      thumbnailPath: null,
      revision: 8,
      metadata: {},
      starred: false,
      lastOpenedAt: null,
      deletedAt: null,
      createdAt: '2026-01-01T00:00:00.000Z',
      updatedAt: '2026-01-01T00:00:00.000Z',
      document: submitted as PenDocument,
    });
    await expect(save).resolves.toBe('Design');

    const state = useDocumentStore.getState();
    expect(state.document.name).toBe('Edited after save started');
    expect(state.isDirty).toBe(true);
    expect(state.cloudRevision).toBe(8);
    expect(state.cloudSaveState).toBe('saved');
    expect(state.cloudSaveConflict).toBeNull();
  });
});
