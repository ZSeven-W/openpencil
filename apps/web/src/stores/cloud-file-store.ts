import { create } from 'zustand';
import type {
  CloudFileLayout,
  CloudFileSort,
  CloudFileSummary,
  CloudFileView,
  CloudFolder,
  CloudProject,
} from '@/types/cloud';
import {
  copyCloudFile,
  createCloudFile,
  createCloudFileShare,
  createCloudFolder,
  createCloudProject,
  deleteCloudFile,
  deleteCloudFolder,
  deleteCloudProject,
  listCloudFiles,
  listCloudFolders,
  listCloudProjects,
  permanentlyDeleteCloudFile,
  restoreCloudFile,
  revokeCloudFileShare,
  updateCloudFileMetadata,
  updateCloudFolder,
  updateCloudProject,
} from '@/services/cloud/cloud-files';
import type { PenDocument } from '@/types/pen';

type OperationName =
  | 'copy'
  | 'delete'
  | 'folder'
  | 'move'
  | 'permanent-delete'
  | 'project'
  | 'rename'
  | 'restore'
  | 'share'
  | 'star';

type OperationMap = Record<string, OperationName | undefined>;

interface CloudFileState {
  files: CloudFileSummary[];
  folders: CloudFolder[];
  projects: CloudProject[];
  loading: boolean;
  foldersLoading: boolean;
  projectsLoading: boolean;
  error: string | null;
  selectedProjectId: string | null;
  selectedFolderId: string | null;
  view: CloudFileView;
  search: string;
  sort: CloudFileSort;
  layout: CloudFileLayout;
  operatingIds: OperationMap;
  initializeLibrary: () => Promise<void>;
  loadProjects: () => Promise<void>;
  loadFolders: () => Promise<void>;
  loadFiles: () => Promise<void>;
  selectProject: (projectId: string) => Promise<void>;
  selectFolder: (folderId: string | null) => Promise<void>;
  setView: (view: CloudFileView) => Promise<void>;
  setSearch: (search: string) => Promise<void>;
  setSort: (sort: CloudFileSort) => Promise<void>;
  setLayout: (layout: CloudFileLayout) => void;
  createProject: (name: string) => Promise<string | null>;
  renameProject: (projectId: string, name: string) => Promise<boolean>;
  deleteProject: (projectId: string) => Promise<boolean>;
  createFolder: (name: string) => Promise<string | null>;
  renameFolder: (folderId: string, name: string) => Promise<boolean>;
  deleteFolder: (folderId: string) => Promise<boolean>;
  createFile: (
    name: string,
    document: PenDocument,
    source?: 'import' | 'manual_save',
  ) => Promise<string | null>;
  renameFile: (id: string, name: string) => Promise<boolean>;
  copyFile: (id: string, name?: string) => Promise<string | null>;
  moveFile: (id: string, folderId: string | null) => Promise<boolean>;
  shareFile: (id: string, email: string, role?: 'viewer' | 'editor') => Promise<boolean>;
  revokeShare: (fileId: string, shareId: string) => Promise<boolean>;
  toggleStarred: (id: string, starred: boolean) => Promise<boolean>;
  deleteFile: (id: string) => Promise<boolean>;
  restoreFile: (id: string) => Promise<boolean>;
  permanentlyDeleteFile: (id: string) => Promise<boolean>;
  reset: () => void;
}

const initialState = {
  files: [],
  folders: [],
  projects: [],
  loading: false,
  foldersLoading: false,
  projectsLoading: false,
  error: null,
  selectedProjectId: null,
  selectedFolderId: null,
  view: 'all' as CloudFileView,
  search: '',
  sort: 'updated_desc' as CloudFileSort,
  layout: 'list' as CloudFileLayout,
  operatingIds: {},
};

function errorMessage(err: unknown, fallback: string): string {
  return err instanceof Error ? err.message : fallback;
}

function setOperation(
  set: (partial: Partial<CloudFileState> | ((state: CloudFileState) => Partial<CloudFileState>)) => void,
  id: string,
  operation: OperationName | null,
) {
  set((state) => {
    const operatingIds = { ...state.operatingIds };
    if (operation) {
      operatingIds[id] = operation;
    } else {
      delete operatingIds[id];
    }
    return { operatingIds };
  });
}

function fileQuery(state: CloudFileState) {
  return {
    projectId: state.selectedProjectId ?? undefined,
    folderId: state.view === 'all' ? state.selectedFolderId : undefined,
    view: state.view,
    search: state.search.trim() || undefined,
    sort: state.sort,
  };
}

export const useCloudFileStore = create<CloudFileState>((set, get) => ({
  ...initialState,

  initializeLibrary: async () => {
    set({ projectsLoading: true, foldersLoading: true, loading: true, error: null });
    try {
      const projects = await listCloudProjects();
      const selectedProjectId = get().selectedProjectId ?? projects[0]?.id ?? null;
      set({ projects, selectedProjectId, projectsLoading: false });

      if (selectedProjectId) {
        await get().loadFolders();
        await get().loadFiles();
      } else {
        set({ folders: [], files: [], foldersLoading: false, loading: false });
      }
    } catch (err) {
      set({
        projectsLoading: false,
        foldersLoading: false,
        loading: false,
        error: errorMessage(err, 'Failed to load cloud library'),
      });
    }
  },

  loadProjects: async () => {
    set({ projectsLoading: true, error: null });
    try {
      const projects = await listCloudProjects();
      const currentProjectId = get().selectedProjectId;
      const selectedProjectId =
        currentProjectId && projects.some((project) => project.id === currentProjectId)
          ? currentProjectId
          : projects[0]?.id ?? null;
      set({ projects, selectedProjectId, projectsLoading: false });
    } catch (err) {
      set({
        projectsLoading: false,
        error: errorMessage(err, 'Failed to load cloud projects'),
      });
    }
  },

  loadFolders: async () => {
    const selectedProjectId = get().selectedProjectId;
    if (!selectedProjectId) {
      set({ folders: [], foldersLoading: false });
      return;
    }

    set({ foldersLoading: true, error: null });
    try {
      const folders = await listCloudFolders({ projectId: selectedProjectId });
      set({ folders, foldersLoading: false });
    } catch (err) {
      set({
        foldersLoading: false,
        error: errorMessage(err, 'Failed to load cloud folders'),
      });
    }
  },

  loadFiles: async () => {
    set({ loading: true, error: null });
    try {
      const files = await listCloudFiles(fileQuery(get()));
      set({ files, loading: false });
    } catch (err) {
      set({
        loading: false,
        error: errorMessage(err, 'Failed to load cloud files'),
      });
    }
  },

  selectProject: async (projectId) => {
    set({
      selectedProjectId: projectId,
      selectedFolderId: null,
      view: 'all',
      error: null,
    });
    await get().loadFolders();
    await get().loadFiles();
  },

  selectFolder: async (folderId) => {
    set({ selectedFolderId: folderId, view: 'all', error: null });
    await get().loadFiles();
  },

  setView: async (view) => {
    set({ view, selectedFolderId: view === 'all' ? get().selectedFolderId : null });
    await get().loadFiles();
  },

  setSearch: async (search) => {
    set({ search });
    await get().loadFiles();
  },

  setSort: async (sort) => {
    set({ sort });
    await get().loadFiles();
  },

  setLayout: (layout) => {
    set({ layout });
  },

  createProject: async (name) => {
    set({ projectsLoading: true, error: null });
    try {
      const project = await createCloudProject({ name });
      set((state) => ({
        projects: [project, ...state.projects],
        selectedProjectId: project.id,
        selectedFolderId: null,
        projectsLoading: false,
      }));
      await get().loadFolders();
      await get().loadFiles();
      return project.id;
    } catch (err) {
      set({
        projectsLoading: false,
        error: errorMessage(err, 'Failed to create project'),
      });
      return null;
    }
  },

  renameProject: async (projectId, name) => {
    setOperation(set, projectId, 'rename');
    set({ error: null });
    try {
      const project = await updateCloudProject({ id: projectId, name });
      set((state) => ({
        projects: state.projects.map((item) => (item.id === projectId ? project : item)),
      }));
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to rename project') });
      return false;
    } finally {
      setOperation(set, projectId, null);
    }
  },

  deleteProject: async (projectId) => {
    setOperation(set, projectId, 'delete');
    set({ projectsLoading: true, error: null });
    try {
      await deleteCloudProject(projectId);
      const projects = await listCloudProjects();
      const currentProjectId = get().selectedProjectId;
      const selectedProjectId =
        currentProjectId &&
        currentProjectId !== projectId &&
        projects.some((project) => project.id === currentProjectId)
          ? currentProjectId
          : projects[0]?.id ?? null;
      set({
        projects,
        selectedProjectId,
        selectedFolderId: selectedProjectId === currentProjectId ? get().selectedFolderId : null,
        projectsLoading: false,
      });
      await get().loadFolders();
      await get().loadFiles();
      return true;
    } catch (err) {
      set({
        projectsLoading: false,
        error: errorMessage(err, 'Failed to delete project'),
      });
      return false;
    } finally {
      setOperation(set, projectId, null);
    }
  },

  createFolder: async (name) => {
    const selectedProjectId = get().selectedProjectId;
    if (!selectedProjectId) return null;

    setOperation(set, `folder:${selectedProjectId}`, 'folder');
    set({ error: null });
    try {
      const folder = await createCloudFolder({
        projectId: selectedProjectId,
        parentId: get().selectedFolderId,
        name,
      });
      await get().loadFolders();
      return folder.id;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to create folder') });
      return null;
    } finally {
      setOperation(set, `folder:${selectedProjectId}`, null);
    }
  },

  renameFolder: async (folderId, name) => {
    setOperation(set, folderId, 'rename');
    set({ error: null });
    try {
      const folder = await updateCloudFolder({ id: folderId, name });
      set((state) => ({
        folders: state.folders.map((item) => (item.id === folder.id ? folder : item)),
      }));
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to rename folder') });
      return false;
    } finally {
      setOperation(set, folderId, null);
    }
  },

  deleteFolder: async (folderId) => {
    setOperation(set, folderId, 'delete');
    set({ error: null });
    try {
      await deleteCloudFolder(folderId);
      set((state) => ({
        folders: state.folders.filter((folder) => folder.id !== folderId),
        files: state.files.filter((file) => file.folderId !== folderId),
        selectedFolderId: state.selectedFolderId === folderId ? null : state.selectedFolderId,
      }));
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to delete folder') });
      return false;
    } finally {
      setOperation(set, folderId, null);
    }
  },

  createFile: async (name, document, source = 'manual_save') => {
    set({ loading: true, error: null });
    try {
      const state = get();
      const file = await createCloudFile({
        name,
        document,
        projectId: state.selectedProjectId,
        folderId: state.selectedFolderId,
        source,
      });
      set({ files: [file, ...get().files], loading: false });
      return file.id;
    } catch (err) {
      set({
        loading: false,
        error: errorMessage(err, 'Failed to create cloud file'),
      });
      return null;
    }
  },

  renameFile: async (id, name) => {
    setOperation(set, id, 'rename');
    set({ error: null });
    try {
      const file = await updateCloudFileMetadata({ id, name });
      set((state) => ({
        files: state.files.map((item) => (item.id === id ? file : item)),
      }));
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to rename cloud file') });
      return false;
    } finally {
      setOperation(set, id, null);
    }
  },

  copyFile: async (id, name) => {
    const state = get();
    setOperation(set, id, 'copy');
    set({ error: null });
    try {
      const file = await copyCloudFile({
        id,
        name,
        projectId: state.selectedProjectId,
        folderId: state.selectedFolderId,
      });
      set({ files: [file, ...get().files] });
      return file.id;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to copy cloud file') });
      return null;
    } finally {
      setOperation(set, id, null);
    }
  },

  moveFile: async (id, folderId) => {
    setOperation(set, id, 'move');
    set({ error: null });
    try {
      const file = await updateCloudFileMetadata({
        id,
        projectId: get().selectedProjectId,
        folderId,
      });
      set((state) => ({
        files: state.files
          .map((item) => (item.id === id ? file : item))
          .filter((item) => state.view !== 'all' || item.folderId === state.selectedFolderId),
      }));
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to move cloud file') });
      return false;
    } finally {
      setOperation(set, id, null);
    }
  },

  shareFile: async (id, email, role = 'viewer') => {
    setOperation(set, id, 'share');
    set({ error: null });
    try {
      await createCloudFileShare({ fileId: id, email, role });
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to share cloud file') });
      return false;
    } finally {
      setOperation(set, id, null);
    }
  },

  revokeShare: async (fileId, shareId) => {
    setOperation(set, fileId, 'share');
    set({ error: null });
    try {
      await revokeCloudFileShare({ fileId, shareId });
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to revoke cloud file share') });
      return false;
    } finally {
      setOperation(set, fileId, null);
    }
  },

  toggleStarred: async (id, starred) => {
    setOperation(set, id, 'star');
    set({ error: null });
    try {
      const file = await updateCloudFileMetadata({ id, starred });
      set((state) => ({
        files: state.files
          .map((item) => (item.id === id ? file : item))
          .filter((item) => state.view !== 'starred' || item.starred),
      }));
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to update favorite') });
      return false;
    } finally {
      setOperation(set, id, null);
    }
  },

  deleteFile: async (id) => {
    setOperation(set, id, 'delete');
    set({ error: null });
    try {
      await deleteCloudFile(id);
      set({ files: get().files.filter((file) => file.id !== id) });
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to delete cloud file') });
      return false;
    } finally {
      setOperation(set, id, null);
    }
  },

  restoreFile: async (id) => {
    setOperation(set, id, 'restore');
    set({ error: null });
    try {
      const file = await restoreCloudFile(id);
      set((state) => ({
        files:
          state.view === 'trash'
            ? state.files.filter((item) => item.id !== id)
            : [file, ...state.files.filter((item) => item.id !== id)],
      }));
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to restore cloud file') });
      return false;
    } finally {
      setOperation(set, id, null);
    }
  },

  permanentlyDeleteFile: async (id) => {
    setOperation(set, id, 'permanent-delete');
    set({ error: null });
    try {
      await permanentlyDeleteCloudFile(id);
      set({ files: get().files.filter((file) => file.id !== id) });
      return true;
    } catch (err) {
      set({ error: errorMessage(err, 'Failed to permanently delete cloud file') });
      return false;
    } finally {
      setOperation(set, id, null);
    }
  },

  reset: () => set(initialState),
}));
