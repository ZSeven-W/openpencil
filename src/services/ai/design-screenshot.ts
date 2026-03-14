/**
 * Screenshot capture utilities for design validation.
 *
 * TODO: Migrate to CanvasKit-based rendering for screenshot capture.
 * The previous Fabric.js implementation has been removed.
 */

/**
 * Capture a screenshot of a specific node and all its descendants.
 * Returns a base64 PNG data URL, or null if the node can't be rendered.
 *
 * Currently returns null — pending CanvasKit migration.
 */
export function captureNodeScreenshot(_nodeId: string): string | null {
  // TODO: implement via CanvasKit/Skia offscreen rendering
  return null
}

/**
 * Capture a screenshot of the entire root frame.
 *
 * Currently returns null — pending CanvasKit migration.
 */
export function captureRootFrameScreenshot(): string | null {
  return null
}
