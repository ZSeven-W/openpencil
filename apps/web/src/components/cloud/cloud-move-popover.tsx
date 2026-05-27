import { FolderInput } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import type { CloudFolder } from '@/types/cloud';

interface CloudMovePopoverProps {
  fileName: string;
  folders: CloudFolder[];
  currentFolderId: string | null;
  folderPath: (folderId: string | null) => string;
  onMove: (folderId: string | null) => void;
}

export function CloudMovePopover({
  fileName,
  folders,
  currentFolderId,
  folderPath,
  onMove,
}: CloudMovePopoverProps) {
  const { t } = useTranslation();
  const targets = [
    { id: null, label: t('cloudLibrary.projectRoot') },
    ...folders
      .slice()
      .sort((a, b) => folderPath(a.id).localeCompare(folderPath(b.id)))
      .map((folder) => ({ id: folder.id, label: folderPath(folder.id) })),
  ];

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          aria-label={t('cloudLibrary.move.chooseTarget', { name: fileName })}
        >
          <FolderInput size={14} />
          {t('cloudLibrary.action.move')}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-72 p-2">
        <p className="mb-2 truncate px-1 text-xs font-medium">
          {t('cloudLibrary.move.title', { name: fileName })}
        </p>
        <div className="flex max-h-72 flex-col gap-1 overflow-y-auto">
          {targets.map((target) => (
            <Button
              key={target.id ?? 'root'}
              type="button"
              variant="ghost"
              size="sm"
              className="w-full justify-between gap-2 px-2 text-left text-xs"
              disabled={target.id === currentFolderId}
              onClick={() => onMove(target.id)}
            >
              <span className="truncate">{target.label}</span>
              {target.id === currentFolderId && (
                <span className="text-[11px] text-muted-foreground">
                  {t('cloudLibrary.move.currentBadge')}
                </span>
              )}
            </Button>
          ))}
        </div>
      </PopoverContent>
    </Popover>
  );
}
