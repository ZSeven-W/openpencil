import { useEffect } from 'react';
import { AlertTriangle } from 'lucide-react';
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
      {/* Backdrop with blur */}
      <div
        className="absolute inset-0 bg-background/60 backdrop-blur-sm animate-in fade-in duration-200"
        onClick={() => onResult('cancel')}
      />

      {/* Dialog */}
      <div className="relative w-[340px] animate-in zoom-in-95 fade-in duration-200">
        {/* Glow ring */}
        <div className="absolute -inset-px rounded-2xl bg-gradient-to-b from-primary/20 to-transparent" />

        <div className="relative rounded-2xl border border-border bg-card p-6 shadow-2xl">
          {/* Icon */}
          <div className="mx-auto mb-4 flex h-11 w-11 items-center justify-center rounded-full bg-amber-500/10 ring-1 ring-amber-500/20">
            <AlertTriangle size={20} className="text-amber-500" />
          </div>

          {/* Content */}
          <h3 className="text-center text-[15px] font-semibold text-foreground mb-1.5">
            {t('unsaved.title')}
          </h3>
          <p className="text-center text-xs text-muted-foreground leading-relaxed mb-6">
            {t('unsaved.message', { name: fileName || t('common.untitled') })}
          </p>

          {/* Actions — stacked for visual weight */}
          <div className="flex flex-col gap-2">
            <Button
              onClick={() => onResult('save')}
              className="h-9 w-full text-xs font-medium rounded-lg"
            >
              {t('common.save')}
            </Button>
            <div className="flex gap-2">
              <Button
                variant="secondary"
                onClick={() => onResult('discard')}
                className="h-8 flex-1 text-[11px] rounded-lg text-destructive hover:text-destructive"
              >
                {t('unsaved.dontSave')}
              </Button>
              <Button
                variant="secondary"
                onClick={() => onResult('cancel')}
                className="h-8 flex-1 text-[11px] rounded-lg"
              >
                {t('common.cancel')}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
