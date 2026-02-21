import { useCallback } from 'react'
import { nanoid } from 'nanoid'
import type { KitComponent, UIKit } from '@/types/uikit'
import type { PenNode } from '@/types/pen'
import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { findReusableNode, deepCloneNode, collectVariableRefs } from '@/uikit/kit-utils'

interface ComponentBrowserCardProps {
  component: KitComponent
  kit: UIKit
}

/** Recursively assign new IDs to a node tree so each insert is independent. */
function reassignIds(node: PenNode): PenNode {
  const clone = { ...node, id: nanoid() }
  if ('children' in clone && Array.isArray(clone.children)) {
    clone.children = clone.children.map(reassignIds)
  }
  return clone as PenNode
}

export default function ComponentBrowserCard({ component, kit }: ComponentBrowserCardProps) {
  const handleInsert = useCallback(() => {
    const { addNode, document } = useDocumentStore.getState()
    const { viewport, fabricCanvas } = useCanvasStore.getState()

    const kitNode = findReusableNode(kit.document, component.id)
    if (!kitNode) return

    // Copy referenced variables from the kit document
    if (kit.document.variables) {
      const refs = collectVariableRefs(kitNode)
      const { setVariable } = useDocumentStore.getState()
      for (const ref of refs) {
        const name = ref.startsWith('$') ? ref.slice(1) : ref
        const varDef = kit.document.variables[name]
        if (varDef && !document.variables?.[name]) {
          setVariable(name, varDef)
        }
      }
    }

    // Deep clone with new IDs, remove reusable flag so it's standalone
    const cloned = reassignIds(deepCloneNode(kitNode))
    if ('reusable' in cloned) {
      delete (cloned as unknown as Record<string, unknown>).reusable
    }

    // Place at viewport center
    const canvasEl = fabricCanvas?.getElement()
    const canvasW = canvasEl?.clientWidth ?? 800
    const canvasH = canvasEl?.clientHeight ?? 600
    const centerX = (-viewport.panX + canvasW / 2) / viewport.zoom
    const centerY = (-viewport.panY + canvasH / 2) / viewport.zoom
    cloned.x = centerX - component.width / 2
    cloned.y = centerY - component.height / 2
    cloned.name = component.name

    addNode(null, cloned)
    useCanvasStore.getState().setSelection([cloned.id], cloned.id)
  }, [component, kit])

  // Compute preview aspect ratio
  const maxPreviewW = 120
  const maxPreviewH = 64
  const scale = Math.min(maxPreviewW / component.width, maxPreviewH / component.height, 1)
  const previewW = Math.round(component.width * scale)
  const previewH = Math.round(component.height * scale)

  // Extract primary fill color from the kit node for preview
  const kitNode = findReusableNode(kit.document, component.id)
  let fillColor = '#E5E7EB'
  if (kitNode && 'fill' in kitNode && Array.isArray(kitNode.fill) && kitNode.fill.length > 0) {
    const f = kitNode.fill[0]
    if (f.type === 'solid' && !f.color.startsWith('$')) {
      fillColor = f.color
    }
  }

  return (
    <button
      type="button"
      onClick={handleInsert}
      className="flex flex-col items-center gap-2 p-3 rounded-lg border border-border bg-card hover:bg-muted transition-colors cursor-pointer group"
    >
      <div className="flex items-center justify-center w-full h-16">
        <div
          className="rounded transition-transform group-hover:scale-105"
          style={{
            width: previewW,
            height: previewH,
            backgroundColor: fillColor,
          }}
        />
      </div>
      <span className="text-xs text-foreground truncate w-full text-center">
        {component.name}
      </span>
    </button>
  )
}
