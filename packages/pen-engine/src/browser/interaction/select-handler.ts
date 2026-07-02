import type { DesignEngine } from '../../core/design-engine.js';
import { DRAG_THRESHOLD } from '../../core/constants.js';

/**
 * Handles selection, node dragging, and marquee selection.
 * Engine-native: uses engine API instead of Zustand stores.
 */
export class EngineSelectHandler {
  isDragging = false;
  isMarquee = false;
  private dragMoved = false;
  private dragNodeIds: string[] = [];
  private dragStartSceneX = 0;
  private dragStartSceneY = 0;
  private dragOrigPositions: { id: string; x: number; y: number }[] = [];

  handleSelectMouseDown(scene: { x: number; y: number }, engine: DesignEngine): void {
    const hit = engine.hitTest(scene.x, scene.y);
    if (hit) {
      const currentSelection = engine.getSelection();
      if (!currentSelection.includes(hit.id)) {
        engine.select([hit.id]);
      }
      this.isDragging = true;
      this.dragMoved = false;
      this.dragStartSceneX = scene.x;
      this.dragStartSceneY = scene.y;
      this.dragNodeIds = engine.getSelection();
      this.dragOrigPositions = this.dragNodeIds.map((id) => {
        const n = engine.getNodeById(id);
        return { id, x: n?.x ?? 0, y: n?.y ?? 0 };
      });
    } else {
      engine.clearSelection();
      this.isMarquee = true;
      this.dragStartSceneX = scene.x;
      this.dragStartSceneY = scene.y;
    }
  }

  handleDragMove(scene: { x: number; y: number }, engine: DesignEngine): void {
    if (!this.isDragging) return;
    const dx = scene.x - this.dragStartSceneX;
    const dy = scene.y - this.dragStartSceneY;
    if (!this.dragMoved && Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
    this.dragMoved = true;

    engine.batch(() => {
      for (const orig of this.dragOrigPositions) {
        engine.updateNode(orig.id, { x: orig.x + dx, y: orig.y + dy });
      }
    });
  }

  handleMarqueeMove(
    scene: { x: number; y: number },
    engine: DesignEngine,
  ): { x1: number; y1: number; x2: number; y2: number } | null {
    if (!this.isMarquee) return null;
    const x1 = this.dragStartSceneX,
      y1 = this.dragStartSceneY;
    const x2 = scene.x,
      y2 = scene.y;
    const hits = engine.searchRect(
      Math.min(x1, x2),
      Math.min(y1, y2),
      Math.abs(x2 - x1),
      Math.abs(y2 - y1),
    );
    engine.select(hits.map((n) => n.id));
    return { x1, y1, x2, y2 };
  }

  handleDragEnd(): void {
    this.isDragging = false;
    this.dragMoved = false;
    this.dragNodeIds = [];
    this.dragOrigPositions = [];
  }

  resetMarquee(): void {
    this.isMarquee = false;
  }

  resetDrag(): void {
    this.isDragging = false;
    this.dragMoved = false;
  }
}
