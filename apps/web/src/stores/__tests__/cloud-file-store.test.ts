import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CloudFileRecord, CloudFileSummary, CloudFolder, CloudProject } from '@/types/cloud';
import type { PenDocument } from '@/types/pen';

const cloudFileMocks = vi.hoisted(() => ({
  listCloudProjects: vi.fn(),
  createCloudProject: vi.fn(),
  updateCloudProject: vi.fn(),
  deleteCloudProject: vi.fn(),
  listCloudFolders: vi.fn(),
  createCloudFolder: vi.fn(),
  updateCloudFolder: vi.fn(),
  deleteCloudFolder: vi.fn(),
  listCloudFiles: vi.fn(),
  createCloudFile: vi.fn(),
  updateCloudFileMetadata: vi.fn(),
  createCloudFileShare: vi.fn(),
  revokeCloudFileShare: vi.fn(),
  copyCloudFile: vi.fn(),
  deleteCloudFile: vi.fn(),
  restoreCloudFile: vi.fn(),
  permanentlyDeleteCloudFile: vi.fn(),
}));

vi.mock('@/services/cloud/cloud-files', () => cloudFileMocks);

import { useCloudFileStore } from '@/stores/cloud-file-store';

const baseSummary: CloudFileSummary = {
  id: 'file-1',
  projectId: 'project-1',
  folderId: null,
  name: 'Dashboard',
  thumbnailPath: null,
  revision: 3,
  metadata: {},
  starred: false,
  lastOpenedAt: null,
  deletedAt: null,
  createdAt: '2026-05-11T08:00:00.000Z',
  updatedAt: '2026-05-12T08:00:00.000Z',
};

const document: PenDocument = {
  version: '1.0.0',
  pages: [{ id: 'page-1', name: 'Page 1', children: [] }],
  children: [],
};

const project: CloudProject = {
  id: 'project-1',
  name: 'Mobile App',
  description: null,
  icon: null,
  color: null,
  createdAt: '2026-05-11T08:00:00.000Z',
  updatedAt: '2026-05-12T08:00:00.000Z',
};

const folder: CloudFolder = {
  id: 'folder-1',
  projectId: 'project-1',
  parentId: null,
  name: 'Flows',
  sortOrder: 0,
  createdAt: '2026-05-11T08:00:00.000Z',
  updatedAt: '2026-05-12T08:00:00.000Z',
};

describe('useCloudFileStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useCloudFileStore.setState({
      files: [],
      folders: [],
      projects: [],
      loading: false,
      foldersLoading: false,
      projectsLoading: false,
      error: null,
      selectedProjectId: null,
      selectedFolderId: null,
      view: 'all',
      search: '',
      sort: 'updated_desc',
      operatingIds: {},
      layout: 'list',
    });
    cloudFileMocks.listCloudProjects.mockResolvedValue([project]);
    cloudFileMocks.createCloudProject.mockResolvedValue({
      ...project,
      id: 'project-2',
      name: 'Web App',
    });
    cloudFileMocks.updateCloudProject.mockImplementation(
      async (input: { id: string; name?: string }) => ({
        ...project,
        id: input.id,
        name: input.name ?? project.name,
      }),
    );
    cloudFileMocks.deleteCloudProject.mockResolvedValue(undefined);
    cloudFileMocks.listCloudFolders.mockResolvedValue([folder]);
    cloudFileMocks.listCloudFiles.mockResolvedValue([baseSummary]);
  });

  it('initializes projects, folders, and root files', async () => {
    await useCloudFileStore.getState().initializeLibrary();

    expect(cloudFileMocks.listCloudProjects).toHaveBeenCalled();
    expect(cloudFileMocks.listCloudFolders).toHaveBeenCalledWith({ projectId: 'project-1' });
    expect(cloudFileMocks.listCloudFiles).toHaveBeenCalledWith({
      projectId: 'project-1',
      folderId: null,
      view: 'all',
      search: undefined,
      sort: 'updated_desc',
    });
    expect(useCloudFileStore.getState()).toMatchObject({
      projects: [project],
      folders: [folder],
      files: [baseSummary],
      selectedProjectId: 'project-1',
      selectedFolderId: null,
      loading: false,
      error: null,
    });
  });

  it('loads files for the current filters and preserves previous files on errors', async () => {
    useCloudFileStore.setState({ files: [baseSummary], selectedProjectId: 'project-1' });
    cloudFileMocks.listCloudFiles.mockRejectedValueOnce(new Error('network unavailable'));

    await useCloudFileStore.getState().loadFiles();

    expect(useCloudFileStore.getState()).toMatchObject({
      files: [baseSummary],
      loading: false,
      error: 'network unavailable',
    });
  });

  it('changes view, search, sort, project, and folder query state', async () => {
    useCloudFileStore.setState({
      projects: [project],
      folders: [folder],
      selectedProjectId: 'project-1',
      selectedFolderId: null,
    });

    await useCloudFileStore.getState().setSearch('login');
    await useCloudFileStore.getState().setSort('name_asc');
    await useCloudFileStore.getState().setView('starred');
    await useCloudFileStore.getState().setView('shared');
    await useCloudFileStore.getState().selectFolder('folder-1');

    expect(cloudFileMocks.listCloudFiles).toHaveBeenLastCalledWith({
      projectId: 'project-1',
      folderId: 'folder-1',
      view: 'all',
      search: 'login',
      sort: 'name_asc',
    });
    expect(useCloudFileStore.getState()).toMatchObject({
      selectedFolderId: 'folder-1',
      view: 'all',
      search: 'login',
      sort: 'name_asc',
    });
    expect(
      cloudFileMocks.listCloudFiles.mock.calls.some((call) => call[0]?.view === 'shared'),
    ).toBe(true);
  });

  it('switches between list and grid layout locally', () => {
    expect(useCloudFileStore.getState().layout).toBe('list');

    useCloudFileStore.getState().setLayout('grid');

    expect(useCloudFileStore.getState().layout).toBe('grid');
  });

  it('creates, renames, and deletes projects while refreshing the selected library', async () => {
    const fallbackProject = { ...project, id: 'project-3', name: 'Archive' };
    useCloudFileStore.setState({
      projects: [project],
      selectedProjectId: 'project-1',
      selectedFolderId: 'folder-1',
      folders: [folder],
      files: [baseSummary],
    });
    cloudFileMocks.listCloudProjects.mockResolvedValueOnce([fallbackProject]);
    cloudFileMocks.listCloudFolders.mockResolvedValueOnce([]).mockResolvedValueOnce([]);
    cloudFileMocks.listCloudFiles.mockResolvedValueOnce([]).mockResolvedValueOnce([]);

    await expect(useCloudFileStore.getState().createProject('Web App')).resolves.toBe('project-2');
    await expect(useCloudFileStore.getState().renameProject('project-2', 'Website')).resolves.toBe(
      true,
    );
    await expect(useCloudFileStore.getState().deleteProject('project-2')).resolves.toBe(true);

    expect(cloudFileMocks.createCloudProject).toHaveBeenCalledWith({ name: 'Web App' });
    expect(cloudFileMocks.updateCloudProject).toHaveBeenCalledWith({
      id: 'project-2',
      name: 'Website',
    });
    expect(cloudFileMocks.deleteCloudProject).toHaveBeenCalledWith('project-2');
    expect(useCloudFileStore.getState()).toMatchObject({
      projects: [fallbackProject],
      selectedProjectId: 'project-3',
      selectedFolderId: null,
      folders: [],
      files: [],
    });
  });

  it('prepends a newly created cloud file and returns its id', async () => {
    useCloudFileStore.setState({
      files: [baseSummary],
      selectedProjectId: 'project-1',
      selectedFolderId: 'folder-1',
    });
    const created: CloudFileRecord = {
      ...baseSummary,
      id: 'file-2',
      name: 'Untitled',
      revision: 1,
      folderId: 'folder-1',
      document,
    };
    cloudFileMocks.createCloudFile.mockResolvedValueOnce(created);

    const id = await useCloudFileStore.getState().createFile('Untitled', document);

    expect(id).toBe('file-2');
    expect(cloudFileMocks.createCloudFile).toHaveBeenCalledWith({
      name: 'Untitled',
      document,
      projectId: 'project-1',
      folderId: 'folder-1',
      source: 'manual_save',
    });
    expect(useCloudFileStore.getState().files.map((file) => file.id)).toEqual([
      'file-2',
      'file-1',
    ]);
  });

  it('creates folders under the selected folder and refreshes the folder tree', async () => {
    useCloudFileStore.setState({
      folders: [folder],
      selectedProjectId: 'project-1',
      selectedFolderId: 'folder-1',
    });
    const childFolder = { ...folder, id: 'folder-2', parentId: 'folder-1', name: 'Checkout' };
    cloudFileMocks.createCloudFolder.mockResolvedValueOnce(childFolder);
    cloudFileMocks.listCloudFolders.mockResolvedValueOnce([folder, childFolder]);

    await expect(useCloudFileStore.getState().createFolder('Checkout')).resolves.toBe('folder-2');

    expect(cloudFileMocks.createCloudFolder).toHaveBeenCalledWith({
      projectId: 'project-1',
      parentId: 'folder-1',
      name: 'Checkout',
    });
    expect(useCloudFileStore.getState().folders.map((item) => item.id)).toEqual([
      'folder-1',
      'folder-2',
    ]);
  });

  it('renames, copies, moves, stars, deletes, restores, and permanently deletes files', async () => {
    useCloudFileStore.setState({ files: [baseSummary], selectedProjectId: 'project-1' });
    cloudFileMocks.updateCloudFileMetadata.mockResolvedValueOnce({
      ...baseSummary,
      name: 'Renamed',
    });
    cloudFileMocks.copyCloudFile.mockResolvedValueOnce({
      ...baseSummary,
      id: 'file-2',
      name: 'Renamed Copy',
      document,
    });
    cloudFileMocks.updateCloudFileMetadata
      .mockResolvedValueOnce({ ...baseSummary, folderId: 'folder-1' })
      .mockResolvedValueOnce({ ...baseSummary, starred: true });
    cloudFileMocks.createCloudFileShare.mockResolvedValueOnce({
      id: 'share-1',
      fileId: 'file-1',
      ownerId: 'user-1',
      sharedWithUserId: null,
      sharedWithEmail: 'bob@example.com',
      role: 'viewer',
      createdAt: '2026-05-12T08:00:00.000Z',
      updatedAt: '2026-05-12T08:00:00.000Z',
    });
    cloudFileMocks.revokeCloudFileShare.mockResolvedValueOnce(undefined);
    cloudFileMocks.deleteCloudFile.mockResolvedValueOnce(undefined);
    cloudFileMocks.restoreCloudFile.mockResolvedValueOnce(baseSummary);
    cloudFileMocks.permanentlyDeleteCloudFile.mockResolvedValueOnce(undefined);

    await expect(useCloudFileStore.getState().renameFile('file-1', 'Renamed')).resolves.toBe(true);
    await expect(useCloudFileStore.getState().copyFile('file-1', 'Renamed Copy')).resolves.toBe(
      'file-2',
    );
    await expect(useCloudFileStore.getState().moveFile('file-1', 'folder-1')).resolves.toBe(true);
    await expect(useCloudFileStore.getState().toggleStarred('file-1', true)).resolves.toBe(true);
    await expect(useCloudFileStore.getState().shareFile('file-1', 'bob@example.com')).resolves.toBe(
      true,
    );
    await expect(useCloudFileStore.getState().revokeShare('file-1', 'share-1')).resolves.toBe(true);
    await expect(useCloudFileStore.getState().deleteFile('file-1')).resolves.toBe(true);
    useCloudFileStore.setState({ files: [baseSummary] });
    await expect(useCloudFileStore.getState().restoreFile('file-1')).resolves.toBe(true);
    useCloudFileStore.setState({ files: [baseSummary] });
    await expect(useCloudFileStore.getState().permanentlyDeleteFile('file-1')).resolves.toBe(true);

    expect(cloudFileMocks.updateCloudFileMetadata).toHaveBeenCalledWith({
      id: 'file-1',
      name: 'Renamed',
    });
    expect(cloudFileMocks.copyCloudFile).toHaveBeenCalledWith({
      id: 'file-1',
      name: 'Renamed Copy',
      projectId: 'project-1',
      folderId: null,
    });
    expect(cloudFileMocks.updateCloudFileMetadata).toHaveBeenCalledWith({
      id: 'file-1',
      folderId: 'folder-1',
      projectId: 'project-1',
    });
    expect(cloudFileMocks.updateCloudFileMetadata).toHaveBeenCalledWith({
      id: 'file-1',
      starred: true,
    });
    expect(cloudFileMocks.createCloudFileShare).toHaveBeenCalledWith({
      fileId: 'file-1',
      email: 'bob@example.com',
      role: 'viewer',
    });
    expect(cloudFileMocks.revokeCloudFileShare).toHaveBeenCalledWith({
      fileId: 'file-1',
      shareId: 'share-1',
    });
    expect(cloudFileMocks.deleteCloudFile).toHaveBeenCalledWith('file-1');
    expect(cloudFileMocks.restoreCloudFile).toHaveBeenCalledWith('file-1');
    expect(cloudFileMocks.permanentlyDeleteCloudFile).toHaveBeenCalledWith('file-1');
    expect(useCloudFileStore.getState().files).toEqual([]);
  });
});
