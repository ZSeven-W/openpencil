import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import type { FabricObjectWithPenId } from './canvas-object-factory'

export function useCanvasSelection() {
  useEffect(() => {
    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      canvas.on('selection:created', (e) => {
        const selected = e.selected ?? []
        const ids = selected
          .map((obj) => (obj as FabricObjectWithPenId).penNodeId)
          .filter(Boolean) as string[]
        useCanvasStore.getState().setSelection(ids, ids[0] ?? null)
      })

      canvas.on('selection:updated', (e) => {
        const selected = e.selected ?? []
        const ids = selected
          .map((obj) => (obj as FabricObjectWithPenId).penNodeId)
          .filter(Boolean) as string[]
        useCanvasStore.getState().setSelection(ids, ids[0] ?? null)
      })

      canvas.on('selection:cleared', () => {
        useCanvasStore.getState().clearSelection()
      })
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
