import {
  ArchiveRestore,
  Copy,
  ExternalLink,
  FileText,
  FolderInput,
  Pencil,
  Share2,
  Star,
  Trash2,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import type { CloudFileSummary } from '@/types/cloud';

interface CloudFileActionsMenuProps {
  file: CloudFileSummary;
  isTrash: boolean;
  canMove: boolean;
  onOpen: () => void;
  onDetails: () => void;
  onFavorite: () => void;
  onRename: () => void;
  onCopy: () => void;
  onMove: () => void;
  onShare: () => void;
  onDelete: () => void;
  onRestore: () => void;
  onPermanentDelete: () => void;
}

export function CloudFileActionsMenu({
  file,
  isTrash,
  canMove,
  onOpen,
  onDetails,
  onFavorite,
  onRename,
  onCopy,
  onMove,
  onShare,
  onDelete,
  onRestore,
  onPermanentDelete,
}: CloudFileActionsMenuProps) {
  const { t } = useTranslation();
  return (
    <div
      role="menu"
      aria-label={t('cloudLibrary.action.fileActionsFor', { name: file.name })}
      className="absolute right-5 z-20 mt-1 w-52 rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md"
    >
      {isTrash ? (
        <>
          <MenuItem
            label={t('cloudLibrary.action.restore')}
            onClick={onRestore}
            icon={<ArchiveRestore size={14} />}
          />
          <MenuItem
            label={t('cloudLibrary.action.deleteForever')}
            onClick={onPermanentDelete}
            icon={<Trash2 size={14} />}
            destructive
          />
        </>
      ) : (
        <>
          <MenuItem
            label={t('cloudLibrary.action.open')}
            onClick={onOpen}
            icon={<ExternalLink size={14} />}
          />
          <MenuItem
            label={t('cloudLibrary.action.details')}
            onClick={onDetails}
            icon={<FileText size={14} />}
          />
          <MenuItem
            label={
              file.starred
                ? t('cloudLibrary.action.unfavorite')
                : t('cloudLibrary.action.favorite')
            }
            onClick={onFavorite}
            icon={<Star size={14} />}
          />
          <MenuItem
            label={t('common.rename')}
            onClick={onRename}
            icon={<Pencil size={14} />}
          />
          <MenuItem
            label={t('cloudLibrary.action.copy')}
            onClick={onCopy}
            icon={<Copy size={14} />}
          />
          <MenuItem
            label={t('cloudLibrary.action.share')}
            onClick={onShare}
            icon={<Share2 size={14} />}
          />
          {canMove && (
            <MenuItem
              label={t('cloudLibrary.action.move')}
              onClick={onMove}
              icon={<FolderInput size={14} />}
            />
          )}
          <MenuItem
            label={t('common.delete')}
            onClick={onDelete}
            icon={<Trash2 size={14} />}
            destructive
          />
        </>
      )}
    </div>
  );
}

function MenuItem({
  label,
  icon,
  destructive,
  onClick,
}: {
  label: string;
  icon: React.ReactNode;
  destructive?: boolean;
  onClick: () => void;
}) {
  return (
    <Button
      role="menuitem"
      variant="ghost"
      size="sm"
      className={`h-8 w-full justify-start ${destructive ? 'hover:text-destructive' : ''}`}
      onClick={onClick}
    >
      {icon}
      {label}
    </Button>
  );
}
