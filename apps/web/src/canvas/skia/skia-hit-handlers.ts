import type { SkiaEngine } from './skia-engine'
import { useCanvasStore } from '@/stores/canvas-store'
import type { EllipseNode } from '@/types/pen'
import { computeArcHandles } from './skia-overlays'

type HandleDir = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w'
type ArcHandleType = 'start' | 'end' | 'inner'

export type { HandleDir, ArcHandleType }

export const HANDLE_HIT_RADIUS = 8
export const ROTATE_OUTER_RADIUS = 16
export const ARC_HANDLE_HIT_RADIUS = 8
export const DRAG_THRESHOLD = 3

export const handleCursors: Record<HandleDir, string> = {
  nw: 'nwse-resize', n: 'ns-resize', ne: 'nesw-resize', e: 'ew-resize',
  se: 'nwse-resize', s: 'ns-resize', sw: 'nesw-resize', w: 'ew-resize',
}

function getSelectedRN(engine: SkiaEngine) {
  const { selectedIds } = useCanvasStore.getState().selection
  if (selectedIds.length !== 1) return null
  return engine.spatialIndex.get(selectedIds[0]) ?? null
}

export function hitTestHandle(
  engine: SkiaEngine,
  sceneX: number,
  sceneY: number,
): { dir: HandleDir; nodeId: string } | null {
  const rn = getSelectedRN(engine)
  if (!rn) return null

  const hitR = HANDLE_HIT_RADIUS / engine.zoom
  const { absX: x, absY: y, absW: w, absH: h } = rn
  const handles: [HandleDir, number, number][] = [
    ['nw', x, y], ['n', x + w / 2, y], ['ne', x + w, y],
    ['w', x, y + h / 2], ['e', x + w, y + h / 2],
    ['sw', x, y + h], ['s', x + w / 2, y + h], ['se', x + w, y + h],
  ]
  for (const [dir, hx, hy] of handles) {
    if (Math.abs(sceneX - hx) <= hitR && Math.abs(sceneY - hy) <= hitR) {
      return { dir, nodeId: rn.node.id }
    }
  }
  return null
}

export function hitTestRotation(
  engine: SkiaEngine,
  sceneX: number,
  sceneY: number,
): { nodeId: string } | null {
  const rn = getSelectedRN(engine)
  if (!rn) return null

  const innerR = HANDLE_HIT_RADIUS / engine.zoom
  const outerR = ROTATE_OUTER_RADIUS / engine.zoom
  const { absX: x, absY: y, absW: w, absH: h } = rn
  const corners = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]]
  for (const [cx, cy] of corners) {
    const dist = Math.hypot(sceneX - cx, sceneY - cy)
    if (dist > innerR && dist <= outerR) {
      return { nodeId: rn.node.id }
    }
  }
  return null
}

export function hitTestArcHandle(
  engine: SkiaEngine,
  sceneX: number,
  sceneY: number,
): { type: ArcHandleType; nodeId: string } | null {
  const { selectedIds } = useCanvasStore.getState().selection
  if (selectedIds.length !== 1) return null
  const rn = engine.spatialIndex.get(selectedIds[0])
  if (!rn || rn.node.type !== 'ellipse') return null
  const eNode = rn.node as EllipseNode
  const handles = computeArcHandles(
    rn.absX, rn.absY, rn.absW, rn.absH,
    eNode.startAngle ?? 0, eNode.sweepAngle ?? 360, eNode.innerRadius ?? 0,
  )
  const hitR = ARC_HANDLE_HIT_RADIUS / engine.zoom
  for (const key of ['start', 'end', 'inner'] as ArcHandleType[]) {
    const h = handles[key]
    if (Math.hypot(sceneX - h.x, sceneY - h.y) <= hitR) {
      return { type: key, nodeId: rn.node.id }
    }
  }
  return null
}
