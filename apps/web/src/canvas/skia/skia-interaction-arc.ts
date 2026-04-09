import type { SkiaEngine } from './skia-engine';
import { useDocumentStore } from '@/stores/document-store';
import type { PenNode, EllipseNode } from '@/types/pen';
import { hitTestArcHandle } from './skia-hit-handlers';
import { type InteractionContext, toolToCursor } from './skia-interaction-types';
import type { ArcHandleType } from './skia-hit-handlers';

/**
 * Handles arc editing interactions on ellipse nodes.
 * Manages start angle, end angle (sweep), and inner radius dragging.
 */
export class ArcHandler {
  private ctx: InteractionContext;

  isDraggingArc = false;
  private arcHandleType: ArcHandleType | null = null;
  private arcNodeId: string | null = null;

  constructor(ctx: InteractionContext) {
    this.ctx = ctx;
  }

  /**
   * Try to start an arc drag. Returns true if an arc handle was hit.
   */
  startArcDrag(scene: { x: number; y: number }, engine: SkiaEngine): boolean {
    const arcHit = hitTestArcHandle(engine, scene.x, scene.y);
    if (!arcHit) return false;

    this.isDraggingArc = true;
    this.arcHandleType = arcHit.type;
    this.arcNodeId = arcHit.nodeId;
    this.ctx.canvasEl.style.cursor = 'pointer';
    return true;
  }

  handleArcMove(scene: { x: number; y: number }, engine: SkiaEngine): void {
    const rn = engine.spatialIndex.get(this.arcNodeId!);
    if (!rn) return;

    const cx = rn.absX + rn.absW / 2;
    const cy = rn.absY + rn.absH / 2;
    const angle = (Math.atan2(scene.y - cy, scene.x - cx) * 180) / Math.PI;
    const normalizedAngle = ((angle % 360) + 360) % 360;
    const eNode = rn.node as EllipseNode;

    if (this.arcHandleType === 'start') {
      const oldStart = eNode.startAngle ?? 0;
      const oldEnd = oldStart + (eNode.sweepAngle ?? 360);
      const newSweep = (((oldEnd - normalizedAngle) % 360) + 360) % 360;
      useDocumentStore.getState().updateNode(this.arcNodeId!, {
        startAngle: normalizedAngle,
        sweepAngle: newSweep || 360,
      } as Partial<PenNode>);
    } else if (this.arcHandleType === 'end') {
      const startA = eNode.startAngle ?? 0;
      const newSweep = (((normalizedAngle - startA) % 360) + 360) % 360;
      useDocumentStore.getState().updateNode(this.arcNodeId!, {
        sweepAngle: newSweep || 360,
      } as Partial<PenNode>);
    } else if (this.arcHandleType === 'inner') {
      const rx = rn.absW / 2;
      const ry = rn.absH / 2;
      const dist = Math.hypot((scene.x - cx) / rx, (scene.y - cy) / ry);
      const newInner = Math.max(0, Math.min(0.99, dist));
      useDocumentStore.getState().updateNode(this.arcNodeId!, {
        innerRadius: newInner,
      } as Partial<PenNode>);
    }
  }

  resetArc(tool: string): void {
    if (!this.isDraggingArc) return;
    this.isDraggingArc = false;
    this.arcHandleType = null;
    this.arcNodeId = null;
    this.ctx.canvasEl.style.cursor = toolToCursor(tool);
  }
}
