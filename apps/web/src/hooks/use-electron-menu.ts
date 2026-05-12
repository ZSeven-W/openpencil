import { useEffect } from 'react';
import i18n from '@/i18n';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import { zoomToFitContent } from '@/canvas/skia-engine-ref';
import { syncCanvasPositionsToStore } from '@/canvas/skia-engine-ref';
import { parseAndPrepareImportedDocument } from '@/utils/import-pen-document';
import { addRecentFile, clearRecentFiles } from '@/utils/recent-files';

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
      /* 继续 */
    }
    const savedName = await useDocumentStore.getState().save();
    if (!savedName) {
      // 用户取消了保存对话框或保存失败 - 中止关闭
      return false;
    }
  }
  return true;
}

function navigateToLocalEditor(): void {
  window.history.pushState({}, '', '/editor/local');
  window.dispatchEvent(new PopStateEvent('popstate'));
}

/**
 * Listens 用于
 * Electron 本机菜单操作并将它们分派到商店。 No-op 在浏览器（非 Electron）环境中运行时。
 */
export function useElectronMenu() {
  useEffect(() => {
    const api = window.electronAPI;
    if (!api?.onMenuAction) return;

    const loadFileFromPath = (filePath: string) => {
      void api.readFile(filePath).then((result) => {
        if (!result) return;
        const name = result.filePath.split(/[/\\]/).pop() || 'untitled.op';
        const prepared = parseAndPrepareImportedDocument(result.content, {
          fileName: name,
          filePath: result.filePath,
        });
        if (!prepared) return;
        useDocumentStore.getState().loadDocument(prepared.doc, name, null, result.filePath);
        navigateToLocalEditor();
      });
    };

    const cleanupOpenFile = api.onOpenFile?.(loadFileFromPath);

    // Pull 冷启动中的任何待处理文件（双击 .op 启动应用程序）
    api.getPendingFile?.().then((filePath) => {
      if (filePath) loadFileFromPath(filePath);
    });

    const cleanup = api.onMenuAction((action: string) => {
      // Handle 最近打开：<filePath> 操作
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
            navigateToLocalEditor();
          })();
          break;

        case 'open':
          (async () => {
            if (useDocumentStore.getState().isDirty) {
              if (!(await confirmUnsaved())) return;
            }
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
                navigateToLocalEditor();
                requestAnimationFrame(() => zoomToFitContent());
              } catch {
                // Invalid 文件
              }
            });
          })();
          break;

        case 'save':
        case 'save-and-close': {
          const closeAfterSave = action === 'save-and-close';
          try {
            syncCanvasPositionsToStore();
          } catch {
            /* 继续 */
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
            /* 继续 */
          }
          (async () => {
            const savedName = await useDocumentStore.getState().exportOp();
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
