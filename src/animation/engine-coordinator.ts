/**
 * Engine Coordinator
 *
 * Ensures only ONE animation engine (v1 playback-loop or v2 playback-controller)
 * controls the Fabric canvas at a time. Each engine registers a stop callback;
 * before either engine starts, it asks the coordinator to stop the other.
 */

export type EngineId = 'v1' | 'v2'

interface EngineEntry {
  stop: () => void
  isPlaying: () => boolean
}

const engines = new Map<EngineId, EngineEntry>()
let activeEngine: EngineId | null = null

/**
 * Register an engine's stop/isPlaying callbacks.
 * Called once at module init (v1) or when a controller is created (v2).
 */
export function registerEngine(
  id: EngineId,
  entry: EngineEntry,
): void {
  engines.set(id, entry)
}

/**
 * Unregister an engine (e.g. when v2 controller is disposed).
 */
export function unregisterEngine(id: EngineId): void {
  if (activeEngine === id) activeEngine = null
  engines.delete(id)
}

/**
 * Called by an engine right before it starts playing.
 * Stops the other engine if it is currently active, then marks
 * the requesting engine as the active owner.
 *
 * Returns false if the requesting engine is not registered.
 */
export function requestPlayback(id: EngineId): boolean {
  if (!engines.has(id)) return false

  // Stop any other engine that is currently playing
  for (const [otherId, entry] of engines) {
    if (otherId !== id && entry.isPlaying()) {
      entry.stop()
    }
  }

  activeEngine = id
  return true
}

/**
 * Called when an engine stops or pauses, so the coordinator
 * knows the canvas is free.
 */
export function releasePlayback(id: EngineId): void {
  if (activeEngine === id) activeEngine = null
}

/**
 * Returns which engine currently owns the canvas, or null if idle.
 */
export function getActiveEngine(): EngineId | null {
  return activeEngine
}

/**
 * Returns true if ANY engine is currently playing.
 * Use this instead of checking a single boolean.
 */
export function isAnyEnginePlaying(): boolean {
  for (const entry of engines.values()) {
    if (entry.isPlaying()) return true
  }
  return false
}
