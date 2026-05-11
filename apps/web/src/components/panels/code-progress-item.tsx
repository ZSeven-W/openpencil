import type { ReactNode } from 'react';
import { AlertTriangle, Check, Loader2, MinusCircle, SkipForward } from 'lucide-react';
import type { ChunkStatus } from '@zseven-w/pen-types';
import { cn } from '@/lib/utils';

interface ProgressItemProps {
  label: string;
  status: ChunkStatus | 'running' | 'done' | 'failed' | 'pending';
  error?: string;
  onRetry?: () => void;
}

export default function ProgressItem({ label, status, error, onRetry }: ProgressItemProps) {
  const icons: Record<string, ReactNode> = {
    pending: <div className="h-3 w-3 rounded-full border border-muted-foreground/20" />,
    running: <Loader2 className="h-3 w-3 animate-spin text-primary" />,
    done: <Check className="h-3 w-3 text-green-500" />,
    degraded: <AlertTriangle className="h-3 w-3 text-amber-500" />,
    failed: <MinusCircle className="h-3 w-3 text-destructive" />,
    skipped: <SkipForward className="h-3 w-3 text-muted-foreground/50" />,
  };

  const sublabels: Record<string, string> = {
    degraded: 'generated without contract',
    skipped: 'skipped (dependency failed)',
  };

  return (
    <div
      className={cn(
        'flex items-start gap-2 rounded-md px-2 py-1.5 text-[12px] transition-colors',
        status === 'running' && 'bg-primary/5',
        status === 'failed' && 'bg-destructive/5',
      )}
    >
      <div className="mt-[3px] shrink-0">{icons[status]}</div>
      <div className="flex-1 min-w-0">
        <div
          className={cn(
            'font-medium truncate',
            status === 'pending' && 'text-muted-foreground/60',
            status === 'running' && 'text-foreground',
            status === 'done' && 'text-foreground/70',
            status === 'failed' && 'text-destructive',
            status === 'degraded' && 'text-amber-600',
            status === 'skipped' && 'text-muted-foreground/50',
          )}
        >
          {label}
        </div>
        {sublabels[status] && (
          <div className="text-[10px] text-muted-foreground/60 mt-0.5">{sublabels[status]}</div>
        )}
        {error && <div className="text-[10px] text-destructive/80 mt-0.5 break-words">{error}</div>}
      </div>
      {onRetry && (
        <button
          type="button"
          className="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium text-primary hover:bg-primary/10 transition-colors"
          onClick={onRetry}
        >
          Retry
        </button>
      )}
    </div>
  );
}
