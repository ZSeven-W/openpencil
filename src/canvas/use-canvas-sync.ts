import { useEffect } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import type { PenNode, ContainerProps } from '@/types/pen'
import {
  createFabricObject,
  type FabricObjectWithPenId,
} from './canvas-object-factory'
import { syncFabricObject } from './canvas-object-sync'
import { isFabricSyncLocked, setFabricSyncLock } from './canvas-sync-lock'

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

// ---------------------------------------------------------------------------
// Layout engine — resolves vertical/horizontal auto-layout to absolute x/y
// ---------------------------------------------------------------------------

interface Padding {
  top: number
  right: number
  bottom: number
  left: number
}

function resolvePadding(
  padding:
    | number
    | [number, number]
    | [number, number, number, number]
    | string
    | undefined,
): Padding {
  if (!padding || typeof padding === 'string')
    return { top: 0, right: 0, bottom: 0, left: 0 }
  if (typeof padding === 'number')
    return { top: padding, right: padding, bottom: padding, left: padding }
  if (padding.length === 2)
    return {
      top: padding[0],
      right: padding[1],
      bottom: padding[0],
      left: padding[1],
    }
  return {
    top: padding[0],
    right: padding[1],
    bottom: padding[2],
    left: padding[3],
  }
}

/** Parse a sizing value. Handles number, "fit_content", "fill_container" and parenthesized forms. */
function parseSizing(value: unknown): number | 'fit' | 'fill' {
  if (typeof value === 'number') return value
  if (typeof value !== 'string') return 0
  if (value.startsWith('fill_container')) return 'fill'
  if (value.startsWith('fit_content')) return 'fit'
  const n = parseFloat(value)
  return isNaN(n) ? 0 : n
}

/** Compute fit-content width from children. */
function fitContentWidth(node: PenNode): number {
  if (!('children' in node) || !node.children?.length) return 0
  const layout = 'layout' in node ? (node as ContainerProps).layout : undefined
  const pad = resolvePadding('padding' in node ? (node as any).padding : undefined)
  const gap = 'gap' in node && typeof (node as any).gap === 'number' ? (node as any).gap : 0
  if (layout === 'horizontal') {
    const childTotal = node.children.reduce((sum, c) => sum + getNodeWidth(c), 0)
    const gapTotal = gap * Math.max(0, node.children.length - 1)
    return childTotal + gapTotal + pad.left + pad.right
  }
  const maxChildW = node.children.reduce((max, c) => Math.max(max, getNodeWidth(c)), 0)
  return maxChildW + pad.left + pad.right
}

/** Compute fit-content height from children. */
function fitContentHeight(node: PenNode): number {
  if (!('children' in node) || !node.children?.length) return 0
  const layout = 'layout' in node ? (node as ContainerProps).layout : undefined
  const pad = resolvePadding('padding' in node ? (node as any).padding : undefined)
  const gap = 'gap' in node && typeof (node as any).gap === 'number' ? (node as any).gap : 0
  if (layout === 'vertical') {
    const childTotal = node.children.reduce((sum, c) => sum + getNodeHeight(c), 0)
    const gapTotal = gap * Math.max(0, node.children.length - 1)
    return childTotal + gapTotal + pad.top + pad.bottom
  }
  const maxChildH = node.children.reduce((max, c) => Math.max(max, getNodeHeight(c)), 0)
  return maxChildH + pad.top + pad.bottom
}

function getNodeWidth(node: PenNode, parentAvail?: number): number {
  if ('width' in node) {
    const s = parseSizing(node.width)
    if (typeof s === 'number' && s > 0) return s
    if (s === 'fill' && parentAvail) return parentAvail
    if (s === 'fit') {
      const fit = fitContentWidth(node)
      if (fit > 0) return fit
    }
  }
  // Containers without explicit width: compute from children
  if ('children' in node && node.children?.length) {
    const fit = fitContentWidth(node)
    if (fit > 0) return fit
  }
  if (node.type === 'text') {
    const fontSize = node.fontSize ?? 16
    const content =
      typeof node.content === 'string'
        ? node.content
        : node.content.map((s) => s.text).join('')
    return Math.max(content.length * fontSize * 0.55, 20)
  }
  return 0
}

function getNodeHeight(node: PenNode, parentAvail?: number): number {
  if ('height' in node) {
    const s = parseSizing(node.height)
    if (typeof s === 'number' && s > 0) return s
    if (s === 'fill' && parentAvail) return parentAvail
    if (s === 'fit') {
      const fit = fitContentHeight(node)
      if (fit > 0) return fit
    }
  }
  // Containers without explicit height: compute from children
  if ('children' in node && node.children?.length) {
    const fit = fitContentHeight(node)
    if (fit > 0) return fit
  }
  if (node.type === 'text') {
    const fontSize = node.fontSize ?? 16
    return fontSize * 1.4
  }
  return 0
}

/** Compute child positions according to the parent's layout rules. */
function computeLayoutPositions(
  parent: PenNode,
  children: PenNode[],
): PenNode[] {
  if (children.length === 0) return children
  const c = parent as PenNode & ContainerProps
  const layout = c.layout
  if (!layout || layout === 'none') return children

  const pW = parseSizing(c.width)
  const pH = parseSizing(c.height)
  const parentW = typeof pW === 'number' ? pW : 100
  const parentH = typeof pH === 'number' ? pH : 100
  const pad = resolvePadding(c.padding)
  const gap = typeof c.gap === 'number' ? c.gap : 0
  const justify = c.justifyContent ?? 'start'
  const align = c.alignItems ?? 'start'

  const isVertical = layout === 'vertical'
  const availW = parentW - pad.left - pad.right
  const availH = parentH - pad.top - pad.bottom
  const availMain = isVertical ? availH : availW
  const totalGapSpace = gap * Math.max(0, children.length - 1)

  // Two-pass sizing: first compute fixed sizes, then allocate remaining space for fill children
  const mainSizing = children.map((ch) => {
    const prop = isVertical ? 'height' : 'width'
    if (prop in ch) {
      const s = parseSizing((ch as any)[prop])
      if (s === 'fill') return 'fill' as const
    }
    return isVertical ? getNodeHeight(ch, availH) : getNodeWidth(ch, availW)
  })
  const fixedTotal = mainSizing.reduce<number>(
    (sum, s) => sum + (typeof s === 'number' ? s : 0),
    0,
  )
  const fillCount = mainSizing.filter((s) => s === 'fill').length
  const remainingMain = Math.max(0, availMain - fixedTotal - totalGapSpace)
  const fillSize = fillCount > 0 ? remainingMain / fillCount : 0

  const sizes = children.map((ch, i) => {
    const mainSize = mainSizing[i] === 'fill' ? fillSize : (mainSizing[i] as number)
    return {
      w: isVertical ? getNodeWidth(ch, availW) : mainSize,
      h: isVertical ? mainSize : getNodeHeight(ch, availH),
    }
  })

  const totalMain = sizes.reduce(
    (sum, s) => sum + (isVertical ? s.h : s.w),
    0,
  )
  const freeSpace = Math.max(0, availMain - totalMain - totalGapSpace)

  let mainPos = 0
  let effectiveGap = gap

  switch (justify) {
    case 'center':
      mainPos = freeSpace / 2
      break
    case 'end':
      mainPos = freeSpace
      break
    case 'space_between':
      effectiveGap =
        children.length > 1
          ? (availMain - totalMain) / (children.length - 1)
          : 0
      break
    case 'space_around': {
      const spacing =
        children.length > 0
          ? (availMain - totalMain) / children.length
          : 0
      mainPos = spacing / 2
      effectiveGap = spacing
      break
    }
    default:
      // 'start' — mainPos stays 0
      break
  }

  return children.map((child, i) => {
    const size = sizes[i]
    const crossAvail = isVertical ? availW : availH
    const childCross = isVertical ? size.w : size.h
    let crossPos = 0

    switch (align) {
      case 'center':
        crossPos = (crossAvail - childCross) / 2
        break
      case 'end':
        crossPos = crossAvail - childCross
        break
      default:
        break
    }

    const x = isVertical ? pad.left + crossPos : pad.left + mainPos
    const y = isVertical ? pad.top + mainPos : pad.top + crossPos

    mainPos += (isVertical ? size.h : size.w) + effectiveGap

    // Always propagate resolved pixel dimensions so nested layout
    // computations and Fabric.js objects get correct numeric sizes.
    const out: Record<string, unknown> = { ...child, x, y, width: size.w, height: size.h }
    return out as unknown as PenNode
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
): PenNode[] {
  const result: PenNode[] = []
  for (const node of nodes) {
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
          r.height = getNodeHeight(node, parentAvailH)
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

      // Compute clip context for children: if this frame has cornerRadius, clip children to it
      let childClip = clipCtx
      const cr = 'cornerRadius' in node ? cornerRadiusVal(node.cornerRadius) : 0
      if (cr > 0) {
        childClip = { x: parentAbsX, y: parentAbsY, w: nodeW, h: nodeH, rx: cr }
      }

      // Children inside layout containers are layout-controlled (position not manually editable)
      const childIsLayoutChild = !!(layout && layout !== 'none')

      result.push(
        ...flattenNodes(positioned, parentAbsX, parentAbsY, childAvailW, childAvailH, childClip, clipMap, childIsLayoutChild),
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
  nodeRenderInfo.clear()
  flattenNodes(state.document.children, 0, 0, undefined, undefined, undefined, new Map())
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
    let prevChildren = useDocumentStore.getState().document.children

    const unsub = useDocumentStore.subscribe((state) => {
      // Always track the latest children reference — even when the sync lock
      // is active — so that unrelated store updates (e.g. markClean setting
      // isDirty) don't trigger a stale re-sync that overwrites canvas state.
      const childrenChanged = state.document.children !== prevChildren
      prevChildren = state.document.children

      if (isFabricSyncLocked()) return

      // Skip re-sync when only non-document fields changed (isDirty, fileName, etc.)
      if (!childrenChanged) return

      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return

      const clipMap = new Map<string, ClipInfo>()
      nodeRenderInfo.clear()
      const flatNodes = flattenNodes(
        state.document.children, 0, 0, undefined, undefined, undefined, clipMap,
      )
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

        let obj: FabricObjectWithPenId | undefined
        const existingObj = objMap.get(node.id)
        if (existingObj) {
          syncFabricObject(existingObj, node)
          obj = existingObj
        } else {
          const newObj = createFabricObject(node)
          if (newObj) {
            canvas.add(newObj)
            obj = newObj
          }
        }

        // Apply clip path from parent frame with cornerRadius
        if (obj) {
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
          } else if (obj.clipPath) {
            obj.clipPath = undefined
          }
        }
      }

      canvas.requestRenderAll()
    })

    // Trigger initial sync for the already-existing document.
    // The subscription only fires on future changes, so force a
    // re-render by creating a new children reference.
    const { document: doc } = useDocumentStore.getState()
    if (doc.children.length > 0) {
      useDocumentStore.setState({
        document: { ...doc, children: [...doc.children] },
      })
    }

    return () => unsub()
  }, [])
}
