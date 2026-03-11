import type { Canvas, FabricObject } from 'fabric'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import type { AnimatableProperties, AnimatableValue } from '@/types/animation'
import {
  getCanvasBinding,
  getAllCanvasBindings,
} from '@/animation/canvas-property-bindings'

// --- Playback state ---

let playbackActive = false

export function isPlaybackActive(): boolean {
  return playbackActive
}

export function setPlaybackActive(active: boolean): void {
  playbackActive = active
}

// --- Cursor guard (boolean flag) ---
// Prevents library cursor drag callbacks from echoing back playback tick values.
// A boolean flag is deterministic; a timing heuristic would fail on slow machines.

let cursorSetByEngine = false

export function markCursorUpdate(): void {
  cursorSetByEngine = true
}

/** Returns true (and resets) if the engine just set the cursor. */
export function consumeCursorGuard(): boolean {
  const was = cursorSetByEngine
  cursorSetByEngine = false
  return was
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

// --- v1 backward compat: Apply Animation Properties ---

/**
 * Directly mutate Fabric.js object properties for animation.
 * Uses direct assignment (not obj.set()) to avoid triggering Fabric events.
 * @deprecated Use applyAnimatedFrame for v2 registry-driven application.
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

// --- v1 backward compat: Capture Current State ---

/** @deprecated Use captureNodeState for v2 registry-driven capture. */
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
  for (const [key, value] of Object.entries(values)) {
    const binding = getCanvasBinding(key)
    if (!binding) continue
    binding.apply(obj, value)
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

// --- v2: Pending asset swap queue ---

interface PendingSwap {
  nodeId: string
  element: HTMLElement
}

const pendingSwaps: PendingSwap[] = []

/** Queue an asset swap during playback, or apply immediately when idle. */
export function queueAssetSwap(nodeId: string, element: HTMLElement): void {
  if (playbackActive) {
    pendingSwaps.push({ nodeId, element })
    return
  }
  // Apply immediately if not playing — no-op for now,
  // concrete swap logic will be added when asset types are implemented
}

/** Process and clear all pending asset swaps. Call after playback stops. */
export function flushPendingSwaps(): void {
  // Process swaps — concrete logic will be added with asset types
  pendingSwaps.length = 0
}

/** Visible for testing only. */
export function _getPendingSwapsForTest(): PendingSwap[] {
  return pendingSwaps
}
