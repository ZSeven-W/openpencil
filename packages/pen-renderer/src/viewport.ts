import { MIN_ZOOM, MAX_ZOOM } from '@zseven-w/pen-core';
import type { ViewportState } from './types.js';

export type { ViewportState } from './types.js';

/**
 * Compute
 * 来自视口状态的 CanvasKit 的 3x3 变换矩阵。 CanvasKit 使用列主
 [scaleX、skewX、transX、skewY、scaleY、transY、pers0、pers1、pers2]
 */
export function viewportMatrix(vp: ViewportState): number[] {
  return [vp.zoom, 0, vp.panX, 0, vp.zoom, vp.panY, 0, 0, 1];
}

/**
 * Convert 屏幕（客户端）坐标到场景坐标。
 */
export function screenToScene(
  clientX: number,
  clientY: number,
  canvasRect: DOMRect,
  vp: ViewportState,
): { x: number; y: number } {
  const sx = clientX - canvasRect.left;
  const sy = clientY - canvasRect.top;
  return {
    x: (sx - vp.panX) / vp.zoom,
    y: (sy - vp.panY) / vp.zoom,
  };
}

/**
 * Convert 场景坐标到屏幕坐标。
 */
export function sceneToScreen(
  sceneX: number,
  sceneY: number,
  canvasRect: DOMRect,
  vp: ViewportState,
): { x: number; y: number } {
  return {
    x: sceneX * vp.zoom + vp.panX + canvasRect.left,
    y: sceneY * vp.zoom + vp.panY + canvasRect.top,
  };
}

/**
 * Zoom 朝向一点（在屏幕坐标中）。
 */
export function zoomToPoint(
  vp: ViewportState,
  screenX: number,
  screenY: number,
  canvasRect: DOMRect,
  newZoom: number,
): ViewportState {
  const clampedZoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, newZoom));
  const sx = screenX - canvasRect.left;
  const sy = screenY - canvasRect.top;

  // The 光标下的场景点应保持固定
  const sceneX = (sx - vp.panX) / vp.zoom;
  const sceneY = (sy - vp.panY) / vp.zoom;

  return {
    zoom: clampedZoom,
    panX: sx - sceneX * clampedZoom,
    panY: sy - sceneY * clampedZoom,
  };
}

/**
 * Get 场景坐标中的视口边界。
 */
export function getViewportBounds(
  vp: ViewportState,
  canvasWidth: number,
  canvasHeight: number,
  margin = 0,
) {
  return {
    left: -vp.panX / vp.zoom - margin,
    top: -vp.panY / vp.zoom - margin,
    right: (-vp.panX + canvasWidth) / vp.zoom + margin,
    bottom: (-vp.panY + canvasHeight) / vp.zoom + margin,
  };
}

/**
 * Check 如果矩形位于视口边界内。
 */
export function isRectInViewport(
  rect: { x: number; y: number; w: number; h: number },
  vpBounds: ReturnType<typeof getViewportBounds>,
): boolean {
  return !(
    rect.x + rect.w < vpBounds.left ||
    rect.x > vpBounds.right ||
    rect.y + rect.h < vpBounds.top ||
    rect.y > vpBounds.bottom
  );
}
