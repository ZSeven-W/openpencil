import { useEffect, useState, useCallback } from 'react'
import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { normalizePenDocument } from '@/utils/normalize-pen-file'
import type { PenDocument } from '@/types/pen'

/**
 * Parse a dropped File into a PenDocument.
 * Returns null if the file is not a valid .op/.pen/.json document.
 */
async function parseDroppedFile(
  file: File,
): Promise<{ doc: PenDocument; fileName: string } | null> {
  const ext = file.name.split('.').pop()?.toLowerCase()
  if (ext !== 'op' && ext !== 'pen' && ext !== 'json') return null

  try {
    const text = await file.text()
    const raw = JSON.parse(text) as PenDocument
    if (
      !raw.version ||
      (!Array.isArray(raw.children) && !Array.isArray(raw.pages))
    ) {
      return null
    }
    const doc = normalizePenDocument(raw)
    return { doc, fileName: file.name }
  } catch {
    return null
  }
}

/**
 * Hook that enables drag-and-drop file opening on the editor.
 * Returns `isDragging` state for rendering a drop zone overlay.
 */
export function useFileDrop() {
  const [isDragging, setIsDragging] = useState(false)

  // Track nested drag enter/leave so overlay doesn't flicker
  const handleDragEnter = useCallback((e: DragEvent) => {
    e.preventDefault()
    if (e.dataTransfer?.types.includes('Files')) {
      setIsDragging(true)
    }
  }, [])

  const handleDragOver = useCallback((e: DragEvent) => {
    e.preventDefault()
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'copy'
    }
  }, [])

  const handleDragLeave = useCallback((e: DragEvent) => {
    // Only close overlay when leaving the window (relatedTarget is null)
    if (!e.relatedTarget) {
      setIsDragging(false)
    }
  }, [])

  const handleDrop = useCallback(async (e: DragEvent) => {
    e.preventDefault()
    setIsDragging(false)

    const file = e.dataTransfer?.files?.[0]
    if (!file) return

    // .fig files → open Figma import dialog with the file pre-loaded
    const ext = file.name.split('.').pop()?.toLowerCase()
    if (ext === 'fig') {
      const store = useCanvasStore.getState()
      store.setPendingFigmaFile(file)
      store.setFigmaImportDialogOpen(true)
      return
    }

    const result = await parseDroppedFile(file)
    if (!result) return

    useDocumentStore.getState().loadDocument(result.doc, result.fileName)

    // Let the canvas sync, then zoom to fit
    const { zoomToFitContent } = await import('@/canvas/use-fabric-canvas')
    requestAnimationFrame(() => zoomToFitContent())
  }, [])

  useEffect(() => {
    window.addEventListener('dragenter', handleDragEnter)
    window.addEventListener('dragover', handleDragOver)
    window.addEventListener('dragleave', handleDragLeave)
    window.addEventListener('drop', handleDrop)
    return () => {
      window.removeEventListener('dragenter', handleDragEnter)
      window.removeEventListener('dragover', handleDragOver)
      window.removeEventListener('dragleave', handleDragLeave)
      window.removeEventListener('drop', handleDrop)
    }
  }, [handleDragEnter, handleDragOver, handleDragLeave, handleDrop])

  return isDragging
}
