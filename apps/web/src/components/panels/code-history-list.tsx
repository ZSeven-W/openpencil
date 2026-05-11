import { CheckCircle2, ExternalLink } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import type { CodegenHistoryEntry } from '@/stores/codegen-store';
import { useTranslation } from 'react-i18next';

interface CodeHistoryListProps {
  history: CodegenHistoryEntry[];
  selectedId?: string;
  onSelect: (id: string) => void;
  onPreview: (entry: CodegenHistoryEntry) => void;
}

function formatHistoryTime(value: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(value));
  } catch {
    return value;
  }
}

export default function CodeHistoryList({
  history,
  selectedId,
  onSelect,
  onPreview,
}: CodeHistoryListProps) {
  const { t } = useTranslation();

  if (history.length === 0) {
    return (
      <div className="flex h-full items-center justify-center p-5 text-center">
        <div className="text-xs text-muted-foreground">
          {t('codePanel.history.empty')}
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-1 overflow-auto p-2">
      {history.map((entry) => {
        const selected = entry.id === selectedId;
        return (
          <div
            key={entry.id}
            className={cn(
              'flex items-start gap-2 rounded-md border px-2.5 py-2 transition-colors',
              selected
                ? 'border-primary/40 bg-primary/8 text-foreground'
                : 'border-border/60 bg-muted/20 text-muted-foreground hover:bg-muted/45 hover:text-foreground',
            )}
          >
            <button
              type="button"
              onClick={() => onSelect(entry.id)}
              className="min-w-0 flex-1 text-left"
            >
              <div className="flex items-center gap-2">
                <span className="text-[11px] font-medium">
                  {formatHistoryTime(entry.createdAt)}
                </span>
                <span
                  className={cn(
                    'rounded px-1.5 py-0.5 text-[10px] uppercase tracking-wide',
                    entry.status === 'failed'
                      ? 'bg-destructive/10 text-destructive'
                      : entry.degraded
                        ? 'bg-amber-500/10 text-amber-600'
                        : 'bg-emerald-500/10 text-emerald-600',
                  )}
                >
                  {entry.status}
                </span>
                {selected && <CheckCircle2 className="ml-auto h-3.5 w-3.5 text-primary" />}
              </div>
              <div className="mt-1 flex flex-wrap gap-x-2 gap-y-1 text-[10px] text-muted-foreground/80">
                <span>rev {entry.documentRevision}</span>
                {entry.model && <span>{entry.model}</span>}
                {entry.provider && <span>{entry.provider}</span>}
                {entry.targetKind && (
                  <span>
                    {entry.targetKind === 'selection'
                      ? t('codePanel.history.selectionTarget', {
                          count: entry.nodeIds?.length ?? 0,
                        })
                      : t('codePanel.history.pageTarget')}
                  </span>
                )}
              </div>
              {entry.error && (
                <div className="mt-1 text-[10px] text-destructive">{entry.error}</div>
              )}
            </button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => onPreview(entry)}
              className="h-7 shrink-0 gap-1 px-2 text-[10px] text-muted-foreground hover:text-foreground"
            >
              <ExternalLink className="h-3 w-3" />
              {t('codePanel.preview.action')}
            </Button>
          </div>
        );
      })}
    </div>
  );
}
