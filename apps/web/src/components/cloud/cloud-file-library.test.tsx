// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  CloudFileRecord,
  CloudFileShare,
  CloudFileSummary,
  CloudFolder,
  CloudProject,
} from '@/types/cloud';

const routerMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
}));

const authMocks = vi.hoisted(() => ({
  signOut: vi.fn(),
  user: { email: 'alice@example.com' },
}));

const cloudFileMocks = vi.hoisted(() => ({
  listCloudProjects: vi.fn(),
  createCloudProject: vi.fn(),
  updateCloudProject: vi.fn(),
  deleteCloudProject: vi.fn(),
  listCloudFolders: vi.fn(),
  listCloudFiles: vi.fn(),
  listCloudFileShares: vi.fn(),
  createCloudFile: vi.fn(),
  createCloudFolder: vi.fn(),
  updateCloudFolder: vi.fn(),
  deleteCloudFolder: vi.fn(),
  updateCloudFileMetadata: vi.fn(),
  createCloudFileShare: vi.fn(),
  revokeCloudFileShare: vi.fn(),
  copyCloudFile: vi.fn(),
  deleteCloudFile: vi.fn(),
  restoreCloudFile: vi.fn(),
  permanentlyDeleteCloudFile: vi.fn(),
}));

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => routerMocks.navigate,
}));

vi.mock('@/stores/cloud-auth-store', () => ({
  useCloudAuthStore: (selector: (state: typeof authMocks) => unknown) => selector(authMocks),
}));

vi.mock('@/services/cloud/cloud-files', () => cloudFileMocks);

vi.mock('@/stores/document-store', () => ({
  createEmptyDocument: () => ({
    version: '1.0.0',
    pages: [{ id: 'page-1', name: 'Page 1', children: [] }],
    children: [],
  }),
}));

vi.mock('@/utils/import-pen-document', () => ({
  parseAndPrepareImportedDocument: vi.fn(),
}));

import { CloudFileLibrary } from './cloud-file-library';
import { useCloudFileStore } from '@/stores/cloud-file-store';

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

const fileSummary: CloudFileSummary = {
  id: 'file-1',
  projectId: 'project-1',
  folderId: null,
  name: 'Home Screen',
  thumbnailPath: null,
  revision: 4,
  metadata: {},
  starred: false,
  lastOpenedAt: null,
  deletedAt: null,
  createdAt: '2026-05-11T08:00:00.000Z',
  updatedAt: '2026-05-12T08:00:00.000Z',
};

const folderFileSummary: CloudFileSummary = {
  ...fileSummary,
  id: 'file-2',
  folderId: 'folder-1',
  name: 'Settings Screen',
  revision: 2,
  updatedAt: '2026-05-12T07:00:00.000Z',
};

const createdFile: CloudFileRecord = {
  ...fileSummary,
  id: 'file-2',
  name: 'Untitled',
  revision: 1,
  document: {
    version: '1.0.0',
    pages: [{ id: 'page-1', name: 'Page 1', children: [] }],
    children: [],
  },
};

const share: CloudFileShare = {
  id: 'share-1',
  fileId: 'file-1',
  ownerId: 'user-1',
  sharedWithUserId: null,
  sharedWithEmail: 'bob@example.com',
  role: 'viewer',
  createdAt: '2026-05-12T08:00:00.000Z',
  updatedAt: '2026-05-12T08:00:00.000Z',
};

beforeEach(() => {
  vi.clearAllMocks();
  cloudFileMocks.listCloudProjects.mockResolvedValue([project]);
  cloudFileMocks.listCloudFolders.mockResolvedValue([folder]);
  cloudFileMocks.listCloudFiles.mockResolvedValue([fileSummary]);
  cloudFileMocks.listCloudFileShares.mockResolvedValue([]);
  cloudFileMocks.createCloudProject.mockResolvedValue({
    ...project,
    id: 'project-2',
    name: 'Web App',
  });
  cloudFileMocks.updateCloudProject.mockImplementation(async (input: { id: string; name?: string }) => ({
    ...project,
    id: input.id,
    name: input.name ?? project.name,
  }));
  cloudFileMocks.deleteCloudProject.mockResolvedValue(undefined);
  cloudFileMocks.createCloudFile.mockResolvedValue(createdFile);
  cloudFileMocks.createCloudFolder.mockResolvedValue(folder);
  cloudFileMocks.updateCloudFolder.mockImplementation(async (input: { id: string; name?: string }) => ({
    ...folder,
    id: input.id,
    name: input.name ?? folder.name,
  }));
  cloudFileMocks.deleteCloudFolder.mockResolvedValue(undefined);
  cloudFileMocks.updateCloudFileMetadata.mockImplementation(async (input: { id: string }) => ({
    ...fileSummary,
    id: input.id,
  }));
  cloudFileMocks.copyCloudFile.mockResolvedValue({
    ...createdFile,
    id: 'file-copy',
    name: 'Home Screen Copy',
  });
  cloudFileMocks.deleteCloudFile.mockResolvedValue(undefined);
  cloudFileMocks.restoreCloudFile.mockResolvedValue(fileSummary);
  cloudFileMocks.permanentlyDeleteCloudFile.mockResolvedValue(undefined);
  cloudFileMocks.createCloudFileShare.mockResolvedValue(share);
  cloudFileMocks.revokeCloudFileShare.mockResolvedValue(undefined);
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
    layout: 'list',
    operatingIds: {},
  });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('CloudFileLibrary', () => {
  it('shows loading and empty file states without exposing stale actions', async () => {
    cloudFileMocks.listCloudFiles.mockImplementation(() => new Promise(() => {}));

    const { unmount } = render(<CloudFileLibrary />);

    expect(await screen.findByText('Loading files...')).toBeTruthy();
    expect(screen.queryByLabelText('Rename Home Screen')).toBeNull();
    unmount();

    cleanup();
    cloudFileMocks.listCloudFiles.mockReset();
    cloudFileMocks.listCloudFiles.mockResolvedValue([]);

    render(<CloudFileLibrary />);

    expect(await screen.findByText('No files in this view')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'New design' })).toBeTruthy();
    expect(screen.getAllByRole('button', { name: 'Import .op' }).length).toBeGreaterThan(0);
  });

  it('loads projects, folders, files and opens a selected file', async () => {
    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    expect(screen.getAllByText('Mobile App').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Flows').length).toBeGreaterThan(0);
    expect(screen.getByText('alice@example.com')).toBeTruthy();
    expect(screen.getByText('1 file')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: /Open Home Screen/i }));

    expect(routerMocks.navigate).toHaveBeenCalledWith({
      to: '/editor/$fileId',
      params: { fileId: 'file-1' },
    });
  });

  it('creates a new cloud file in the selected location', async () => {
    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /^New$/i }));

    await waitFor(() => {
      expect(cloudFileMocks.createCloudFile).toHaveBeenCalledWith({
        name: 'Untitled',
        document: expect.objectContaining({ version: '1.0.0' }),
        projectId: 'project-1',
        folderId: null,
        source: 'manual_save',
      });
      expect(routerMocks.navigate).toHaveBeenCalledWith({
        to: '/editor/$fileId',
        params: { fileId: 'file-2' },
      });
    });
  });

  it('searches, sorts, switches views, and selects folders', async () => {
    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.change(screen.getByPlaceholderText('Search files'), {
      target: { value: 'login' },
    });
    fireEvent.change(screen.getByLabelText('Sort files'), {
      target: { value: 'name_asc' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Starred' }));
    fireEvent.click(screen.getByRole('button', { name: 'Folder Flows' }));

    await waitFor(() => {
      expect(cloudFileMocks.listCloudFiles).toHaveBeenLastCalledWith({
        projectId: 'project-1',
        folderId: 'folder-1',
        view: 'all',
        search: 'login',
        sort: 'name_asc',
      });
    });
  });

  it('creates, renames, and deletes projects and folders from the library sidebar', async () => {
    vi.spyOn(window, 'prompt')
      .mockReturnValueOnce('Web App')
      .mockReturnValueOnce('Mobile Redesign')
      .mockReturnValueOnce('Checkout')
      .mockReturnValueOnce('Checkout Flow');
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    cloudFileMocks.listCloudProjects
      .mockResolvedValueOnce([project])
      .mockResolvedValueOnce([{ ...project, id: 'project-2', name: 'Archive' }]);
    cloudFileMocks.listCloudFolders.mockResolvedValue([folder]);
    cloudFileMocks.listCloudFiles.mockResolvedValue([fileSummary]);

    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.click(screen.getByLabelText('New project'));
    await waitFor(() => {
      expect(cloudFileMocks.createCloudProject).toHaveBeenCalledWith({ name: 'Web App' });
    });

    fireEvent.click(screen.getByLabelText('Rename project Mobile App'));
    await waitFor(() => {
      expect(cloudFileMocks.updateCloudProject).toHaveBeenCalledWith({
        id: 'project-1',
        name: 'Mobile Redesign',
      });
    });

    fireEvent.click(screen.getByLabelText('New folder'));
    await waitFor(() => {
      expect(cloudFileMocks.createCloudFolder).toHaveBeenCalledWith({
        projectId: 'project-2',
        parentId: null,
        name: 'Checkout',
      });
    });

    fireEvent.click(screen.getByLabelText('Rename folder Flows'));
    await waitFor(() => {
      expect(cloudFileMocks.updateCloudFolder).toHaveBeenCalledWith({
        id: 'folder-1',
        name: 'Checkout Flow',
      });
    });

    fireEvent.click(screen.getByLabelText('Delete folder Checkout Flow'));
    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledWith(
        'Delete folder "Checkout Flow"? Files will move to the project root.',
      );
      expect(cloudFileMocks.deleteCloudFolder).toHaveBeenCalledWith('folder-1');
    });

    fireEvent.click(screen.getByLabelText('Delete project Mobile Redesign'));
    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledWith(
        'Delete project "Mobile Redesign"? Files will move to your default project.',
      );
      expect(cloudFileMocks.deleteCloudProject).toHaveBeenCalledWith('project-1');
    });
  });

  it('runs rename, copy, star, delete, restore, and permanent delete actions', async () => {
    const promptSpy = vi
      .spyOn(window, 'prompt')
      .mockReturnValueOnce('Renamed')
      .mockReturnValueOnce('Home Screen Copy');
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);

    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.click(screen.getByLabelText('Rename Home Screen'));

    await waitFor(() => {
      expect(promptSpy).toHaveBeenCalledWith('Rename file', 'Home Screen');
      expect(cloudFileMocks.updateCloudFileMetadata).toHaveBeenCalledWith({
        id: 'file-1',
        name: 'Renamed',
      });
    });

    fireEvent.click(screen.getByLabelText('Copy Home Screen'));
    await waitFor(() => {
      expect(promptSpy).toHaveBeenCalledWith('Copy file as', 'Home Screen Copy');
      expect(cloudFileMocks.copyCloudFile).toHaveBeenCalledWith({
        id: 'file-1',
        name: 'Home Screen Copy',
        projectId: 'project-1',
        folderId: null,
      });
    });

    fireEvent.click(screen.getByLabelText('Favorite Home Screen'));
    await waitFor(() => {
      expect(cloudFileMocks.updateCloudFileMetadata).toHaveBeenCalledWith({
        id: 'file-1',
        starred: true,
      });
    });

    fireEvent.click(screen.getByLabelText('Delete Home Screen'));
    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledWith('Delete "Home Screen"?');
      expect(cloudFileMocks.deleteCloudFile).toHaveBeenCalledWith('file-1');
    });

    useCloudFileStore.setState({ files: [{ ...fileSummary, deletedAt: '2026-05-12T09:00:00Z' }] });
    fireEvent.click(screen.getByRole('button', { name: 'Trash' }));
    fireEvent.click(await screen.findByLabelText('Restore Home Screen'));

    await waitFor(() => {
      expect(cloudFileMocks.restoreCloudFile).toHaveBeenCalledWith('file-1');
    });

    useCloudFileStore.setState({ files: [{ ...fileSummary, deletedAt: '2026-05-12T09:00:00Z' }] });
    fireEvent.click(await screen.findByLabelText('Permanently delete Home Screen'));

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledWith('Permanently delete "Home Screen"?');
      expect(cloudFileMocks.permanentlyDeleteCloudFile).toHaveBeenCalledWith('file-1');
    });
  });

  it('shows a trash-specific empty state and only destructive trash actions', async () => {
    cloudFileMocks.listCloudFiles.mockResolvedValueOnce([fileSummary]).mockResolvedValueOnce([]);

    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Trash' }));

    expect(await screen.findByText('Trash is empty')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'New design' })).toBeNull();

    useCloudFileStore.setState({
      files: [{ ...fileSummary, deletedAt: '2026-05-12T09:00:00.000Z' }],
      view: 'trash',
    });

    expect(await screen.findByLabelText('Restore Home Screen')).toBeTruthy();
    expect(screen.getByLabelText('Permanently delete Home Screen')).toBeTruthy();
    expect(screen.queryByLabelText('Rename Home Screen')).toBeNull();
    expect(screen.queryByLabelText('Share Home Screen')).toBeNull();
  });

  it('loads the shared view from the file list API', async () => {
    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Shared with me' }));

    await waitFor(() => {
      expect(cloudFileMocks.listCloudFiles).toHaveBeenCalledWith({
        projectId: 'project-1',
        folderId: undefined,
        view: 'shared',
        search: undefined,
        sort: 'updated_desc',
      });
    });
  });

  it('supports context menu actions and the file detail panel', async () => {
    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.contextMenu(screen.getByLabelText('Open Home Screen'));

    expect(screen.getByRole('menu', { name: 'File actions for Home Screen' })).toBeTruthy();
    fireEvent.click(screen.getByRole('menuitem', { name: 'Details' }));

    const details = screen.getByRole('complementary', { name: 'File details' });
    expect(details).toBeTruthy();
    expect(within(details).getByText('Home Screen')).toBeTruthy();
    expect(within(details).getByText('Revision')).toBeTruthy();
    expect(within(details).getByText('rev 4')).toBeTruthy();
    expect(within(details).getByText('Location')).toBeTruthy();
    expect(within(details).getByText('Project root')).toBeTruthy();
  });

  it('manages file shares from the details panel', async () => {
    cloudFileMocks.listCloudFileShares.mockResolvedValue([share]);

    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.click(screen.getByLabelText('Details Home Screen'));

    const details = screen.getByRole('complementary', { name: 'File details' });
    expect(await within(details).findByText('Shared access')).toBeTruthy();
    expect(await within(details).findByText('bob@example.com')).toBeTruthy();

    fireEvent.change(within(details).getByLabelText('Share email'), {
      target: { value: 'carol@example.com' },
    });
    fireEvent.change(within(details).getByLabelText('Share role'), {
      target: { value: 'editor' },
    });
    fireEvent.click(within(details).getByRole('button', { name: 'Add share' }));

    await waitFor(() => {
      expect(cloudFileMocks.createCloudFileShare).toHaveBeenCalledWith({
        fileId: 'file-1',
        email: 'carol@example.com',
        role: 'editor',
      });
    });

    fireEvent.click(within(details).getByLabelText('Revoke share for bob@example.com'));

    await waitFor(() => {
      expect(cloudFileMocks.revokeCloudFileShare).toHaveBeenCalledWith({
        fileId: 'file-1',
        shareId: 'share-1',
      });
    });
  });

  it('shows an empty sharing state in file details', async () => {
    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.click(screen.getByLabelText('Details Home Screen'));

    const details = screen.getByRole('complementary', { name: 'File details' });
    expect(await within(details).findByText('No active shares')).toBeTruthy();
  });

  it('runs batch favorite and delete actions for selected files', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    cloudFileMocks.listCloudFiles.mockResolvedValue([fileSummary, folderFileSummary]);

    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    expect(await screen.findByText('Settings Screen')).toBeTruthy();
    fireEvent.click(screen.getByLabelText('Select Home Screen'));
    fireEvent.click(screen.getByLabelText('Select Settings Screen'));

    expect(screen.getByText('2 selected')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Favorite selected' }));
    await waitFor(() => {
      expect(cloudFileMocks.updateCloudFileMetadata).toHaveBeenCalledWith({
        id: 'file-1',
        starred: true,
      });
      expect(cloudFileMocks.updateCloudFileMetadata).toHaveBeenCalledWith({
        id: 'file-2',
        starred: true,
      });
    });

    fireEvent.click(screen.getByRole('button', { name: 'Delete selected' }));
    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledWith('Delete 2 selected files?');
      expect(cloudFileMocks.deleteCloudFile).toHaveBeenCalledWith('file-1');
      expect(cloudFileMocks.deleteCloudFile).toHaveBeenCalledWith('file-2');
    });
  });

  it('switches between list and grid layouts', async () => {
    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Grid view' }));

    expect(useCloudFileStore.getState().layout).toBe('grid');
    expect(screen.getByRole('article')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'List view' }));

    expect(useCloudFileStore.getState().layout).toBe('list');
  });

  it('moves files through the target selector and shares by email', async () => {
    const promptSpy = vi.spyOn(window, 'prompt').mockReturnValue('bob@example.com');
    cloudFileMocks.listCloudFiles.mockResolvedValue([fileSummary, folderFileSummary]);
    cloudFileMocks.updateCloudFileMetadata.mockImplementation(async (input: { id: string; folderId?: string | null }) => ({
      ...(input.id === 'file-2' ? folderFileSummary : fileSummary),
      folderId: input.folderId ?? null,
    }));
    cloudFileMocks.createCloudFileShare.mockResolvedValue({
      id: 'share-1',
      fileId: 'file-1',
      ownerId: 'user-1',
      sharedWithUserId: null,
      sharedWithEmail: 'bob@example.com',
      role: 'viewer',
      createdAt: '2026-05-12T08:00:00.000Z',
      updatedAt: '2026-05-12T08:00:00.000Z',
    });

    render(<CloudFileLibrary />);

    expect(await screen.findByText('Home Screen')).toBeTruthy();
    fireEvent.click(screen.getByLabelText('Share Home Screen'));
    await waitFor(() => {
      expect(promptSpy).toHaveBeenCalledWith('Share with email');
      expect(cloudFileMocks.createCloudFileShare).toHaveBeenCalledWith({
        fileId: 'file-1',
        email: 'bob@example.com',
        role: 'viewer',
      });
    });

    fireEvent.click(screen.getByLabelText('Move Home Screen'));
    const moveDialog = screen.getByRole('dialog', { name: 'Move file' });
    expect(moveDialog).toBeTruthy();
    fireEvent.click(within(moveDialog).getByRole('button', { name: /Flows/i }));

    await waitFor(() => {
      expect(cloudFileMocks.updateCloudFileMetadata).toHaveBeenCalledWith({
        id: 'file-1',
        projectId: 'project-1',
        folderId: 'folder-1',
      });
    });
  });
});
