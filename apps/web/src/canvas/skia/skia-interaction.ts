import { screenToScene } from './skia-engine';
import type { SkiaEngine } from './skia-engine';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { createNodeForTool, isDrawingTool } from '../canvas-node-creator';
import { SkiaPenTool } from './skia-pen-tool';
import type { TextNode } from '@/types/pen';
import {
  createInteractionContext,
  toolToCursor,
  type TextEditState,
} from './skia-interaction-types';
import { SelectHandler } from './skia-interaction-select';
import { ResizeRotateHandler } from './skia-interaction-resize';
import { DrawHandler } from './skia-interaction-draw';
import { ArcHandler } from './skia-interaction-arc';

export { toolToCursor, type TextEditState } from './skia-interaction-types';

/**
 * Coordinates all canvas mouse/keyboard interactions.
 * Delegates to focused handler classes for each interaction mode.
 * Keeps panning, text creation, and event listener registration inline.
 */
export class SkiaInteractionManager {
  private engineRef: { current: SkiaEngine | null };
  private canvasEl: HTMLCanvasElement;
  private onEditText: (state: TextEditState | null) => void;

  // Handlers
  private select: SelectHandler;
  private resizeRotate: ResizeRotateHandler;
  private draw: DrawHandler;
  private arc: ArcHandler;
  private penTool: SkiaPenTool;

  // Pan state (shared across all modes)
  private isPanning = false;
  private spacePressed = false;
  private lastX = 0;
  private lastY = 0;

  constructor(
    engineRef: { current: SkiaEngine | null },
    canvasEl: HTMLCanvasElement,
    onEditText: (state: TextEditState | null) => void,
  ) {
    this.engineRef = engineRef;
    this.canvasEl = canvasEl;
    this.onEditText = onEditText;

    const ctx = createInteractionContext(engineRef, canvasEl);
    this.resizeRotate = new ResizeRotateHandler(ctx);
    this.arc = new ArcHandler(ctx);
    this.select = new SelectHandler(ctx, this.resizeRotate, this.arc);
    this.draw = new DrawHandler();
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
  // Mouse down — dispatch to the active mode
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
      this.draw.startDrawing(tool, scene, engine);
      return;
    }

    // Select tool
    if (tool === 'select') {
      this.select.handleSelectMouseDown(e, scene, engine);
    }
  };

  // ---------------------------------------------------------------------------
  // Mouse move — delegate to the active handler
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

    if (this.resizeRotate.isResizing) {
      this.resizeRotate.handleResizeMove(scene, engine);
      return;
    }
    if (this.resizeRotate.isRotating) {
      this.resizeRotate.handleRotateMove(scene, e.shiftKey);
      return;
    }
    if (this.arc.isDraggingArc) {
      this.arc.handleArcMove(scene, engine);
      return;
    }
    if (this.draw.isDrawing && engine.previewShape) {
      this.draw.handleDrawingMove(scene, engine);
      return;
    }
    if (this.select.isDragging) {
      this.select.handleDragMove(scene, engine);
      return;
    }
    if (this.select.isMarquee && engine.marquee) {
      this.select.handleMarqueeMove(scene, engine);
      return;
    }

    // Hover cursor (select tool only)
    if (this.getTool() === 'select' && !this.spacePressed) {
      this.select.handleHoverCursor(scene, engine);
    }
  };

  // ---------------------------------------------------------------------------
  // Mouse up — delegate cleanup to the active handler
  // ---------------------------------------------------------------------------

  private onMouseUp = () => {
    const engine = this.getEngine();
    const tool = this.getTool();

    if (this.penTool.onMouseUp()) return;

    if (this.isPanning) {
      this.isPanning = false;
      this.canvasEl.style.cursor = this.spacePressed ? 'grab' : toolToCursor(tool);
    }

    this.resizeRotate.resetResize(tool);
    this.resizeRotate.resetRotation(tool);
    this.arc.resetArc(tool);

    if (this.draw.isDrawing && engine) {
      this.draw.finishDrawing(engine);
      return;
    }

    // Select tool: end drag / marquee
    if (engine) {
      this.select.handleDragEnd(engine);
      this.select.resetMarquee(engine);
    } else {
      this.select.resetDrag();
    }
  };

  // ---------------------------------------------------------------------------
  // Double click — text editing + group enter
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

    // Double-click on a selected group/frame -> enter it and select the child
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
      x: topHit.absX * engine.zoom + engine.panX,
      y: topHit.absY * engine.zoom + engine.panY,
      w: topHit.absW * engine.zoom,
      h: topHit.absH * engine.zoom,
      content:
        typeof tNode.content === 'string'
          ? tNode.content
          : Array.isArray(tNode.content)
            ? tNode.content.map((s) => s.text ?? '').join('')
            : '',
      fontSize: (tNode.fontSize ?? 16) * engine.zoom,
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
    const onContextMenu = (e: MouseEvent) => e.preventDefault();

    // Tool change -> cursor + cancel pen if switching away
    const unsubTool = useCanvasStore.subscribe((state) => {
      if (!this.spacePressed && !this.resizeRotate.isResizing)
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
