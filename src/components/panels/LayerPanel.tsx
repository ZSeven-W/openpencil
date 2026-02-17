import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import type { PenNode } from '@/types/pen'
import LayerItem from './LayerItem'

function renderLayerTree(
  nodes: PenNode[],
  depth: number,
  selectedIds: string[],
  onSelect: (id: string) => void,
  onRename: (id: string, name: string) => void,
) {
  // Render in reverse order so top items appear at top of panel
  return [...nodes].reverse().map((node) => (
    <div key={node.id} className="group">
      <LayerItem
        id={node.id}
        name={node.name ?? node.type}
        type={node.type}
        depth={depth}
        selected={selectedIds.includes(node.id)}
        onSelect={onSelect}
        onRename={onRename}
      />
      {'children' in node &&
        node.children &&
        node.children.length > 0 &&
        renderLayerTree(
          node.children,
          depth + 1,
          selectedIds,
          onSelect,
          onRename,
        )}
    </div>
  ))
}

export default function LayerPanel() {
  const children = useDocumentStore((s) => s.document.children)
  const updateNode = useDocumentStore((s) => s.updateNode)
  const selectedIds = useCanvasStore((s) => s.selection.selectedIds)
  const setSelection = useCanvasStore((s) => s.setSelection)
  const fabricCanvas = useCanvasStore((s) => s.fabricCanvas)

  const handleSelect = (id: string) => {
    setSelection([id], id)

    // Also select the corresponding fabric object
    if (fabricCanvas) {
      const objects = fabricCanvas.getObjects()
      const target = objects.find(
        (o) => (o as FabricObjectWithPenId).penNodeId === id,
      )
      if (target) {
        fabricCanvas.setActiveObject(target)
        fabricCanvas.requestRenderAll()
      }
    }
  }

  const handleRename = (id: string, name: string) => {
    updateNode(id, { name })
  }

  return (
    <div className="w-56 bg-gray-800 border-r border-gray-700 flex flex-col shrink-0">
      <div className="h-8 flex items-center px-3 border-b border-gray-700">
        <span className="text-xs font-medium text-gray-300 uppercase tracking-wider">
          Layers
        </span>
      </div>
      <div className="flex-1 overflow-y-auto py-1 px-1">
        {children.length === 0 ? (
          <p className="text-xs text-gray-500 text-center mt-4 px-2">
            No layers yet. Use the toolbar to draw shapes.
          </p>
        ) : (
          renderLayerTree(
            children,
            0,
            selectedIds,
            handleSelect,
            handleRename,
          )
        )}
      </div>
    </div>
  )
}
