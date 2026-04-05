import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';

export type UnsavedResult = 'save' | 'discard' | 'cancel';

interface UnsavedChangesDialogProps {
  open: boolean;
  fileName: string;
  onResult: (result: UnsavedResult) => void;
}

export default function UnsavedChangesDialog({
  open,
  fileName,
  onResult,
}: UnsavedChangesDialogProps) {
  const { t } = useTranslation();

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onResult('cancel');
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onResult]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80" onClick={() => onResult('cancel')} />
      <div className="relative bg-card rounded-lg border border-border p-5 w-80 shadow-xl">
        <h3 className="text-sm font-medium text-foreground mb-2">{t('unsaved.title')}</h3>
        <p className="text-xs text-muted-foreground mb-5">
          {t('unsaved.message', { name: fileName || t('common.untitled') })}
        </p>
        <div className="flex items-center justify-end gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onResult('discard')}
            className="h-7 px-3 text-[11px] text-destructive hover:text-destructive"
          >
            {t('unsaved.dontSave')}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => onResult('cancel')}
            className="h-7 px-3 text-[11px]"
          >
            {t('common.cancel')}
          </Button>
          <Button size="sm" onClick={() => onResult('save')} className="h-7 px-3 text-[11px]">
            {t('common.save')}
          </Button>
        </div>
      </div>
    </div>
  );
}
