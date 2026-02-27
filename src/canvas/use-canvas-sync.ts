import { useEffect } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, findNodeInTree, getActivePageChildren, setActivePageChildren, getAllChildren } from '@/stores/document-store'
import type { PenNode, ContainerProps } from '@/types/pen'
import {
  createFabricObject,
  type FabricObjectWithPenId,
} from './canvas-object-factory'
import { syncFabricObject } from './canvas-object-sync'
import { isFabricSyncLocked, setFabricSyncLock } from './canvas-sync-lock'
import { pendingAnimationNodes, getNextStaggerDelay } from '@/services/ai/design-animation'
import { resolveNodeForCanvas, getDefaultTheme } from '@/variables/resolve-variables'
import { COMPONENT_COLOR, INSTANCE_COLOR, SELECTION_BLUE } from './canvas-constants'
import {
  type Padding,
  resolvePadding,
  isNodeVisible,
  getNodeWidth,
  getNodeHeight,
  computeLayoutPositions,
} from './canvas-layout-engine'
import { parseSizing } from './canvas-text-measure'

// ---------------------------------------------------------------------------
// Clip info — tracks parent frame bounds for child clipping
// ---------------------------------------------------------------------------

interface ClipInfo {
  x: number
  y: number
  w: number
  h: number
  rx: number
}

// ---------------------------------------------------------------------------
// Render info — tracks parent offset & layout status for each node.
// Used by use-canvas-events to convert absolute ↔ relative positions.
// ---------------------------------------------------------------------------

export interface NodeRenderInfo {
  parentOffsetX: number
  parentOffsetY: number
  isLayoutChild: boolean
}

/** Rebuilt every sync cycle. Maps nodeId → parent offset + layout child status. */
export const nodeRenderInfo = new Map<string, NodeRenderInfo>()

/** Maps root-frame IDs to their absolute bounds. Rebuilt every sync cycle. */
export const rootFrameBounds = new Map<string, { x: number; y: number; w: number; h: number }>()

/** Info for layout containers — used by drag-into-layout for hit detection. */
export interface LayoutContainerInfo {
  x: number; y: number; w: number; h: number
  layout: 'vertical' | 'horizontal'
  padding: Padding
  gap: number
}

/** Maps layout container IDs to their absolute bounds + layout info. Rebuilt every sync cycle. */
export const layoutContainerBounds = new Map<string, LayoutContainerInfo>()

// ---------------------------------------------------------------------------
// Resolve RefNodes — expand instances by looking up their referenced component
// ---------------------------------------------------------------------------

/** Give children unique IDs scoped to the instance, apply overrides from descendants. */
function remapInstanceChildIds(
  children: PenNode[],
  refId: string,
  overrides?: Record<string, Partial<PenNode>>,
): PenNode[] {
  return children.map((child) => {
    const virtualId = `${refId}__${child.id}`
    const ov = overrides?.[child.id] ?? {}
    const mapped = { ...child, ...ov, id: virtualId } as PenNode
    if ('children' in mapped && mapped.children) {
      ;(mapped as PenNode & { children: PenNode[] }).children =
        remapInstanceChildIds(mapped.children, refId, overrides)
    }
    return mapped
  })
}

/**
 * Recursively resolve all RefNodes in the tree by expanding them
 * with their referenced component's structure.
 */
function resolveRefs(
  nodes: PenNode[],
  rootNodes: PenNode[],
  visited = new Set<string>(),
): PenNode[] {
  return nodes.flatMap((node) => {
    if (node.type !== 'ref') {
      if ('children' in node && node.children) {
        return [
          {
            ...node,
            children: resolveRefs(node.children, rootNodes, visited),
          } as PenNode,
        ]
      }
      return [node]
    }

    // Resolve RefNode
    if (visited.has(node.ref)) return [] // circular reference guard
    const component = findNodeInTree(rootNodes, node.ref)
    if (!component) return []

    visited.add(node.ref)

    const refNode = node as PenNode & { descendants?: Record<string, Partial<PenNode>> }
    // Apply top-level visual overrides from descendants[componentId]
    const topOverrides = refNode.descendants?.[node.ref] ?? {}

    // Build resolved node: component base → overrides → RefNode's own properties
    const resolved: Record<string, unknown> = { ...component, ...topOverrides }
    // Apply all explicitly-defined RefNode properties (position, size, opacity, etc.)
    for (const [key, val] of Object.entries(node)) {
      if (key === 'type' || key === 'ref' || key === 'descendants' || key === 'children') continue
      if (val !== undefined) {
        resolved[key] = val
      }
    }
    // Use component's type (not 'ref') and ensure name fallback
    resolved.type = component.type
    if (!resolved.name) resolved.name = component.name
    // Clear the reusable flag — this is an instance, not the component
    delete resolved.reusable
    const resolvedNode = resolved as unknown as PenNode

    // Remap children IDs to avoid clashes with the original component
    if ('children' in resolvedNode && resolvedNode.children) {
      ;(resolvedNode as PenNode & { children: PenNode[] }).children =
        remapInstanceChildIds(
          resolvedNode.children,
          node.id,
          refNode.descendants,
        )
    }

    visited.delete(node.ref)
    return [resolvedNode]
  })
}

// ---------------------------------------------------------------------------
// Flatten document tree → absolute-positioned list for Fabric.js
// ---------------------------------------------------------------------------

function cornerRadiusVal(
  cr: number | [number, number, number, number] | undefined,
): number {
  if (cr === undefined) return 0
  if (typeof cr === 'number') return cr
  return cr[0]
}

function flattenNodes(
  nodes: PenNode[],
  offsetX = 0,
  offsetY = 0,
  parentAvailW?: number,
  parentAvailH?: number,
  clipCtx?: ClipInfo,
  clipMap?: Map<string, ClipInfo>,
  isLayoutChild = false,
  depth = 0,
): PenNode[] {
  const result: PenNode[] = []
  for (const node of nodes) {
    if (!isNodeVisible(node)) continue

    // Store render info for position conversion in canvas events
    nodeRenderInfo.set(node.id, {
      parentOffsetX: offsetX,
      parentOffsetY: offsetY,
      isLayoutChild,
    })

    // Resolve fill_container / fit_content string sizes into pixel values
    let resolved = node
    if (parentAvailW !== undefined || parentAvailH !== undefined) {
      let changed = false
      const r: Record<string, unknown> = { ...node }
      if ('width' in node && typeof node.width !== 'number') {
        const s = parseSizing(node.width)
        if (s === 'fill' && parentAvailW) {
          r.width = parentAvailW
          changed = true
        } else if (s === 'fit') {
          r.width = getNodeWidth(node, parentAvailW)
          changed = true
        }
      }
      if ('height' in node && typeof node.height !== 'number') {
        const s = parseSizing(node.height)
        if (s === 'fill' && parentAvailH) {
          r.height = parentAvailH
          changed = true
        } else if (s === 'fit') {
          r.height = getNodeHeight(node, parentAvailH, parentAvailW)
          changed = true
        }
      }
      if (changed) resolved = r as unknown as PenNode
    }

    // Apply parent offset to get absolute position for rendering
    const absoluteNode =
      offsetX !== 0 || offsetY !== 0
        ? {
            ...resolved,
            x: (resolved.x ?? 0) + offsetX,
            y: (resolved.y ?? 0) + offsetY,
          }
        : resolved

    // Store clip info from parent frame (if any)
    if (clipCtx && clipMap) {
      clipMap.set(node.id, clipCtx)
    }

    result.push(absoluteNode as PenNode)

    const children = 'children' in node ? node.children : undefined
    if (children && children.length > 0) {
      const parentAbsX = (resolved.x ?? 0) + offsetX
      const parentAbsY = (resolved.y ?? 0) + offsetY

      // Compute available dimensions for children
      const nodeW = getNodeWidth(resolved, parentAvailW)
      const nodeH = getNodeHeight(resolved, parentAvailH)
      const pad = resolvePadding(
        'padding' in resolved ? (resolved as any).padding : undefined,
      )
      const childAvailW = Math.max(0, nodeW - pad.left - pad.right)
      const childAvailH = Math.max(0, nodeH - pad.top - pad.bottom)

      // If the parent has an auto-layout, compute child positions first
      const layout = 'layout' in node ? (node as ContainerProps).layout : undefined
      const positioned =
        layout && layout !== 'none'
          ? computeLayoutPositions(resolved, children)
          : children

      // Compute clip context for children:
      // - Root frames (depth 0, type frame) always clip their children
      // - Non-root frames clip only when they have cornerRadius
      let childClip = clipCtx
      const cr = 'cornerRadius' in node ? cornerRadiusVal(node.cornerRadius) : 0
      const isRootFrame = node.type === 'frame' && depth === 0
      const hasClipContent = 'clipContent' in node && (node as ContainerProps).clipContent === true
      if (isRootFrame || cr > 0 || hasClipContent) {
        childClip = { x: parentAbsX, y: parentAbsY, w: nodeW, h: nodeH, rx: cr }
      }

      // Track root frame bounds for drag-out reparenting
      if (isRootFrame) {
        rootFrameBounds.set(node.id, { x: parentAbsX, y: parentAbsY, w: nodeW, h: nodeH })
      }

      // Track layout container bounds for drag-into detection
      if (layout && layout !== 'none') {
        const gap = 'gap' in node && typeof (node as any).gap === 'number' ? (node as any).gap : 0
        layoutContainerBounds.set(node.id, {
          x: parentAbsX, y: parentAbsY, w: nodeW, h: nodeH,
          layout: layout as 'vertical' | 'horizontal',
          padding: pad, gap,
        })
      }

      // Children inside layout containers are layout-controlled (position not manually editable)
      const childIsLayoutChild = !!(layout && layout !== 'none')

      result.push(
        ...flattenNodes(positioned, parentAbsX, parentAbsY, childAvailW, childAvailH, childClip, clipMap, childIsLayoutChild, depth + 1),
      )
    }
  }
  return result
}

/**
 * Rebuild nodeRenderInfo from the current document state.
 * Called after locked syncs (e.g. object:modified) so that subsequent
 * panel-driven property changes use fresh parent-offset data.
 */
export function rebuildNodeRenderInfo() {
  const state = useDocumentStore.getState()
  const activePageId = useCanvasStore.getState().activePageId
  const pageChildren = getActivePageChildren(state.document, activePageId)
  const allNodes = getAllChildren(state.document)
  nodeRenderInfo.clear()
  rootFrameBounds.clear()
  layoutContainerBounds.clear()
  const resolvedTree = resolveRefs(pageChildren, allNodes)
  flattenNodes(resolvedTree, 0, 0, undefined, undefined, undefined, new Map())
}

/**
 * Force-sync every Fabric object's position/size back to the document store.
 * Call this before saving to guarantee the file captures the latest canvas state,
 * even if a real-time sync was missed for any reason.
 */
export function syncCanvasPositionsToStore() {
  const canvas = useCanvasStore.getState().fabricCanvas
  if (!canvas) return

  // Ensure nodeRenderInfo is fresh
  rebuildNodeRenderInfo()

  const objects = canvas.getObjects() as FabricObjectWithPenId[]
  setFabricSyncLock(true)
  try {
    for (const obj of objects) {
      if (!obj.penNodeId) continue

      const info = nodeRenderInfo.get(obj.penNodeId)
      const offsetX = info?.parentOffsetX ?? 0
      const offsetY = info?.parentOffsetY ?? 0
      const scaleX = obj.scaleX ?? 1
      const scaleY = obj.scaleY ?? 1

      const updates: Record<string, unknown> = {
        x: (obj.left ?? 0) - offsetX,
        y: (obj.top ?? 0) - offsetY,
        rotation: obj.angle ?? 0,
      }

      if (obj.width !== undefined) {
        updates.width = obj.width * scaleX
      }
      if (obj.height !== undefined) {
        updates.height = obj.height * scaleY
      }

      // Sync text content too
      if ('text' in obj && typeof (obj as any).text === 'string') {
        updates.content = (obj as any).text
      }

      useDocumentStore
        .getState()
        .updateNode(obj.penNodeId, updates as Partial<PenNode>)
    }
  } finally {
    setFabricSyncLock(false)
  }
}

export function useCanvasSync() {
  useEffect(() => {
    // Track the previous document reference so we only re-sync Fabric when
    // the document tree actually changes — not on every store update (e.g.
    // `isDirty`, `fileName`).  Without this guard, operations like
    // `markClean()` trigger a full re-sync that overwrites canvas-side
    // changes (drag positions, edited text) with stale store data if those
    // changes failed to write back to the store for any reason.
    let prevPageChildren = getActivePageChildren(
      useDocumentStore.getState().document,
      useCanvasStore.getState().activePageId,
    )
    let prevVariables = useDocumentStore.getState().document.variables
    let prevThemes = useDocumentStore.getState().document.themes
    let prevActivePageId = useCanvasStore.getState().activePageId

    // Subscribe to page switches
    const unsubCanvas = useCanvasStore.subscribe((cs) => {
      if (cs.activePageId !== prevActivePageId) {
        prevActivePageId = cs.activePageId
        // Force a full re-sync by resetting prevPageChildren
        prevPageChildren = null as unknown as PenNode[]
        // Trigger document subscription by creating a new reference
        const { document: doc } = useDocumentStore.getState()
        const pageChildren = getActivePageChildren(doc, cs.activePageId)
        if (pageChildren.length > 0) {
          useDocumentStore.setState({
            document: setActivePageChildren(doc, cs.activePageId, [...pageChildren]),
          })
        }
      }
    })

    const unsub = useDocumentStore.subscribe((state) => {
      const activePageId = useCanvasStore.getState().activePageId
      const pageChildren = getActivePageChildren(state.document, activePageId)

      // Always track the latest references — even when the sync lock
      // is active — so that unrelated store updates (e.g. markClean setting
      // isDirty) don't trigger a stale re-sync that overwrites canvas state.
      const childrenChanged = pageChildren !== prevPageChildren
      const variablesChanged = state.document.variables !== prevVariables
      const themesChanged = state.document.themes !== prevThemes
      prevPageChildren = pageChildren
      prevVariables = state.document.variables
      prevThemes = state.document.themes

      if (isFabricSyncLocked()) return

      // Skip re-sync when only non-document fields changed (isDirty, fileName, etc.)
      if (!childrenChanged && !variablesChanged && !themesChanged) return

      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return

      // Build variable resolution context
      const variables = state.document.variables ?? {}
      const activeTheme = getDefaultTheme(state.document.themes)

      // Use active page children for rendering, all children for ref resolution
      const allNodes = getAllChildren(state.document)

      const clipMap = new Map<string, ClipInfo>()
      nodeRenderInfo.clear()
      rootFrameBounds.clear()
      layoutContainerBounds.clear()
      // Resolve RefNodes before flattening so instances render as their component
      const resolvedTree = resolveRefs(pageChildren, allNodes)
      const flatNodes = flattenNodes(
        resolvedTree, 0, 0, undefined, undefined, undefined, clipMap,
      ).map((node) => resolveNodeForCanvas(node, variables, activeTheme))
      const nodeMap = new Map(flatNodes.map((n) => [n.id, n]))
      const objects = canvas.getObjects() as FabricObjectWithPenId[]
      const objMap = new Map(
        objects
          .filter((o) => o.penNodeId)
          .map((o) => [o.penNodeId!, o]),
      )

      // Collect component and instance IDs for selection styling
      const reusableIds = new Set<string>()
      const instanceIds = new Set<string>()
      ;(function collectComponentIds(nodes: PenNode[]) {
        for (const n of nodes) {
          if ('reusable' in n && n.reusable === true) reusableIds.add(n.id)
          if (n.type === 'ref') instanceIds.add(n.id)
          if ('children' in n && n.children) collectComponentIds(n.children)
        }
      })(pageChildren)

      // Remove objects that no longer exist in the document
      for (const obj of objects) {
        if (obj.penNodeId && !nodeMap.has(obj.penNodeId)) {
          canvas.remove(obj)
        }
      }

      // Add or update objects
      for (const node of flatNodes) {
        if (node.type === 'ref') continue // Skip unresolved refs

        let obj: FabricObjectWithPenId | undefined
        let existingObj = objMap.get(node.id)

        // Some object types require recreation when their underlying Fabric
        // class/shape data changes (e.g. IText↔Textbox).
        // Path `d` changes are handled in-place by syncFabricObject.
        let objectRecreated = false
        // Text nodes may need recreation when textGrowth mode changes
        // (IText ↔ Textbox are different Fabric classes).
        if (existingObj && node.type === 'text') {
          const growth = node.textGrowth
          const needsTextbox = growth === 'fixed-width' || growth === 'fixed-width-height'
          const isTextbox = existingObj instanceof fabric.Textbox
          if (needsTextbox !== isTextbox) {
            canvas.remove(existingObj)
            existingObj = undefined
            objectRecreated = true
          }
        }

        if (existingObj) {
          // Skip objects inside an ActiveSelection — their left/top are
          // group-relative, not absolute.  Setting absolute values from
          // the store would move them to wrong positions (snap-back bug).
          if (existingObj.group instanceof fabric.ActiveSelection) {
            continue
          }
          syncFabricObject(existingObj, node)
          obj = existingObj
        } else {
          const newObj = createFabricObject(node)
          if (newObj) {
            const shouldAnimate = pendingAnimationNodes.has(node.id)
            if (shouldAnimate) {
              const targetOpacity = newObj.opacity ?? 1
              const delay = getNextStaggerDelay()
              newObj.set({ opacity: 0 })
              canvas.add(newObj)
              // Fire-and-forget: the setTimeout yields to the macrotask queue,
              // so it runs between SSE stream chunks without blocking the stream.
              setTimeout(() => {
                newObj.animate({ opacity: targetOpacity }, {
                  duration: 250,
                  easing: fabric.util.ease.easeOutCubic,
                  onChange: () => canvas.requestRenderAll(),
                  onComplete: () => pendingAnimationNodes.delete(node.id),
                })
              }, delay)
            } else {
              canvas.add(newObj)
            }
            // Restore Fabric selection when an object was recreated
            if (objectRecreated) {
              const { activeId } = useCanvasStore.getState().selection
              if (activeId === node.id) {
                canvas.setActiveObject(newObj)
              }
            }
            obj = newObj
          }
        }

        if (obj) {
          // Component/instance selection border styling
          if (reusableIds.has(node.id)) {
            obj.borderColor = COMPONENT_COLOR
            obj.cornerColor = COMPONENT_COLOR
            obj.borderDashArray = []
          } else if (instanceIds.has(node.id)) {
            obj.borderColor = INSTANCE_COLOR
            obj.cornerColor = INSTANCE_COLOR
            obj.borderDashArray = [4, 4]
          } else if (obj.borderColor === COMPONENT_COLOR || obj.borderColor === INSTANCE_COLOR) {
            obj.borderColor = SELECTION_BLUE
            obj.cornerColor = SELECTION_BLUE
            obj.borderDashArray = []
          }

          // Apply clip path from parent frame with clipContent / cornerRadius
          const clip = clipMap.get(node.id)
          if (clip) {
            obj.clipPath = new fabric.Rect({
              left: clip.x,
              top: clip.y,
              width: clip.w,
              height: clip.h,
              rx: clip.rx,
              ry: clip.rx,
              originX: 'left',
              originY: 'top',
              absolutePositioned: true,
            })
            // Invalidate the object cache so the clipPath takes effect on next render.
            // Without this, canvas.add() caches the object without the clip and
            // requestRenderAll() reuses the stale cache.
            obj.dirty = true
          } else if (obj.clipPath) {
            obj.clipPath = undefined
            obj.dirty = true
          }
        }
      }

      canvas.requestRenderAll()
    })

    // Trigger initial sync for the already-existing document.
    // The subscription only fires on future changes, so force a
    // re-render by creating a new children reference.
    const { document: doc } = useDocumentStore.getState()
    const initActivePageId = useCanvasStore.getState().activePageId
    const initChildren = getActivePageChildren(doc, initActivePageId)
    if (initChildren.length > 0) {
      useDocumentStore.setState({
        document: setActivePageChildren(doc, initActivePageId, [...initChildren]),
      })
    }

    return () => {
      unsub()
      unsubCanvas()
    }
  }, [])
}
