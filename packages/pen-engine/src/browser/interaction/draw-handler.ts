import type { DesignEngine } from '../../core/design-engine.js';
import type { ToolType } from '@zseven-w/pen-types';
import { createNodeForTool } from '../../core/node-creator.js';
import { MIN_DRAW_SIZE, MIN_LINE_LENGTH } from '../../core/constants.js';

/**
 * Handles rubber-band drawing interactions for shape tools.
 * Engine-native: no DOM store references.
 */
export class EngineDrawHandler {
  isDrawing = false;
  private drawTool: ToolType = 'select';
  private drawStartX = 0;
  private drawStartY = 0;
  previewShape: { type: string; x: number; y: number; w: number; h: number } | null = null;

  startDrawing(tool: ToolType, scene: { x: number; y: number }): void {
    this.isDrawing = true;
    this.drawTool = tool;
    this.drawStartX = scene.x;
    this.drawStartY = scene.y;
    this.previewShape = {
      type: tool,
      x: scene.x,
      y: scene.y,
      w: 0,
      h: 0,
    };
  }

  handleDrawingMove(scene: { x: number; y: number }): void {
    const dx = scene.x - this.drawStartX;
    const dy = scene.y - this.drawStartY;
    if (this.drawTool === 'line') {
      this.previewShape = { type: 'line', x: this.drawStartX, y: this.drawStartY, w: dx, h: dy };
    } else {
      this.previewShape = {
        type: this.drawTool,
        x: dx < 0 ? scene.x : this.drawStartX,
        y: dy < 0 ? scene.y : this.drawStartY,
        w: Math.abs(dx),
        h: Math.abs(dy),
      };
    }
  }

  finishDrawing(engine: DesignEngine): boolean {
    if (!this.isDrawing || !this.previewShape) {
      this.isDrawing = false;
      return false;
    }
    const { type, x, y, w, h } = this.previewShape;
    this.previewShape = null;
    this.isDrawing = false;

    const minSize =
      type === 'line'
        ? Math.hypot(w, h) >= MIN_LINE_LENGTH
        : w >= MIN_DRAW_SIZE && h >= MIN_DRAW_SIZE;
    if (minSize) {
      const node = createNodeForTool(this.drawTool, x, y, w, h);
      if (node) {
        engine.addNode(null, node);
        engine.select([node.id]);
      }
    }
    engine.setActiveTool('select');
    return true;
  }
}
