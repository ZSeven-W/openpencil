import { FolderInput, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { CloudFileSummary, CloudFolder } from '@/types/cloud';

interface CloudMoveTargetPanelProps {
  file: CloudFileSummary;
  folders: CloudFolder[];
  folderPath: (folderId: string | null) => string;
  onMove: (folderId: string | null) => void;
  onClose: () => void;
}

export function CloudMoveTargetPanel({
  file,
  folders,
  folderPath,
  onMove,
  onClose,
}: CloudMoveTargetPanelProps) {
  const targets = [
    { id: null, label: 'Project root' },
    ...folders
      .slice()
      .sort((a, b) => folderPath(a.id).localeCompare(folderPath(b.id)))
      .map((folder) => ({ id: folder.id, label: folderPath(folder.id) })),
  ];

  return (
    <section
      role="dialog"
      aria-label="Move file"
      className="mb-3 rounded-md border border-border bg-card p-3"
    >
      <div className="mb-3 flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">Move {file.name}</p>
          <p className="text-xs text-muted-foreground">Current: {folderPath(file.folderId)}</p>
        </div>
        <Button variant="ghost" size="icon-sm" aria-label="Close move target selector" onClick={onClose}>
          <X size={14} />
        </Button>
      </div>
      <div className="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-3">
        {targets.map((target) => (
          <Button
            key={target.id ?? 'root'}
            variant={target.id === file.folderId ? 'secondary' : 'outline'}
            size="sm"
            className="justify-start"
            disabled={target.id === file.folderId}
            onClick={() => onMove(target.id)}
          >
            <FolderInput size={14} />
            <span className="truncate">{target.label}</span>
          </Button>
        ))}
      </div>
    </section>
  );
}
