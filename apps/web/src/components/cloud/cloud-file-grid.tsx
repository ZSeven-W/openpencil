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
    <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
      {files.map((file) => {
        const busy = Boolean(operatingIds[file.id]);
        const canMove = canMoveFile(file);
        return (
          <article
            key={file.id}
            className={cn(
              'relative rounded-md border border-border bg-card p-3',
              selectedFileIds.includes(file.id) && 'bg-accent/60',
            )}
            onContextMenu={(event) => {
              event.preventDefault();
              onToggleActions(file.id);
            }}
          >
            <div className="mb-3 flex items-start justify-between gap-2">
              <input
                type="checkbox"
                aria-label={t('cloudLibrary.action.selectFile', { name: file.name })}
                checked={selectedFileIds.includes(file.id)}
                onChange={() => onToggleSelected(file.id)}
                className="mt-1 h-4 w-4 rounded border-border"
              />
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t('cloudLibrary.action.moreActionsFor', { name: file.name })}
                disabled={busy}
                onClick={() => onToggleActions(actionsFileId === file.id ? null : file.id)}
              >
                <MoreHorizontal size={14} />
              </Button>
            </div>
            <button
              type="button"
              aria-label={t('cloudLibrary.action.openFile', { name: file.name })}
              className="block w-full text-left"
              onClick={() => onOpen(file.id)}
              disabled={isTrashView}
            >
              <div className="mb-3 flex aspect-[16/10] items-center justify-center rounded-md border border-border bg-background">
                <FileJson size={28} className="text-muted-foreground" />
              </div>
              <span className="block truncate text-sm font-medium">{file.name}</span>
              <span className="mt-1 block truncate text-xs text-muted-foreground">
                {folderPath(file.folderId)}
              </span>
              <span className="mt-2 block text-xs text-muted-foreground">
                rev {file.revision} · {formatDate(file.updatedAt)}
              </span>
            </button>
            <div className="mt-3 flex items-center justify-between gap-1 border-t border-border pt-2">
              {isTrashView ? (
                <>
                  <IconButton
                    label={t('cloudLibrary.action.restoreFile', { name: file.name })}
                    disabled={busy}
                    onClick={() => onRestore(file)}
                  >
                    <ArchiveRestore size={14} />
                  </IconButton>
                  <IconButton
                    label={t('cloudLibrary.action.permanentlyDeleteFile', { name: file.name })}
                    disabled={busy}
                    destructive
                    onClick={() => onPermanentDelete(file)}
                  >
                    <Trash2 size={14} />
                  </IconButton>
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
                  <IconButton
                    label={t('cloudLibrary.action.deleteFile', { name: file.name })}
                    disabled={busy}
                    destructive
                    onClick={() => onDelete(file)}
                  >
                    <Trash2 size={14} />
                  </IconButton>
                </>
              )}
            </div>
            {actionsFileId === file.id && (
              <CloudFileActionsMenu
                file={file}
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
            )}
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
