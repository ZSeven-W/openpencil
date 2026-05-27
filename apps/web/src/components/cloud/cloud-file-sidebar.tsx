import { Folder, FolderPlus, Pencil, Plus, Trash2 } from 'lucide-react';
import type { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { ConfirmActionButton } from '@/components/workbench/confirm-action-button';
import { cn } from '@/lib/utils';
import type { CloudFileView, CloudFolder, CloudProject } from '@/types/cloud';
import { viewItems } from './cloud-file-library-utils';

interface CloudFileSidebarProps {
  projects: CloudProject[];
  projectsLoading: boolean;
  foldersLoading: boolean;
  rootFolders: CloudFolder[];
  selectedProjectId: string | null;
  selectedFolderId: string | null;
  view: CloudFileView;
  operatingIds: Record<string, string | undefined>;
  creatingProject: boolean;
  creatingFolder: boolean;
  onAddProject: () => void;
  onSelectProject: (projectId: string) => void;
  onRenameProject: (projectId: string, projectName: string) => void;
  onDeleteProject: (projectId: string) => void | Promise<void>;
  onSetView: (view: CloudFileView) => void;
  onAddFolder: () => void;
  onSelectFolder: (folderId: string | null) => void;
  onRenameFolder: (folderId: string, folderName: string) => void;
  onDeleteFolder: (folderId: string) => void | Promise<void>;
  t: ReturnType<typeof useTranslation>['t'];
}

export function CloudFileSidebar({
  projects,
  projectsLoading,
  foldersLoading,
  rootFolders,
  selectedProjectId,
  selectedFolderId,
  view,
  operatingIds,
  creatingProject,
  creatingFolder,
  onAddProject,
  onSelectProject,
  onRenameProject,
  onDeleteProject,
  onSetView,
  onAddFolder,
  onSelectFolder,
  onRenameFolder,
  onDeleteFolder,
  t,
}: CloudFileSidebarProps) {
  return (
    <aside className="border-r border-border bg-card p-2">
      <section>
        <div className="mb-1 flex h-7 items-center justify-between">
          <p className="text-xs font-medium text-muted-foreground">{t('cloudLibrary.projects')}</p>
          <div className="flex items-center gap-1">
            {projectsLoading && (
              <span className="text-[11px] text-muted-foreground">
                {t('cloudLibrary.loading')}
              </span>
            )}
            <Button
              variant="ghost"
              size="icon-sm"
              className="size-7 rounded"
              onClick={onAddProject}
              disabled={creatingProject}
              aria-label={t('cloudLibrary.action.newProject')}
            >
              <Plus size={14} />
            </Button>
          </div>
        </div>
        <div className="flex flex-col gap-0.5">
          {projects.map((project) => (
            <div
              key={project.id}
              className={cn(
                'group flex items-center gap-1 rounded px-1.5 py-0.5 text-xs transition-colors hover:bg-accent',
                selectedProjectId === project.id && 'bg-accent text-accent-foreground',
              )}
            >
              <Button
                type="button"
                variant="ghost"
                className="h-7 min-w-0 flex-1 justify-start gap-1.5 px-0 py-0 text-left text-xs hover:bg-transparent"
                onClick={() => onSelectProject(project.id)}
              >
                <span className="h-2 w-2 shrink-0 rounded-full bg-primary" />
                <span className="truncate">{project.name}</span>
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                className="size-7 rounded opacity-0 group-hover:opacity-100"
                disabled={Boolean(operatingIds[project.id])}
                onClick={() => onRenameProject(project.id, project.name)}
                aria-label={t('cloudLibrary.action.renameProject', { name: project.name })}
              >
                <Pencil size={13} />
              </Button>
              <ConfirmActionButton
                variant="ghost"
                size="icon-sm"
                className="size-7 rounded opacity-0 hover:text-destructive group-hover:opacity-100"
                disabled={projects.length <= 1 || Boolean(operatingIds[project.id])}
                title={t('cloudLibrary.confirm.deleteProject', { name: project.name })}
                description={t('cloudLibrary.confirm.deleteProject', { name: project.name })}
                confirmLabel={t('common.delete')}
                cancelLabel={t('common.cancel')}
                onConfirm={() => onDeleteProject(project.id)}
                ariaLabel={t('cloudLibrary.action.deleteProject', { name: project.name })}
              >
                <Trash2 size={13} />
              </ConfirmActionButton>
            </div>
          ))}
        </div>
      </section>

      <section className="mt-3 border-t border-border pt-2">
        <p className="mb-1 px-1 text-xs font-medium text-muted-foreground">{t('cloudLibrary.views')}</p>
        <div className="flex flex-col gap-0.5">
          {viewItems.map((item) => {
            const Icon = item.icon;
            return (
              <Button
                key={item.id}
                type="button"
                variant={view === item.id ? 'secondary' : 'ghost'}
                className={cn(
                  'h-7 w-full justify-start gap-1.5 rounded px-1.5 py-0 text-left text-xs',
                  view === item.id && 'bg-accent text-accent-foreground',
                )}
                onClick={() => onSetView(item.id)}
              >
                <Icon size={15} />
                <span>{t(item.labelKey)}</span>
              </Button>
            );
          })}
        </div>
      </section>

      <section className="mt-3 border-t border-border pt-2">
        <div className="mb-1 flex h-7 items-center justify-between">
          <p className="text-xs font-medium text-muted-foreground">{t('cloudLibrary.folders')}</p>
          <Button
            variant="ghost"
            size="icon-sm"
            className="size-7 rounded"
            onClick={onAddFolder}
            disabled={!selectedProjectId || creatingFolder}
            aria-label={t('cloudLibrary.action.newFolder')}
          >
            <FolderPlus size={14} />
          </Button>
        </div>
        <div className="flex flex-col gap-1">
          <Button
            type="button"
            variant={view === 'all' && selectedFolderId === null ? 'secondary' : 'ghost'}
            className={cn(
              'h-7 w-full justify-start gap-1.5 rounded px-1.5 py-0 text-left text-xs',
              view === 'all' && selectedFolderId === null && 'bg-accent text-accent-foreground',
            )}
            onClick={() => onSelectFolder(null)}
          >
            <Folder size={15} />
            <span>{t('cloudLibrary.projectRoot')}</span>
          </Button>
          {rootFolders.map((folder) => (
            <div
              key={folder.id}
              className={cn(
                'group flex items-center gap-1 rounded px-1.5 py-0.5 text-xs transition-colors hover:bg-accent',
                selectedFolderId === folder.id &&
                  view === 'all' &&
                  'bg-accent text-accent-foreground',
              )}
            >
              <Button
                type="button"
                variant="ghost"
                className="h-7 min-w-0 flex-1 justify-start gap-1.5 px-0 py-0 text-left text-xs hover:bg-transparent"
                onClick={() => onSelectFolder(folder.id)}
                aria-label={t('cloudLibrary.action.folder', { name: folder.name })}
              >
                <Folder size={15} className="shrink-0" />
                <span className="truncate">{folder.name}</span>
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                className="size-7 rounded opacity-0 group-hover:opacity-100"
                disabled={Boolean(operatingIds[folder.id])}
                onClick={() => onRenameFolder(folder.id, folder.name)}
                aria-label={t('cloudLibrary.action.renameFolder', { name: folder.name })}
              >
                <Pencil size={13} />
              </Button>
              <ConfirmActionButton
                variant="ghost"
                size="icon-sm"
                className="size-7 rounded opacity-0 hover:text-destructive group-hover:opacity-100"
                disabled={Boolean(operatingIds[folder.id])}
                title={t('cloudLibrary.confirm.deleteFolder', { name: folder.name })}
                description={t('cloudLibrary.confirm.deleteFolder', { name: folder.name })}
                confirmLabel={t('common.delete')}
                cancelLabel={t('common.cancel')}
                onConfirm={() => onDeleteFolder(folder.id)}
                ariaLabel={t('cloudLibrary.action.deleteFolder', { name: folder.name })}
              >
                <Trash2 size={13} />
              </ConfirmActionButton>
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
  );
}
