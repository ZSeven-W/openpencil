import { useEffect } from 'react';
import i18n from '@/i18n';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import { zoomToFitContent } from '@/canvas/skia-engine-ref';
import { syncCanvasPositionsToStore } from '@/canvas/skia-engine-ref';
import { parseAndPrepareImportedDocument } from '@/utils/import-pen-document';
import { addRecentFile, clearRecentFiles } from '@/utils/recent-files';
import { supportsFileSystemAccess, openDocumentFS, openDocument } from '@/utils/file-operations';
import { loadOpFileFromPath } from '@/utils/load-op-file';

async function confirmUnsaved(): Promise<boolean> {
  const showDialog = (window as any).__showUnsavedDialog;
  if (!showDialog) return window.confirm(i18n.t('topbar.closeConfirmMessage'));
  const fileName = useDocumentStore.getState().fileName || i18n.t('common.untitled');
  const result = await showDialog(fileName);
  if (result === 'cancel') return false;
  if (result === 'save') {
    try {
      syncCanvasPositionsToStore();
    } catch {
      /* continue */
    }
    const savedName = await useDocumentStore.getState().save();
    if (!savedName) {
      // user cancelled the save dialog or save failed — abort the close
      return false;
    }
  }
  return true;
}

/**
 * Listens for Electron native menu actions and dispatches them to stores.
 * No-op when running in a browser (non-Electron) environment.
 */
export function useElectronMenu() {
  useEffect(() => {
    const api = window.electronAPI;
    if (!api?.onMenuAction) return;

    const loadFileFromPath = (filePath: string) => {
      void loadOpFileFromPath(filePath);
    };

    const cleanupOpenFile = api.onOpenFile?.(loadFileFromPath);

    // Pull any pending file from cold start (double-click .op to launch app)
    api.getPendingFile?.().then((filePath) => {
      if (filePath) loadFileFromPath(filePath);
    });

    const cleanup = api.onMenuAction((action: string) => {
      // Handle open-recent:<filePath> actions
      if (action.startsWith('open-recent:')) {
        const recentPath = action.slice('open-recent:'.length);
        (async () => {
          if (useDocumentStore.getState().isDirty) {
            if (!(await confirmUnsaved())) return;
          }
          loadFileFromPath(recentPath);
        })();
        return;
      }

      switch (action) {
        case 'new':
          (async () => {
            if (useDocumentStore.getState().isDirty) {
              if (!(await confirmUnsaved())) return;
            }
            useDocumentStore.getState().newDocument();
            requestAnimationFrame(() => zoomToFitContent());
          })();
          break;

        case 'open':
          (async () => {
            if (useDocumentStore.getState().isDirty) {
              if (!(await confirmUnsaved())) return;
            }
            if (api) {
              api.openFile().then((result) => {
                if (!result) return;
                try {
                  const name = result.filePath.split(/[/\\]/).pop() || 'untitled.op';
                  const prepared = parseAndPrepareImportedDocument(result.content, {
                    fileName: name,
                    filePath: result.filePath,
                  });
                  if (!prepared) return;
                  const { doc } = prepared;
                  useDocumentStore.getState().loadDocument(doc, name, null, result.filePath);
                  requestAnimationFrame(() => zoomToFitContent());
                } catch {
                  // Invalid file
                }
              });
            } else if (supportsFileSystemAccess()) {
              openDocumentFS().then((result) => {
                if (result) {
                  useDocumentStore
                    .getState()
                    .loadDocument(result.doc, result.fileName, result.handle);
                  requestAnimationFrame(() => zoomToFitContent());
                }
              });
            } else {
              openDocument().then((result) => {
                if (result) {
                  useDocumentStore.getState().loadDocument(result.doc, result.fileName);
                  requestAnimationFrame(() => zoomToFitContent());
                }
              });
            }
          })();
          break;

        case 'save':
        case 'save-and-close': {
          const closeAfterSave = action === 'save-and-close';
          try {
            syncCanvasPositionsToStore();
          } catch {
            /* continue */
          }
          (async () => {
            const savedName = await useDocumentStore.getState().save();
            if (savedName) {
              const filePath = useDocumentStore.getState().filePath;
              addRecentFile({ fileName: savedName, filePath });
              if (closeAfterSave) api.confirmClose();
            }
          })().catch((err) => console.error('[Save] Failed:', err));
          break;
        }

        case 'save-as': {
          try {
            syncCanvasPositionsToStore();
          } catch {
            /* continue */
          }
          (async () => {
            const savedName = await useDocumentStore.getState().saveAs();
            if (savedName) {
              const filePath = useDocumentStore.getState().filePath;
              addRecentFile({ fileName: savedName, filePath });
            }
          })().catch((err) => console.error('[SaveAs] Failed:', err));
          break;
        }

        case 'clear-recent-files':
          clearRecentFiles();
          break;

        case 'import-figma':
          useCanvasStore.getState().setFigmaImportDialogOpen(true);
          break;

        case 'export-image':
          useCanvasStore.getState().setExportDialogOpen(true);
          break;

        case 'undo': {
          const currentDoc = useDocumentStore.getState().document;
          const prev = useHistoryStore.getState().undo(currentDoc);
          if (prev) {
            useDocumentStore.getState().applyHistoryState(prev);
          }
          useCanvasStore.getState().clearSelection();
          break;
        }

        case 'redo': {
          const currentDoc = useDocumentStore.getState().document;
          const next = useHistoryStore.getState().redo(currentDoc);
          if (next) {
            useDocumentStore.getState().applyHistoryState(next);
          }
          useCanvasStore.getState().clearSelection();
          break;
        }
      }
    });

    return () => {
      cleanup();
      cleanupOpenFile?.();
    };
  }, []);
}
