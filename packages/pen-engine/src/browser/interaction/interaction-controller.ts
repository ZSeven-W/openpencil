import type { DesignEngine } from '../../core/design-engine.js';
import type { ToolType } from '@zseven-w/pen-types';
import { EngineSelectHandler } from './select-handler.js';
import { EngineDrawHandler } from './draw-handler.js';
import { EngineResizeHandler } from './resize-handler.js';
import { EngineArcHandler } from './arc-handler.js';
import { EnginePenToolHandler } from './pen-tool-handler.js';
import { createNodeForTool, isDrawingTool } from '../../core/node-creator.js';

function toolToCursor(tool: ToolType | string): string {
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

/**
 * Bind DOM mouse/keyboard events to drive engine interaction.
 * Returns a detach function that removes all listeners.
 *
 * Extracted from apps/web/src/canvas/skia/skia-interaction.ts.
 * Uses engine API instead of Zustand stores.
 */
export function attachInteraction(
  engine: DesignEngine,
  canvas: HTMLCanvasElement,
  options?: { cursor?: (cursor: string) => void },
): () => void {
  const setCursor =
    options?.cursor ??
    ((c: string) => {
      canvas.style.cursor = c;
    });

  const select = new EngineSelectHandler();
  const draw = new EngineDrawHandler();
  const resize = new EngineResizeHandler();
  const arc = new EngineArcHandler();
  const penTool = new EnginePenToolHandler();

  let isPanning = false;
  let spacePressed = false;
  let lastX = 0;
  let lastY = 0;

  function getScene(e: MouseEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    return engine.screenToScene(e.clientX - rect.left, e.clientY - rect.top);
  }

  function onMouseDown(e: MouseEvent) {
    if (e.button === 2) return;
    const tool = engine.getActiveTool();

    if (spacePressed || tool === 'hand' || e.button === 1) {
      isPanning = true;
      lastX = e.clientX;
      lastY = e.clientY;
      setCursor('grabbing');
      return;
    }

    const scene = getScene(e);

    if (tool === 'text') {
      const node = createNodeForTool('text', scene.x, scene.y, 0, 0);
      if (node) {
        engine.addNode(null, node);
        engine.select([node.id]);
      }
      engine.setActiveTool('select');
      return;
    }

    if (tool === 'path') {
      penTool.onMouseDown(scene, engine.zoom);
      return;
    }

    if (isDrawingTool(tool)) {
      draw.startDrawing(tool, scene);
      return;
    }

    if (tool === 'select') {
      if (arc.startArcDrag(scene, engine)) return;
      select.handleSelectMouseDown(scene, engine);
    }
  }

  function onMouseMove(e: MouseEvent) {
    if (isPanning) {
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      lastX = e.clientX;
      lastY = e.clientY;
      engine.setViewport(engine.zoom, engine.panX + dx, engine.panY + dy);
      return;
    }

    const scene = getScene(e);

    if (penTool.onMouseMove(scene)) return;
    if (resize.isResizing) return;
    if (resize.isRotating) return;
    if (arc.isDraggingArc) {
      arc.handleArcMove(scene, engine);
      return;
    }
    if (draw.isDrawing) {
      draw.handleDrawingMove(scene);
      return;
    }
    if (select.isDragging) {
      select.handleDragMove(scene, engine);
      return;
    }
    if (select.isMarquee) {
      select.handleMarqueeMove(scene, engine);
      return;
    }

    // Hover hit test (result available for future use)
    if (engine.getActiveTool() === 'select' && !spacePressed) {
      engine.hitTest(scene.x, scene.y);
    }
  }

  function onMouseUp() {
    const tool = engine.getActiveTool();
    if (penTool.onMouseUp()) return;

    if (isPanning) {
      isPanning = false;
      setCursor(spacePressed ? 'grab' : toolToCursor(tool));
    }

    resize.resetResize();
    resize.resetRotation();
    arc.resetArc();

    if (draw.isDrawing) {
      draw.finishDrawing(engine);
      return;
    }

    select.handleDragEnd();
    select.resetMarquee();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (penTool.onKeyDown(e.key)) {
      e.preventDefault();
      return;
    }
    if (e.code === 'Space' && !e.repeat) {
      spacePressed = true;
      setCursor('grab');
    }
  }

  function onKeyUp(e: KeyboardEvent) {
    if (e.code === 'Space') {
      spacePressed = false;
      isPanning = false;
      setCursor(toolToCursor(engine.getActiveTool()));
    }
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
  }

  // Subscribe to tool changes
  const unsubTool = engine.on('tool:change', (tool) => {
    if (!spacePressed && !resize.isResizing) {
      setCursor(toolToCursor(tool));
    }
  });

  document.addEventListener('keydown', onKeyDown);
  document.addEventListener('keyup', onKeyUp);
  canvas.addEventListener('mousedown', onMouseDown);
  canvas.addEventListener('contextmenu', onContextMenu);
  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);

  return () => {
    document.removeEventListener('keydown', onKeyDown);
    document.removeEventListener('keyup', onKeyUp);
    canvas.removeEventListener('mousedown', onMouseDown);
    canvas.removeEventListener('contextmenu', onContextMenu);
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('mouseup', onMouseUp);
    unsubTool();
  };
}
