import type { RenderNode, ViewportState } from '@zseven-w/pen-renderer';

import { MAX_ZOOM, MIN_ZOOM } from '../canvas-constants';

export interface SceneBounds {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

interface FitOptions {
  padding?: number;
  maxZoom?: number;
}

export function getFocusBounds(
  renderNodes: RenderNode[],
  selectedIds: Iterable<string>,
): SceneBounds | null {
  if (renderNodes.length === 0) return null;

  const selectedSet = new Set(selectedIds);
  const selectedNodes = renderNodes.filter((rn) => selectedSet.has(rn.node.id));
  const targetNodes =
    selectedNodes.length > 0 ? selectedNodes : renderNodes.filter((rn) => !rn.clipRect);

  if (targetNodes.length === 0) return null;

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const rn of targetNodes) {
    minX = Math.min(minX, rn.absX);
    minY = Math.min(minY, rn.absY);
    maxX = Math.max(maxX, rn.absX + rn.absW);
    maxY = Math.max(maxY, rn.absY + rn.absH);
  }

  if (
    !Number.isFinite(minX) ||
    !Number.isFinite(minY) ||
    !Number.isFinite(maxX) ||
    !Number.isFinite(maxY)
  ) {
    return null;
  }

  return { minX, minY, maxX, maxY };
}

export function fitSceneBoundsToViewport(
  bounds: SceneBounds,
  canvasWidth: number,
  canvasHeight: number,
  options: FitOptions = {},
): ViewportState | null {
  if (canvasWidth <= 0 || canvasHeight <= 0) return null;

  const padding = options.padding ?? 64;
  const fitWidth = Math.max(canvasWidth - padding * 2, 1);
  const fitHeight = Math.max(canvasHeight - padding * 2, 1);
  const boundsWidth = Math.max(bounds.maxX - bounds.minX, 1);
  const boundsHeight = Math.max(bounds.maxY - bounds.minY, 1);

  let zoom = Math.min(fitWidth / boundsWidth, fitHeight / boundsHeight);
  if (typeof options.maxZoom === 'number') {
    zoom = Math.min(zoom, options.maxZoom);
  }
  zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, zoom));

  const centerX = (bounds.minX + bounds.maxX) / 2;
  const centerY = (bounds.minY + bounds.maxY) / 2;

  return {
    zoom,
    panX: canvasWidth / 2 - centerX * zoom,
    panY: canvasHeight / 2 - centerY * zoom,
  };
}
