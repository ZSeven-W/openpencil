import type { DesignEngine } from '../../core/design-engine.js';

/**
 * Handles
 * 椭圆节点上的弧编辑交互。 Extracted 来自 apps/web/src/canvas/skia
 /skia-interaction-arc.ts。
 */
export class EngineArcHandler {
  isDraggingArc = false;

  startArcDrag(_scene: { x: number; y: number }, _engine: DesignEngine): boolean {
    return false;
  }

  handleArcMove(_scene: { x: number; y: number }, _engine: DesignEngine): void {}

  resetArc(): void {
    this.isDraggingArc = false;
  }
}
