import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useDocumentStore, createEmptyDocument } from '@/stores/document-store';
import { CloudApiError } from '@/services/cloud/cloud-fetch';
import { saveCloudFile } from '@/services/cloud/cloud-files';
import type { CloudFileRecord } from '@/types/cloud';
import type { PenDocument } from '@/types/pen';

vi.mock('@/services/cloud/cloud-files', () => ({
  createCloudFile: vi.fn(),
  saveCloudFile: vi.fn(),
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
      name: 'Design',
      thumbnailPath: null,
      revision: 8,
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
    expect(state.cloudSaveState).toBe('saved');
    expect(state.cloudSaveError).toBeNull();
  });

  it('surfaces a real revision conflict when no newer local save has succeeded', async () => {
    vi.mocked(saveCloudFile).mockRejectedValueOnce(
      new CloudApiError(409, 'revision_conflict', 'Cloud file has a newer revision', {
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
      name: 'Design',
      thumbnailPath: null,
      revision: 8,
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
  });
});
