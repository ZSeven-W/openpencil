import { FileCode2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { cn } from '@/lib/utils';
import type { SaveCodegenFileInput } from '@/types/cloud';

interface CodeFileTreeProps {
  files: SaveCodegenFileInput[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
}

function getDepth(path: string): number {
  return Math.max(0, path.split('/').length - 1);
}

export default function CodeFileTree({ files, selectedPath, onSelect }: CodeFileTreeProps) {
  const { t } = useTranslation();
  if (files.length === 0) return null;

  return (
    <div className="shrink-0 border-b border-border/50 bg-muted/20">
      <div className="flex items-center justify-between px-2 py-1.5 text-[10px] font-medium uppercase tracking-[0.08em] text-muted-foreground">
        <span>{t('codePanel.files')}</span>
        <span>{files.length}</span>
      </div>
      <div className="max-h-28 overflow-auto px-1 pb-1">
        {files.map((file) => {
          const isSelected = selectedPath === file.path;
          const depth = getDepth(file.path);
          return (
            <button
              key={file.path}
              type="button"
              onClick={() => onSelect(file.path)}
              className={cn(
                'flex h-7 w-full items-center gap-1.5 rounded px-1.5 text-left text-[11px] transition-colors',
                isSelected
                  ? 'bg-background text-foreground shadow-sm'
                  : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground',
              )}
              style={{ paddingLeft: `${6 + depth * 10}px` }}
            >
              <FileCode2 className="h-3 w-3 shrink-0" />
              <span className="min-w-0 flex-1 truncate font-mono">{file.path}</span>
              <span className="shrink-0 rounded bg-muted px-1 py-0.5 text-[9px] uppercase text-muted-foreground">
                {file.role}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
