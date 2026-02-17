import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import type { PenNode } from '@/types/pen'
import {
  createFabricObject,
  type FabricObjectWithPenId,
} from './canvas-object-factory'
import { syncFabricObject } from './canvas-object-sync'
import { fabricSyncLock } from './canvas-sync-lock'

function flattenNodes(nodes: PenNode[]): PenNode[] {
  const result: PenNode[] = []
  for (const node of nodes) {
    result.push(node)
    if ('children' in node && node.children) {
      result.push(...flattenNodes(node.children))
    }
  }
  return result
}

export function useCanvasSync() {
  useEffect(() => {
    const unsub = useDocumentStore.subscribe((state) => {
      if (fabricSyncLock) return

      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return

      const flatNodes = flattenNodes(state.document.children)
      const nodeMap = new Map(flatNodes.map((n) => [n.id, n]))
      const objects = canvas.getObjects() as FabricObjectWithPenId[]
      const objMap = new Map(
        objects
          .filter((o) => o.penNodeId)
          .map((o) => [o.penNodeId!, o]),
      )

      // Remove objects that no longer exist in the document
      for (const obj of objects) {
        if (obj.penNodeId && !nodeMap.has(obj.penNodeId)) {
          canvas.remove(obj)
        }
      }

      // Add or update objects
      for (const node of flatNodes) {
        if (node.type === 'ref') continue // Skip unresolved refs

        const existingObj = objMap.get(node.id)
        if (existingObj) {
          syncFabricObject(existingObj, node)
        } else {
          const newObj = createFabricObject(node)
          if (newObj) {
            canvas.add(newObj)
          }
        }
      }

      canvas.requestRenderAll()
    })

    return () => unsub()
  }, [])
}
