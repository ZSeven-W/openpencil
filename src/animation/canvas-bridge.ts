import type { Canvas, FabricObject } from 'fabric'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import type { AnimatableValue } from '@/types/animation'
import {
  getCanvasBinding,
  getAllCanvasBindings,
} from '@/animation/canvas-property-bindings'
import { isAnyEnginePlaying } from '@/animation/engine-coordinator'

// --- Playback state ---

/**
 * Returns true if the animation engine is currently playing.
 */
export function isPlaybackActive(): boolean {
  return isAnyEnginePlaying()
}

// --- Cursor guard (counter-based) ---
// Prevents library cursor drag callbacks from echoing back playback tick values.
// Uses a monotonic counter so multiple consumers can independently detect
// engine-driven cursor updates without destructive reads.

let cursorUpdateCount = 0

export function markCursorUpdate(): void {
  cursorUpdateCount++
}

/** Returns the current cursor update count. Non-destructive read. */
export function getCursorUpdateCount(): number {
  return cursorUpdateCount
}

// --- Object Lookup (shared Map cache) ---

const fabricObjectMap = new Map<string, FabricObjectWithPenId>()

/** Build Map cache from canvas objects. Call at playback start. */
export function buildFabricObjectMap(canvas: Canvas): void {
  fabricObjectMap.clear()
  const objects = canvas.getObjects() as FabricObjectWithPenId[]
  for (const obj of objects) {
    if (obj.penNodeId) {
      fabricObjectMap.set(obj.penNodeId, obj)
    }
  }
}

/** Clear Map cache. Call at playback stop. */
export function clearFabricObjectMap(): void {
  fabricObjectMap.clear()
}

/** Shared lookup — used by canvas-bridge and video-sync. */
export function findFabricObject(
  canvas: Canvas,
  nodeId: string,
): FabricObjectWithPenId | null {
  // Use cache during playback, linear scan otherwise
  if (fabricObjectMap.size > 0) {
    return fabricObjectMap.get(nodeId) ?? null
  }
  const objects = canvas.getObjects() as FabricObjectWithPenId[]
  return objects.find((obj) => obj.penNodeId === nodeId) ?? null
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

// --- v2: Registry-driven property application ---

/**
 * Apply animated values to a Fabric object using the canvas property registry.
 * Sets obj.dirty only when cache-invalidating properties were touched.
 * Does NOT call setCoords() — call recalcCoordsForAnimatedObjects() on stop.
 */
export function applyAnimatedFrame(
  obj: FabricObject,
  values: Record<string, AnimatableValue>,
): void {
  let needsCacheInvalidation = false
  for (const key in values) {
    const binding = getCanvasBinding(key)
    if (!binding) continue
    binding.apply(obj, values[key])
    if (binding.requiresCacheInvalidation) needsCacheInvalidation = true
  }
  if (needsCacheInvalidation) {
    obj.dirty = true
  }
}

// --- v2: Registry-driven state capture ---

/** Capture current values for all registered properties from a Fabric object. */
export function captureNodeState(
  obj: FabricObject,
): Record<string, AnimatableValue> {
  const state: Record<string, AnimatableValue> = {}
  for (const binding of getAllCanvasBindings()) {
    state[binding.key] = binding.capture(obj) as AnimatableValue
  }
  return state
}

// --- v2: Coordinate recalculation on playback stop ---

/** Call setCoords() on all cached objects. Use after playback stops. */
export function recalcCoordsForAnimatedObjects(): void {
  for (const obj of fabricObjectMap.values()) {
    obj.setCoords()
  }
}

// --- v2: Restore saved states ---

/** Restore saved states on playback stop. Calls setCoords per object. */
export function restoreNodeStates(
  canvas: Canvas,
  savedStates: Map<string, Record<string, AnimatableValue>>,
): void {
  for (const [nodeId, values] of savedStates) {
    const obj = fabricObjectMap.get(nodeId) ?? findFabricObject(canvas, nodeId)
    if (!obj) continue
    applyAnimatedFrame(obj as FabricObject, values)
    obj.setCoords()
  }
}