/** Default 最大 undo/redo 历史状态。 */
export const DEFAULT_MAX_HISTORY = 300;

/** Rapid pushState 此窗口内的调用合并为一个撤消步骤。 */
export const HISTORY_DEBOUNCE_MS = 300;

/** Default 画布背景颜色（深色）。 */
export const DEFAULT_BACKGROUND_COLOR = '#1a1a1a';

/** Minimum 绘制工具提交的形状大小（像素）。 */
export const MIN_DRAW_SIZE = 2;

/** Minimum 绘制工具提交的行长度（像素）。 */
export const MIN_LINE_LENGTH = 2;

/** Drag 单击变为拖动之前的距离阈值（像素）。 */
export const DRAG_THRESHOLD = 3;

/** Hit 测试手柄半径（像素、屏幕空间）。 */
export const HANDLE_HIT_RADIUS = 8;

/** Rotation 区域外半径（像素、屏幕空间）。 */
export const ROTATE_OUTER_RADIUS = 16;

/** Arc 手柄命中半径（像素，屏幕空间）。 */
export const ARC_HANDLE_HIT_RADIUS = 8;

/** Handle 方向光标。 */
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
