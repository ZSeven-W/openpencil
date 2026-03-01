import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { zoomToFitContent } from '@/canvas/use-fabric-canvas'
import { syncCanvasPositionsToStore } from '@/canvas/use-canvas-sync'
import {
  supportsFileSystemAccess,
  writeToFileHandle,
  saveDocumentAs,
  downloadDocument,
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

    const cleanup = api.onMenuAction((action: string) => {
      switch (action) {
        case 'new':
          useDocumentStore.getState().newDocument()
          requestAnimationFrame(() => zoomToFitContent())
          break

        case 'open':
          if (supportsFileSystemAccess()) {
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
          syncCanvasPositionsToStore()
          const store = useDocumentStore.getState()
          const { document: doc, fileName, fileHandle } = store

          if (fileHandle) {
            writeToFileHandle(fileHandle, doc).then(() => store.markClean())
          } else if (fileName) {
            downloadDocument(doc, fileName)
            store.markClean()
          } else if (supportsFileSystemAccess()) {
            saveDocumentAs(doc, 'untitled.op').then((result) => {
              if (result) {
                useDocumentStore.setState({
                  fileName: result.fileName,
                  fileHandle: result.handle,
                  isDirty: false,
                })
              }
            })
          } else {
            store.setSaveDialogOpen(true)
          }
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
          const canvas = useCanvasStore.getState().fabricCanvas
          if (canvas) {
            canvas.discardActiveObject()
            canvas.requestRenderAll()
          }
          break
        }

        case 'redo': {
          const currentDoc = useDocumentStore.getState().document
          const next = useHistoryStore.getState().redo(currentDoc)
          if (next) {
            useDocumentStore.getState().applyHistoryState(next)
          }
          useCanvasStore.getState().clearSelection()
          const canvas = useCanvasStore.getState().fabricCanvas
          if (canvas) {
            canvas.discardActiveObject()
            canvas.requestRenderAll()
          }
          break
        }
      }
    })

    return cleanup
  }, [])
}
