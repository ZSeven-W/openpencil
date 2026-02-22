import { useEffect } from 'react'
import type { FabricObject } from 'fabric'
import { ActiveSelection } from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { resolveTargetAtDepth } from './selection-context'

/**
 * When true, the next selection event will skip depth-resolution and
 * pass through as-is. Used by the layer panel to programmatically select
 * children without the handler resolving them back to their parent.
 */
let skipNextDepthResolve = false
export function setSkipNextDepthResolve() {
  skipNextDepthResolve = true
}

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

      // Guard against re-entry: setActiveObject fires selection events
      // synchronously, which would call handleSelection again and cause
      // infinite recursion.
      let updatingSelection = false

      const handleSelection = (e: { selected?: FabricObject[]; e?: unknown }) => {
        if (updatingSelection) return

        // Programmatic selection from layer panel — skip depth resolution
        if (skipNextDepthResolve) {
          skipNextDepthResolve = false
          return
        }

        // `selection:updated` payload `selected` may contain only delta objects.
        // Always read the full active selection from canvas for accurate multi-select.
        const selected = canvas.getActiveObjects()
        const fallbackSelected = e.selected ?? []
        const effectiveSelected = selected.length > 0 ? selected : fallbackSelected
        const prevIds = useCanvasStore.getState().selection.selectedIds
        const mouseEvent = e.e as MouseEvent | undefined

        // If user already has a multi-selection and clicks one selected object
        // (without Shift), keep the whole selection so dragging moves all.
        if (!mouseEvent?.shiftKey && prevIds.length > 1 && effectiveSelected.length === 1) {
          const clicked = effectiveSelected[0] as FabricObjectWithPenId
          const resolvedClicked = clicked?.penNodeId
            ? resolveTargetAtDepth(clicked.penNodeId)
            : null

          if (resolvedClicked && prevIds.includes(resolvedClicked)) {
            const objects = canvas.getObjects() as FabricObjectWithPenId[]
            const prevSet = new Set(prevIds)
            const restoredObjects = objects.filter(
              (o) => o.penNodeId && prevSet.has(o.penNodeId),
            )

            if (restoredObjects.length > 1) {
              updatingSelection = true
              try {
                const restored = new ActiveSelection(restoredObjects, { canvas })
                canvas.setActiveObject(restored)
                canvas.requestRenderAll()
                useCanvasStore.getState().setSelection(prevIds, prevIds[0] ?? null)
              } finally {
                updatingSelection = false
              }
              return
            }
          }
        }

        const ids = resolveIds(effectiveSelected)
        useCanvasStore.getState().setSelection(ids, ids[0] ?? null)

        // Correct Fabric's active object to match the depth-resolved target.
        // Without this, Fabric keeps the deeply-nested child as active,
        // showing selection handles on the wrong element.
        if (ids.length === 0) return

        const objects = canvas.getObjects() as FabricObjectWithPenId[]

        updatingSelection = true
        try {
          if (ids.length === 1) {
            const currentActive = effectiveSelected[0] as FabricObjectWithPenId
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
        } finally {
          updatingSelection = false
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
