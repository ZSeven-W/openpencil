import { Upload, Download, Sparkles, X, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { cn } from '@/lib/utils';
import { useDocumentStore } from '@/stores/document-store';
import { useDesignMdStore } from '@/stores/design-md-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { importDesignMd, exportDesignMd } from '@/utils/design-md-io';
import { designMdColorsToVariables, extractDesignMdFromDocument } from '@/utils/design-md-parser';
import type { DesignMdSpec, DesignMdColor } from '@/types/design-md';

// ---------------------------------------------------------------------------
// useDesignMdActions — import/export, sync colors, clear
// ---------------------------------------------------------------------------

export function useDesignMdActions() {
  const designMd = useDesignMdStore((s) => s.designMd);
  const setDesignMd = useDesignMdStore((s) => s.setDesignMd);
  const setVariable = useDocumentStore((s) => s.setVariable);

  const handleImport = async () => {
    const spec = await importDesignMd();
    if (spec) setDesignMd(spec);
  };

  const handleExport = async () => {
    const spec = designMd ?? extractDesignMdFromDocument(useDocumentStore.getState().document);
    await exportDesignMd(spec);
  };

  const handleSyncColor = (color: DesignMdColor) => {
    const vars = designMdColorsToVariables([color]);
    for (const [name, def] of Object.entries(vars)) setVariable(name, def);
  };

  const handleSyncAllColors = () => {
    if (!designMd?.colorPalette) return;
    const vars = designMdColorsToVariables(designMd.colorPalette);
    for (const [name, def] of Object.entries(vars)) setVariable(name, def);
  };

  const handleClear = () => setDesignMd(undefined);

  return {
    designMd,
    handleImport,
    handleExport,
    handleSyncColor,
    handleSyncAllColors,
    handleClear,
  };
}

// ---------------------------------------------------------------------------
// DesignMdHeaderActions — import/export/AI buttons in the header bar
// ---------------------------------------------------------------------------

interface DesignMdHeaderActionsProps {
  onImport: () => void;
  onExport: () => void;
  onAutoGenerate: () => void;
  isGenerating: boolean;
  hasAI: boolean;
}

export function DesignMdHeaderActions({
  onImport,
  onExport,
  onAutoGenerate,
  isGenerating,
  hasAI,
}: DesignMdHeaderActionsProps) {
  const { t } = useTranslation();
  const togglePanel = useCanvasStore((s) => s.toggleDesignMdPanel);

  return (
    <div className="flex items-center gap-0.5">
      <button
        onClick={onImport}
        className="p-1.5 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
        title={t('designMd.import')}
      >
        <Upload size={14} />
      </button>
      <button
        onClick={onExport}
        className="p-1.5 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
        title={t('designMd.export')}
      >
        <Download size={14} />
      </button>
      {hasAI && (
        <button
          onClick={onAutoGenerate}
          className={cn(
            'p-1.5 rounded-md transition-colors',
            isGenerating
              ? 'text-primary bg-primary/10'
              : 'text-muted-foreground hover:bg-muted hover:text-foreground',
          )}
          title={t('designMd.autoGenerate')}
        >
          {isGenerating ? <Loader2 size={14} className="animate-spin" /> : <Sparkles size={14} />}
        </button>
      )}
      <button
        onClick={togglePanel}
        className="p-1.5 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
      >
        <X size={14} />
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// DesignMdEmptyState — shown when there is no parsed content
// ---------------------------------------------------------------------------

interface DesignMdEmptyStateProps {
  designMd: DesignMdSpec | undefined;
  onImport: () => void;
  onAutoGenerate: () => void;
  onClear: () => void;
  isGenerating: boolean;
  hasAI: boolean;
}

export function DesignMdEmptyState({
  designMd,
  onImport,
  onAutoGenerate,
  onClear,
  isGenerating,
  hasAI,
}: DesignMdEmptyStateProps) {
  const { t } = useTranslation();

  if (designMd?.raw) {
    // designMd exists but parser couldn't extract sections — show raw
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4 px-6">
        <div className="w-full h-full flex flex-col gap-3 py-3">
          <p className="text-[11px] text-muted-foreground leading-relaxed whitespace-pre-wrap flex-1 overflow-y-auto font-mono">
            {designMd.raw}
          </p>
          <div className="flex items-center gap-2 shrink-0">
            <button
              onClick={onAutoGenerate}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
            >
              {isGenerating ? (
                <Loader2 size={12} className="animate-spin" />
              ) : (
                <Sparkles size={12} />
              )}
              {t('designMd.autoGenerateCta')}
            </button>
            <button
              onClick={onClear}
              className="text-[10px] text-muted-foreground hover:text-destructive transition-colors"
            >
              {t('designMd.remove')}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // No designMd at all — empty state
  return (
    <div className="flex flex-col items-center justify-center h-full gap-4 px-6">
      <div className="w-10 h-10 rounded-xl bg-muted/50 flex items-center justify-center">
        <Sparkles size={20} className="text-muted-foreground" />
      </div>
      <p className="text-xs text-muted-foreground text-center">{t('designMd.empty')}</p>
      <div className="flex gap-2">
        <button
          onClick={onImport}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
        >
          <Upload size={12} /> {t('designMd.importCta')}
        </button>
        {hasAI && (
          <button
            onClick={onAutoGenerate}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg border border-border text-foreground hover:bg-muted transition-colors"
          >
            {isGenerating ? <Loader2 size={12} className="animate-spin" /> : <Sparkles size={12} />}
            {t('designMd.autoGenerateCta')}
          </button>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// DesignMdGeneratingOverlay — shown while AI is streaming
// ---------------------------------------------------------------------------

interface DesignMdGeneratingOverlayProps {
  onStop: () => void;
}

export function DesignMdGeneratingOverlay({ onStop }: DesignMdGeneratingOverlayProps) {
  const { t } = useTranslation();

  return (
    <div className="absolute inset-0 top-[37px] bg-card/80 backdrop-blur-sm flex flex-col items-center justify-center gap-3 z-10">
      <Loader2 size={24} className="text-primary animate-spin" />
      <p className="text-xs text-muted-foreground">{t('ai.generating')}</p>
      <button
        onClick={onStop}
        className="text-[10px] px-2 py-1 rounded-md border border-border hover:bg-muted text-muted-foreground transition-colors"
      >
        {t('ai.stopGenerating')}
      </button>
    </div>
  );
}
