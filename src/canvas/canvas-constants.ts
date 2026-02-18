export const MIN_ZOOM = 0.02
export const MAX_ZOOM = 256
export const ZOOM_STEP = 0.1
export const SNAP_THRESHOLD = 5
export const DEFAULT_FILL = '#d1d5db'
export const DEFAULT_STROKE = '#374151'
export const DEFAULT_STROKE_WIDTH = 1
export const CANVAS_BACKGROUND_LIGHT = '#e5e5e5'
export const CANVAS_BACKGROUND_DARK = '#1a1a1a'

export function getCanvasBackground(): string {
  if (typeof document === 'undefined') return CANVAS_BACKGROUND_DARK
  return document.documentElement.classList.contains('light')
    ? CANVAS_BACKGROUND_LIGHT
    : CANVAS_BACKGROUND_DARK
}
export const SELECTION_BLUE = '#0d99ff'

// Smart guides
export const GUIDE_COLOR = '#FF6B35'
export const GUIDE_LINE_WIDTH = 1
export const GUIDE_DASH = [3, 3]
