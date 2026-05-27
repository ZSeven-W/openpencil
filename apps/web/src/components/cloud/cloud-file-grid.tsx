import {
  ArchiveRestore,
  Copy,
  FileJson,
  FolderInput,
  MoreHorizontal,
  Pencil,
  Share2,
  Star,
  Trash2,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { ConfirmActionButton } from '@/components/workbench/confirm-action-button';
import { CloudFileActionsMenu } from './cloud-file-actions-menu';
import { cn } from '@/lib/utils';
import type { CloudFileSummary } from '@/types/cloud';

interface CloudFileGridProps {
  files: CloudFileSummary[];
  selectedFileIds: string[];
  actionsFileId: string | null;
  operatingIds: Record<string, string | undefined>;
  isTrashView: boolean;
  canMoveFile: (file: CloudFileSummary) => boolean;
  formatDate: (value: string) => string;
  folderPath: (folderId: string | null) => string;
  onToggleSelected: (fileId: string) => void;
  onOpen: (fileId: string) => void;
  onShowDetails: (fileId: string) => void;
  onToggleFavorite: (file: CloudFileSummary) => void;
  onRename: (file: CloudFileSummary) => void;
  onCopy: (file: CloudFileSummary) => void;
  onMove: (file: CloudFileSummary) => void;
  onShare: (file: CloudFileSummary) => void;
  onDelete: (file: CloudFileSummary) => void;
  onRestore: (file: CloudFileSummary) => void;
  onPermanentDelete: (file: CloudFileSummary) => void;
  onToggleActions: (fileId: string | null) => void;
}

export function CloudFileGrid({
  files,
  selectedFileIds,
  actionsFileId,
  operatingIds,
  isTrashView,
  canMoveFile,
  formatDate,
  folderPath,
  onToggleSelected,
  onOpen,
  onShowDetails,
  onToggleFavorite,
  onRename,
  onCopy,
  onMove,
  onShare,
  onDelete,
  onRestore,
  onPermanentDelete,
  onToggleActions,
}: CloudFileGridProps) {
  const { t } = useTranslation();
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-2">
      {files.map((file) => {
        const busy = Boolean(operatingIds[file.id]);
        const canMove = canMoveFile(file);
        return (
          <article
            key={file.id}
            className={cn(
              'relative rounded border border-border bg-card p-2.5',
              selectedFileIds.includes(file.id) && 'bg-accent/60',
            )}
            onContextMenu={(event) => {
              event.preventDefault();
              onToggleActions(file.id);
            }}
          >
            <div className="mb-2 flex items-start justify-between gap-2">
              <Checkbox
                aria-label={t('cloudLibrary.action.selectFile', { name: file.name })}
                checked={selectedFileIds.includes(file.id)}
                onCheckedChange={() => onToggleSelected(file.id)}
                className="mt-1"
              />
              <CloudFileActionsMenu
                file={file}
                open={actionsFileId === file.id}
                onOpenChange={(open) => onToggleActions(open ? file.id : null)}
                trigger={
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={t('cloudLibrary.action.moreActionsFor', { name: file.name })}
                    disabled={busy}
                  >
                    <MoreHorizontal size={14} />
                  </Button>
                }
                isTrash={isTrashView}
                canMove={canMove}
                onOpen={() => onOpen(file.id)}
                onDetails={() => onShowDetails(file.id)}
                onFavorite={() => onToggleFavorite(file)}
                onRename={() => onRename(file)}
                onCopy={() => onCopy(file)}
                onMove={() => onMove(file)}
                onShare={() => onShare(file)}
                onDelete={() => onDelete(file)}
                onRestore={() => onRestore(file)}
                onPermanentDelete={() => onPermanentDelete(file)}
              />
            </div>
            <Button
              type="button"
              variant="ghost"
              aria-label={t('cloudLibrary.action.openFile', { name: file.name })}
              className="h-auto w-full justify-start p-0 text-left"
              onClick={() => onOpen(file.id)}
              disabled={isTrashView}
            >
              <span className="block min-w-0 flex-1">
                <span className="mb-2 flex aspect-[16/10] items-center justify-center rounded border border-border bg-background">
                  <FileJson size={24} className="text-muted-foreground" />
                </span>
                <span className="block truncate text-xs font-medium">{file.name}</span>
                <span className="mt-1 block truncate text-xs text-muted-foreground">
                  {folderPath(file.folderId)}
                </span>
                <span className="mt-2 block text-xs text-muted-foreground">
                  rev {file.revision} · {formatDate(file.updatedAt)}
                </span>
              </span>
            </Button>
            <div className="mt-2 flex items-center justify-between gap-1 border-t border-border pt-1.5">
              {isTrashView ? (
                <>
                  <IconButton
                    label={t('cloudLibrary.action.restoreFile', { name: file.name })}
                    disabled={busy}
                    onClick={() => onRestore(file)}
                  >
                    <ArchiveRestore size={14} />
                  </IconButton>
                  <ConfirmIconButton
                    label={t('cloudLibrary.action.permanentlyDeleteFile', { name: file.name })}
                    disabled={busy}
                    title={t('cloudLibrary.confirm.permanentlyDeleteFile', { name: file.name })}
                    onConfirm={() => onPermanentDelete(file)}
                  >
                    <Trash2 size={14} />
                  </ConfirmIconButton>
                </>
              ) : (
                <>
                  <IconButton
                    label={t('cloudLibrary.action.detailsFile', { name: file.name })}
                    disabled={busy}
                    onClick={() => onShowDetails(file.id)}
                  >
                    <FileJson size={14} />
                  </IconButton>
                  <IconButton
                    label={t('cloudLibrary.action.favoriteFile', { name: file.name })}
                    disabled={busy}
                    onClick={() => onToggleFavorite(file)}
                  >
                    <Star size={14} className={cn(file.starred && 'fill-current text-primary')} />
                  </IconButton>
                  <IconButton
                    label={t('cloudLibrary.action.renameFile', { name: file.name })}
                    disabled={busy}
                    onClick={() => onRename(file)}
                  >
                    <Pencil size={14} />
                  </IconButton>
                  <IconButton
                    label={t('cloudLibrary.action.copyFile', { name: file.name })}
                    disabled={busy}
                    onClick={() => onCopy(file)}
                  >
                    <Copy size={14} />
                  </IconButton>
                  {canMove && (
                    <IconButton
                      label={t('cloudLibrary.action.moveFile', { name: file.name })}
                      disabled={busy}
                      onClick={() => onMove(file)}
                    >
                      <FolderInput size={14} />
                    </IconButton>
                  )}
                  <IconButton
                    label={t('cloudLibrary.action.shareFile', { name: file.name })}
                    disabled={busy}
                    onClick={() => onShare(file)}
                  >
                    <Share2 size={14} />
                  </IconButton>
                  <ConfirmIconButton
                    label={t('cloudLibrary.action.deleteFile', { name: file.name })}
                    disabled={busy}
                    title={t('cloudLibrary.confirm.deleteFile', { name: file.name })}
                    onConfirm={() => onDelete(file)}
                  >
                    <Trash2 size={14} />
                  </ConfirmIconButton>
                </>
              )}
            </div>
          </article>
        );
      })}
    </div>
  );
}

function IconButton({
  label,
  disabled,
  destructive,
  children,
  onClick,
}: {
  label: string;
  disabled: boolean;
  destructive?: boolean;
  children: React.ReactNode;
  onClick: () => void;
}) {
  return (
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={label}
      disabled={disabled}
      className={destructive ? 'text-muted-foreground hover:text-destructive' : undefined}
      onClick={onClick}
    >
      {children}
    </Button>
  );
}

function ConfirmIconButton({
  label,
  disabled,
  title,
  children,
  onConfirm,
}: {
  label: string;
  disabled: boolean;
  title: string;
  children: React.ReactNode;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();
  return (
    <ConfirmActionButton
      variant="ghost"
      size="icon-sm"
      ariaLabel={label}
      disabled={disabled}
      title={title}
      description={title}
      confirmLabel={t('common.delete')}
      cancelLabel={t('common.cancel')}
      className="text-muted-foreground hover:text-destructive"
      onConfirm={onConfirm}
    >
      {children}
    </ConfirmActionButton>
  );
}
