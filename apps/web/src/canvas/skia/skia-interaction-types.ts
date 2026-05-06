import type { SkiaEngine } from './skia-engine';
import { screenToScene } from './skia-engine';
import type { ToolType } from '@/types/canvas';

export interface TextEditState {
  nodeId: string;
  x: number;
  y: number;
  w: number;
  h: number;
  content: string;
  fontSize: number;
  fontFamily: string;
  fontWeight: string;
  textAlign: string;
  color: string;
  lineHeight: number;
}

export function toolToCursor(tool: ToolType | string): string {
  switch (tool) {
    case 'hand':
      return 'grab';
    case 'text':
      return 'text';
    case 'select':
      return 'default';
    default:
      return 'crosshair';
  }
}

/**
 * 每个交互处理程序接收的
 * Shared 上下文。 Provides 访问引擎、坐标转换和画布元素。
 */
export interface InteractionContext {
  getEngine(): SkiaEngine | null;
  getScene(e: MouseEvent): { x: number; y: number } | null;
  readonly canvasEl: HTMLCanvasElement;
}

export function createInteractionContext(
  engineRef: { current: SkiaEngine | null },
  canvasEl: HTMLCanvasElement,
): InteractionContext {
  return {
    getEngine() {
      return engineRef.current;
    },
    getScene(e: MouseEvent) {
      const engine = engineRef.current;
      if (!engine) return null;
      const rect = engine.getCanvasRect();
      if (!rect) return null;
      return screenToScene(e.clientX, e.clientY, rect, {
        zoom: engine.zoom,
        panX: engine.panX,
        panY: engine.panY,
      });
    },
    canvasEl,
  };
}
