import { useEffect, useState } from 'react';
import type { NavigateFn } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import EditorLayout from '@/components/editor/editor-layout';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';
import { useBeforeUnload } from '@/hooks/use-before-unload';
import { useCloudAuthStore } from '@/stores/cloud-auth-store';
import { useDocumentStore } from '@/stores/document-store';
import { getCloudFile } from '@/services/cloud/cloud-files';

interface CloudEditorPageProps {
  fileId: string;
  generationId?: string;
  navigate: NavigateFn;
}

export function CloudEditorPage({ fileId, generationId, navigate }: CloudEditorPageProps) {
  const { t } = useTranslation();
  const status = useCloudAuthStore((s) => s.status);
  const initialized = useCloudAuthStore((s) => s.initialized);
  const initialize = useCloudAuthStore((s) => s.initialize);
  const [loadState, setLoadState] = useState<'idle' | 'loading' | 'ready' | 'error'>('idle');
  const [error, setError] = useState<string | null>(null);

  useKeyboardShortcuts();
  useBeforeUnload();

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    if (!initialized || status === 'loading') return;
    if (status !== 'authenticated') {
      void navigate({ to: '/' });
      return;
    }

    let cancelled = false;
    setLoadState('loading');
    setError(null);
    getCloudFile(fileId)
      .then((file) => {
        if (cancelled) return;
        useDocumentStore.getState().loadCloudDocument(file);
        setLoadState('ready');
      })
      .catch((err) => {
        if (cancelled) return;
        setLoadState('error');
        setError(err instanceof Error ? err.message : t('cloudEditor.errorLoad'));
      });
    return () => {
      cancelled = true;
    };
  }, [fileId, initialized, navigate, status]);

  if (loadState !== 'ready') {
    return (
      <div className="min-h-screen bg-background text-foreground flex items-center justify-center">
        <div className="text-center">
          <p className="text-sm text-muted-foreground">
            {loadState === 'error' ? (error ?? t('cloudEditor.errorLoad')) : t('cloudEditor.loading')}
          </p>
          {loadState === 'error' && (
            <button
              type="button"
              className="mt-3 text-sm text-primary hover:underline"
              onClick={() => void navigate({ to: '/cloud' })}
            >
              {t('topbar.backToFiles')}
            </button>
          )}
        </div>
      </div>
    );
  }

  return <EditorLayout generationId={generationId} />;
}
