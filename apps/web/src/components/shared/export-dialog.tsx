import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { useDocumentStore } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import {
  exportActivePageImage,
  exportDocumentPdf,
  downloadBlob,
  sanitizeFilename,
  type GlobalExportFormat,
} from '@/utils/global-export';

interface ExportDialogProps {
  open: boolean;
  onClose: () => void;
}

const FORMATS: { value: GlobalExportFormat; label: string }[] = [
  { value: 'png', label: 'PNG' },
  { value: 'jpeg', label: 'JPEG' },
  { value: 'webp', label: 'WEBP' },
  { value: 'pdf', label: 'PDF' },
];

export default function ExportDialog({ open, onClose }: ExportDialogProps) {
  const { t } = useTranslation();
  const [format, setFormat] = useState<GlobalExportFormat>('png');
  const [scale, setScale] = useState(2);
  const [busy, setBusy] = useState(false);
  const pageCount = useDocumentStore((s) => s.document.pages?.length ?? 0);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  const handleExport = () => {
    if (busy) return;
    setBusy(true);
    // Yield to React so the spinner shows before the heavy CanvasKit work blocks the main thread.
    setTimeout(() => {
      try {
        const doc = useDocumentStore.getState().document;
        const fileName = useDocumentStore.getState().fileName;
        const baseName = sanitizeFilename(
          (fileName || '').replace(/\.(op|pen|json)$/i, '') || 'untitled',
          'untitled',
        );

        if (format === 'pdf') {
          const blob = exportDocumentPdf(doc, scale);
          if (!blob) {
            console.error('[ExportDialog] PDF export produced no output');
            return;
          }
          downloadBlob(blob, `${baseName}.pdf`);
          onClose();
          return;
        }

        const activePageId = useCanvasStore.getState().activePageId;
        const result = exportActivePageImage(doc, activePageId, format, scale);
        if (!result) {
          console.error('[ExportDialog] Image export produced no output');
          return;
        }
        downloadBlob(result.blob, `${baseName}.${result.ext}`);
        onClose();
      } finally {
        setBusy(false);
      }
    }, 0);
  };

  const isPdf = format === 'pdf';
  const pdfPageHint =
    isPdf && pageCount > 1 ? t('export.pdfMultiPage', { count: pageCount }) : null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80" onClick={onClose} />
      <div className="relative bg-card rounded-lg border border-border p-4 w-80 shadow-xl">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-foreground">{t('export.title')}</h3>
          <Button variant="ghost" size="icon-sm" onClick={onClose} disabled={busy}>
            <X size={14} />
          </Button>
        </div>

        {/* Format */}
        <div className="mb-3">
          <label className="text-xs text-muted-foreground block mb-1.5">{t('export.format')}</label>
          <div className="grid grid-cols-4 gap-1.5">
            {FORMATS.map((f) => (
              <button
                key={f.value}
                type="button"
                disabled={busy}
                onClick={() => setFormat(f.value)}
                className={cn(
                  'text-[11px] py-1.5 rounded transition-colors disabled:opacity-50',
                  format === f.value
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
                )}
              >
                {f.label}
              </button>
            ))}
          </div>
        </div>

        {/* Scale */}
        <div className="mb-3">
          <label className="text-xs text-muted-foreground block mb-1.5">{t('export.scale')}</label>
          <div className="flex gap-1.5">
            {[1, 2, 3].map((s) => (
              <button
                key={s}
                type="button"
                disabled={busy}
                onClick={() => setScale(s)}
                className={cn(
                  'flex-1 text-[11px] py-1.5 rounded transition-colors disabled:opacity-50',
                  scale === s
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
                )}
              >
                {s}x
              </button>
            ))}
          </div>
        </div>

        {pdfPageHint && (
          <p className="text-[10px] text-muted-foreground mb-3 leading-relaxed">{pdfPageHint}</p>
        )}

        <Button onClick={handleExport} disabled={busy} className="w-full" size="sm">
          {busy && <Loader2 size={12} className="mr-1.5 animate-spin" />}
          {t('export.exportFormat', { format: format.toUpperCase() })}
        </Button>
      </div>
    </div>
  );
}
