import { useEffect, useMemo, useRef, useState, type RefObject } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import {
  FileJson,
  Folder,
  FolderPlus,
  Grid2X2,
  List,
  ListTodo,
  Plus,
  RefreshCw,
  Search,
  Star,
  Trash2,
  Upload,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldLabel,
} from '@/components/ui/field';
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import { Skeleton } from '@/components/ui/skeleton';
import { ConfirmActionButton } from '@/components/workbench/confirm-action-button';
import { WorkbenchPagination } from '@/components/workbench/workbench-pagination';
import { WorkbenchShell } from '@/components/workbench/workbench-shell';
import { CloudFileActionDialog, type CloudFileDialogAction } from './cloud-file-action-dialog';
import { CloudFileDetailsPanel } from './cloud-file-details-panel';
import { CloudFileGrid } from './cloud-file-grid';
import { CloudFileSidebar } from './cloud-file-sidebar';
import { CloudFileTable } from './cloud-file-table';
import { CloudMovePopover } from './cloud-move-popover';
import { CloudMoveTargetPanel } from './cloud-move-target-panel';
import { cn } from '@/lib/utils';
import { useCloudFileStore } from '@/stores/cloud-file-store';
import { createEmptyDocument } from '@/stores/document-store';
import { parseAndPrepareImportedDocument } from '@/utils/import-pen-document';
import {
  childFolders,
  emptyStateForView,
  folderPath,
  formatDate,
  sortLabelKeys,
  viewItems,
} from './cloud-file-library-utils';
import type { CloudFileSort, CloudFileSummary } from '@/types/cloud';
import type { PenDocument } from '@/types/pen';

const TABLE_PAGE_SIZE = 10;

function CloudImportField({
  inputRef,
  disabled,
  onImport,
  label,
  description,
}: {
  inputRef: RefObject<HTMLInputElement | null>;
  disabled: boolean;
  onImport: (file: File) => void;
  label: string;
  description: string;
}) {
  return (
    <Field className="sr-only">
      <FieldLabel htmlFor="cloud-import-file">{label}</FieldLabel>
      <FieldContent>
        <Input
          ref={inputRef}
          id="cloud-import-file"
          type="file"
          accept=".op,.pen,.json"
          disabled={disabled}
          onChange={(event) => {
            const file = event.currentTarget.files?.[0];
            event.currentTarget.value = '';
            if (file) onImport(file);
          }}
        />
        <FieldDescription>{description}</FieldDescription>
      </FieldContent>
    </Field>
  );
}

export function CloudFileLibrary() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const {
    files,
    filePage,
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
    setFilePage,
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
  const [dialogAction, setDialogAction] = useState<CloudFileDialogAction | null>(null);
  const selectedProject = projects.find((project) => project.id === selectedProjectId);
  const selectedFiles = files.filter((file) => selectedFileIds.includes(file.id));
  const detailsFile = files.find((file) => file.id === detailsFileId) ?? null;
  const movingFile = files.find((file) => file.id === movingFileId) ?? null;
  const currentFilePage = Math.floor(filePage.offset / filePage.limit) + 1;
  const fileCountLabel = t('cloudLibrary.fileCount', { count: filePage.total });
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

  const addFolder = async (name: string) => {
    setCreatingFolder(true);
    try {
      await createFolder(name);
    } finally {
      setCreatingFolder(false);
    }
  };

  const addProject = async (name: string) => {
    setCreatingProject(true);
    try {
      await createProject(name);
    } finally {
      setCreatingProject(false);
    }
  };

  const renameSelectedProject = async (projectId: string, projectName: string, name: string) => {
    if (!name || name === projectName) return;
    await renameProject(projectId, name);
  };

  const removeSelectedProject = async (projectId: string) => {
    await deleteProject(projectId);
  };

  const renameSelectedFolder = async (folderId: string, folderName: string, name: string) => {
    if (!name || name === folderName) return;
    await renameFolder(folderId, name);
  };

  const removeSelectedFolder = async (folderId: string) => {
    await deleteFolder(folderId);
  };

  const renameSelectedFile = async (fileId: string, fileName: string, name: string) => {
    if (!name || name === fileName) return;
    await renameFile(fileId, name);
  };

  const copySelectedFile = async (fileId: string, name: string) => {
    if (!name) return;
    await copyFile(fileId, name);
  };

  const shareSelectedFile = async (fileId: string, email: string, role: 'viewer' | 'editor') => {
    if (!email) return;
    await shareFile(fileId, email, role);
  };

  const removeFile = async (fileId: string, fileName: string) => {
    void fileName;
    await deleteFile(fileId);
  };

  const permanentlyRemoveFile = async (fileId: string, fileName: string) => {
    void fileName;
    await permanentlyDeleteFile(fileId);
  };

  const toggleSelectedFile = (fileId: string) => {
    setSelectedFileIds((ids) =>
      ids.includes(fileId) ? ids.filter((id) => id !== fileId) : [...ids, fileId],
    );
  };

  const toggleSelectedFiles = (fileIds: string[]) => {
    if (fileIds.length === 0) return;
    setSelectedFileIds((ids) => {
      const allSelected = fileIds.every((id) => ids.includes(id));
      if (allSelected) return ids.filter((id) => !fileIds.includes(id));
      return Array.from(new Set([...ids, ...fileIds]));
    });
  };

  const favoriteSelectedFiles = async () => {
    for (const file of selectedFiles) {
      await toggleStarred(file.id, true);
    }
  };

  const deleteSelectedFiles = async () => {
    if (selectedFiles.length === 0) return;
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

  const submitDialogAction = async ({
    name,
    email,
    role = 'viewer',
  }: {
    name?: string;
    email?: string;
    role?: 'viewer' | 'editor';
  }) => {
    const action = dialogAction;
    if (!action) return;
    if (action.kind === 'new-project' && name) await addProject(name);
    if (action.kind === 'new-folder' && name) await addFolder(name);
    if (action.kind === 'rename-project' && name) {
      await renameSelectedProject(action.id, action.defaultValue, name);
    }
    if (action.kind === 'rename-folder' && name) {
      await renameSelectedFolder(action.id, action.defaultValue, name);
    }
    if (action.kind === 'rename-file' && name) {
      await renameSelectedFile(action.id, action.defaultValue, name);
    }
    if (action.kind === 'copy-file' && name) await copySelectedFile(action.id, name);
    if (action.kind === 'share-file' && email) await shareSelectedFile(action.id, email, role);
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
    closeActions(() =>
      setDialogAction({ kind: 'rename-file', id: file.id, defaultValue: file.name }),
    );
  };

  const copyTableFile = (file: CloudFileSummary) => {
    closeActions(() =>
      setDialogAction({
        kind: 'copy-file',
        id: file.id,
        defaultValue: t('cloudLibrary.copyName', { name: file.name }),
      }),
    );
  };

  const moveTableFile = (file: CloudFileSummary) => {
    closeActions(() => setMovingFileId(file.id));
  };

  const shareTableFile = (file: CloudFileSummary) => {
    closeActions(() => setDialogAction({ kind: 'share-file', id: file.id, defaultValue: '' }));
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
  const contentColumns = 'grid-cols-[14rem_minmax(0,1fr)]';

  return (
    <WorkbenchShell
      active="cloud"
      title={t('cloudLibrary.title')}
      description={fileCountLabel}
      onCreateDesign={() => void createNew()}
      createDisabled={creating || importing}
      toolbar={
        <CloudImportField
          inputRef={inputRef}
          disabled={creating || importing}
          onImport={(file) => void importOp(file)}
          label={t('cloudLibrary.importOp')}
          description={t('cloudLibrary.empty.default.description')}
        />
      }
      contentClassName="p-0"
    >
      <main className={cn('grid min-h-[calc(100vh-2.5rem)]', contentColumns)}>
        <CloudFileSidebar
          projects={projects}
          projectsLoading={projectsLoading}
          foldersLoading={foldersLoading}
          rootFolders={rootFolders}
          selectedProjectId={selectedProjectId}
          selectedFolderId={selectedFolderId}
          view={view}
          operatingIds={operatingIds}
          creatingProject={creatingProject}
          creatingFolder={creatingFolder}
          onAddProject={() =>
            setDialogAction({
              kind: 'new-project',
              defaultValue: t('cloudLibrary.prompt.newProject'),
            })
          }
          onSelectProject={(projectId) => void selectProject(projectId)}
          onRenameProject={(projectId, projectName) =>
            setDialogAction({
              kind: 'rename-project',
              id: projectId,
              defaultValue: projectName,
            })
          }
          onDeleteProject={removeSelectedProject}
          onSetView={(nextView) => void setView(nextView)}
          onAddFolder={() =>
            setDialogAction({
              kind: 'new-folder',
              defaultValue: t('cloudLibrary.prompt.newFolder'),
            })
          }
          onSelectFolder={(folderId) => void selectFolder(folderId)}
          onRenameFolder={(folderId, folderName) =>
            setDialogAction({
              kind: 'rename-folder',
              id: folderId,
              defaultValue: folderName,
            })
          }
          onDeleteFolder={removeSelectedFolder}
          t={t}
        />

        <section className="min-w-0 p-2">
          {error && (
            <div className="mb-2 rounded border border-destructive/40 bg-destructive/10 px-2.5 py-2 text-xs text-destructive">
              {error}
            </div>
          )}

          <div className="mb-2 flex min-h-8 flex-wrap items-center justify-between gap-2 border-b border-border pb-2">
            <div className="min-w-0">
              <p className="text-sm font-semibold">{selectedProject?.name ?? t('cloudLibrary.cloudFiles')}</p>
              <p className="text-xs text-muted-foreground">
                {view === 'all'
                  ? folderLabel
                  : t(viewItems.find((item) => item.id === view)?.labelKey ?? 'cloudLibrary.view.all')}
              </p>
            </div>
            <div className="flex items-center gap-1.5">
              <div className="relative">
                <Search
                  size={14}
                  className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground"
                />
                <Input
                  value={search}
                  onChange={(event) => void setSearch(event.target.value)}
                  placeholder={t('cloudLibrary.searchFiles')}
                  className="h-7 w-52 rounded bg-card pl-7 text-xs"
                />
              </div>
              <Select value={sort} onValueChange={(value) => void setSort(value as CloudFileSort)}>
                <SelectTrigger size="sm" aria-label={t('cloudLibrary.sortFiles')}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {Object.entries(sortLabelKeys).map(([value, labelKey]) => (
                      <SelectItem key={value} value={value}>
                        {t(labelKey)}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
              <div className="flex rounded border border-border bg-card p-0.5">
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
              <Button
                variant="outline"
                size="sm"
                className="h-7 px-2 text-xs shadow-none"
                onClick={() =>
                  setDialogAction({
                    kind: 'new-folder',
                    defaultValue: t('cloudLibrary.prompt.newFolder'),
                  })
                }
                disabled={creatingFolder}
              >
                <FolderPlus size={14} />
                {t('cloudLibrary.folder')}
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-7 px-2 text-xs shadow-none"
                onClick={() => inputRef.current?.click()}
                disabled={creating || importing}
              >
                <Upload size={14} />
                {t('cloudLibrary.importOp')}
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-7 px-2 text-xs shadow-none"
                onClick={() => void loadFiles({ force: true })}
                disabled={loading}
              >
                <RefreshCw size={14} />
                {t('cloudLibrary.refresh')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs"
                onClick={() => void navigate({ to: '/tasks' })}
              >
                <ListTodo size={14} />
                {t('tasks.title')}
              </Button>
            </div>
          </div>

          {view === 'all' && selectedFolderChildren.length > 0 && (
            <div className="mb-2 grid grid-cols-2 gap-1.5 lg:grid-cols-4">
              {selectedFolderChildren.map((folder) => (
                <Button
                  key={folder.id}
                  type="button"
                  aria-label={t('cloudLibrary.action.folder', { name: folder.name })}
                  variant="outline"
                  className="h-auto justify-start gap-2 rounded bg-card px-2.5 py-1.5 text-left text-xs shadow-none"
                  onClick={() => void selectFolder(folder.id)}
                >
                  <Folder size={16} className="text-muted-foreground" />
                  <span className="truncate">{folder.name}</span>
                </Button>
              ))}
            </div>
          )}

          {selectedFiles.length > 0 && view !== 'shared' && (
            <div className="mb-2 flex items-center justify-between rounded border border-border bg-card px-2.5 py-1.5 text-xs">
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
                <ConfirmActionButton
                  variant="outline"
                  size="sm"
                  className="hover:text-destructive"
                  title={t('cloudLibrary.confirm.deleteSelectedFiles', {
                    count: selectedFiles.length,
                  })}
                  description={t('cloudLibrary.confirm.deleteSelectedFiles', {
                    count: selectedFiles.length,
                  })}
                  confirmLabel={t('common.delete')}
                  cancelLabel={t('common.cancel')}
                  onConfirm={deleteSelectedFiles}
                >
                  <Trash2 size={14} />
                  {t('cloudLibrary.selection.delete')}
                </ConfirmActionButton>
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
            <div className="rounded border border-border bg-card p-3">
              <Skeleton className="h-8 w-56" />
              <Skeleton className="mt-4 h-28 w-full" />
              <Skeleton className="mt-3 h-28 w-full" />
            </div>
          ) : files.length === 0 ? (
            <Empty className="rounded border border-border bg-card">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <FileJson />
                </EmptyMedia>
                <EmptyTitle>{t(emptyState.titleKey)}</EmptyTitle>
                <EmptyDescription>{t(emptyState.descriptionKey)}</EmptyDescription>
              </EmptyHeader>
              {emptyState.showCreateActions && (
                <EmptyContent>
                  <div className="flex justify-center gap-2">
                  <Button onClick={() => void createNew()} disabled={creating}>
                    <Plus size={15} />
                    {t('cloudLibrary.newDesign')}
                  </Button>
                  <Button variant="outline" onClick={() => inputRef.current?.click()} disabled={creating}>
                    <Upload size={15} />
                    {t('cloudLibrary.importOp')}
                  </Button>
                  </div>
                </EmptyContent>
              )}
            </Empty>
          ) : layout === 'grid' ? (
            <>
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
              <div className="mt-3">
                <WorkbenchPagination
                  page={currentFilePage}
                  pageSize={filePage.limit || TABLE_PAGE_SIZE}
                  total={filePage.total}
                  onPageChange={(page) => void setFilePage(page)}
                  labels={{
                    range: (values) => t('tasks.pageRange', values),
                    previous: t('tasks.previousPage'),
                    next: t('tasks.nextPage'),
                  }}
                />
              </div>
            </>
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
              onToggleAll={toggleSelectedFiles}
              pagination={{
                page: currentFilePage,
                pageSize: filePage.limit || TABLE_PAGE_SIZE,
                total: filePage.total,
                onPageChange: (page) => void setFilePage(page),
              }}
            />
          )}
        </section>
      </main>
      <Sheet open={Boolean(detailsFile)} onOpenChange={(open) => !open && setDetailsFileId(null)}>
        <SheetContent side="right" className="w-full gap-0 overflow-y-auto p-0 sm:max-w-md">
          <SheetHeader className="sr-only">
            <SheetTitle>{t('cloudLibrary.details.title')}</SheetTitle>
            <SheetDescription>{detailsFile?.name ?? t('cloudLibrary.details.cloudDesignFile')}</SheetDescription>
          </SheetHeader>
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
        </SheetContent>
      </Sheet>
      <CloudFileActionDialog
        action={dialogAction}
        onOpenChange={(open) => !open && setDialogAction(null)}
        onSubmit={submitDialogAction}
        t={t}
      />
    </WorkbenchShell>
  );
}
