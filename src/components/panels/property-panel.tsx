import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import type { PenNode } from '@/types/pen'
import SizeSection from './size-section'
import FillSection from './fill-section'
import StrokeSection from './stroke-section'
import AppearanceSection from './appearance-section'
import CornerRadiusSection from './corner-radius-section'
import TextSection from './text-section'
import EffectsSection from './effects-section'

export default function PropertyPanel() {
  const activeId = useCanvasStore((s) => s.selection.activeId)
  const children = useDocumentStore((s) => s.document.children)
  const getNodeById = useDocumentStore((s) => s.getNodeById)
  const updateNode = useDocumentStore((s) => s.updateNode)

  // Subscribe to `children` so we re-render when nodes change
  void children
  const node = activeId ? getNodeById(activeId) : undefined

  const handleUpdate = (updates: Partial<PenNode>) => {
    if (activeId) {
      updateNode(activeId, updates)
    }
  }

  if (!node) {
    return (
      <div className="w-64 bg-card border-l border-border flex flex-col shrink-0">
        <div className="h-8 flex items-center px-3 border-b border-border">
          <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
            Properties
          </span>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <p className="text-xs text-muted-foreground px-4 text-center">
            Select an element to view its properties.
          </p>
        </div>
      </div>
    )
  }

  const hasFill =
    node.type !== 'line' && node.type !== 'ref'
  const hasStroke = node.type !== 'ref'
  const hasCornerRadius =
    node.type === 'rectangle' || node.type === 'frame'
  const hasEffects = node.type !== 'ref'
  const isText = node.type === 'text'

  return (
    <div className="w-64 bg-card border-l border-border flex flex-col shrink-0">
      <div className="h-8 flex items-center px-3 border-b border-border">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          {node.name ?? node.type}
        </span>
      </div>
      <div className="flex-1 overflow-y-auto p-3 space-y-4">
        <SizeSection node={node} onUpdate={handleUpdate} />

        {hasFill && (
          <FillSection
            fills={'fill' in node ? node.fill : undefined}
            onUpdate={handleUpdate}
          />
        )}

        {hasStroke && (
          <StrokeSection
            stroke={'stroke' in node ? node.stroke : undefined}
            onUpdate={handleUpdate}
          />
        )}

        {hasCornerRadius && (
          <CornerRadiusSection
            cornerRadius={
              'cornerRadius' in node ? node.cornerRadius : undefined
            }
            onUpdate={handleUpdate}
          />
        )}

        <AppearanceSection node={node} onUpdate={handleUpdate} />

        {hasEffects && (
          <EffectsSection
            effects={'effects' in node ? node.effects : undefined}
            onUpdate={handleUpdate}
          />
        )}

        {isText && node.type === 'text' && (
          <TextSection node={node} onUpdate={handleUpdate} />
        )}
      </div>
    </div>
  )
}
