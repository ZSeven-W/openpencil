import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { Separator } from '@/components/ui/separator'
import type { PenNode } from '@/types/pen'
import SizeSection from './size-section'
import FillSection from './fill-section'
import StrokeSection from './stroke-section'
import AppearanceSection from './appearance-section'
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
    return null
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
        <span className="text-[11px] font-medium text-foreground">
          {node.name ?? node.type}
        </span>
      </div>
      <div className="flex-1 overflow-y-auto">
        <div className="px-3 py-2">
          <SizeSection
            node={node}
            onUpdate={handleUpdate}
            hasCornerRadius={hasCornerRadius}
            cornerRadius={
              'cornerRadius' in node ? node.cornerRadius : undefined
            }
          />
        </div>

        <Separator />

        {hasFill && (
          <>
            <div className="px-3 py-2">
              <FillSection
                fills={'fill' in node ? node.fill : undefined}
                onUpdate={handleUpdate}
              />
            </div>
            <Separator />
          </>
        )}

        {hasStroke && (
          <>
            <div className="px-3 py-2">
              <StrokeSection
                stroke={'stroke' in node ? node.stroke : undefined}
                onUpdate={handleUpdate}
              />
            </div>
            <Separator />
          </>
        )}

        <div className="px-3 py-2">
          <AppearanceSection node={node} onUpdate={handleUpdate} />
        </div>

        {hasEffects && (
          <>
            <Separator />
            <div className="px-3 py-2">
              <EffectsSection
                effects={'effects' in node ? node.effects : undefined}
                onUpdate={handleUpdate}
              />
            </div>
          </>
        )}

        {isText && node.type === 'text' && (
          <>
            <Separator />
            <div className="px-3 py-2">
              <TextSection node={node} onUpdate={handleUpdate} />
            </div>
          </>
        )}
      </div>
    </div>
  )
}
