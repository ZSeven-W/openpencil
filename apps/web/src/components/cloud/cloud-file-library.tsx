import { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import {
  Clock3,
  FileJson,
  Folder,
  FolderPlus,
  Grid2X2,
  List,
  LogOut,
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

const viewItems: Array<{ id: CloudFileView; label: string; icon: typeof Grid2X2 }> = [
  { id: 'all', label: 'All files', icon: Grid2X2 },
  { id: 'recent', label: 'Recent', icon: Clock3 },
  { id: 'starred', label: 'Starred', icon: Star },
  { id: 'shared', label: 'Shared with me', icon: Share2 },
  { id: 'trash', label: 'Trash', icon: Trash2 },
];

const sortLabels: Record<CloudFileSort, string> = {
  updated_desc: 'Updated newest',
  updated_asc: 'Updated oldest',
  name_asc: 'Name A-Z',
  name_desc: 'Name Z-A',
  created_desc: 'Created newest',
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

function folderPath(folders: CloudFolder[], folderId: string | null): string {
  if (!folderId) return 'Project root';
  const byId = new Map(folders.map((folder) => [folder.id, folder]));
  const names: string[] = [];
  let current = byId.get(folderId);
  while (current) {
    names.unshift(current.name);
    current = current.parentId ? byId.get(current.parentId) : undefined;
  }
  return names.length > 0 ? names.join(' / ') : 'Folder';
}

function emptyStateForView(view: CloudFileView) {
  if (view === 'trash') {
    return {
      title: 'Trash is empty',
      description: 'Deleted cloud files stay here until you restore or permanently delete them.',
      showCreateActions: false,
    };
  }
  if (view === 'shared') {
    return {
      title: 'No shared files',
      description: 'Files shared with your account will appear here.',
      showCreateActions: false,
    };
  }
  return {
    title: 'No files in this view',
    description: 'Create a design, import an `.op` file, or adjust your filters.',
    showCreateActions: true,
  };
}

export function CloudFileLibrary() {
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
  const fileCountLabel = files.length === 1 ? '1 file' : `${files.length} files`;
  const folderLabel = useMemo(
    () => folderPath(folders, selectedFolderId),
    [folders, selectedFolderId],
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
      const id = await createFile('Untitled', createEmptyDocument() as PenDocument);
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
      const name = file.name.replace(/\.(pen|op|json)$/i, '') || 'Imported design';
      const id = await createFile(name, prepared.doc, 'import');
      if (id) openFile(id);
    } finally {
      setImporting(false);
    }
  };

  const addFolder = async () => {
    const name = window.prompt('New folder name', 'New folder')?.trim();
    if (!name) return;
    setCreatingFolder(true);
    try {
      await createFolder(name);
    } finally {
      setCreatingFolder(false);
    }
  };

  const addProject = async () => {
    const name = window.prompt('New project name', 'New project')?.trim();
    if (!name) return;
    setCreatingProject(true);
    try {
      await createProject(name);
    } finally {
      setCreatingProject(false);
    }
  };

  const renameSelectedProject = async (projectId: string, projectName: string) => {
    const name = window.prompt('Rename project', projectName)?.trim();
    if (!name || name === projectName) return;
    await renameProject(projectId, name);
  };

  const removeSelectedProject = async (projectId: string, projectName: string) => {
    if (!window.confirm(`Delete project "${projectName}"? Files will move to your default project.`)) {
      return;
    }
    await deleteProject(projectId);
  };

  const renameSelectedFolder = async (folderId: string, folderName: string) => {
    const name = window.prompt('Rename folder', folderName)?.trim();
    if (!name || name === folderName) return;
    await renameFolder(folderId, name);
  };

  const removeSelectedFolder = async (folderId: string, folderName: string) => {
    if (!window.confirm(`Delete folder "${folderName}"? Files will move to the project root.`)) {
      return;
    }
    await deleteFolder(folderId);
  };

  const renameSelectedFile = async (fileId: string, fileName: string) => {
    const name = window.prompt('Rename file', fileName)?.trim();
    if (!name || name === fileName) return;
    await renameFile(fileId, name);
  };

  const copySelectedFile = async (fileId: string, fileName: string) => {
    const name = window.prompt('Copy file as', `${fileName} Copy`)?.trim();
    if (!name) return;
    await copyFile(fileId, name);
  };

  const shareSelectedFile = async (fileId: string) => {
    const email = window.prompt('Share with email')?.trim();
    if (!email) return;
    await shareFile(fileId, email, 'viewer');
  };

  const removeFile = async (fileId: string, fileName: string) => {
    if (!window.confirm(`Delete "${fileName}"?`)) return;
    await deleteFile(fileId);
  };

  const permanentlyRemoveFile = async (fileId: string, fileName: string) => {
    if (!window.confirm(`Permanently delete "${fileName}"?`)) return;
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
    if (!window.confirm(`Delete ${selectedFiles.length} selected files?`)) return;
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
    folderId ? folderPath(folders, folderId) : 'Project root';
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
                <h1 className="truncate text-sm font-semibold">OpenPencil Cloud</h1>
                <span className="text-[11px] text-muted-foreground">{fileCountLabel}</span>
              </div>
              <p className="truncate text-xs text-muted-foreground">{user?.email}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={() => void loadFiles()} disabled={loading}>
              <RefreshCw size={15} />
              Refresh
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => inputRef.current?.click()}
              disabled={creating || importing}
            >
              <Upload size={15} />
              Import .op
            </Button>
            <Button size="sm" onClick={() => void createNew()} disabled={creating || importing}>
              <Plus size={15} />
              {creating ? 'Creating...' : 'New'}
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => {
                reset();
                void signOut();
              }}
              aria-label="Sign out"
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
              <p className="text-xs font-medium text-muted-foreground">Projects</p>
              <div className="flex items-center gap-1">
                {projectsLoading && <span className="text-[11px] text-muted-foreground">Loading</span>}
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => void addProject()}
                  disabled={creatingProject}
                  aria-label="New project"
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
                    aria-label={`Rename project ${project.name}`}
                  >
                    <Pencil size={13} />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="opacity-0 hover:text-destructive group-hover:opacity-100"
                    disabled={projects.length <= 1 || Boolean(operatingIds[project.id])}
                    onClick={() => void removeSelectedProject(project.id, project.name)}
                    aria-label={`Delete project ${project.name}`}
                  >
                    <X size={13} />
                  </Button>
                </div>
              ))}
            </div>
          </section>

          <section className="mt-5">
            <p className="mb-2 text-xs font-medium text-muted-foreground">Views</p>
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
                    <span>{item.label}</span>
                  </button>
                );
              })}
            </div>
          </section>

          <section className="mt-5">
            <div className="mb-2 flex items-center justify-between">
              <p className="text-xs font-medium text-muted-foreground">Folders</p>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => void addFolder()}
                disabled={!selectedProjectId || creatingFolder}
                aria-label="New folder"
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
                <span>Project root</span>
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
                    aria-label={`Folder ${folder.name}`}
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
                    aria-label={`Rename folder ${folder.name}`}
                  >
                    <Pencil size={13} />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="opacity-0 hover:text-destructive group-hover:opacity-100"
                    disabled={Boolean(operatingIds[folder.id])}
                    onClick={() => void removeSelectedFolder(folder.id, folder.name)}
                    aria-label={`Delete folder ${folder.name}`}
                  >
                    <X size={13} />
                  </Button>
                </div>
              ))}
              {foldersLoading && (
                <p className="px-2 py-2 text-xs text-muted-foreground">Loading folders...</p>
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
              <p className="text-sm font-semibold">{selectedProject?.name ?? 'Cloud files'}</p>
              <p className="text-xs text-muted-foreground">{view === 'all' ? folderLabel : viewItems.find((item) => item.id === view)?.label}</p>
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
                  placeholder="Search files"
                  className="h-8 w-56 rounded-md border border-input bg-background pl-7 pr-2 text-xs outline-none focus:border-ring"
                />
              </div>
              <select
                aria-label="Sort files"
                value={sort}
                onChange={(event) => void setSort(event.target.value as CloudFileSort)}
                className="h-8 rounded-md border border-input bg-background px-2 text-xs outline-none focus:border-ring"
              >
                {Object.entries(sortLabels).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
              <div className="flex rounded-md border border-input bg-background p-0.5">
                <Button
                  variant={layout === 'list' ? 'secondary' : 'ghost'}
                  size="icon-sm"
                  aria-label="List view"
                  onClick={() => setLayout('list')}
                >
                  <List size={14} />
                </Button>
                <Button
                  variant={layout === 'grid' ? 'secondary' : 'ghost'}
                  size="icon-sm"
                  aria-label="Grid view"
                  onClick={() => setLayout('grid')}
                >
                  <Grid2X2 size={14} />
                </Button>
              </div>
              <Button variant="outline" size="sm" onClick={() => void addFolder()} disabled={creatingFolder}>
                <FolderPlus size={14} />
                Folder
              </Button>
            </div>
          </div>

          {view === 'all' && selectedFolderChildren.length > 0 && (
            <div className="mb-4 grid grid-cols-2 gap-2 lg:grid-cols-4">
              {selectedFolderChildren.map((folder) => (
                <button
                  key={folder.id}
                  type="button"
                  aria-label={`Folder ${folder.name}`}
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
              <span className="font-medium">{selectedFiles.length} selected</span>
              <div className="flex items-center gap-2">
                <Button variant="ghost" size="sm" onClick={() => setSelectedFileIds([])}>
                  Clear
                </Button>
                <CloudMovePopover
                  fileName={`${selectedFiles.length} selected files`}
                  folders={folders}
                  currentFolderId={selectedFolderId}
                  folderPath={fileLocation}
                  onMove={(folderId) => void moveSelectedFiles(folderId)}
                />
                <Button variant="outline" size="sm" onClick={() => void favoriteSelectedFiles()}>
                  <Star size={14} />
                  Favorite selected
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="hover:text-destructive"
                  onClick={() => void deleteSelectedFiles()}
                >
                  <Trash2 size={14} />
                  Delete selected
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
              Loading files...
            </div>
          ) : files.length === 0 ? (
            <div className="border border-border bg-card p-10 text-center">
              <p className="text-sm font-medium">{emptyState.title}</p>
              <p className="mt-1 text-sm text-muted-foreground">{emptyState.description}</p>
              {emptyState.showCreateActions && (
                <div className="mt-5 flex justify-center gap-2">
                  <Button onClick={() => void createNew()} disabled={creating}>
                    <Plus size={15} />
                    New design
                  </Button>
                  <Button variant="outline" onClick={() => inputRef.current?.click()} disabled={creating}>
                    <Upload size={15} />
                    Import .op
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
            projectName={selectedProject?.name ?? 'Cloud files'}
            updatedLabel={formatDate(detailsFile.updatedAt)}
            createdLabel={formatDate(detailsFile.createdAt)}
            onClose={() => setDetailsFileId(null)}
          />
        )}
      </main>
    </div>
  );
}
