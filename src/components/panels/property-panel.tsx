import { useState, useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { Separator } from '@/components/ui/separator'
import type { PenNode, ContainerProps, RefNode } from '@/types/pen'
import { Component, Diamond, ArrowUpRight, Unlink } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import SizeSection from './size-section'
import LayoutSection from './layout-section'
import FillSection from './fill-section'
import StrokeSection from './stroke-section'
import AppearanceSection from './appearance-section'
import TextSection from './text-section'
import EffectsSection from './effects-section'

/** Properties stored directly on the RefNode (instance-level), not as overrides. */
const INSTANCE_DIRECT_PROPS = new Set([
  'x', 'y', 'width', 'height', 'name', 'visible', 'locked', 'rotation', 'opacity', 'flipX', 'flipY', 'enabled', 'theme',
])

export default function PropertyPanel() {
  const activeId = useCanvasStore((s) => s.selection.activeId)
  const setSelection = useCanvasStore((s) => s.setSelection)
  const fabricCanvas = useCanvasStore((s) => s.fabricCanvas)
  const children = useDocumentStore((s) => s.document.children)
  const getNodeById = useDocumentStore((s) => s.getNodeById)
  const updateNode = useDocumentStore((s) => s.updateNode)
  const makeReusable = useDocumentStore((s) => s.makeReusable)
  const detachComponent = useDocumentStore((s) => s.detachComponent)

  // Subscribe to `children` so we re-render when nodes change
  void children
  const node = activeId ? getNodeById(activeId) : undefined

  if (!node) {
    return null
  }

  const nodeIsReusable = 'reusable' in node && node.reusable === true
  const nodeIsInstance = node.type === 'ref'

  // For RefNodes, resolve the referenced component to get visual properties.
  // The display node merges: component base → instance overrides → RefNode position/meta.
  let displayNode = node
  if (nodeIsInstance) {
    const refNode = node as RefNode
    const component = getNodeById(refNode.ref)
    if (component) {
      const topOverrides = refNode.descendants?.[refNode.ref] ?? {}
      const merged: Record<string, unknown> = { ...component, ...topOverrides }
      // Apply RefNode's own explicitly defined properties
      for (const [key, val] of Object.entries(node)) {
        if (key === 'type' || key === 'ref' || key === 'descendants' || key === 'children') continue
        if (val !== undefined) {
          merged[key] = val
        }
      }
      // Use component's type (frame/rect/etc.) not 'ref'
      merged.type = component.type
      if (!merged.name) merged.name = component.name
      displayNode = merged as unknown as PenNode
    }
  }

  const handleUpdate = (updates: Partial<PenNode>) => {
    if (!activeId) return
    if (nodeIsInstance && node.type === 'ref') {
      const refNode = node as RefNode
      const refNodeUpdate: Record<string, unknown> = {}
      const overrideProps: Record<string, unknown> = {}

      for (const [key, value] of Object.entries(updates)) {
        if (INSTANCE_DIRECT_PROPS.has(key)) {
          refNodeUpdate[key] = value
        } else {
          overrideProps[key] = value
        }
      }

      // Store visual properties as overrides in descendants[ref]
      if (Object.keys(overrideProps).length > 0) {
        const currentDescendants = refNode.descendants ?? {}
        const existing = currentDescendants[refNode.ref] ?? {}
        refNodeUpdate.descendants = {
          ...currentDescendants,
          [refNode.ref]: { ...existing, ...overrideProps },
        }
      }

      if (Object.keys(refNodeUpdate).length > 0) {
        updateNode(activeId, refNodeUpdate as Partial<PenNode>)
      }
    } else {
      updateNode(activeId, updates)
    }
  }

  const handleGoToComponent = () => {
    if (!nodeIsInstance || node.type !== 'ref') return
    const refId = (node as RefNode).ref
    setSelection([refId], refId)
    if (fabricCanvas) {
      const objects = fabricCanvas.getObjects() as FabricObjectWithPenId[]
      const target = objects.find((o) => (o as FabricObjectWithPenId).penNodeId === refId)
      if (target) {
        fabricCanvas.setActiveObject(target)
        fabricCanvas.requestRenderAll()
      }
    }
  }

  const isContainer =
    displayNode.type === 'frame' || displayNode.type === 'group' || displayNode.type === 'rectangle'
  const hasLayout = isContainer
  const hasFill = displayNode.type !== 'line'
  const hasStroke = true
  const hasCornerRadius =
    displayNode.type === 'rectangle' || displayNode.type === 'frame'
  const hasEffects = true
  const isText = displayNode.type === 'text'

  const [isEditingName, setIsEditingName] = useState(false)
  const [editName, setEditName] = useState('')

  // Reset editing state when selection changes
  useEffect(() => {
    setIsEditingName(false)
  }, [activeId])

  const handleNameClick = () => {
    setEditName(node.name ?? node.type)
    setIsEditingName(true)
  }

  const handleNameBlur = () => {
    setIsEditingName(false)
    const trimmed = editName.trim()
    if (trimmed && trimmed !== (node.name ?? node.type)) {
      updateNode(activeId!, { name: trimmed } as Partial<PenNode>)
    }
  }

  const handleNameKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
    if (e.key === 'Escape') {
      setIsEditingName(false)
      setEditName(node.name ?? node.type)
    }
  }

  return (
    <div className="w-64 bg-card border-l border-border flex flex-col shrink-0">
      {/* Header */}
      <div className="h-8 flex items-center px-2 border-b border-border gap-1">
        {(nodeIsReusable || nodeIsInstance) && (
          <Diamond size={12} className={`shrink-0 ${nodeIsReusable ? 'text-purple-400' : 'text-[#9281f7]'}`} />
        )}
        {isEditingName ? (
          <input
            type="text"
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            onBlur={handleNameBlur}
            onKeyDown={handleNameKeyDown}
            className={`text-[11px] font-medium flex-1 min-w-0 bg-secondary rounded px-1.5 py-0.5 border border-ring focus:outline-none ${
              nodeIsReusable
                ? 'text-purple-400'
                : nodeIsInstance
                  ? 'text-[#9281f7]'
                  : 'text-foreground'
            }`}
            autoFocus
          />
        ) : (
          <span
            className={`text-[11px] font-medium flex-1 truncate cursor-text ${
              nodeIsReusable
                ? 'text-purple-400 border border-purple-400/50 rounded px-1.5 py-0.5'
                : nodeIsInstance
                  ? 'text-[#9281f7] border border-dashed border-[#9281f7]/50 rounded px-1.5 py-0.5'
                  : 'text-foreground px-1'
            }`}
            onClick={handleNameClick}
          >
            {node.name ?? node.type}
          </span>
        )}
        {nodeIsInstance && (
          <>
            <button
              type="button"
              title="Go to component"
              className="p-1 rounded hover:bg-accent/50 text-muted-foreground hover:text-foreground shrink-0"
              onClick={handleGoToComponent}
            >
              <ArrowUpRight size={12} />
            </button>
            <button
              type="button"
              title="Detach instance"
              className="p-1 rounded hover:bg-accent/50 text-muted-foreground hover:text-foreground shrink-0"
              onClick={() => activeId && detachComponent(activeId)}
            >
              <Unlink size={12} />
            </button>
          </>
        )}
      </div>

      <div className="flex-1 overflow-y-auto">
        {(isContainer || nodeIsInstance) && (
          <div className="px-3 py-2">
            {nodeIsReusable ? (
              <Button
                variant="outline"
                size="sm"
                className="w-full h-7 text-xs gap-1.5 text-purple-400 border-purple-500/30 hover:bg-purple-500/10"
                onClick={() => activeId && detachComponent(activeId)}
              >
                <Unlink size={12} />
                Detach Component
              </Button>
            ) : (
              <Button
                variant="outline"
                size="sm"
                className="w-full h-7 text-xs gap-1.5"
                onClick={() => {
                  if (!activeId) return
                  if (nodeIsInstance) {
                    const newId = detachComponent(activeId)
                    if (newId) {
                      makeReusable(newId)
                      setSelection([newId], newId)
                    }
                    return
                  }
                  makeReusable(activeId)
                }}
              >
                <Component size={12} />
                Create Component
              </Button>
            )}
          </div>
        )}

        <div className="px-3 py-2">
          <SizeSection
            node={displayNode}
            onUpdate={handleUpdate}
            hasCornerRadius={hasCornerRadius}
            cornerRadius={
              'cornerRadius' in displayNode ? displayNode.cornerRadius : undefined
            }
          />
        </div>

        {hasLayout && (
          <>
            <Separator />
            <div className="px-3 py-2">
              <LayoutSection node={displayNode as PenNode & ContainerProps} onUpdate={handleUpdate} />
            </div>
          </>
        )}

        <Separator />

        {hasFill && (
          <>
            <div className="px-3 py-2">
              <FillSection
                fills={'fill' in displayNode ? displayNode.fill : undefined}
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
                stroke={'stroke' in displayNode ? displayNode.stroke : undefined}
                onUpdate={handleUpdate}
              />
            </div>
            <Separator />
          </>
        )}

        <div className="px-3 py-2">
          <AppearanceSection node={displayNode} onUpdate={handleUpdate} />
        </div>

        {hasEffects && (
          <>
            <Separator />
            <div className="px-3 py-2">
              <EffectsSection
                effects={'effects' in displayNode ? displayNode.effects : undefined}
                onUpdate={handleUpdate}
              />
            </div>
          </>
        )}

        {isText && displayNode.type === 'text' && (
          <>
            <Separator />
            <div className="px-3 py-2">
              <TextSection node={displayNode} onUpdate={handleUpdate} />
            </div>
          </>
        )}
      </div>
    </div>
  )
}
