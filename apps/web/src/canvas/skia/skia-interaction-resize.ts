import type { SkiaEngine } from './skia-engine';
import { useDocumentStore } from '@/stores/document-store';
import type { PenNode, ContainerProps, TextNode } from '@/types/pen';
import { type HandleDir, handleCursors, hitTestHandle, hitTestRotation } from './skia-hit-handlers';
import { type InteractionContext, toolToCursor } from './skia-interaction-types';

/**
 * Handles resize and rotation interactions on selected nodes.
 * Both modes are triggered from handle hit-testing in `handleSelectMouseDown`
 * and share lifecycle concerns, so they live together.
 */
export class ResizeRotateHandler {
  private ctx: InteractionContext;

  // Resize state
  isResizing = false;
  private resizeHandle: HandleDir | null = null;
  private resizeNodeId: string | null = null;
  private resizeOrigX = 0;
  private resizeOrigY = 0;
  private resizeOrigW = 0;
  private resizeOrigH = 0;
  private resizeStartSceneX = 0;
  private resizeStartSceneY = 0;

  // Rotation state
  isRotating = false;
  private rotateNodeId: string | null = null;
  private rotateOrigAngle = 0;
  private rotateCenterX = 0;
  private rotateCenterY = 0;
  private rotateStartAngle = 0;

  constructor(ctx: InteractionContext) {
    this.ctx = ctx;
  }

  /**
   * Try to start a resize from a handle hit. Returns true if a handle was hit.
   */
  startResize(scene: { x: number; y: number }, engine: SkiaEngine): boolean {
    const handleHit = hitTestHandle(engine, scene.x, scene.y);
    if (!handleHit) return false;

    this.isResizing = true;
    this.resizeHandle = handleHit.dir;
    this.resizeNodeId = handleHit.nodeId;
    this.resizeStartSceneX = scene.x;
    this.resizeStartSceneY = scene.y;
    const docNode = useDocumentStore.getState().getNodeById(handleHit.nodeId);
    this.resizeOrigX = docNode?.x ?? 0;
    this.resizeOrigY = docNode?.y ?? 0;
    const resizeRN = engine.spatialIndex.get(handleHit.nodeId);
    const docNodeAny = docNode as (PenNode & ContainerProps) | undefined;
    this.resizeOrigW =
      resizeRN?.absW ?? (typeof docNodeAny?.width === 'number' ? docNodeAny.width : 100);
    this.resizeOrigH =
      resizeRN?.absH ?? (typeof docNodeAny?.height === 'number' ? docNodeAny.height : 100);
    this.ctx.canvasEl.style.cursor = handleCursors[handleHit.dir];
    return true;
  }

  handleResizeMove(scene: { x: number; y: number }, engine: SkiaEngine): void {
    const dx = scene.x - this.resizeStartSceneX;
    const dy = scene.y - this.resizeStartSceneY;
    let newX = this.resizeOrigX;
    let newY = this.resizeOrigY;
    let newW = this.resizeOrigW;
    let newH = this.resizeOrigH;

    const dir = this.resizeHandle!;
    if (dir.includes('w')) {
      newX = this.resizeOrigX + dx;
      newW = this.resizeOrigW - dx;
    }
    if (dir.includes('e')) {
      newW = this.resizeOrigW + dx;
    }
    if (dir.includes('n')) {
      newY = this.resizeOrigY + dy;
      newH = this.resizeOrigH - dy;
    }
    if (dir.includes('s')) {
      newH = this.resizeOrigH + dy;
    }

    const MIN = 2;
    if (newW < MIN) {
      if (dir.includes('w')) newX = this.resizeOrigX + this.resizeOrigW - MIN;
      newW = MIN;
    }
    if (newH < MIN) {
      if (dir.includes('n')) newY = this.resizeOrigY + this.resizeOrigH - MIN;
      newH = MIN;
    }

    const resizedNode = useDocumentStore.getState().getNodeById(this.resizeNodeId!);
    const updates: Record<string, unknown> = { x: newX, y: newY, width: newW, height: newH };
    if (resizedNode?.type === 'text' && !(resizedNode as TextNode).textGrowth) {
      updates.textGrowth = 'fixed-width';
    }
    useDocumentStore.getState().updateNode(this.resizeNodeId!, updates as Partial<PenNode>);

    if (resizedNode && 'children' in resizedNode && resizedNode.children?.length) {
      const resizeRN2 = engine.spatialIndex.get(this.resizeNodeId!);
      const resizedContainer = resizedNode as PenNode & ContainerProps;
      const oldW =
        resizeRN2?.absW ??
        (typeof resizedContainer.width === 'number' ? resizedContainer.width : 0);
      const oldH =
        resizeRN2?.absH ??
        (typeof resizedContainer.height === 'number' ? resizedContainer.height : 0);
      if (oldW > 0 && oldH > 0) {
        const scaleX = newW / oldW;
        const scaleY = newH / oldH;
        useDocumentStore.getState().scaleDescendantsInStore(this.resizeNodeId!, scaleX, scaleY);
      }
    }
  }

  /**
   * Try to start a rotation from a rotation zone hit. Returns true if hit.
   */
  startRotation(scene: { x: number; y: number }, engine: SkiaEngine): boolean {
    const rotHit = hitTestRotation(engine, scene.x, scene.y);
    if (!rotHit) return false;

    this.isRotating = true;
    this.rotateNodeId = rotHit.nodeId;
    const docNode = useDocumentStore.getState().getNodeById(rotHit.nodeId);
    this.rotateOrigAngle = docNode?.rotation ?? 0;
    const rn = engine.spatialIndex.get(rotHit.nodeId)!;
    this.rotateCenterX = rn.absX + rn.absW / 2;
    this.rotateCenterY = rn.absY + rn.absH / 2;
    this.rotateStartAngle =
      (Math.atan2(scene.y - this.rotateCenterY, scene.x - this.rotateCenterX) * 180) / Math.PI;
    this.ctx.canvasEl.style.cursor = 'grabbing';
    return true;
  }

  handleRotateMove(scene: { x: number; y: number }, shiftKey: boolean): void {
    const currentAngle =
      (Math.atan2(scene.y - this.rotateCenterY, scene.x - this.rotateCenterX) * 180) / Math.PI;
    let newAngle = this.rotateOrigAngle + (currentAngle - this.rotateStartAngle);
    if (shiftKey) {
      newAngle = Math.round(newAngle / 15) * 15;
    }
    useDocumentStore
      .getState()
      .updateNode(this.rotateNodeId!, { rotation: newAngle } as Partial<PenNode>);
  }

  resetResize(tool: string): void {
    if (!this.isResizing) return;
    this.isResizing = false;
    this.resizeHandle = null;
    this.resizeNodeId = null;
    this.ctx.canvasEl.style.cursor = toolToCursor(tool);
  }

  resetRotation(tool: string): void {
    if (!this.isRotating) return;
    this.isRotating = false;
    this.rotateNodeId = null;
    this.ctx.canvasEl.style.cursor = toolToCursor(tool);
  }
}
