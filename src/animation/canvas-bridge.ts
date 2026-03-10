import type { Canvas } from 'fabric'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import type { AnimatableProperties } from '@/types/animation'

let playbackActive = false

export function isPlaybackActive(): boolean {
  return playbackActive
}

export function setPlaybackActive(active: boolean): void {
  playbackActive = active
}

// --- Object Lookup ---

function findFabricObject(
  canvas: Canvas,
  nodeId: string,
): FabricObjectWithPenId | null {
  const objects = canvas.getObjects() as FabricObjectWithPenId[]
  return objects.find((obj) => obj.penNodeId === nodeId) ?? null
}

// --- Apply Animation Properties ---

/**
 * Directly mutate Fabric.js object properties for animation.
 * Uses direct assignment (not obj.set()) to avoid triggering Fabric events.
 */
export function applyAnimatedProperties(
  canvas: Canvas,
  nodeId: string,
  properties: Partial<AnimatableProperties>,
): void {
  const obj = findFabricObject(canvas, nodeId)
  if (!obj) return

  if (properties.x !== undefined) obj.left = properties.x
  if (properties.y !== undefined) obj.top = properties.y
  if (properties.scaleX !== undefined) obj.scaleX = properties.scaleX
  if (properties.scaleY !== undefined) obj.scaleY = properties.scaleY
  if (properties.rotation !== undefined) obj.angle = properties.rotation
  if (properties.opacity !== undefined) obj.opacity = properties.opacity
}

// --- Capture Current State ---

export function captureCurrentState(
  canvas: Canvas,
  nodeId: string,
): AnimatableProperties | null {
  const obj = findFabricObject(canvas, nodeId)
  if (!obj) return null

  return {
    x: obj.left ?? 0,
    y: obj.top ?? 0,
    scaleX: obj.scaleX ?? 1,
    scaleY: obj.scaleY ?? 1,
    rotation: obj.angle ?? 0,
    opacity: obj.opacity ?? 1,
  }
}

// --- Object Interaction Lock ---

export function lockObjectInteraction(
  canvas: Canvas,
  nodeId: string,
): void {
  const obj = findFabricObject(canvas, nodeId)
  if (!obj) return
  obj.selectable = false
  obj.evented = false
}

export function unlockObjectInteraction(
  canvas: Canvas,
  nodeId: string,
): void {
  const obj = findFabricObject(canvas, nodeId)
  if (!obj) return
  obj.selectable = true
  obj.evented = true
}
