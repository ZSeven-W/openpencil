import { screenToScene } from './skia-engine';
import type { SkiaEngine } from './skia-engine';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import { createNodeForTool, isDrawingTool } from '../canvas-node-creator';
import { inferLayout } from '../canvas-layout-engine';
import { SkiaPenTool } from './skia-pen-tool';
import type { ToolType } from '@/types/canvas';
import type {
  PenNode,
  ContainerProps,
  TextNode,
  EllipseNode,
  PathNode,
  PenPathAnchor,
} from '@/types/pen';
import {
  type HandleDir,
  type ArcHandleType,
  type PathControlType,
  DRAG_THRESHOLD,
  handleCursors,
  hitTestHandle,
  hitTestRotation,
  hitTestArcHandle,
  hitTestPathControl,
} from './skia-hit-handlers';
import { bakeSceneAnchorsToPathNode, getEditablePathState, movePathControl } from './path-editing';
import { shouldAutoReparentOnDragOutsideParent } from './drag-reparent-policy';

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

export interface PathAnchorContextMenuState {
  x: number;
  y: number;
  nodeId: string;
  anchorIndex: number;
}

interface RenderNodeSnapshot {
  node: PenNode;
  absX: number;
  absY: number;
  absW: number;
  absH: number;
  clipRect?: { x: number; y: number; w: number; h: number; rx: number };
}

interface PreviewRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function toolToCursor(tool: ToolType): string {
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

function hasImageVisual(node: PenNode | undefined): boolean {
  if (!node) return false;
  if (node.type === 'image') return true;
  if (!('fill' in node)) return false;
  return Array.isArray(node.fill) && node.fill.some((fill: any) => fill?.type === 'image');
}

/**
 * Encapsulates all canvas mouse/keyboard interaction state and handlers.
 * Extracted from SkiaCanvas to keep the component focused on lifecycle and rendering.
 */
export class SkiaInteractionManager {
  private engineRef: { current: SkiaEngine | null };
  private canvasEl: HTMLCanvasElement;
  private onEditText: (state: TextEditState | null) => void;
  private onPathAnchorContextMenu: (state: PathAnchorContextMenuState | null) => void;

  // Shared state
  private isPanning = false;
  private spacePressed = false;
  private lastX = 0;
  private lastY = 0;

  // Select tool state
  private isDragging = false;
  private dragMoved = false;
  private isMarquee = false;
  private dragNodeIds: string[] = [];
  private dragStartSceneX = 0;
  private dragStartSceneY = 0;
  private dragOrigPositions: { id: string; x: number; y: number }[] = [];
  private dragPrevDx = 0;
  private dragPrevDy = 0;
  private dragAllIds: Set<string> | null = null;

  // Resize handle state
  private isResizing = false;
  private resizeHandle: HandleDir | null = null;
  private resizeNodeId: string | null = null;
  private resizeOrigX = 0;
  private resizeOrigY = 0;
  private resizeOrigW = 0;
  private resizeOrigH = 0;
  private resizeStartSceneX = 0;
  private resizeStartSceneY = 0;
  private resizePreviewNodes: Map<string, RenderNodeSnapshot> | null = null;
  private resizeLatestPatch: Partial<PenNode> | null = null;
  private resizeLatestScale: { scaleX: number; scaleY: number } | null = null;
  private resizeMoved = false;

  // Rotation state
  private isRotating = false;
  private rotateNodeId: string | null = null;
  private rotateOrigAngle = 0;
  private rotateCenterX = 0;
  private rotateCenterY = 0;
  private rotateStartAngle = 0;
  private rotatePreviewNodes: Map<string, RenderNodeSnapshot> | null = null;
  private rotateLatestAngle: number | null = null;
  private rotateMoved = false;

  // Arc handle state
  private isDraggingArc = false;
  private arcHandleType: ArcHandleType | null = null;
  private arcNodeId: string | null = null;
  private arcPreviewNode: PenNode | null = null;
  private arcLatestPatch: Partial<EllipseNode> | null = null;
  private arcMoved = false;

  // Path control state
  private isDraggingPathControl = false;
  private pathControlType: PathControlType | null = null;
  private pathControlAnchorIndex: number | null = null;
  private pathNodeId: string | null = null;
  private pathPrevSceneX = 0;
  private pathPrevSceneY = 0;
  private pathSceneAnchors: PenPathAnchor[] | null = null;
  private pathClosed = false;
  private pathParentSceneOrigin: { x: number; y: number } | null = null;
  private pathLatestPatch: Pick<
    PathNode,
    'x' | 'y' | 'width' | 'height' | 'd' | 'anchors' | 'closed'
  > | null = null;
  private pathControlMoved = false;

  // Drawing tool state
  private isDrawing = false;
  private drawTool: ToolType = 'select';
  private drawStartX = 0;
  private drawStartY = 0;

  // Pen tool
  private penTool: SkiaPenTool;

  constructor(
    engineRef: { current: SkiaEngine | null },
    canvasEl: HTMLCanvasElement,
    onEditText: (state: TextEditState | null) => void,
    onPathAnchorContextMenu: (state: PathAnchorContextMenuState | null) => void = () => {},
  ) {
    this.engineRef = engineRef;
    this.canvasEl = canvasEl;
    this.onEditText = onEditText;
    this.onPathAnchorContextMenu = onPathAnchorContextMenu;
    this.penTool = new SkiaPenTool(() => this.engineRef.current);
  }

  private getEngine() {
    return this.engineRef.current;
  }
  private getTool() {
    return useCanvasStore.getState().activeTool;
  }

  private getScene(e: MouseEvent) {
    const engine = this.getEngine();
    if (!engine) return null;
    const rect = engine.getCanvasRect();
    if (!rect) return null;
    return screenToScene(e.clientX, e.clientY, rect, {
      zoom: engine.zoom,
      panX: engine.panX,
      panY: engine.panY,
    });
  }

  // ---------------------------------------------------------------------------
  // Mouse down
  // ---------------------------------------------------------------------------

  private onMouseDown = (e: MouseEvent) => {
    const engine = this.getEngine();
    if (!engine) return;

    if (e.button === 2) return;

    // Pan: space+click, hand tool, or middle mouse
    if (this.spacePressed || this.getTool() === 'hand' || e.button === 1) {
      this.isPanning = true;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      this.canvasEl.style.cursor = 'grabbing';
      return;
    }

    const tool = this.getTool();
    const scene = this.getScene(e);
    if (!scene) return;

    // Text tool: click to create immediately
    if (tool === 'text') {
      const node = createNodeForTool('text', scene.x, scene.y, 0, 0);
      if (node) {
        useDocumentStore.getState().addNode(null, node);
        useCanvasStore.getState().setSelection([node.id], node.id);
      }
      useCanvasStore.getState().setActiveTool('select');
      return;
    }

    // Pen tool
    if (tool === 'path') {
      this.penTool.onMouseDown(scene, engine.zoom || 1);
      return;
    }

    // Drawing tools: start rubber-band
    if (isDrawingTool(tool)) {
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
      return;
    }

    // Select tool
    if (tool === 'select') {
      this.handleSelectMouseDown(e, scene, engine);
    }
  };

  private handleSelectMouseDown(
    e: MouseEvent,
    scene: { x: number; y: number },
    engine: SkiaEngine,
  ) {
    const pathHit = hitTestPathControl(engine, scene.x, scene.y);
    if (pathHit) {
      const rn = engine.spatialIndex.get(pathHit.nodeId);
      if (!rn || rn.node.type !== 'path') return;
      const pathState = getEditablePathState(rn.node as PathNode, {
        x: rn.absX,
        y: rn.absY,
        width: rn.absW,
        height: rn.absH,
      });
      if (!pathState) return;

      this.isDraggingPathControl = true;
      this.pathControlType = pathHit.type;
      this.pathControlAnchorIndex = pathHit.anchorIndex;
      this.pathNodeId = pathHit.nodeId;
      this.pathPrevSceneX = scene.x;
      this.pathPrevSceneY = scene.y;
      this.pathSceneAnchors = pathState.sceneAnchors;
      this.pathClosed = pathState.closed;
      this.pathParentSceneOrigin = {
        x: rn.absX - ((rn.node as PathNode).x ?? 0),
        y: rn.absY - ((rn.node as PathNode).y ?? 0),
      };
      this.pathLatestPatch = null;
      this.pathControlMoved = false;
      engine.dragSyncSuppressed = true;
      this.canvasEl.style.cursor = 'pointer';
      return;
    }

    // Check arc handles first
    const arcHit = hitTestArcHandle(engine, scene.x, scene.y);
    if (arcHit) {
      this.isDraggingArc = true;
      this.arcHandleType = arcHit.type;
      this.arcNodeId = arcHit.nodeId;
      this.arcPreviewNode = null;
      this.arcLatestPatch = null;
      this.arcMoved = false;
      engine.dragSyncSuppressed = true;
      this.canvasEl.style.cursor = 'pointer';
      return;
    }

    // Check resize handle
    const handleHit = hitTestHandle(engine, scene.x, scene.y);
    if (handleHit) {
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
      this.resizePreviewNodes = null;
      this.resizeLatestPatch = null;
      this.resizeLatestScale = null;
      this.resizeMoved = false;
      engine.dragSyncSuppressed = true;
      this.canvasEl.style.cursor = handleCursors[handleHit.dir];
      return;
    }

    // Check rotation zone
    const rotHit = hitTestRotation(engine, scene.x, scene.y);
    if (rotHit) {
      this.isRotating = true;
      this.rotateNodeId = rotHit.nodeId;
      const docNode = useDocumentStore.getState().getNodeById(rotHit.nodeId);
      this.rotateOrigAngle = docNode?.rotation ?? 0;
      const rn = engine.spatialIndex.get(rotHit.nodeId)!;
      this.rotateCenterX = rn.absX + rn.absW / 2;
      this.rotateCenterY = rn.absY + rn.absH / 2;
      this.rotateStartAngle =
        (Math.atan2(scene.y - this.rotateCenterY, scene.x - this.rotateCenterX) * 180) / Math.PI;
      this.rotatePreviewNodes = null;
      this.rotateLatestAngle = null;
      this.rotateMoved = false;
      engine.dragSyncSuppressed = true;
      this.canvasEl.style.cursor = 'grabbing';
      return;
    }

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
        // Don't change selection
      } else if (!currentSelection.includes(nodeId)) {
        const clickedNode = docStore.getNodeById(nodeId);
        const parent = docStore.getParentOf(nodeId);
        if (
          !hasImageVisual(clickedNode) &&
          parent &&
          (parent.type === 'frame' || parent.type === 'group')
        ) {
          const grandparent = docStore.getParentOf(parent.id);
          if (!grandparent || grandparent.type === 'frame') {
            nodeId = parent.id;
          }
        }

        if (e.shiftKey) {
          if (currentSelection.includes(nodeId)) {
            const next = currentSelection.filter((id) => id !== nodeId);
            useCanvasStore.getState().setSelection(next, next[0] ?? null);
          } else {
            useCanvasStore.getState().setSelection([...currentSelection, nodeId], nodeId);
          }
        } else {
          useCanvasStore.getState().setSelection([nodeId], nodeId);
        }
      }

      // Start drag
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
      // Empty space → start marquee or clear selection
      if (!e.shiftKey) {
        useCanvasStore.getState().clearSelection();
      }
      this.isMarquee = true;
      this.lastX = scene.x;
      this.lastY = scene.y;
      engine.marquee = { x1: scene.x, y1: scene.y, x2: scene.x, y2: scene.y };
    }
  }

  // ---------------------------------------------------------------------------
  // Mouse move
  // ---------------------------------------------------------------------------

  private onMouseMove = (e: MouseEvent) => {
    const engine = this.getEngine();
    if (!engine) return;

    if (this.isPanning) {
      const dx = e.clientX - this.lastX;
      const dy = e.clientY - this.lastY;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      engine.pan(dx, dy);
      return;
    }

    const scene = this.getScene(e);
    if (!scene) return;

    if (this.penTool.onMouseMove(scene)) return;

    if (
      this.isDraggingPathControl &&
      this.pathNodeId &&
      this.pathControlType != null &&
      this.pathControlAnchorIndex != null
    ) {
      this.handlePathControlMove(scene, engine);
      return;
    }

    if (this.isResizing && this.resizeHandle && this.resizeNodeId) {
      this.handleResizeMove(scene, engine);
      return;
    }

    if (this.isRotating && this.rotateNodeId) {
      this.handleRotateMove(scene, e.shiftKey);
      return;
    }

    if (this.isDraggingArc && this.arcNodeId && this.arcHandleType) {
      this.handleArcMove(scene, engine);
      return;
    }

    if (this.isDrawing && engine.previewShape) {
      this.handleDrawingMove(scene, engine);
      return;
    }

    if (this.isDragging && this.dragNodeIds.length > 0) {
      this.handleDragMove(scene, engine);
      return;
    }

    if (this.isMarquee && engine.marquee) {
      this.handleMarqueeMove(scene, engine);
      return;
    }

    // Hover + handle cursor (select tool only)
    if (this.getTool() === 'select' && !this.spacePressed) {
      this.handleHoverCursor(scene, engine);
    }
  };

  private collectSubtreeIds(rootId: string): Set<string> {
    const ids = new Set<string>();
    const visit = (nodeId: string) => {
      if (ids.has(nodeId)) return;
      ids.add(nodeId);
      const node = useDocumentStore.getState().getNodeById(nodeId);
      if (node && 'children' in node && node.children) {
        for (const child of node.children) visit(child.id);
      }
    };
    visit(rootId);
    return ids;
  }

  private captureRenderNodeSnapshots(
    rootId: string,
    engine: SkiaEngine,
  ): Map<string, RenderNodeSnapshot> {
    const ids = this.collectSubtreeIds(rootId);
    const snapshots = new Map<string, RenderNodeSnapshot>();
    for (const rn of engine.renderNodes) {
      if (!ids.has(rn.node.id)) continue;
      snapshots.set(rn.node.id, {
        node: structuredClone(rn.node),
        absX: rn.absX,
        absY: rn.absY,
        absW: rn.absW,
        absH: rn.absH,
        ...(rn.clipRect ? { clipRect: { ...rn.clipRect } } : {}),
      });
    }
    return snapshots;
  }

  private scalePreviewRect(rect: PreviewRect, from: PreviewRect, to: PreviewRect): PreviewRect {
    const scaleX = from.w !== 0 ? to.w / from.w : 1;
    const scaleY = from.h !== 0 ? to.h / from.h : 1;
    return {
      x: to.x + (rect.x - from.x) * scaleX,
      y: to.y + (rect.y - from.y) * scaleY,
      w: rect.w * scaleX,
      h: rect.h * scaleY,
    };
  }

  private rotatePreviewRect(
    rect: PreviewRect,
    centerX: number,
    centerY: number,
    angleDeg: number,
  ): PreviewRect {
    const rad = (angleDeg * Math.PI) / 180;
    const cosA = Math.cos(rad);
    const sinA = Math.sin(rad);
    const rectCx = rect.x + rect.w / 2;
    const rectCy = rect.y + rect.h / 2;
    const dx = rectCx - centerX;
    const dy = rectCy - centerY;
    const nextCx = centerX + dx * cosA - dy * sinA;
    const nextCy = centerY + dx * sinA + dy * cosA;
    return {
      x: nextCx - rect.w / 2,
      y: nextCy - rect.h / 2,
      w: rect.w,
      h: rect.h,
    };
  }

  private ensureResizePreviewNodes(engine: SkiaEngine): Map<string, RenderNodeSnapshot> | null {
    if (!this.resizeNodeId) return null;
    if (!this.resizePreviewNodes) {
      this.resizePreviewNodes = this.captureRenderNodeSnapshots(this.resizeNodeId, engine);
    }
    return this.resizePreviewNodes;
  }

  private ensureRotatePreviewNodes(engine: SkiaEngine): Map<string, RenderNodeSnapshot> | null {
    if (!this.rotateNodeId) return null;
    if (!this.rotatePreviewNodes) {
      this.rotatePreviewNodes = this.captureRenderNodeSnapshots(this.rotateNodeId, engine);
    }
    return this.rotatePreviewNodes;
  }

  private handleResizeMove(scene: { x: number; y: number }, engine: SkiaEngine) {
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

    const previewNodes = this.ensureResizePreviewNodes(engine);
    const rootSnapshot = previewNodes?.get(this.resizeNodeId!);
    const rootRn = engine.spatialIndex.get(this.resizeNodeId!);
    if (!previewNodes || !rootSnapshot || !rootRn) return;

    const sourceRect = {
      x: rootSnapshot.absX,
      y: rootSnapshot.absY,
      w: rootSnapshot.absW,
      h: rootSnapshot.absH,
    };
    const nextRootRect = {
      x: dir.includes('w') ? rootSnapshot.absX + dx : rootSnapshot.absX,
      y: dir.includes('n') ? rootSnapshot.absY + dy : rootSnapshot.absY,
      w: newW,
      h: newH,
    };
    const scaleX = sourceRect.w !== 0 ? nextRootRect.w / sourceRect.w : 1;
    const scaleY = sourceRect.h !== 0 ? nextRootRect.h / sourceRect.h : 1;

    rootRn.node = {
      ...rootSnapshot.node,
      ...updates,
    } as PenNode;
    rootRn.absX = nextRootRect.x;
    rootRn.absY = nextRootRect.y;
    rootRn.absW = nextRootRect.w;
    rootRn.absH = nextRootRect.h;
    rootRn.clipRect = rootSnapshot.clipRect
      ? {
          ...this.scalePreviewRect(
            {
              x: rootSnapshot.clipRect.x,
              y: rootSnapshot.clipRect.y,
              w: rootSnapshot.clipRect.w,
              h: rootSnapshot.clipRect.h,
            },
            sourceRect,
            nextRootRect,
          ),
          rx: rootSnapshot.clipRect.rx * Math.min(scaleX, scaleY),
        }
      : undefined;

    for (const [id, snapshot] of previewNodes) {
      if (id === this.resizeNodeId) continue;
      const rn = engine.spatialIndex.get(id);
      if (!rn) continue;
      const scaled = this.scalePreviewRect(
        { x: snapshot.absX, y: snapshot.absY, w: snapshot.absW, h: snapshot.absH },
        sourceRect,
        nextRootRect,
      );
      rn.node = snapshot.node;
      rn.absX = scaled.x;
      rn.absY = scaled.y;
      rn.absW = scaled.w;
      rn.absH = scaled.h;
      rn.clipRect = snapshot.clipRect
        ? {
            ...this.scalePreviewRect(
              {
                x: snapshot.clipRect.x,
                y: snapshot.clipRect.y,
                w: snapshot.clipRect.w,
                h: snapshot.clipRect.h,
              },
              sourceRect,
              nextRootRect,
            ),
            rx: snapshot.clipRect.rx * Math.min(scaleX, scaleY),
          }
        : undefined;
    }

    this.resizeLatestPatch = updates as Partial<PenNode>;
    this.resizeLatestScale = { scaleX, scaleY };
    this.resizeMoved = true;
    engine.spatialIndex.rebuild(engine.renderNodes);
    engine.markDirty();
  }

  private handleRotateMove(scene: { x: number; y: number }, shiftKey: boolean) {
    const engine = this.getEngine();
    if (!engine) return;

    const currentAngle =
      (Math.atan2(scene.y - this.rotateCenterY, scene.x - this.rotateCenterX) * 180) / Math.PI;
    let newAngle = this.rotateOrigAngle + (currentAngle - this.rotateStartAngle);
    if (shiftKey) {
      newAngle = Math.round(newAngle / 15) * 15;
    }

    const previewNodes = this.ensureRotatePreviewNodes(engine);
    const rootSnapshot = previewNodes?.get(this.rotateNodeId!);
    const rootRn = this.rotateNodeId ? engine.spatialIndex.get(this.rotateNodeId) : null;
    if (!previewNodes || !rootSnapshot || !rootRn) return;

    const angleDelta = newAngle - this.rotateOrigAngle;
    rootRn.node = {
      ...rootSnapshot.node,
      rotation: newAngle,
    } as PenNode;
    rootRn.absX = rootSnapshot.absX;
    rootRn.absY = rootSnapshot.absY;
    rootRn.absW = rootSnapshot.absW;
    rootRn.absH = rootSnapshot.absH;
    rootRn.clipRect = rootSnapshot.clipRect ? { ...rootSnapshot.clipRect } : undefined;

    const centerX = rootSnapshot.absX + rootSnapshot.absW / 2;
    const centerY = rootSnapshot.absY + rootSnapshot.absH / 2;
    for (const [id, snapshot] of previewNodes) {
      if (id === this.rotateNodeId) continue;
      const rn = engine.spatialIndex.get(id);
      if (!rn) continue;
      const rotated = this.rotatePreviewRect(
        { x: snapshot.absX, y: snapshot.absY, w: snapshot.absW, h: snapshot.absH },
        centerX,
        centerY,
        angleDelta,
      );
      rn.node = {
        ...snapshot.node,
        x: rotated.x,
        y: rotated.y,
        rotation: (snapshot.node.rotation ?? 0) + angleDelta,
      } as PenNode;
      rn.absX = rotated.x;
      rn.absY = rotated.y;
      rn.absW = rotated.w;
      rn.absH = rotated.h;
      rn.clipRect = snapshot.clipRect
        ? {
            ...this.rotatePreviewRect(
              {
                x: snapshot.clipRect.x,
                y: snapshot.clipRect.y,
                w: snapshot.clipRect.w,
                h: snapshot.clipRect.h,
              },
              centerX,
              centerY,
              angleDelta,
            ),
            rx: snapshot.clipRect.rx,
          }
        : undefined;
    }

    this.rotateLatestAngle = newAngle;
    this.rotateMoved = true;
    engine.spatialIndex.rebuild(engine.renderNodes);
    engine.markDirty();
  }

  private handleArcMove(scene: { x: number; y: number }, engine: SkiaEngine) {
    const rn = engine.spatialIndex.get(this.arcNodeId!);
    if (!rn) return;

    if (!this.arcPreviewNode) {
      this.arcPreviewNode = structuredClone(rn.node);
    }

    const cx = rn.absX + rn.absW / 2;
    const cy = rn.absY + rn.absH / 2;
    const angle = (Math.atan2(scene.y - cy, scene.x - cx) * 180) / Math.PI;
    const normalizedAngle = ((angle % 360) + 360) % 360;
    const eNode = this.arcPreviewNode as EllipseNode;
    const updates: Partial<EllipseNode> = {};

    if (this.arcHandleType === 'start') {
      const oldStart = eNode.startAngle ?? 0;
      const oldEnd = oldStart + (eNode.sweepAngle ?? 360);
      const newSweep = (((oldEnd - normalizedAngle) % 360) + 360) % 360;
      updates.startAngle = normalizedAngle;
      updates.sweepAngle = newSweep || 360;
    } else if (this.arcHandleType === 'end') {
      const startA = eNode.startAngle ?? 0;
      const newSweep = (((normalizedAngle - startA) % 360) + 360) % 360;
      updates.sweepAngle = newSweep || 360;
    } else if (this.arcHandleType === 'inner') {
      const rx = rn.absW / 2;
      const ry = rn.absH / 2;
      const dist = Math.hypot((scene.x - cx) / rx, (scene.y - cy) / ry);
      updates.innerRadius = Math.max(0, Math.min(0.99, dist));
    }

    rn.node = {
      ...this.arcPreviewNode,
      ...updates,
    } as PenNode;
    this.arcLatestPatch = updates;
    this.arcMoved = true;
    engine.markDirty();
  }

  private handlePathControlMove(scene: { x: number; y: number }, engine: SkiaEngine) {
    const rn = engine.spatialIndex.get(this.pathNodeId!);
    if (!rn || rn.node.type !== 'path') return;

    const dx = scene.x - this.pathPrevSceneX;
    const dy = scene.y - this.pathPrevSceneY;
    if (dx === 0 && dy === 0) return;

    this.pathPrevSceneX = scene.x;
    this.pathPrevSceneY = scene.y;

    if (!this.pathSceneAnchors || !this.pathParentSceneOrigin) return;

    this.pathSceneAnchors = movePathControl(
      this.pathSceneAnchors,
      this.pathControlAnchorIndex!,
      this.pathControlType!,
      dx,
      dy,
    );
    const patch = bakeSceneAnchorsToPathNode(
      this.pathSceneAnchors,
      this.pathClosed,
      this.pathParentSceneOrigin,
    );
    if (!patch) return;

    this.pathLatestPatch = patch;
    this.pathControlMoved = true;
    const patchX = typeof patch.x === 'number' ? patch.x : 0;
    const patchY = typeof patch.y === 'number' ? patch.y : 0;
    const patchW = typeof patch.width === 'number' ? patch.width : rn.absW;
    const patchH = typeof patch.height === 'number' ? patch.height : rn.absH;

    rn.node = {
      ...rn.node,
      ...patch,
    } as PenNode;
    rn.absX = this.pathParentSceneOrigin.x + patchX;
    rn.absY = this.pathParentSceneOrigin.y + patchY;
    rn.absW = patchW;
    rn.absH = patchH;
    engine.markDirty();
  }

  private handleDrawingMove(scene: { x: number; y: number }, engine: SkiaEngine) {
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

  private handleDragMove(scene: { x: number; y: number }, engine: SkiaEngine) {
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
        if (rn.clipRect) {
          rn.clipRect = {
            ...rn.clipRect,
            x: rn.clipRect.x + incrDx,
            y: rn.clipRect.y + incrDy,
          };
        }
        rn.node = { ...rn.node, x: rn.absX, y: rn.absY };
      }
    }
    engine.spatialIndex.rebuild(engine.renderNodes);
    engine.markDirty();
  }

  private handleMarqueeMove(scene: { x: number; y: number }, engine: SkiaEngine) {
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
    useCanvasStore.getState().setSelection(ids, ids[0] ?? null);
  }

  private handleHoverCursor(scene: { x: number; y: number }, engine: SkiaEngine) {
    const pathHoverHit = hitTestPathControl(engine, scene.x, scene.y);
    if (pathHoverHit) {
      this.canvasEl.style.cursor = 'pointer';
      return;
    }
    const arcHoverHit = hitTestArcHandle(engine, scene.x, scene.y);
    if (arcHoverHit) {
      this.canvasEl.style.cursor = 'pointer';
      return;
    }
    const handleHit = hitTestHandle(engine, scene.x, scene.y);
    if (handleHit) {
      this.canvasEl.style.cursor = handleCursors[handleHit.dir];
    } else if (hitTestRotation(engine, scene.x, scene.y)) {
      this.canvasEl.style.cursor =
        "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2'%3E%3Cpath d='M21 2v6h-6'/%3E%3Cpath d='M21 13a9 9 0 1 1-3-7.7L21 8'/%3E%3C/svg%3E\") 12 12, crosshair";
    } else {
      const hoverHits = engine.spatialIndex.hitTest(scene.x, scene.y);
      const newHoveredId = hoverHits.length > 0 ? hoverHits[0].node.id : null;
      this.canvasEl.style.cursor = newHoveredId ? 'move' : 'default';
      if (newHoveredId !== engine.hoveredNodeId) {
        engine.hoveredNodeId = newHoveredId;
        useCanvasStore.getState().setHoveredId(newHoveredId);
        engine.markDirty();
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Mouse up
  // ---------------------------------------------------------------------------

  private onMouseUp = () => {
    const engine = this.getEngine();

    if (this.penTool.onMouseUp()) return;

    if (this.isPanning) {
      this.isPanning = false;
      this.canvasEl.style.cursor = this.spacePressed ? 'grab' : toolToCursor(this.getTool());
    }

    if (this.isResizing) {
      const resizeNodeId = this.resizeNodeId;
      const resizePatch = this.resizeLatestPatch;
      const resizeScale = this.resizeLatestScale;
      const resizePreviewNodes = this.resizePreviewNodes;
      const shouldCommitResize = this.resizeMoved && !!resizeNodeId && !!resizePatch;
      this.isResizing = false;
      this.resizeHandle = null;
      this.resizeNodeId = null;
      this.resizePreviewNodes = null;
      this.resizeLatestPatch = null;
      this.resizeLatestScale = null;
      this.resizeMoved = false;
      if (shouldCommitResize) {
        const history = useHistoryStore.getState();
        const docStore = useDocumentStore.getState();
        history.startBatch(docStore.document);
        docStore.updateNode(resizeNodeId!, resizePatch as Partial<PenNode>);
        const resizedNode = docStore.getNodeById(resizeNodeId!);
        if (
          resizeScale &&
          resizedNode &&
          'children' in resizedNode &&
          resizedNode.children?.length &&
          (resizeScale.scaleX !== 1 || resizeScale.scaleY !== 1)
        ) {
          docStore.scaleDescendantsInStore(resizeNodeId!, resizeScale.scaleX, resizeScale.scaleY);
        }
        history.endBatch(useDocumentStore.getState().document);
      } else if (resizePreviewNodes && engine) {
        engine.dragSyncSuppressed = false;
        engine.syncFromDocument();
      }
      this.canvasEl.style.cursor = toolToCursor(this.getTool());
    }

    if (this.isDraggingArc) {
      const arcNodeId = this.arcNodeId;
      const arcPatch = this.arcLatestPatch;
      const arcPreviewNode = this.arcPreviewNode;
      const shouldCommitArc = this.arcMoved && !!arcNodeId && !!arcPatch;
      this.isDraggingArc = false;
      this.arcHandleType = null;
      this.arcNodeId = null;
      this.arcPreviewNode = null;
      this.arcLatestPatch = null;
      this.arcMoved = false;
      if (shouldCommitArc) {
        useDocumentStore.getState().updateNode(arcNodeId!, arcPatch as Partial<PenNode>);
      } else if (arcPreviewNode && engine) {
        engine.dragSyncSuppressed = false;
        engine.syncFromDocument();
      }
      this.canvasEl.style.cursor = toolToCursor(this.getTool());
    }

    if (this.isDraggingPathControl) {
      const patch = this.pathLatestPatch;
      const pathNodeId = this.pathNodeId;
      this.isDraggingPathControl = false;
      this.pathControlType = null;
      this.pathControlAnchorIndex = null;
      this.pathNodeId = null;
      this.pathSceneAnchors = null;
      this.pathParentSceneOrigin = null;
      this.pathLatestPatch = null;
      const shouldCommitPathControl = this.pathControlMoved && !!patch && !!pathNodeId;
      this.pathControlMoved = false;
      if (engine) {
        engine.dragSyncSuppressed = false;
      }
      if (shouldCommitPathControl) {
        useDocumentStore.getState().updateNode(pathNodeId!, patch as Partial<PenNode>);
      }
      this.canvasEl.style.cursor = toolToCursor(this.getTool());
    }

    if (this.isRotating) {
      const rotateNodeId = this.rotateNodeId;
      const rotateAngle = this.rotateLatestAngle;
      const rotatePreviewNodes = this.rotatePreviewNodes;
      const shouldCommitRotate =
        this.rotateMoved && !!rotateNodeId && typeof rotateAngle === 'number';
      this.isRotating = false;
      this.rotateNodeId = null;
      this.rotatePreviewNodes = null;
      this.rotateLatestAngle = null;
      this.rotateMoved = false;
      if (shouldCommitRotate) {
        useDocumentStore
          .getState()
          .updateNode(rotateNodeId!, { rotation: rotateAngle } as Partial<PenNode>);
      } else if (rotatePreviewNodes && engine) {
        engine.dragSyncSuppressed = false;
        engine.syncFromDocument();
      }
      this.canvasEl.style.cursor = toolToCursor(this.getTool());
    }

    if (this.isDrawing && engine?.previewShape) {
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
      return;
    }
    this.isDrawing = false;

    // Select tool: end drag / marquee
    if (this.isDragging && this.dragMoved && this.dragOrigPositions.length > 0 && engine) {
      this.handleDragEnd(engine);
    } else if (engine) {
      engine.dragSyncSuppressed = false;
    }
    this.isDragging = false;
    this.dragNodeIds = [];
    this.dragOrigPositions = [];
    this.dragAllIds = null;
    if (this.isMarquee && engine) {
      engine.marquee = null;
      engine.markDirty();
    }
    this.isMarquee = false;
  };

  private handleDragEnd(engine: SkiaEngine) {
    const dx = this.dragPrevDx;
    const dy = this.dragPrevDy;
    const docStore = useDocumentStore.getState();

    for (const orig of this.dragOrigPositions) {
      const parent = docStore.getParentOf(orig.id);
      const draggedNode = docStore.getNodeById(orig.id);
      const draggedRN = engine.renderNodes.find((rn) => rn.node.id === orig.id);
      const objBounds = draggedRN
        ? { x: draggedRN.absX, y: draggedRN.absY, w: draggedRN.absW, h: draggedRN.absH }
        : { x: orig.x + dx, y: orig.y + dy, w: 100, h: 100 };

      // Check if dragged completely outside parent → reparent
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

          if (outside && shouldAutoReparentOnDragOutsideParent(draggedNode)) {
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
  }

  // ---------------------------------------------------------------------------
  // Double click — text editing
  // ---------------------------------------------------------------------------

  private onDblClick = (e: MouseEvent) => {
    const engine = this.getEngine();
    if (!engine) return;

    if (this.penTool.onDblClick()) return;

    if (this.getTool() !== 'select') return;

    const scene = this.getScene(e);
    if (!scene) return;

    const hits = engine.spatialIndex.hitTest(scene.x, scene.y);
    if (hits.length === 0) return;

    const topHit = hits[0];
    const currentSelection = useCanvasStore.getState().selection.selectedIds;

    // Double-click on a selected group/frame → enter it and select the child
    if (currentSelection.length === 1) {
      const selectedNode = useDocumentStore.getState().getNodeById(currentSelection[0]);
      if (
        selectedNode &&
        (selectedNode.type === 'frame' || selectedNode.type === 'group') &&
        'children' in selectedNode &&
        selectedNode.children?.length
      ) {
        const childId = topHit.node.id;
        if (childId !== currentSelection[0]) {
          useCanvasStore.getState().setSelection([childId], childId);
          return;
        }
      }
    }

    if (topHit.node.type !== 'text') return;

    const tNode = topHit.node as TextNode;
    const fills = tNode.fill;
    const firstFill = Array.isArray(fills) ? fills[0] : undefined;
    const color = firstFill?.type === 'solid' ? firstFill.color : '#000000';

    this.onEditText({
      nodeId: topHit.node.id,
      x: topHit.absX,
      y: topHit.absY,
      w: topHit.absW,
      h: topHit.absH,
      content:
        typeof tNode.content === 'string'
          ? tNode.content
          : Array.isArray(tNode.content)
            ? tNode.content.map((s) => s.text ?? '').join('')
            : '',
      fontSize: tNode.fontSize ?? 16,
      fontFamily:
        tNode.fontFamily ??
        'Inter, -apple-system, "Noto Sans SC", "PingFang SC", system-ui, sans-serif',
      fontWeight: String(tNode.fontWeight ?? '400'),
      textAlign: tNode.textAlign ?? 'left',
      color,
      lineHeight: tNode.lineHeight ?? 1.4,
    });
  };

  // ---------------------------------------------------------------------------
  // Keyboard: space for panning
  // ---------------------------------------------------------------------------

  private onKeyDown = (e: KeyboardEvent) => {
    if (this.penTool.onKeyDown(e.key)) {
      e.preventDefault();
      return;
    }
    if (e.code === 'Space' && !e.repeat) {
      this.spacePressed = true;
      this.canvasEl.style.cursor = 'grab';
    }
  };

  private onKeyUp = (e: KeyboardEvent) => {
    if (e.code === 'Space') {
      this.spacePressed = false;
      this.isPanning = false;
      this.canvasEl.style.cursor = toolToCursor(this.getTool());
    }
  };

  // ---------------------------------------------------------------------------
  // Attach / detach event listeners
  // ---------------------------------------------------------------------------

  attach(): () => void {
    const canvasEl = this.canvasEl;

    const onContextMenu = (e: MouseEvent) => {
      e.preventDefault();

      const engine = this.getEngine();
      if (!engine || this.getTool() !== 'select') return;

      const scene = this.getScene(e);
      if (!scene) return;

      const pathHit = hitTestPathControl(engine, scene.x, scene.y);
      if (!pathHit) {
        this.onPathAnchorContextMenu(null);
        return;
      }

      useCanvasStore.getState().setSelection([pathHit.nodeId], pathHit.nodeId);
      this.onPathAnchorContextMenu({
        x: e.clientX,
        y: e.clientY,
        nodeId: pathHit.nodeId,
        anchorIndex: pathHit.anchorIndex,
      });
    };

    // Tool change → cursor + cancel pen if switching away
    const unsubTool = useCanvasStore.subscribe((state) => {
      if (!this.spacePressed && !this.isResizing)
        canvasEl.style.cursor = toolToCursor(state.activeTool);
      this.penTool.onToolChange(state.activeTool);
    });

    document.addEventListener('keydown', this.onKeyDown);
    document.addEventListener('keyup', this.onKeyUp);
    canvasEl.addEventListener('mousedown', this.onMouseDown);
    canvasEl.addEventListener('dblclick', this.onDblClick);
    canvasEl.addEventListener('contextmenu', onContextMenu);
    window.addEventListener('mousemove', this.onMouseMove);
    window.addEventListener('mouseup', this.onMouseUp);

    return () => {
      document.removeEventListener('keydown', this.onKeyDown);
      document.removeEventListener('keyup', this.onKeyUp);
      canvasEl.removeEventListener('mousedown', this.onMouseDown);
      canvasEl.removeEventListener('dblclick', this.onDblClick);
      canvasEl.removeEventListener('contextmenu', onContextMenu);
      window.removeEventListener('mousemove', this.onMouseMove);
      window.removeEventListener('mouseup', this.onMouseUp);
      unsubTool();
    };
  }
}
