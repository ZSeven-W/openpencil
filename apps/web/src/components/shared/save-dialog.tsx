import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useDocumentStore } from '@/stores/document-store';
import { syncCanvasPositionsToStore } from '@/canvas/skia-engine-ref';

interface SaveDialogProps {
  open: boolean;
  onClose: () => void;
}

export default function SaveDialog({ open, onClose }: SaveDialogProps) {
  const { t } = useTranslation();
  const [name, setName] = useState(t('common.untitled'));
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    // Pre-fill with existing name (without extension)
    const fn = useDocumentStore.getState().fileName;
    if (fn) {
      setName(fn.replace(/\.op$|\.pen$|\.json$/, ''));
    } else {
      setName(t('common.untitled'));
    }
    // Focus + select on open
    requestAnimationFrame(() => inputRef.current?.select());
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  const handleSave = async () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    // Force-sync all Fabric object positions to the store before serializing
    syncCanvasPositionsToStore();
    // Pass the typed name as an explicit suggestion. The store action handles
    // dialog/picker, write, fileName/filePath mutation, isDirty=false, and the
    // 'saved' emission — but ONLY on confirmed success. We do not pre-mutate
    // store state; if save fails or the user cancels, store stays untouched
    // and the dialog stays open so the user can retry or change the name.
    const savedName = await useDocumentStore.getState().saveAs(trimmed);
    if (savedName) {
      onClose();
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80" onClick={onClose} />
      <div className="relative bg-card rounded-lg border border-border p-4 w-72 shadow-xl">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-foreground">{t('save.saveAs')}</h3>
          <Button variant="ghost" size="icon-sm" onClick={onClose}>
            <X size={14} />
          </Button>
        </div>

        <label className="text-xs text-muted-foreground block mb-1">{t('save.fileName')}</label>
        <div className="flex items-center gap-1 mb-4">
          <input
            ref={inputRef}
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSave();
            }}
            className="flex-1 bg-secondary border border-input rounded px-2 py-1.5 text-sm text-foreground focus:outline-none focus:border-ring"
            autoFocus
          />
          <span className="text-xs text-muted-foreground">.op</span>
        </div>

        <div className="flex gap-2">
          <Button variant="secondary" size="sm" onClick={onClose} className="flex-1">
            {t('common.cancel')}
          </Button>
          <Button size="sm" onClick={handleSave} disabled={!name.trim()} className="flex-1">
            {t('common.save')}
          </Button>
        </div>
      </div>
    </div>
  );
}
