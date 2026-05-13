import { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import {
  Clock3,
  FileJson,
  Folder,
  FolderPlus,
  Grid2X2,
  List,
  LogOut,
  ListTodo,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  Share2,
  Star,
  Trash2,
  Upload,
  X,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { CloudFileDetailsPanel } from './cloud-file-details-panel';
import { CloudFileGrid } from './cloud-file-grid';
import { CloudFileTable } from './cloud-file-table';
import { CloudMovePopover } from './cloud-move-popover';
import { CloudMoveTargetPanel } from './cloud-move-target-panel';
import { cn } from '@/lib/utils';
import { useCloudAuthStore } from '@/stores/cloud-auth-store';
import { useCloudFileStore } from '@/stores/cloud-file-store';
import { createEmptyDocument } from '@/stores/document-store';
import { parseAndPrepareImportedDocument } from '@/utils/import-pen-document';
import type { CloudFileSort, CloudFileSummary, CloudFileView, CloudFolder } from '@/types/cloud';
import type { PenDocument } from '@/types/pen';

const viewItems: Array<{ id: CloudFileView; labelKey: string; icon: typeof Grid2X2 }> = [
  { id: 'all', labelKey: 'cloudLibrary.view.all', icon: Grid2X2 },
  { id: 'recent', labelKey: 'cloudLibrary.view.recent', icon: Clock3 },
  { id: 'starred', labelKey: 'cloudLibrary.view.starred', icon: Star },
  { id: 'shared', labelKey: 'cloudLibrary.view.shared', icon: Share2 },
  { id: 'trash', labelKey: 'cloudLibrary.view.trash', icon: Trash2 },
];

const sortLabelKeys: Record<CloudFileSort, string> = {
  updated_desc: 'cloudLibrary.sort.updated_desc',
  updated_asc: 'cloudLibrary.sort.updated_asc',
  name_asc: 'cloudLibrary.sort.name_asc',
  name_desc: 'cloudLibrary.sort.name_desc',
  created_desc: 'cloudLibrary.sort.created_desc',
};

function formatDate(value: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(value));
  } catch {
    return value;
  }
}

function childFolders(folders: CloudFolder[], parentId: string | null): CloudFolder[] {
  return folders
    .filter((folder) => folder.parentId === parentId)
    .sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name));
}

function folderPath(
  folders: CloudFolder[],
  folderId: string | null,
  rootLabel: string,
  fallbackLabel: string,
): string {
  if (!folderId) return rootLabel;
  const byId = new Map(folders.map((folder) => [folder.id, folder]));
  const names: string[] = [];
  let current = byId.get(folderId);
  while (current) {
    names.unshift(current.name);
    current = current.parentId ? byId.get(current.parentId) : undefined;
  }
  return names.length > 0 ? names.join(' / ') : fallbackLabel;
}

function emptyStateForView(view: CloudFileView) {
  if (view === 'trash') {
    return {
      titleKey: 'cloudLibrary.empty.trash.title',
      descriptionKey: 'cloudLibrary.empty.trash.description',
      showCreateActions: false,
    };
  }
  if (view === 'shared') {
    return {
      titleKey: 'cloudLibrary.empty.shared.title',
      descriptionKey: 'cloudLibrary.empty.shared.description',
      showCreateActions: false,
    };
  }
  return {
    titleKey: 'cloudLibrary.empty.default.title',
    descriptionKey: 'cloudLibrary.empty.default.description',
    showCreateActions: true,
  };
}

export function CloudFileLibrary() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const user = useCloudAuthStore((s) => s.user);
  const signOut = useCloudAuthStore((s) => s.signOut);
  const {
    files,
    folders,
    projects,
    loading,
    foldersLoading,
    projectsLoading,
    error,
    selectedProjectId,
    selectedFolderId,
    view,
    search,
    sort,
    layout,
    operatingIds,
    initializeLibrary,
    loadFiles,
    selectProject,
    selectFolder,
    setView,
    setSearch,
    setSort,
    setLayout,
    createProject,
    renameProject,
    deleteProject,
    createFolder,
    renameFolder,
    deleteFolder,
    createFile,
    renameFile,
    copyFile,
    moveFile,
    shareFile,
    toggleStarred,
    deleteFile,
    restoreFile,
    permanentlyDeleteFile,
    reset,
  } = useCloudFileStore();
  const inputRef = useRef<HTMLInputElement>(null);
  const [creating, setCreating] = useState(false);
  const [importing, setImporting] = useState(false);
  const [creatingFolder, setCreatingFolder] = useState(false);
  const [creatingProject, setCreatingProject] = useState(false);
  const [selectedFileIds, setSelectedFileIds] = useState<string[]>([]);
  const [detailsFileId, setDetailsFileId] = useState<string | null>(null);
  const [actionsFileId, setActionsFileId] = useState<string | null>(null);
  const [movingFileId, setMovingFileId] = useState<string | null>(null);
  const selectedProject = projects.find((project) => project.id === selectedProjectId);
  const selectedFiles = files.filter((file) => selectedFileIds.includes(file.id));
  const detailsFile = files.find((file) => file.id === detailsFileId) ?? null;
  const movingFile = files.find((file) => file.id === movingFileId) ?? null;
  const fileCountLabel = t('cloudLibrary.fileCount', { count: files.length });
  const folderLabel = useMemo(
    () =>
      folderPath(
        folders,
        selectedFolderId,
        t('cloudLibrary.projectRoot'),
        t('cloudLibrary.folder'),
      ),
    [folders, selectedFolderId, t],
  );

  useEffect(() => {
    void initializeLibrary();
  }, [initializeLibrary]);

  useEffect(() => {
    setSelectedFileIds((ids) => ids.filter((id) => files.some((file) => file.id === id)));
    if (detailsFileId && !files.some((file) => file.id === detailsFileId)) {
      setDetailsFileId(null);
    }
    if (actionsFileId && !files.some((file) => file.id === actionsFileId)) {
      setActionsFileId(null);
    }
  }, [actionsFileId, detailsFileId, files]);

  const openFile = (fileId: string) => {
    void navigate({ to: '/editor/$fileId', params: { fileId } });
  };

  const createNew = async () => {
    setCreating(true);
    try {
      const id = await createFile(t('common.untitled'), createEmptyDocument() as PenDocument);
      if (id) openFile(id);
    } finally {
      setCreating(false);
    }
  };

  const importOp = async (file: File) => {
    setImporting(true);
    try {
      const text = await file.text();
      const prepared = parseAndPrepareImportedDocument(text, { fileName: file.name });
      if (!prepared) return;
      const name = file.name.replace(/\.(pen|op|json)$/i, '') || t('cloudLibrary.importedDesign');
      const id = await createFile(name, prepared.doc, 'import');
      if (id) openFile(id);
    } finally {
      setImporting(false);
    }
  };

  const addFolder = async () => {
    const name = window
      .prompt(t('cloudLibrary.prompt.newFolderName'), t('cloudLibrary.prompt.newFolder'))
      ?.trim();
    if (!name) return;
    setCreatingFolder(true);
    try {
      await createFolder(name);
    } finally {
      setCreatingFolder(false);
    }
  };

  const addProject = async () => {
    const name = window
      .prompt(t('cloudLibrary.prompt.newProjectName'), t('cloudLibrary.prompt.newProject'))
      ?.trim();
    if (!name) return;
    setCreatingProject(true);
    try {
      await createProject(name);
    } finally {
      setCreatingProject(false);
    }
  };

  const renameSelectedProject = async (projectId: string, projectName: string) => {
    const name = window.prompt(t('cloudLibrary.prompt.renameProject'), projectName)?.trim();
    if (!name || name === projectName) return;
    await renameProject(projectId, name);
  };

  const removeSelectedProject = async (projectId: string, projectName: string) => {
    if (
      !window.confirm(t('cloudLibrary.confirm.deleteProject', { name: projectName }))
    ) {
      return;
    }
    await deleteProject(projectId);
  };

  const renameSelectedFolder = async (folderId: string, folderName: string) => {
    const name = window.prompt(t('cloudLibrary.prompt.renameFolder'), folderName)?.trim();
    if (!name || name === folderName) return;
    await renameFolder(folderId, name);
  };

  const removeSelectedFolder = async (folderId: string, folderName: string) => {
    if (!window.confirm(t('cloudLibrary.confirm.deleteFolder', { name: folderName }))) {
      return;
    }
    await deleteFolder(folderId);
  };

  const renameSelectedFile = async (fileId: string, fileName: string) => {
    const name = window.prompt(t('cloudLibrary.prompt.renameFile'), fileName)?.trim();
    if (!name || name === fileName) return;
    await renameFile(fileId, name);
  };

  const copySelectedFile = async (fileId: string, fileName: string) => {
    const name = window
      .prompt(t('cloudLibrary.prompt.copyFileAs'), t('cloudLibrary.copyName', { name: fileName }))
      ?.trim();
    if (!name) return;
    await copyFile(fileId, name);
  };

  const shareSelectedFile = async (fileId: string) => {
    const email = window.prompt(t('cloudLibrary.prompt.shareWithEmail'))?.trim();
    if (!email) return;
    await shareFile(fileId, email, 'viewer');
  };

  const removeFile = async (fileId: string, fileName: string) => {
    if (!window.confirm(t('cloudLibrary.confirm.deleteFile', { name: fileName }))) return;
    await deleteFile(fileId);
  };

  const permanentlyRemoveFile = async (fileId: string, fileName: string) => {
    if (!window.confirm(t('cloudLibrary.confirm.permanentlyDeleteFile', { name: fileName }))) {
      return;
    }
    await permanentlyDeleteFile(fileId);
  };

  const toggleSelectedFile = (fileId: string) => {
    setSelectedFileIds((ids) =>
      ids.includes(fileId) ? ids.filter((id) => id !== fileId) : [...ids, fileId],
    );
  };

  const favoriteSelectedFiles = async () => {
    for (const file of selectedFiles) {
      await toggleStarred(file.id, true);
    }
  };

  const deleteSelectedFiles = async () => {
    if (selectedFiles.length === 0) return;
    if (!window.confirm(t('cloudLibrary.confirm.deleteSelectedFiles', { count: selectedFiles.length }))) {
      return;
    }
    for (const file of selectedFiles) {
      await deleteFile(file.id);
    }
    setSelectedFileIds([]);
  };

  const moveSelectedFiles = async (folderId: string | null) => {
    if (selectedFiles.length === 0) return;
    for (const file of selectedFiles) {
      await moveFile(file.id, folderId);
    }
    setSelectedFileIds([]);
  };

  const closeActions = (action: () => void) => {
    setActionsFileId(null);
    action();
  };

  const fileLocation = (folderId: string | null) =>
    folderPath(
      folders,
      folderId,
      t('cloudLibrary.projectRoot'),
      t('cloudLibrary.folder'),
    );
  const canMoveFile = (file: CloudFileSummary) =>
    folders.some((folder) => folder.id !== file.folderId) || file.folderId !== null;

  const showFileDetails = (fileId: string) => {
    setDetailsFileId(fileId);
    setActionsFileId(null);
  };

  const toggleFileFavorite = (file: CloudFileSummary) => {
    closeActions(() => void toggleStarred(file.id, !file.starred));
  };

  const renameTableFile = (file: CloudFileSummary) => {
    closeActions(() => void renameSelectedFile(file.id, file.name));
  };

  const copyTableFile = (file: CloudFileSummary) => {
    closeActions(() => void copySelectedFile(file.id, file.name));
  };

  const moveTableFile = (file: CloudFileSummary) => {
    closeActions(() => setMovingFileId(file.id));
  };

  const shareTableFile = (file: CloudFileSummary) => {
    closeActions(() => void shareSelectedFile(file.id));
  };

  const deleteTableFile = (file: CloudFileSummary) => {
    closeActions(() => void removeFile(file.id, file.name));
  };

  const restoreTableFile = (file: CloudFileSummary) => {
    closeActions(() => void restoreFile(file.id));
  };

  const permanentlyDeleteTableFile = (file: CloudFileSummary) => {
    closeActions(() => void permanentlyRemoveFile(file.id, file.name));
  };

  const rootFolders = childFolders(folders, null);
  const selectedFolderChildren = childFolders(folders, selectedFolderId);
  const emptyState = emptyStateForView(view);
  const contentColumns = detailsFile
    ? 'grid-cols-[260px_minmax(0,1fr)_280px]'
    : 'grid-cols-[260px_minmax(0,1fr)]';

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card">
        <div className="flex h-14 items-center justify-between px-5">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex h-8 w-8 items-center justify-center rounded-md border border-border bg-background">
              <FileJson size={17} className="text-primary" />
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <h1 className="truncate text-sm font-semibold">{t('cloudLibrary.title')}</h1>
                <span className="text-[11px] text-muted-foreground">{fileCountLabel}</span>
              </div>
              <p className="truncate text-xs text-muted-foreground">{user?.email}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={() => void loadFiles()} disabled={loading}>
              <RefreshCw size={15} />
              {t('cloudLibrary.refresh')}
            </Button>
            <Button variant="ghost" size="sm" onClick={() => void navigate({ to: '/tasks' })}>
              <ListTodo size={15} />
              {t('tasks.title')}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => inputRef.current?.click()}
              disabled={creating || importing}
            >
              <Upload size={15} />
              {t('cloudLibrary.importOp')}
            </Button>
            <Button size="sm" onClick={() => void createNew()} disabled={creating || importing}>
              <Plus size={15} />
              {creating ? t('cloudLibrary.creating') : t('cloudLibrary.new')}
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => {
                reset();
                void signOut();
              }}
              aria-label={t('auth.signOut')}
            >
              <LogOut size={15} />
            </Button>
          </div>
          <input
            ref={inputRef}
            type="file"
            accept=".op,.pen,.json"
            className="hidden"
            onChange={(event) => {
              const file = event.currentTarget.files?.[0];
              event.currentTarget.value = '';
              if (file) void importOp(file);
            }}
          />
        </div>
      </header>

      <main className={cn('grid min-h-[calc(100vh-3.5rem)]', contentColumns)}>
        <aside className="border-r border-border bg-background p-4">
          <section>
            <div className="mb-2 flex items-center justify-between">
              <p className="text-xs font-medium text-muted-foreground">{t('cloudLibrary.projects')}</p>
              <div className="flex items-center gap-1">
                {projectsLoading && <span className="text-[11px] text-muted-foreground">{t('cloudLibrary.loading')}</span>}
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => void addProject()}
                  disabled={creatingProject}
                  aria-label={t('cloudLibrary.action.newProject')}
                >
                  <Plus size={14} />
                </Button>
              </div>
            </div>
            <div className="space-y-1">
              {projects.map((project) => (
                <div
                  key={project.id}
                  className={cn(
                    'group flex items-center gap-1 rounded-md px-2 py-1 text-sm transition-colors hover:bg-accent',
                    selectedProjectId === project.id && 'bg-accent text-accent-foreground',
                  )}
                >
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-2 py-0.5 text-left"
                    onClick={() => void selectProject(project.id)}
                  >
                    <span className="h-2 w-2 shrink-0 rounded-full bg-primary" />
                    <span className="truncate">{project.name}</span>
                  </button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="opacity-0 group-hover:opacity-100"
                    disabled={Boolean(operatingIds[project.id])}
                    onClick={() => void renameSelectedProject(project.id, project.name)}
                    aria-label={t('cloudLibrary.action.renameProject', { name: project.name })}
                  >
                    <Pencil size={13} />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="opacity-0 hover:text-destructive group-hover:opacity-100"
                    disabled={projects.length <= 1 || Boolean(operatingIds[project.id])}
                    onClick={() => void removeSelectedProject(project.id, project.name)}
                    aria-label={t('cloudLibrary.action.deleteProject', { name: project.name })}
                  >
                    <X size={13} />
                  </Button>
                </div>
              ))}
            </div>
          </section>

          <section className="mt-5">
            <p className="mb-2 text-xs font-medium text-muted-foreground">{t('cloudLibrary.views')}</p>
            <div className="space-y-1">
              {viewItems.map((item) => {
                const Icon = item.icon;
                return (
                  <button
                    key={item.id}
                    type="button"
                    className={cn(
                      'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors hover:bg-accent',
                      view === item.id && 'bg-accent text-accent-foreground',
                    )}
                    onClick={() => void setView(item.id)}
                  >
                    <Icon size={15} />
                    <span>{t(item.labelKey)}</span>
                  </button>
                );
              })}
            </div>
          </section>

          <section className="mt-5">
            <div className="mb-2 flex items-center justify-between">
              <p className="text-xs font-medium text-muted-foreground">{t('cloudLibrary.folders')}</p>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => void addFolder()}
                disabled={!selectedProjectId || creatingFolder}
                aria-label={t('cloudLibrary.action.newFolder')}
              >
                <FolderPlus size={14} />
              </Button>
            </div>
            <div className="space-y-1">
              <button
                type="button"
                className={cn(
                  'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors hover:bg-accent',
                  view === 'all' && selectedFolderId === null && 'bg-accent text-accent-foreground',
                )}
                onClick={() => void selectFolder(null)}
              >
                <Folder size={15} />
                <span>{t('cloudLibrary.projectRoot')}</span>
              </button>
              {rootFolders.map((folder) => (
                <div
                  key={folder.id}
                  className={cn(
                    'group flex items-center gap-1 rounded-md px-2 py-1 text-sm transition-colors hover:bg-accent',
                    selectedFolderId === folder.id && view === 'all' && 'bg-accent text-accent-foreground',
                  )}
                >
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-2 py-0.5 text-left"
                    onClick={() => void selectFolder(folder.id)}
                    aria-label={t('cloudLibrary.action.folder', { name: folder.name })}
                  >
                    <Folder size={15} className="shrink-0" />
                    <span className="truncate">{folder.name}</span>
                  </button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="opacity-0 group-hover:opacity-100"
                    disabled={Boolean(operatingIds[folder.id])}
                    onClick={() => void renameSelectedFolder(folder.id, folder.name)}
                    aria-label={t('cloudLibrary.action.renameFolder', { name: folder.name })}
                  >
                    <Pencil size={13} />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="opacity-0 hover:text-destructive group-hover:opacity-100"
                    disabled={Boolean(operatingIds[folder.id])}
                    onClick={() => void removeSelectedFolder(folder.id, folder.name)}
                    aria-label={t('cloudLibrary.action.deleteFolder', { name: folder.name })}
                  >
                    <X size={13} />
                  </Button>
                </div>
              ))}
              {foldersLoading && (
                <p className="px-2 py-2 text-xs text-muted-foreground">
                  {t('cloudLibrary.loadingFolders')}
                </p>
              )}
            </div>
          </section>
        </aside>

        <section className="min-w-0 p-5">
          {error && (
            <div className="mb-4 border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </div>
          )}

          <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
            <div className="min-w-0">
              <p className="text-sm font-semibold">{selectedProject?.name ?? t('cloudLibrary.cloudFiles')}</p>
              <p className="text-xs text-muted-foreground">
                {view === 'all'
                  ? folderLabel
                  : t(viewItems.find((item) => item.id === view)?.labelKey ?? 'cloudLibrary.view.all')}
              </p>
            </div>
            <div className="flex items-center gap-2">
              <div className="relative">
                <Search
                  size={14}
                  className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground"
                />
                <input
                  value={search}
                  onChange={(event) => void setSearch(event.target.value)}
                  placeholder={t('cloudLibrary.searchFiles')}
                  className="h-8 w-56 rounded-md border border-input bg-background pl-7 pr-2 text-xs outline-none focus:border-ring"
                />
              </div>
              <select
                aria-label={t('cloudLibrary.sortFiles')}
                value={sort}
                onChange={(event) => void setSort(event.target.value as CloudFileSort)}
                className="h-8 rounded-md border border-input bg-background px-2 text-xs outline-none focus:border-ring"
              >
                {Object.entries(sortLabelKeys).map(([value, labelKey]) => (
                  <option key={value} value={value}>
                    {t(labelKey)}
                  </option>
                ))}
              </select>
              <div className="flex rounded-md border border-input bg-background p-0.5">
                <Button
                  variant={layout === 'list' ? 'secondary' : 'ghost'}
                  size="icon-sm"
                  aria-label={t('cloudLibrary.layout.list')}
                  onClick={() => setLayout('list')}
                >
                  <List size={14} />
                </Button>
                <Button
                  variant={layout === 'grid' ? 'secondary' : 'ghost'}
                  size="icon-sm"
                  aria-label={t('cloudLibrary.layout.grid')}
                  onClick={() => setLayout('grid')}
                >
                  <Grid2X2 size={14} />
                </Button>
              </div>
              <Button variant="outline" size="sm" onClick={() => void addFolder()} disabled={creatingFolder}>
                <FolderPlus size={14} />
                {t('cloudLibrary.folder')}
              </Button>
            </div>
          </div>

          {view === 'all' && selectedFolderChildren.length > 0 && (
            <div className="mb-4 grid grid-cols-2 gap-2 lg:grid-cols-4">
              {selectedFolderChildren.map((folder) => (
                <button
                  key={folder.id}
                  type="button"
                  aria-label={t('cloudLibrary.action.folder', { name: folder.name })}
                  className="flex items-center gap-2 rounded-md border border-border bg-card px-3 py-2 text-left text-sm hover:border-primary/50"
                  onClick={() => void selectFolder(folder.id)}
                >
                  <Folder size={16} className="text-muted-foreground" />
                  <span className="truncate">{folder.name}</span>
                </button>
              ))}
            </div>
          )}

          {selectedFiles.length > 0 && view !== 'shared' && (
            <div className="mb-3 flex items-center justify-between border border-border bg-card px-3 py-2 text-sm">
              <span className="font-medium">
                {t('common.selected', { count: selectedFiles.length })}
              </span>
              <div className="flex items-center gap-2">
                <Button variant="ghost" size="sm" onClick={() => setSelectedFileIds([])}>
                  {t('cloudLibrary.selection.clear')}
                </Button>
                <CloudMovePopover
                  fileName={t('cloudLibrary.selection.files', { count: selectedFiles.length })}
                  folders={folders}
                  currentFolderId={selectedFolderId}
                  folderPath={fileLocation}
                  onMove={(folderId) => void moveSelectedFiles(folderId)}
                />
                <Button variant="outline" size="sm" onClick={() => void favoriteSelectedFiles()}>
                  <Star size={14} />
                  {t('cloudLibrary.selection.favorite')}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="hover:text-destructive"
                  onClick={() => void deleteSelectedFiles()}
                >
                  <Trash2 size={14} />
                  {t('cloudLibrary.selection.delete')}
                </Button>
              </div>
            </div>
          )}

          {movingFile && (
            <CloudMoveTargetPanel
              file={movingFile}
              folders={folders}
              folderPath={fileLocation}
              onClose={() => setMovingFileId(null)}
              onMove={(folderId) => {
                setMovingFileId(null);
                void moveFile(movingFile.id, folderId);
              }}
            />
          )}

          {loading && files.length === 0 ? (
            <div className="border border-border bg-card p-10 text-center text-sm text-muted-foreground">
                {t('cloudLibrary.loadingFiles')}
            </div>
          ) : files.length === 0 ? (
            <div className="border border-border bg-card p-10 text-center">
              <p className="text-sm font-medium">{t(emptyState.titleKey)}</p>
              <p className="mt-1 text-sm text-muted-foreground">{t(emptyState.descriptionKey)}</p>
              {emptyState.showCreateActions && (
                <div className="mt-5 flex justify-center gap-2">
                  <Button onClick={() => void createNew()} disabled={creating}>
                    <Plus size={15} />
                    {t('cloudLibrary.newDesign')}
                  </Button>
                  <Button variant="outline" onClick={() => inputRef.current?.click()} disabled={creating}>
                    <Upload size={15} />
                    {t('cloudLibrary.importOp')}
                  </Button>
                </div>
              )}
            </div>
          ) : layout === 'grid' ? (
            <CloudFileGrid
              files={files}
              selectedFileIds={selectedFileIds}
              actionsFileId={actionsFileId}
              operatingIds={operatingIds}
              isTrashView={view === 'trash'}
              canMoveFile={canMoveFile}
              formatDate={formatDate}
              folderPath={fileLocation}
              onToggleSelected={toggleSelectedFile}
              onOpen={openFile}
              onShowDetails={showFileDetails}
              onToggleFavorite={toggleFileFavorite}
              onRename={renameTableFile}
              onCopy={copyTableFile}
              onMove={moveTableFile}
              onShare={shareTableFile}
              onDelete={deleteTableFile}
              onRestore={restoreTableFile}
              onPermanentDelete={permanentlyDeleteTableFile}
              onToggleActions={setActionsFileId}
            />
          ) : (
            <CloudFileTable
              files={files}
              selectedFileIds={selectedFileIds}
              actionsFileId={actionsFileId}
              operatingIds={operatingIds}
              isTrashView={view === 'trash'}
              canMoveFile={canMoveFile}
              formatDate={formatDate}
              folderPath={fileLocation}
              onToggleSelected={toggleSelectedFile}
              onOpen={openFile}
              onShowDetails={showFileDetails}
              onToggleFavorite={toggleFileFavorite}
              onRename={renameTableFile}
              onCopy={copyTableFile}
              onMove={moveTableFile}
              onShare={shareTableFile}
              onDelete={deleteTableFile}
              onRestore={restoreTableFile}
              onPermanentDelete={permanentlyDeleteTableFile}
              onToggleActions={setActionsFileId}
            />
          )}
        </section>
        {detailsFile && (
          <CloudFileDetailsPanel
            file={detailsFile}
            location={fileLocation(detailsFile.folderId)}
            projectName={selectedProject?.name ?? t('cloudLibrary.cloudFiles')}
            updatedLabel={formatDate(detailsFile.updatedAt)}
            createdLabel={formatDate(detailsFile.createdAt)}
            onClose={() => setDetailsFileId(null)}
          />
        )}
      </main>
    </div>
  );
}
