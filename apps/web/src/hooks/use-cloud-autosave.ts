import { useEffect, useRef } from 'react';
import { useDocumentStore } from '@/stores/document-store';
import { syncCanvasPositionsToStore } from '@/canvas/skia-engine-ref';

const AUTOSAVE_DELAY_MS = 3000;
const VERSION_INTERVAL_MS = 120_000;

export function useCloudAutosave() {
  const isDirty = useDocumentStore((s) => s.isDirty);
  const cloudFileId = useDocumentStore((s) => s.cloudFileId);
  const cloudSaveState = useDocumentStore((s) => s.cloudSaveState);
  const lastVersionAtRef = useRef(0);

  useEffect(() => {
    if (!isDirty || !cloudFileId || cloudSaveState === 'saving' || cloudSaveState === 'conflict') {
      return;
    }

    const timer = window.setTimeout(() => {
      try {
        syncCanvasPositionsToStore();
      } catch {
        /* best-effort autosave */
      }
      const now = Date.now();
      const shouldVersion = now - lastVersionAtRef.current >= VERSION_INTERVAL_MS;
      if (shouldVersion) lastVersionAtRef.current = now;
      void useDocumentStore
        .getState()
        .saveCloud('autosave', shouldVersion ? 'Autosave' : undefined, shouldVersion);
    }, AUTOSAVE_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, [cloudFileId, cloudSaveState, isDirty]);
}
