const DOM_DELTA_LINE = 1;
const DOM_DELTA_PAGE = 2;
const WHEEL_LINE_PX = 40;
const MODIFIED_ZOOM_GAIN = 4;
const MAX_ZOOM_DELTA = 175;

export interface ViewerWheelInput {
  deltaX: number;
  deltaY: number;
  zoom: boolean;
}

export function normalizeViewerWheelDelta(
  delta: number,
  deltaMode: number,
  pageExtent: number,
): number {
  if (!Number.isFinite(delta)) return 0;
  const multiplier =
    deltaMode === DOM_DELTA_LINE
      ? WHEEL_LINE_PX
      : deltaMode === DOM_DELTA_PAGE && Number.isFinite(pageExtent) && pageExtent > 0
        ? pageExtent
        : 1;
  const normalized = delta * multiplier;
  return Number.isFinite(normalized) ? normalized : 0;
}

export function viewerWheelInput(
  event: Pick<WheelEvent, 'deltaX' | 'deltaY' | 'deltaMode' | 'ctrlKey' | 'metaKey' | 'altKey'>,
  viewportWidth: number,
  viewportHeight: number,
): ViewerWheelInput {
  const zoom = event.ctrlKey || event.metaKey || event.altKey;
  const deltaX = normalizeViewerWheelDelta(event.deltaX, event.deltaMode, viewportWidth);
  const normalizedY = normalizeViewerWheelDelta(event.deltaY, event.deltaMode, viewportHeight);
  return {
    deltaX,
    deltaY: zoom
      ? Math.max(-MAX_ZOOM_DELTA, Math.min(MAX_ZOOM_DELTA, normalizedY * MODIFIED_ZOOM_GAIN))
      : normalizedY,
    zoom,
  };
}
