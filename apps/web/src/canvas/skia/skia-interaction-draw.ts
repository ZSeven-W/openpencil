import type { SkiaEngine } from './skia-engine';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { createNodeForTool } from '../canvas-node-creator';
import type { ToolType } from '@/types/canvas';

/**
 * Handles rubber-band drawing interactions for shape tools
 * (rectangle, ellipse, frame, line, polygon).
 */
export class DrawHandler {
  isDrawing = false;
  private drawTool: ToolType = 'select';
  private drawStartX = 0;
  private drawStartY = 0;

  startDrawing(tool: ToolType, scene: { x: number; y: number }, engine: SkiaEngine): void {
    this.isDrawing = true;
    this.drawTool = tool;
    this.drawStartX = scene.x;
    this.drawStartY = scene.y;
    engine.previewShape = {
      type: tool as 'rectangle' | 'ellipse' | 'frame' | 'line' | 'polygon',
      x: scene.x,
      y: scene.y,
      w: 0,
      h: 0,
    };
    engine.markDirty();
  }

  handleDrawingMove(scene: { x: number; y: number }, engine: SkiaEngine): void {
    const dx = scene.x - this.drawStartX;
    const dy = scene.y - this.drawStartY;

    if (this.drawTool === 'line') {
      engine.previewShape = {
        type: 'line',
        x: this.drawStartX,
        y: this.drawStartY,
        w: dx,
        h: dy,
      };
    } else {
      engine.previewShape = {
        type: this.drawTool as 'rectangle' | 'ellipse' | 'frame' | 'line' | 'polygon',
        x: dx < 0 ? scene.x : this.drawStartX,
        y: dy < 0 ? scene.y : this.drawStartY,
        w: Math.abs(dx),
        h: Math.abs(dy),
      };
    }
    engine.markDirty();
  }

  /**
   * Finish drawing: commit the shape to the document if it meets minimum size.
   * Returns true if a shape was committed.
   */
  finishDrawing(engine: SkiaEngine): boolean {
    if (!this.isDrawing || !engine.previewShape) {
      this.isDrawing = false;
      return false;
    }

    const { type, x, y, w, h } = engine.previewShape;
    engine.previewShape = null;
    engine.markDirty();
    this.isDrawing = false;

    const minSize = type === 'line' ? Math.hypot(w, h) >= 2 : w >= 2 && h >= 2;
    if (minSize) {
      const node = createNodeForTool(this.drawTool, x, y, w, h);
      if (node) {
        useDocumentStore.getState().addNode(null, node);
        useCanvasStore.getState().setSelection([node.id], node.id);
      }
    }
    useCanvasStore.getState().setActiveTool('select');
    return true;
  }
}
