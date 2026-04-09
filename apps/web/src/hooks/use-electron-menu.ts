import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { zoomToFitContent } from '@/canvas/skia-engine-ref'
import { parseAndPrepareImportedDocument } from '@/utils/import-pen-document'
import {
  supportsFileSystemAccess,
  openDocumentFS,
  openDocument,
} from '@/utils/file-operations'
import { saveCurrentDocument } from '@/utils/save-current-document'
import { confirmContinueWithUnsavedChanges } from '@/utils/unsaved-changes'

/**
 * Listens for Electron native menu actions and dispatches them to stores.
 * No-op when running in a browser (non-Electron) environment.
 */
export function useElectronMenu() {
  useEffect(() => {
    const api = window.electronAPI
    if (!api?.onMenuAction) return

    const loadElectronDocument = async (content: string, filePath: string) => {
      const name = filePath.split(/[/\\]/).pop() || 'untitled.op'
      const prepared = parseAndPrepareImportedDocument(content, {
        fileName: name,
        filePath,
      })
      if (!prepared) return
      const { doc } = prepared
      useDocumentStore.getState().loadDocument(doc, name, null, filePath)
      requestAnimationFrame(() => zoomToFitContent())
    }

    const loadFileFromPath = (filePath: string) => {
      api.readFile?.(filePath).then((result) => {
        if (!result) return
        void loadElectronDocument(result.content, filePath).catch(() => {
          // Invalid file — ignore
        })
      })
    }

    const cleanupOpenFile = api.onOpenFile?.(loadFileFromPath)

    // Pull any pending file from cold start (double-click .op to launch app)
    api.getPendingFile?.().then((filePath) => {
      if (filePath) loadFileFromPath(filePath)
    })

    const cleanup = api.onMenuAction((action: string) => {
      switch (action) {
        case 'new':
          void (async () => {
            if (!(await confirmContinueWithUnsavedChanges())) return
            useDocumentStore.getState().newDocument()
            requestAnimationFrame(() => zoomToFitContent())
          })()
          break

        case 'open':
          void (async () => {
            if (!(await confirmContinueWithUnsavedChanges())) return
            if (api) {
              api.openFile().then((result) => {
                if (!result) return
                void loadElectronDocument(result.content, result.filePath).catch(() => {
                  // Invalid file
                })
              })
            } else if (supportsFileSystemAccess()) {
              openDocumentFS().then((result) => {
                if (result) {
                  useDocumentStore
                    .getState()
                    .loadDocument(result.doc, result.fileName, result.handle)
                  requestAnimationFrame(() => zoomToFitContent())
                }
              })
            } else {
              openDocument().then((result) => {
                if (result) {
                  useDocumentStore
                    .getState()
                    .loadDocument(result.doc, result.fileName)
                  requestAnimationFrame(() => zoomToFitContent())
                }
              })
            }
          })()
          break

        case 'save':
        case 'save-and-close': {
          const closeAfterSave = action === 'save-and-close'
          void saveCurrentDocument().then((saved) => {
            if (saved && closeAfterSave) api.confirmClose()
          }).catch((err) => console.error('[Save] Failed:', err))
          break
        }

        case 'import-figma':
          useCanvasStore.getState().setFigmaImportDialogOpen(true)
          break

        case 'undo': {
          const currentDoc = useDocumentStore.getState().document
          const prev = useHistoryStore.getState().undo(currentDoc)
          if (prev) {
            useDocumentStore.getState().applyHistoryState(prev)
          }
          useCanvasStore.getState().clearSelection()
          break
        }

        case 'redo': {
          const currentDoc = useDocumentStore.getState().document
          const next = useHistoryStore.getState().redo(currentDoc)
          if (next) {
            useDocumentStore.getState().applyHistoryState(next)
          }
          useCanvasStore.getState().clearSelection()
          break
        }
      }
    })

    return () => {
      cleanup()
      cleanupOpenFile?.()
    }
  }, [])
}
