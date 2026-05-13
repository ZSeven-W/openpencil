import type { SkiaEngine } from './skia-engine';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { inferLayout } from '../canvas-layout-engine';
import type { PenNode, ContainerProps } from '@/types/pen';
import {
  DRAG_THRESHOLD,
  handleCursors,
  hitTestHandle,
  hitTestRotation,
  hitTestArcHandle,
} from './skia-hit-handlers';
import type { InteractionContext } from './skia-interaction-types';
import type { ResizeRotateHandler } from './skia-interaction-resize';
import type { ArcHandler } from './skia-interaction-arc';

/**
 * Handles
 * 选择、节点拖动和选取框选择。 On 鼠标按下，首先检查 arc/resize/rotate
 * 句柄（委托给相应的处理程序），然后进行命中测试选择。
 */
export class SelectHandler {
  private ctx: InteractionContext;
  private resizeRotate: ResizeRotateHandler;
  private arc: ArcHandler;

  // Drag 状态
  isDragging = false;
  private dragMoved = false;
  isMarquee = false;
  private dragNodeIds: string[] = [];
  private dragStartSceneX = 0;
  private dragStartSceneY = 0;
  private dragOrigPositions: { id: string; x: number; y: number }[] = [];
  private dragPrevDx = 0;
  private dragPrevDy = 0;
  private dragAllIds: Set<string> | null = null;

  constructor(ctx: InteractionContext, resizeRotate: ResizeRotateHandler, arc: ArcHandler) {
    this.ctx = ctx;
    this.resizeRotate = resizeRotate;
    this.arc = arc;
  }

  /**
   * Handle 在选择模式
   * 下按下鼠标。首先使用 Checks 进行手柄点击，然后对节点选择和拖动启动进行空间点击测试。
   */
  handleSelectMouseDown(e: MouseEvent, scene: { x: number; y: number }, engine: SkiaEngine): void {
    // Check 弧线首先处理
    if (this.arc.startArcDrag(scene, engine)) return;

    // Check 调整手柄大小
    if (this.resizeRotate.startResize(scene, engine)) return;

    // Check 旋转区域
    if (this.resizeRotate.startRotation(scene, engine)) return;

    const hits = engine.spatialIndex.hitTest(scene.x, scene.y);

    if (hits.length > 0) {
      const topHit = hits[0];
      let nodeId = topHit.node.id;
      const currentSelection = useCanvasStore.getState().selection.selectedIds;
      const docStore = useDocumentStore.getState();

      const isChildOfSelected = currentSelection.some(
        (selId) => selId !== nodeId && docStore.isDescendantOf(nodeId, selId),
      );
      if (isChildOfSelected) {
        // Don 不更改选择
      } else if (!currentSelection.includes(nodeId)) {
        const parent = docStore.getParentOf(nodeId);
        if (parent && (parent.type === 'frame' || parent.type === 'group')) {
          const grandparent = docStore.getParentOf(parent.id);
          if (!grandparent || grandparent.type === 'frame') {
            nodeId = parent.id;
          }
        }

        if (e.shiftKey) {
          if (currentSelection.includes(nodeId)) {
            const next = currentSelection.filter((id) => id !== nodeId);
            useCanvasStore.getState().setSelection(next, null);
          } else {
            useCanvasStore.getState().setSelection([...currentSelection, nodeId], nodeId);
          }
        } else {
          useCanvasStore.getState().setSelection([nodeId], nodeId);
        }
      }

      // Start 拖动
      const selectedIds = useCanvasStore.getState().selection.selectedIds;
      this.isDragging = true;
      this.dragMoved = false;
      this.dragNodeIds = selectedIds;
      this.dragStartSceneX = scene.x;
      this.dragStartSceneY = scene.y;
      this.dragOrigPositions = selectedIds.map((id) => {
        const n = useDocumentStore.getState().getNodeById(id);
        return { id, x: n?.x ?? 0, y: n?.y ?? 0 };
      });
    } else {
      // Empty space -> 开始选取框或清除选择
      if (!e.shiftKey) {
        useCanvasStore.getState().clearSelection();
      }
      this.isMarquee = true;
      engine.marquee = { x1: scene.x, y1: scene.y, x2: scene.x, y2: scene.y };
    }
  }

  handleDragMove(scene: { x: number; y: number }, engine: SkiaEngine): void {
    const dx = scene.x - this.dragStartSceneX;
    const dy = scene.y - this.dragStartSceneY;

    if (!this.dragMoved) {
      const screenDist = Math.hypot(dx * engine.zoom, dy * engine.zoom);
      if (screenDist < DRAG_THRESHOLD) return;
      this.dragMoved = true;
      engine.dragSyncSuppressed = true;
      this.dragPrevDx = 0;
      this.dragPrevDy = 0;
      this.dragAllIds = new Set(this.dragNodeIds);
      for (const id of this.dragNodeIds) {
        const collectDescs = (nodeId: string) => {
          const n = useDocumentStore.getState().getNodeById(nodeId);
          if (n && 'children' in n && n.children) {
            for (const child of n.children) {
              this.dragAllIds!.add(child.id);
              collectDescs(child.id);
            }
          }
        };
        collectDescs(id);
      }
    }

    const incrDx = dx - this.dragPrevDx;
    const incrDy = dy - this.dragPrevDy;
    this.dragPrevDx = dx;
    this.dragPrevDy = dy;

    for (const rn of engine.renderNodes) {
      if (this.dragAllIds!.has(rn.node.id)) {
        rn.absX += incrDx;
        rn.absY += incrDy;
        rn.node = { ...rn.node, x: rn.absX, y: rn.absY };
      }
    }
    engine.spatialIndex.rebuild(engine.renderNodes);
    engine.markDirty();
  }

  handleMarqueeMove(scene: { x: number; y: number }, engine: SkiaEngine): void {
    engine.marquee!.x2 = scene.x;
    engine.marquee!.y2 = scene.y;
    engine.markDirty();

    const marqueeHits = engine.spatialIndex.searchRect(
      engine.marquee!.x1,
      engine.marquee!.y1,
      engine.marquee!.x2,
      engine.marquee!.y2,
    );
    const ids = marqueeHits.map((rn) => rn.node.id);
    useCanvasStore.getState().setSelection(ids, null);
  }

  handleHoverCursor(scene: { x: number; y: number }, engine: SkiaEngine): void {
    const arcHoverHit = hitTestArcHandle(engine, scene.x, scene.y);
    if (arcHoverHit) {
      this.ctx.canvasEl.style.cursor = 'pointer';
      return;
    }
    const handleHit = hitTestHandle(engine, scene.x, scene.y);
    if (handleHit) {
      this.ctx.canvasEl.style.cursor = handleCursors[handleHit.dir];
    } else if (hitTestRotation(engine, scene.x, scene.y)) {
      this.ctx.canvasEl.style.cursor =
        "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2'%3E%3Cpath d='M21 2v6h-6'/%3E%3Cpath d='M21 13a9 9 0 1 1-3-7.7L21 8'/%3E%3C/svg%3E\") 12 12, crosshair";
    } else {
      const hoverHits = engine.spatialIndex.hitTest(scene.x, scene.y);
      const newHoveredId = hoverHits.length > 0 ? hoverHits[0].node.id : null;
      this.ctx.canvasEl.style.cursor = newHoveredId ? 'move' : 'default';
      if (newHoveredId !== engine.hoveredNodeId) {
        engine.hoveredNodeId = newHoveredId;
        useCanvasStore.getState().setHoveredId(newHoveredId);
        engine.markDirty();
      }
    }
  }

  /**
   * Finalize 拖动操作：提交位置更新并处理重定父级。
   */
  handleDragEnd(engine: SkiaEngine): void {
    if (!this.isDragging || !this.dragMoved || this.dragOrigPositions.length === 0) {
      engine.dragSyncSuppressed = false;
      this.resetDrag();
      return;
    }

    const dx = this.dragPrevDx;
    const dy = this.dragPrevDy;
    const docStore = useDocumentStore.getState();

    for (const orig of this.dragOrigPositions) {
      const parent = docStore.getParentOf(orig.id);
      const draggedRN = engine.renderNodes.find((rn) => rn.node.id === orig.id);
      const objBounds = draggedRN
        ? { x: draggedRN.absX, y: draggedRN.absY, w: draggedRN.absW, h: draggedRN.absH }
        : { x: orig.x + dx, y: orig.y + dy, w: 100, h: 100 };

      // Check 如果完全拖到父级之外 -> 重新父级
      if (parent) {
        const parentRN = engine.renderNodes.find((rn) => rn.node.id === parent.id);
        if (parentRN) {
          const pBounds = {
            x: parentRN.absX,
            y: parentRN.absY,
            w: parentRN.absW,
            h: parentRN.absH,
          };
          const outside =
            objBounds.x + objBounds.w <= pBounds.x ||
            objBounds.x >= pBounds.x + pBounds.w ||
            objBounds.y + objBounds.h <= pBounds.y ||
            objBounds.y >= pBounds.y + pBounds.h;

          if (outside) {
            docStore.updateNode(orig.id, { x: objBounds.x, y: objBounds.y } as Partial<PenNode>);
            docStore.moveNode(orig.id, null, 0);
            continue;
          }
        }
      }

      const parentLayout = parent
        ? (parent as PenNode & ContainerProps).layout || inferLayout(parent)
        : undefined;

      if (parentLayout && parentLayout !== 'none' && parent) {
        const siblings = ('children' in parent ? (parent.children ?? []) : []).filter(
          (c) => c.id !== orig.id,
        );
        const isVertical = parentLayout === 'vertical';

        let newIndex = siblings.length;
        for (let i = 0; i < siblings.length; i++) {
          const sibRN = engine.renderNodes.find((rn) => rn.node.id === siblings[i].id);
          const sibMid = sibRN
            ? isVertical
              ? sibRN.absY + sibRN.absH / 2
              : sibRN.absX + sibRN.absW / 2
            : 0;
          const dragMid = isVertical
            ? objBounds.y + objBounds.h / 2
            : objBounds.x + objBounds.w / 2;
          if (dragMid < sibMid) {
            newIndex = i;
            break;
          }
        }
        docStore.moveNode(orig.id, parent.id, newIndex);
      } else {
        docStore.updateNode(orig.id, {
          x: orig.x + dx,
          y: orig.y + dy,
        } as Partial<PenNode>);
      }
    }

    engine.dragSyncSuppressed = false;
    engine.syncFromDocument();
    this.resetDrag();
  }

  /**
   * Reset 所有 drag/marquee 状态。鼠标放在 Called 上。
   */
  resetDrag(): void {
    this.isDragging = false;
    this.dragMoved = false;
    this.dragNodeIds = [];
    this.dragOrigPositions = [];
    this.dragAllIds = null;
  }

  resetMarquee(engine: SkiaEngine): void {
    if (!this.isMarquee) return;
    engine.marquee = null;
    engine.markDirty();
    this.isMarquee = false;
  }
}
