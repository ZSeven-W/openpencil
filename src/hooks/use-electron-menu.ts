import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { zoomToFitContent } from '@/canvas/skia-engine-ref'
import { syncCanvasPositionsToStore } from '@/canvas/skia-engine-ref'
import { normalizePenDocument } from '@/utils/normalize-pen-file'
import {
  supportsFileSystemAccess,
  writeToFilePath,
  openDocumentFS,
  openDocument,
} from '@/utils/file-operations'

/**
 * Listens for Electron native menu actions and dispatches them to stores.
 * No-op when running in a browser (non-Electron) environment.
 */
export function useElectronMenu() {
  useEffect(() => {
    const api = window.electronAPI
    if (!api?.onMenuAction) return

    const loadFileFromPath = (filePath: string) => {
      api.readFile?.(filePath).then((result) => {
        if (!result) return
        try {
          const raw = JSON.parse(result.content)
          if (!raw.version || (!Array.isArray(raw.children) && !Array.isArray(raw.pages))) return
          const doc = normalizePenDocument(raw)
          const name = filePath.split(/[/\\]/).pop() || 'untitled.op'
          useDocumentStore.getState().loadDocument(doc, name, null, filePath)
          requestAnimationFrame(() => zoomToFitContent())
        } catch {
          // Invalid file — ignore
        }
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
          useDocumentStore.getState().newDocument()
          requestAnimationFrame(() => zoomToFitContent())
          break

        case 'open':
          if (api) {
            // Electron: use native IPC to get full file path for save-in-place
            api.openFile().then((result) => {
              if (!result) return
              try {
                const raw = JSON.parse(result.content)
                if (!raw.version || (!Array.isArray(raw.children) && !Array.isArray(raw.pages))) return
                const doc = normalizePenDocument(raw)
                const name = result.filePath.split(/[/\\]/).pop() || 'untitled.op'
                useDocumentStore
                  .getState()
                  .loadDocument(doc, name, null, result.filePath)
                requestAnimationFrame(() => zoomToFitContent())
              } catch {
                // Invalid file
              }
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
          break

        case 'save': {
          try { syncCanvasPositionsToStore() } catch { /* continue */ }
          const store = useDocumentStore.getState()
          const { document: doc, fileName, filePath } = store
          const isOpFile = fileName ? /\.op$/i.test(fileName) : false
          const suggestedName = fileName
            ? fileName.replace(/\.(pen|op|json)$/i, '') + '.op'
            : 'untitled.op'

          const doSave = async () => {
            // Known .op path → direct write
            if (filePath && isOpFile) {
              await writeToFilePath(filePath, doc)
              store.markClean()
              return
            }
            // No in-place target → save as .op via native dialog
            const savedPath = await api.saveFile(
              JSON.stringify(doc), suggestedName,
            )
            if (savedPath) {
              useDocumentStore.setState({
                fileName: savedPath.split(/[/\\]/).pop() || suggestedName,
                filePath: savedPath,
                fileHandle: null,
                isDirty: false,
              })
            }
          }
          doSave().catch((err) => console.error('[Save] Failed:', err))
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
