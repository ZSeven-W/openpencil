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
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import type { CloudFileSummary } from '@/types/cloud';

interface CloudFileActionsMenuProps {
  file: CloudFileSummary;
  trigger: ReactNode;
  open: boolean;
  onOpenChange: (open: boolean) => void;
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
  trigger,
  open,
  onOpenChange,
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
    <DropdownMenu open={open} onOpenChange={onOpenChange}>
      <DropdownMenuTrigger asChild>{trigger}</DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-52">
        <DropdownMenuGroup>
          {isTrash ? (
            <>
              <DropdownMenuItem onClick={onRestore}>
                <ArchiveRestore />
                {t('cloudLibrary.action.restore')}
              </DropdownMenuItem>
              <ConfirmDropdownItem
                title={t('cloudLibrary.confirm.permanentlyDeleteFile', { name: file.name })}
                label={t('cloudLibrary.action.deleteForever')}
                onConfirm={onPermanentDelete}
              />
            </>
          ) : (
            <>
              <DropdownMenuItem onClick={onOpen}>
                <ExternalLink />
                {t('cloudLibrary.action.open')}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={onDetails}>
                <FileText />
                {t('cloudLibrary.action.details')}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={onFavorite}>
                <Star />
                {file.starred
                  ? t('cloudLibrary.action.unfavorite')
                  : t('cloudLibrary.action.favorite')}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={onRename}>
                <Pencil />
                {t('common.rename')}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={onCopy}>
                <Copy />
                {t('cloudLibrary.action.copy')}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={onShare}>
                <Share2 />
                {t('cloudLibrary.action.share')}
              </DropdownMenuItem>
              {canMove && (
                <DropdownMenuItem onClick={onMove}>
                  <FolderInput />
                  {t('cloudLibrary.action.move')}
                </DropdownMenuItem>
              )}
              <ConfirmDropdownItem
                title={t('cloudLibrary.confirm.deleteFile', { name: file.name })}
                label={t('common.delete')}
                onConfirm={onDelete}
              />
            </>
          )}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function ConfirmDropdownItem({
  title,
  label,
  onConfirm,
}: {
  title: string;
  label: string;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();

  return (
    <AlertDialog>
      <AlertDialogTrigger asChild>
        <DropdownMenuItem variant="destructive" onSelect={(event) => event.preventDefault()}>
          <Trash2 />
          {label}
        </DropdownMenuItem>
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{title}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
          <AlertDialogAction variant="destructive" onClick={onConfirm}>
            {t('common.delete')}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
