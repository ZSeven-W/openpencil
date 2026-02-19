import { useEffect } from 'react'
import type { FabricObject } from 'fabric'
import { ActiveSelection } from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { resolveTargetAtDepth } from './selection-context'

/**
 * Resolve a list of Fabric selected objects to node IDs at the current
 * entered-frame depth. If any target falls outside the current context,
 * exits all frames and retries at root level.
 */
function resolveIds(selected: FabricObject[]): string[] {
  const resolved = new Set<string>()
  let hasUnresolved = false

  for (const obj of selected) {
    const penId = (obj as FabricObjectWithPenId).penNodeId
    if (!penId) continue
    const target = resolveTargetAtDepth(penId)
    if (target) resolved.add(target)
    else hasUnresolved = true
  }

  // If any target is outside current context, exit all frames and retry
  if (hasUnresolved) {
    useCanvasStore.getState().exitAllFrames()
    resolved.clear()
    for (const obj of selected) {
      const penId = (obj as FabricObjectWithPenId).penNodeId
      if (!penId) continue
      const target = resolveTargetAtDepth(penId)
      if (target) resolved.add(target)
    }
  }

  return [...resolved]
}

export function useCanvasSelection() {
  useEffect(() => {
    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      const handleSelection = (e: { selected?: FabricObject[] }) => {
        const selected = e.selected ?? []
        const ids = resolveIds(selected)
        useCanvasStore.getState().setSelection(ids, ids[0] ?? null)

        // Correct Fabric's active object to match the depth-resolved target.
        // Without this, Fabric keeps the deeply-nested child as active,
        // showing selection handles on the wrong element.
        if (ids.length === 0) return

        const objects = canvas.getObjects() as FabricObjectWithPenId[]

        if (ids.length === 1) {
          const currentActive = selected[0] as FabricObjectWithPenId
          if (currentActive?.penNodeId !== ids[0]) {
            const correctObj = objects.find((o) => o.penNodeId === ids[0])
            if (correctObj) {
              canvas.setActiveObject(correctObj)
              canvas.requestRenderAll()
            }
          }
        } else {
          // Multi-select: build an ActiveSelection from the resolved objects
          const resolvedSet = new Set(ids)
          const resolvedObjs = objects.filter(
            (o) => o.penNodeId && resolvedSet.has(o.penNodeId),
          )
          if (resolvedObjs.length > 1) {
            const sel = new ActiveSelection(resolvedObjs, { canvas })
            canvas.setActiveObject(sel)
            canvas.requestRenderAll()
          } else if (resolvedObjs.length === 1) {
            canvas.setActiveObject(resolvedObjs[0])
            canvas.requestRenderAll()
          }
        }
      }

      canvas.on('selection:created', handleSelection)
      canvas.on('selection:updated', handleSelection)

      canvas.on('selection:cleared', () => {
        useCanvasStore.getState().clearSelection()
      })
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
