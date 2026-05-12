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
  return (
    <div
      role="menu"
      aria-label={`File actions for ${file.name}`}
      className="absolute right-5 z-20 mt-1 w-52 rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md"
    >
      {isTrash ? (
        <>
          <MenuItem label="Restore" onClick={onRestore} icon={<ArchiveRestore size={14} />} />
          <MenuItem
            label="Delete forever"
            onClick={onPermanentDelete}
            icon={<Trash2 size={14} />}
            destructive
          />
        </>
      ) : (
        <>
          <MenuItem label="Open" onClick={onOpen} icon={<ExternalLink size={14} />} />
          <MenuItem label="Details" onClick={onDetails} icon={<FileText size={14} />} />
          <MenuItem
            label={file.starred ? 'Unfavorite' : 'Favorite'}
            onClick={onFavorite}
            icon={<Star size={14} />}
          />
          <MenuItem label="Rename" onClick={onRename} icon={<Pencil size={14} />} />
          <MenuItem label="Copy" onClick={onCopy} icon={<Copy size={14} />} />
          <MenuItem label="Share" onClick={onShare} icon={<Share2 size={14} />} />
          {canMove && (
            <MenuItem label="Move" onClick={onMove} icon={<FolderInput size={14} />} />
          )}
          <MenuItem label="Delete" onClick={onDelete} icon={<Trash2 size={14} />} destructive />
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
