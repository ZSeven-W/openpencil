import type { DesignEngine } from '../../core/design-engine.js';

/**
 * Handles arc editing interactions on ellipse nodes.
 * Extracted from apps/web/src/canvas/skia/skia-interaction-arc.ts.
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
