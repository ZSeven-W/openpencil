import type * as fabric from 'fabric'
import { SNAP_THRESHOLD } from './canvas-constants'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface GuideLine {
  orientation: 'horizontal' | 'vertical'
  /** x for vertical guides, y for horizontal guides (scene coords) */
  position: number
  /** start of line extent (scene coords) */
  start: number
  /** end of line extent (scene coords) */
  end: number
}

interface Edges {
  left: number
  right: number
  top: number
  bottom: number
  centerX: number
  centerY: number
}

interface SnapCandidate {
  position: number
  delta: number
  absDelta: number
}

// ---------------------------------------------------------------------------
// Module-level state — shared between event handler and render hook
// ---------------------------------------------------------------------------

export let activeGuides: GuideLine[] = []

export function clearGuides() {
  activeGuides = []
}

// ---------------------------------------------------------------------------
// Edge extraction
// ---------------------------------------------------------------------------

function getEdges(obj: fabric.FabricObject): Edges {
  const left = obj.left ?? 0
  const top = obj.top ?? 0
  const w = (obj.width ?? 0) * (obj.scaleX ?? 1)
  const h = (obj.height ?? 0) * (obj.scaleY ?? 1)
  return {
    left,
    right: left + w,
    top,
    bottom: top + h,
    centerX: left + w / 2,
    centerY: top + h / 2,
  }
}

// ---------------------------------------------------------------------------
// Main calculation — returns guide lines + snap deltas
// ---------------------------------------------------------------------------

function calculateGuides(
  target: fabric.FabricObject,
  canvas: fabric.Canvas,
  threshold: number,
): { guides: GuideLine[]; deltaX: number; deltaY: number } {
  const guides: GuideLine[] = []
  const te = getEdges(target)

  // Collect other visible objects that aren't the target or part of an ActiveSelection
  const activeSelection = canvas.getActiveObject()
  const selectedSet = new Set<fabric.FabricObject>()
  if (activeSelection?.isType?.('activeSelection')) {
    for (const child of (activeSelection as fabric.ActiveSelection).getObjects()) {
      selectedSet.add(child)
    }
  }
  selectedSet.add(target)

  const others = canvas.getObjects().filter(
    (obj) => !selectedSet.has(obj) && obj.visible !== false,
  )

  if (others.length === 0) return { guides, deltaX: 0, deltaY: 0 }

  let bestX: SnapCandidate | null = null
  let bestY: SnapCandidate | null = null

  // Accumulate guide extents per snap position
  const xExtents = new Map<number, { start: number; end: number }>()
  const yExtents = new Map<number, { start: number; end: number }>()

  for (const obj of others) {
    const oe = getEdges(obj)

    // X-axis alignment: target edges vs. object edges → vertical guide lines
    const xPairs: [number, number][] = [
      [te.left, oe.left],
      [te.left, oe.right],
      [te.left, oe.centerX],
      [te.right, oe.left],
      [te.right, oe.right],
      [te.right, oe.centerX],
      [te.centerX, oe.left],
      [te.centerX, oe.right],
      [te.centerX, oe.centerX],
    ]

    for (const [tX, oX] of xPairs) {
      const delta = oX - tX
      const absDelta = Math.abs(delta)
      if (absDelta >= threshold) continue

      if (!bestX || absDelta < bestX.absDelta) {
        bestX = { position: oX, delta, absDelta }
      }

      // Track the vertical extent of this guide
      const key = Math.round(oX * 10) / 10
      const yMin = Math.min(te.top, oe.top)
      const yMax = Math.max(te.bottom, oe.bottom)
      const existing = xExtents.get(key)
      if (existing) {
        existing.start = Math.min(existing.start, yMin)
        existing.end = Math.max(existing.end, yMax)
      } else {
        xExtents.set(key, { start: yMin, end: yMax })
      }
    }

    // Y-axis alignment: target edges vs. object edges → horizontal guide lines
    const yPairs: [number, number][] = [
      [te.top, oe.top],
      [te.top, oe.bottom],
      [te.top, oe.centerY],
      [te.bottom, oe.top],
      [te.bottom, oe.bottom],
      [te.bottom, oe.centerY],
      [te.centerY, oe.top],
      [te.centerY, oe.bottom],
      [te.centerY, oe.centerY],
    ]

    for (const [tY, oY] of yPairs) {
      const delta = oY - tY
      const absDelta = Math.abs(delta)
      if (absDelta >= threshold) continue

      if (!bestY || absDelta < bestY.absDelta) {
        bestY = { position: oY, delta, absDelta }
      }

      const key = Math.round(oY * 10) / 10
      const xMin = Math.min(te.left, oe.left)
      const xMax = Math.max(te.right, oe.right)
      const existing = yExtents.get(key)
      if (existing) {
        existing.start = Math.min(existing.start, xMin)
        existing.end = Math.max(existing.end, xMax)
      } else {
        yExtents.set(key, { start: xMin, end: xMax })
      }
    }
  }

  // Build result guides from best snap positions only
  if (bestX) {
    const key = Math.round(bestX.position * 10) / 10
    const ext = xExtents.get(key)
    if (ext) {
      guides.push({
        orientation: 'vertical',
        position: bestX.position,
        start: ext.start,
        end: ext.end,
      })
    }
  }

  if (bestY) {
    const key = Math.round(bestY.position * 10) / 10
    const ext = yExtents.get(key)
    if (ext) {
      guides.push({
        orientation: 'horizontal',
        position: bestY.position,
        start: ext.start,
        end: ext.end,
      })
    }
  }

  return {
    guides,
    deltaX: bestX?.delta ?? 0,
    deltaY: bestY?.delta ?? 0,
  }
}

// ---------------------------------------------------------------------------
// Public API — called from use-canvas-events during object:moving
// ---------------------------------------------------------------------------

/** Calculate guides, snap target position, and update activeGuides state. */
export function calculateAndSnap(
  target: fabric.FabricObject,
  canvas: fabric.Canvas,
): void {
  const result = calculateGuides(target, canvas, SNAP_THRESHOLD)

  // Apply snapping
  if (result.deltaX !== 0) {
    target.set('left', (target.left ?? 0) + result.deltaX)
  }
  if (result.deltaY !== 0) {
    target.set('top', (target.top ?? 0) + result.deltaY)
  }
  if (result.deltaX !== 0 || result.deltaY !== 0) {
    target.setCoords()
  }

  activeGuides = result.guides
}
