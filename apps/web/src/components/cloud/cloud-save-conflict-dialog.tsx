import { useState } from 'react';
import { AlertTriangle, Copy, RefreshCw, Save, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { createCloudFile, getCloudFile } from '@/services/cloud/cloud-files';
import { useDocumentStore } from '@/stores/document-store';

interface CloudSaveConflictDialogProps {
  onSavedCopy?: (fileId: string) => void;
}

export function CloudSaveConflictDialog({ onSavedCopy }: CloudSaveConflictDialogProps) {
  const { t } = useTranslation();
  const cloudFileId = useDocumentStore((s) => s.cloudFileId);
  const cloudSaveState = useDocumentStore((s) => s.cloudSaveState);
  const cloudSaveError = useDocumentStore((s) => s.cloudSaveError);
  const cloudSaveConflict = useDocumentStore((s) => s.cloudSaveConflict);
  const [busyAction, setBusyAction] = useState<'reload' | 'copy' | 'force' | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (cloudSaveState !== 'conflict' || !cloudFileId || !cloudSaveConflict) return null;

  const reloadRemote = async () => {
    setBusyAction('reload');
    setError(null);
    try {
      const file = await getCloudFile(cloudFileId);
      useDocumentStore.getState().loadCloudDocument(file);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('cloudConflict.errorReload'));
    } finally {
      setBusyAction(null);
    }
  };

  const saveCopy = async () => {
    setBusyAction('copy');
    setError(null);
    try {
      const state = useDocumentStore.getState();
      const file = await createCloudFile({
        name: t('cloudConflict.copyName', {
          name: state.fileName ?? t('common.untitled'),
        }),
        document: state.document,
        source: 'manual_save',
      });
      state.clearCloudError();
      onSavedCopy?.(file.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('cloudConflict.errorSaveCopy'));
    } finally {
      setBusyAction(null);
    }
  };

  const forceOverwrite = async () => {
    setBusyAction('force');
    setError(null);
    try {
      const result = await useDocumentStore
        .getState()
        .saveCloud('manual_save', t('cloudConflict.forceOverwriteLabel'), true, { force: true });
      if (!result) setError(t('cloudConflict.errorOverwrite'));
    } catch (err) {
      setError(err instanceof Error ? err.message : t('cloudConflict.errorOverwrite'));
    } finally {
      setBusyAction(null);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/70 p-4">
      <section
        role="dialog"
        aria-modal="true"
        aria-label={t('cloudConflict.title')}
        className="w-full max-w-md rounded-lg border border-border bg-card p-4 text-card-foreground shadow-lg"
      >
        <div className="mb-3 flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-3">
            <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-destructive/10 text-destructive">
              <AlertTriangle size={16} />
            </span>
            <div className="min-w-0">
              <h2 className="text-sm font-semibold">{t('cloudConflict.title')}</h2>
              <p className="mt-1 text-xs text-muted-foreground">
                {cloudSaveError ?? t('cloudConflict.description')}
              </p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t('cloudConflict.dismiss')}
            onClick={() => useDocumentStore.getState().clearCloudError()}
          >
            <X size={14} />
          </Button>
        </div>

        <div className="mb-4 grid grid-cols-2 gap-2 text-xs">
          <div className="rounded-md border border-border bg-background p-3">
            <p className="text-muted-foreground">{t('cloudConflict.localBase')}</p>
            <p className="mt-1 font-medium">
              {t('cloudConflict.localRevision', {
                revision: cloudSaveConflict.expectedRevision,
              })}
            </p>
          </div>
          <div className="rounded-md border border-border bg-background p-3">
            <p className="text-muted-foreground">{t('cloudConflict.remoteFile')}</p>
            <p className="mt-1 font-medium">
              {t('cloudConflict.remoteRevision', {
                revision: cloudSaveConflict.serverRevision ?? t('cloudConflict.unknownRevision'),
              })}
            </p>
          </div>
        </div>

        {error && <p className="mb-3 text-xs text-destructive">{error}</p>}

        <div className="flex flex-wrap justify-end gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => void saveCopy()}
            disabled={busyAction !== null}
          >
            <Copy size={14} />
            {busyAction === 'copy' ? t('cloudConflict.saving') : t('cloudConflict.saveAsCopy')}
          </Button>
          <Button size="sm" onClick={() => void reloadRemote()} disabled={busyAction !== null}>
            <RefreshCw size={14} />
            {busyAction === 'reload' ? t('cloudConflict.reloading') : t('cloudConflict.reloadRemote')}
          </Button>
          <Button
            variant="destructive"
            size="sm"
            onClick={() => void forceOverwrite()}
            disabled={busyAction !== null}
          >
            <Save size={14} />
            {busyAction === 'force' ? t('cloudConflict.overwriting') : t('cloudConflict.overwriteRemote')}
          </Button>
        </div>
      </section>
    </div>
  );
}
