/**
 * Engine Coordinator
 *
 * Manages the v2 playback-controller's ownership of the Fabric canvas.
 * Provides a registry pattern so canvas-bridge can check if playback is active.
 */

interface EngineEntry {
  stop: () => void
  isPlaying: () => boolean
}

let engine: EngineEntry | null = null

/**
 * Register the v2 engine's stop/isPlaying callbacks.
 * Called when a controller is created.
 */
export function registerEngine(
  _id: string,
  entry: EngineEntry,
): void {
  engine = entry
}

/**
 * Unregister the engine (e.g. when v2 controller is disposed).
 */
export function unregisterEngine(_id: string): void {
  engine = null
}

/**
 * Called by the engine right before it starts playing.
 * Returns false if the engine is not registered.
 */
export function requestPlayback(_id: string): boolean {
  if (!engine) return false
  return true
}

/**
 * Called when the engine stops or pauses, so the coordinator
 * knows the canvas is free.
 */
export function releasePlayback(_id: string): void {
  // No-op — isAnyEnginePlaying() delegates to the engine's isPlaying()
}

/**
 * Returns true if the engine is currently playing.
 */
export function isAnyEnginePlaying(): boolean {
  return engine?.isPlaying() ?? false
}
