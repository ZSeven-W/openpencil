import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  copyCloudFile,
  createCloudFolder,
  createCloudFileShare,
  listCloudFileVersions,
  createCloudProject,
  listCloudFileShares,
  listCloudFiles,
  listCloudFolders,
  permanentlyDeleteCloudFile,
  restoreCloudFile,
  restoreCloudFileVersion,
  revokeCloudFileShare,
  saveCloudFile,
  listCloudFileActivity,
  updateCloudFileVersionLabel,
  updateCloudFileMetadata,
} from '../cloud-files';

const cloudFetchMock = vi.hoisted(() => vi.fn());

vi.mock('../cloud-fetch', () => ({
  cloudFetch: cloudFetchMock,
}));

describe('cloud file service', () => {
  beforeEach(() => {
    cloudFetchMock.mockReset();
  });

  it('serializes list filters for the full cloud library views', async () => {
    cloudFetchMock.mockResolvedValueOnce({ data: [] });

    await listCloudFiles({
      projectId: 'project-1',
      folderId: null,
      view: 'starred',
      search: 'dashboard',
      sort: 'name_asc',
      limit: 25,
    });

    expect(cloudFetchMock).toHaveBeenCalledWith(
      '/api/cloud/files?projectId=project-1&folderId=null&view=starred&search=dashboard&sort=name_asc&limit=25',
    );
  });

  it('calls project and folder management APIs', async () => {
    cloudFetchMock.mockResolvedValue({ data: { id: 'ok' } });

    await createCloudProject({ name: 'Mobile App', color: '#3b82f6' });
    await listCloudFolders({ projectId: 'project-1', parentId: null });
    await createCloudFolder({ projectId: 'project-1', parentId: null, name: 'Flows' });

    expect(cloudFetchMock).toHaveBeenNthCalledWith(1, '/api/cloud/projects', {
      method: 'POST',
      body: JSON.stringify({ name: 'Mobile App', color: '#3b82f6' }),
    });
    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      2,
      '/api/cloud/folders?projectId=project-1&parentId=null',
    );
    expect(cloudFetchMock).toHaveBeenNthCalledWith(3, '/api/cloud/folders', {
      method: 'POST',
      body: JSON.stringify({ projectId: 'project-1', parentId: null, name: 'Flows' }),
    });
  });

  it('calls file metadata, copy, restore, and permanent delete APIs', async () => {
    cloudFetchMock.mockResolvedValue({ data: { id: 'file-1' } });

    await updateCloudFileMetadata({ id: 'file-1', name: 'Renamed', starred: true });
    await copyCloudFile({ id: 'file-1', name: 'Copy' });
    await restoreCloudFile('file-1');
    await permanentlyDeleteCloudFile('file-1');

    expect(cloudFetchMock).toHaveBeenNthCalledWith(1, '/api/cloud/files/file-1', {
      method: 'PATCH',
      body: JSON.stringify({ id: 'file-1', name: 'Renamed', starred: true }),
    });
    expect(cloudFetchMock).toHaveBeenNthCalledWith(2, '/api/cloud/files/file-1/copy', {
      method: 'POST',
      body: JSON.stringify({ id: 'file-1', name: 'Copy' }),
    });
    expect(cloudFetchMock).toHaveBeenNthCalledWith(3, '/api/cloud/files/file-1/restore', {
      method: 'POST',
    });
    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      4,
      '/api/cloud/files/file-1/permanent-delete',
      { method: 'DELETE' },
    );
  });

  it('sends expectedRevision when saving a cloud document', async () => {
    cloudFetchMock.mockResolvedValueOnce({ data: { id: 'file-1' } });
    const document = { version: '1.0.0', name: 'Design', children: [] } as any;

    await saveCloudFile({
      id: 'file-1',
      name: 'Design',
      document,
      expectedRevision: 7,
      source: 'manual_save',
      label: 'Manual save',
      snapshot: true,
      force: true,
    });

    const [, request] = cloudFetchMock.mock.calls[0] ?? [];
    expect(cloudFetchMock).toHaveBeenCalledWith('/api/cloud/files/file-1', {
      method: 'PATCH',
      body: expect.any(String),
    });
    expect(JSON.parse((request as RequestInit).body as string)).toMatchObject({
      id: 'file-1',
      name: 'Design',
      document,
      expectedRevision: 7,
      source: 'manual_save',
      label: 'Manual save',
      snapshot: true,
      force: true,
    });
  });

  it('calls cloud file sharing APIs', async () => {
    cloudFetchMock.mockResolvedValue({ data: [{ id: 'share-1' }] });

    await listCloudFileShares('file-1');
    await createCloudFileShare({ fileId: 'file-1', email: 'bob@example.com', role: 'editor' });
    await revokeCloudFileShare({ fileId: 'file-1', shareId: 'share-1' });

    expect(cloudFetchMock).toHaveBeenNthCalledWith(1, '/api/cloud/files/file-1/shares');
    expect(cloudFetchMock).toHaveBeenNthCalledWith(2, '/api/cloud/files/file-1/shares', {
      method: 'POST',
      body: JSON.stringify({ fileId: 'file-1', email: 'bob@example.com', role: 'editor' }),
    });
    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      3,
      '/api/cloud/files/file-1/shares/share-1',
      { method: 'DELETE' },
    );
  });

  it('lists and restores cloud file versions', async () => {
    cloudFetchMock.mockResolvedValue({ data: [{ id: 'version-1', revision: 3 }] });

    await listCloudFileVersions('file-1');
    await restoreCloudFileVersion({ fileId: 'file-1', versionId: 'version-1' });
    await updateCloudFileVersionLabel({
      fileId: 'file-1',
      versionId: 'version-1',
      label: 'Approved',
    });
    await listCloudFileActivity('file-1', {
      type: 'file_saved',
      cursor: '2026-05-12T08:00:00.000Z',
      limit: 10,
    });

    expect(cloudFetchMock).toHaveBeenNthCalledWith(1, '/api/cloud/files/file-1/versions');
    expect(cloudFetchMock).toHaveBeenNthCalledWith(2, '/api/cloud/files/file-1/restore-version', {
      method: 'POST',
      body: JSON.stringify({ versionId: 'version-1' }),
    });
    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      3,
      '/api/cloud/files/file-1/versions/version-1',
      {
        method: 'PATCH',
        body: JSON.stringify({ label: 'Approved' }),
      },
    );
    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      4,
      '/api/cloud/files/file-1/activity?type=file_saved&cursor=2026-05-12T08%3A00%3A00.000Z&limit=10',
    );
  });
});
