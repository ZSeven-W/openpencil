/** Default maximum undo/redo history states. */
export const DEFAULT_MAX_HISTORY = 300;

/** Rapid pushState calls within this window merge into one undo step. */
export const HISTORY_DEBOUNCE_MS = 300;

/** Default canvas background color (dark). */
export const DEFAULT_BACKGROUND_COLOR = '#1a1a1a';

/** Minimum shape size for draw tool commit (pixels). */
export const MIN_DRAW_SIZE = 2;

/** Minimum line length for draw tool commit (pixels). */
export const MIN_LINE_LENGTH = 2;

/** Drag distance threshold before a click becomes a drag (pixels). */
export const DRAG_THRESHOLD = 3;

/** Hit test handle radius (pixels, screen space). */
export const HANDLE_HIT_RADIUS = 8;

/** Rotation zone outer radius (pixels, screen space). */
export const ROTATE_OUTER_RADIUS = 16;

/** Arc handle hit radius (pixels, screen space). */
export const ARC_HANDLE_HIT_RADIUS = 8;

/** Handle direction cursors. */
export type HandleDir = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w';

export const HANDLE_CURSORS: Record<HandleDir, string> = {
  nw: 'nwse-resize',
  n: 'ns-resize',
  ne: 'nesw-resize',
  e: 'ew-resize',
  se: 'nwse-resize',
  s: 'ns-resize',
  sw: 'nesw-resize',
  w: 'ew-resize',
};
