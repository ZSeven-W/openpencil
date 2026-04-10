import type { ViewportState } from '@/types/canvas';
import type { TextEditState } from './skia-interaction';

export interface TextEditOverlayStyle {
  left: number;
  top: number;
  width: number;
  minHeight: number;
  fontSize: number;
}

export function projectTextEditStateToViewport(
  editingText: TextEditState,
  viewport: ViewportState,
): TextEditOverlayStyle {
  return {
    left: editingText.x * viewport.zoom + viewport.panX,
    top: editingText.y * viewport.zoom + viewport.panY,
    width: Math.max(editingText.w * viewport.zoom, 1),
    minHeight: Math.max(editingText.h * viewport.zoom, 1),
    fontSize: editingText.fontSize * viewport.zoom,
  };
}
