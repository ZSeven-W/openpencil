/**
 * Module-level singleton reference to the active SkiaEngine instance.
 * Set by SkiaCanvas on mount, cleared on unmount.
 * Allows external code (keyboard shortcuts, AI orchestrator, etc.) to call
 * engine methods like zoomToFitContent() without prop-drilling.
 */

import type { SkiaEngine } from './skia/skia-engine'

let _engine: SkiaEngine | null = null

export function setSkiaEngineRef(engine: SkiaEngine | null) {
  _engine = engine
}

export function getSkiaEngineRef(): SkiaEngine | null {
  return _engine
}

/**
 * Zoom and pan so all document content fits in the visible canvas area.
 * Delegates to the active SkiaEngine instance.
 */
export function zoomToFitContent() {
  _engine?.zoomToFitContent()
}

/**
 * Returns the canvas element dimensions in CSS pixels.
 * Falls back to 800x600 if no engine is mounted.
 */
export function getCanvasSize(): { width: number; height: number } {
  return _engine?.getCanvasSize() ?? { width: 800, height: 600 }
}

/**
 * No-op — with the Skia engine, document-store is always in sync.
 * Previously needed for Fabric.js where canvas objects held authoritative positions.
 */
export function syncCanvasPositionsToStore() {
  // Skia engine writes positions directly to document-store during interactions.
  // No sync needed before save.
}

/**
 * Flag to skip depth-resolution on the next selection event.
 * Used by layer panel to programmatically select children without
 * auto-resolving them to their parent group.
 */
let _skipNextDepthResolve = false
export function setSkipNextDepthResolve() {
  _skipNextDepthResolve = true
}
export function consumeSkipNextDepthResolve(): boolean {
  const v = _skipNextDepthResolve
  _skipNextDepthResolve = false
  return v
}
