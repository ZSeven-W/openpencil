import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useDocumentStore, createEmptyDocument } from '@/stores/document-store';
import { CloudApiError } from '@/services/cloud/cloud-fetch';
import {
  createCloudFile,
  getCloudFile,
  saveCloudFile,
  saveCloudFilePatches,
} from '@/services/cloud/cloud-files';

vi.mock('@/services/cloud/cloud-files', () => ({
  createCloudFile: vi.fn(),
  getCloudFile: vi.fn(),
  saveCloudFile: vi.fn(),
  saveCloudFilePatches: vi.fn(),
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
      cloudBaseDocument: createEmptyDocument(),
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
      cloudBaseDocument: null,
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
    let resolveSecond!: (file: {
      id: string;
      name: string;
      revision: number;
      updatedAt: string;
      checkpointRevision: number;
      snapshotCreated: boolean;
    }) => void;
    vi.mocked(saveCloudFilePatches)
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
      name: 'Design',
      revision: 8,
      updatedAt: '2026-01-01T00:00:00.000Z',
      checkpointRevision: 8,
      snapshotCreated: true,
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
    expect(state.cloudSaveState).toBe('saved');
    expect(state.cloudSaveError).toBeNull();
    expect(state.cloudSaveConflict).toBeNull();
  });

  it('surfaces a real revision conflict when no newer local save has succeeded', async () => {
    vi.mocked(saveCloudFilePatches).mockRejectedValueOnce(
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
    expect(saveCloudFilePatches).not.toHaveBeenCalled();
    expect(useDocumentStore.getState()).toMatchObject({
      cloudShareRole: 'viewer',
      cloudSaveState: 'error',
      cloudSaveError: 'View-only shared files cannot be saved',
    });
  });

  it('does not overwrite newer local edits when an older save response returns', async () => {
    let resolveSave!: (file: {
      id: string;
      name: string;
      revision: number;
      updatedAt: string;
      checkpointRevision: number;
      snapshotCreated: boolean;
    }) => void;
    const submitted = createEmptyDocument();
    submitted.name = 'Submitted';
    const edited = createEmptyDocument();
    edited.name = 'Edited after save started';
    useDocumentStore.setState({ document: submitted, isDirty: true });
    vi.mocked(saveCloudFilePatches).mockReturnValueOnce(
      new Promise((resolve) => {
        resolveSave = resolve;
      }),
    );

    const save = useDocumentStore.getState().saveCloud('manual_save', 'Manual save', true);
    useDocumentStore.setState({ document: edited, isDirty: true });

    resolveSave({
      id: 'file-1',
      name: 'Design',
      revision: 8,
      updatedAt: '2026-01-01T00:00:00.000Z',
      checkpointRevision: 8,
      snapshotCreated: true,
    });
    await expect(save).resolves.toBe('Design');

    const state = useDocumentStore.getState();
    expect(state.document.name).toBe('Edited after save started');
    expect(state.isDirty).toBe(true);
    expect(state.cloudRevision).toBe(8);
    expect(state.cloudSaveState).toBe('saved');
    expect(state.cloudSaveConflict).toBeNull();
  });

  it('sends autosave document patches instead of the full document', async () => {
    const base = createEmptyDocument();
    const current = createEmptyDocument();
    current.name = 'Edited';
    useDocumentStore.setState({
      document: current,
      cloudBaseDocument: base,
      cloudRevision: 7,
      isDirty: true,
    });
    vi.mocked(saveCloudFilePatches).mockResolvedValueOnce({
      id: 'file-1',
      name: 'Design',
      revision: 8,
      updatedAt: '2026-01-01T00:00:00.000Z',
      checkpointRevision: 7,
      snapshotCreated: false,
    });

    await expect(useDocumentStore.getState().saveCloud('autosave', undefined, false)).resolves.toBe(
      'Design',
    );

    expect(saveCloudFile).not.toHaveBeenCalled();
    expect(saveCloudFilePatches).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'file-1',
        baseRevision: 7,
        source: 'autosave',
        snapshot: false,
        patches: expect.arrayContaining([
          expect.objectContaining({ op: 'set-doc-field', field: 'name', value: 'Edited' }),
        ]),
      }),
    );
    expect(useDocumentStore.getState()).toMatchObject({
      cloudRevision: 8,
      cloudBaseDocument: current,
      isDirty: false,
    });
  });

  it('maps large Figma autosave string allocation failures to a cloud size message', async () => {
    const stringify = vi.spyOn(JSON, 'stringify').mockImplementationOnce(() => {
      throw new RangeError('Invalid string length');
    });

    await expect(useDocumentStore.getState().saveCloud('autosave', undefined, false)).resolves.toBeNull();

    expect(saveCloudFilePatches).not.toHaveBeenCalled();
    expect(useDocumentStore.getState().cloudSaveState).toBe('error');
    expect(useDocumentStore.getState().cloudSaveError).toContain(
      'This design is too large to send to the cloud API',
    );

    stringify.mockRestore();
  });

  it('merges remote changes and retries once after a patch revision conflict', async () => {
    const base = createEmptyDocument();
    base.pages![0].children = [
      { id: 'ours', type: 'rectangle', width: 10, height: 10 },
    ] as any;
    const ours = createEmptyDocument();
    ours.pages![0].children = [
      { id: 'ours', type: 'rectangle', width: 20, height: 10 },
    ] as any;
    const theirs = createEmptyDocument();
    theirs.pages![0].children = [
      { id: 'ours', type: 'rectangle', width: 10, height: 10 },
      { id: 'theirs', type: 'rectangle', width: 8, height: 8 },
    ] as any;
    useDocumentStore.setState({
      document: ours,
      cloudBaseDocument: base,
      cloudRevision: 7,
      isDirty: true,
    });
    vi.mocked(saveCloudFilePatches)
      .mockRejectedValueOnce(
        new CloudApiError(409, 'revision_conflict', 'Cloud file has a newer revision', {
          serverRevision: 8,
        }),
      )
      .mockResolvedValueOnce({
        id: 'file-1',
        name: 'Design',
        revision: 9,
        updatedAt: '2026-01-01T00:00:00.000Z',
        checkpointRevision: 9,
        snapshotCreated: true,
      });
    vi.mocked(getCloudFile).mockResolvedValueOnce({
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
      document: theirs,
    });

    await expect(
      useDocumentStore.getState().saveCloud('manual_save', 'Manual save', true),
    ).resolves.toBe('Design');

    expect(saveCloudFilePatches).toHaveBeenCalledTimes(2);
    expect(saveCloudFilePatches).toHaveBeenLastCalledWith(
      expect.objectContaining({ baseRevision: 8 }),
    );
    expect(useDocumentStore.getState().document.pages?.[0].children.map((node) => node.id)).toEqual([
      'ours',
      'theirs',
    ]);
    expect(useDocumentStore.getState().cloudRevision).toBe(9);
  });
});
