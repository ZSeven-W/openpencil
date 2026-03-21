import { MIN_ZOOM, MAX_ZOOM } from '../canvas-constants'

export interface ViewportState {
  zoom: number
  panX: number
  panY: number
}

/**
 * Compute the 3x3 transform matrix for CanvasKit from viewport state.
 * CanvasKit uses column-major [scaleX, skewX, transX, skewY, scaleY, transY, pers0, pers1, pers2]
 */
export function viewportMatrix(vp: ViewportState): number[] {
  return [
    vp.zoom, 0, vp.panX,
    0, vp.zoom, vp.panY,
    0, 0, 1,
  ]
}

/**
 * Convert screen (client) coordinates to scene coordinates.
 */
export function screenToScene(
  clientX: number, clientY: number,
  canvasRect: DOMRect,
  vp: ViewportState,
): { x: number; y: number } {
  const sx = clientX - canvasRect.left
  const sy = clientY - canvasRect.top
  return {
    x: (sx - vp.panX) / vp.zoom,
    y: (sy - vp.panY) / vp.zoom,
  }
}

/**
 * Convert scene coordinates to screen coordinates.
 */
export function sceneToScreen(
  sceneX: number, sceneY: number,
  canvasRect: DOMRect,
  vp: ViewportState,
): { x: number; y: number } {
  return {
    x: sceneX * vp.zoom + vp.panX + canvasRect.left,
    y: sceneY * vp.zoom + vp.panY + canvasRect.top,
  }
}

/**
 * Zoom towards a point (in screen coordinates).
 */
export function zoomToPoint(
  vp: ViewportState,
  screenX: number, screenY: number,
  canvasRect: DOMRect,
  newZoom: number,
): ViewportState {
  const clampedZoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, newZoom))
  const sx = screenX - canvasRect.left
  const sy = screenY - canvasRect.top

  // The scene point under the cursor should stay fixed
  const sceneX = (sx - vp.panX) / vp.zoom
  const sceneY = (sy - vp.panY) / vp.zoom

  return {
    zoom: clampedZoom,
    panX: sx - sceneX * clampedZoom,
    panY: sy - sceneY * clampedZoom,
  }
}

/**
 * Get viewport bounds in scene coordinates.
 */
export function getViewportBounds(
  vp: ViewportState,
  canvasWidth: number,
  canvasHeight: number,
  margin = 0,
) {
  return {
    left: (-vp.panX) / vp.zoom - margin,
    top: (-vp.panY) / vp.zoom - margin,
    right: (-vp.panX + canvasWidth) / vp.zoom + margin,
    bottom: (-vp.panY + canvasHeight) / vp.zoom + margin,
  }
}

/**
 * Check if a rect is within the viewport bounds.
 */
export function isRectInViewport(
  rect: { x: number; y: number; w: number; h: number },
  vpBounds: ReturnType<typeof getViewportBounds>,
): boolean {
  return !(
    rect.x + rect.w < vpBounds.left
    || rect.x > vpBounds.right
    || rect.y + rect.h < vpBounds.top
    || rect.y > vpBounds.bottom
  )
}
