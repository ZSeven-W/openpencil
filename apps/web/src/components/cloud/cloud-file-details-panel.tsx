import { FileJson, Star, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { CloudFileSharingSection } from './cloud-file-sharing-section';
import type { CloudFileSummary } from '@/types/cloud';

interface CloudFileDetailsPanelProps {
  file: CloudFileSummary;
  location: string;
  projectName: string;
  updatedLabel: string;
  createdLabel: string;
  onClose: () => void;
}

export function CloudFileDetailsPanel({
  file,
  location,
  projectName,
  updatedLabel,
  createdLabel,
  onClose,
}: CloudFileDetailsPanelProps) {
  return (
    <aside
      aria-label="File details"
      className="border-l border-border bg-background p-4"
    >
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-3">
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-border bg-card">
            <FileJson size={16} className="text-muted-foreground" />
          </span>
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold">{file.name}</p>
            <p className="text-xs text-muted-foreground">{file.starred ? 'Favorite file' : 'Cloud design file'}</p>
          </div>
        </div>
        <Button variant="ghost" size="icon-sm" aria-label="Close details" onClick={onClose}>
          <X size={14} />
        </Button>
      </div>

      <dl className="space-y-3 text-xs">
        <DetailRow label="Revision" value={`rev ${file.revision}`} />
        <DetailRow label="Project" value={projectName} />
        <DetailRow label="Location" value={location} />
        <DetailRow label="Updated" value={updatedLabel} />
        <DetailRow label="Created" value={createdLabel} />
        <div className="flex items-center justify-between gap-3 border-t border-border pt-3">
          <dt className="text-muted-foreground">Starred</dt>
          <dd className="flex items-center gap-1 font-medium">
            <Star size={13} className={file.starred ? 'fill-current text-primary' : 'text-muted-foreground'} />
            {file.starred ? 'Yes' : 'No'}
          </dd>
        </div>
      </dl>
      <CloudFileSharingSection file={file} />
    </aside>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="truncate text-right font-medium">{value}</dd>
    </div>
  );
}
