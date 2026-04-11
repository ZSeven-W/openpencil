import { useCallback, useEffect, useRef, useState } from 'react';
import { loadCanvasKit } from './skia-init';
import { SkiaEngine } from './skia-engine';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { setSkiaEngineRef } from '../skia-engine-ref';
import type { PathNode, PenNode, PenPathPointType } from '@/types/pen';
import {
  bakeSceneAnchorsToPathNode,
  getEditablePathState,
  resetPathPointHandles,
  setPathPointType,
} from './path-editing';
import {
  SkiaInteractionManager,
  type PathAnchorContextMenuState,
  type TextEditState,
} from './skia-interaction';
import { createDocumentSyncScheduler } from './document-sync-scheduler';
import { projectTextEditStateToViewport } from './text-edit-overlay';

export default function SkiaCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const engineRef = useRef<SkiaEngine | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editingText, setEditingText] = useState<TextEditState | null>(null);
  const [pathContextMenu, setPathContextMenu] = useState<PathAnchorContextMenuState | null>(null);
  const viewport = useCanvasStore((state) => state.viewport);
  const editingTextOverlay = editingText
    ? projectTextEditStateToViewport(editingText, viewport)
    : null;

  const closePathContextMenu = useCallback(() => {
    setPathContextMenu(null);
  }, []);

  const updatePathAnchor = useCallback(
    (menuState: PathAnchorContextMenuState, action: PenPathPointType | 'reset') => {
      const engine = engineRef.current;
      if (!engine) return;

      const rn = engine.spatialIndex.get(menuState.nodeId);
      if (!rn || rn.node.type !== 'path') return;

      const node = rn.node as PathNode;
      const state = getEditablePathState(node, {
        x: rn.absX,
        y: rn.absY,
        width: rn.absW,
        height: rn.absH,
      });
      if (!state) return;

      const nextSceneAnchors =
        action === 'reset'
          ? resetPathPointHandles(state.sceneAnchors, menuState.anchorIndex, state.closed)
          : setPathPointType(state.sceneAnchors, menuState.anchorIndex, action, state.closed);

      const parentSceneOrigin = {
        x: rn.absX - (node.x ?? 0),
        y: rn.absY - (node.y ?? 0),
      };
      const patch = bakeSceneAnchorsToPathNode(nextSceneAnchors, state.closed, parentSceneOrigin);
      if (!patch) return;

      useDocumentStore.getState().updateNode(menuState.nodeId, patch as Partial<PenNode>);
      useCanvasStore.getState().setSelection([menuState.nodeId], menuState.nodeId);
    },
    [],
  );

  // Initialize CanvasKit + engine
  useEffect(() => {
    let disposed = false;

    async function init() {
      try {
        const ck = await loadCanvasKit();
        if (disposed) return;

        const canvasEl = canvasRef.current;
        if (!canvasEl) return;

        const engine = new SkiaEngine(ck);
        engine.init(canvasEl);
        engineRef.current = engine;
        setSkiaEngineRef(engine);

        // Initial sync
        engine.syncFromDocument();
        requestAnimationFrame(() => engine.zoomToFitContent());
      } catch (err) {
        console.error('SkiaCanvas init failed:', err);
        setError(String(err));
      }
    }

    init();

    return () => {
      disposed = true;
      setSkiaEngineRef(null);
      engineRef.current?.dispose();
      engineRef.current = null;
    };
  }, []);

  // Resize observer
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const observer = new ResizeObserver((entries) => {
      const engine = engineRef.current;
      if (!engine) return;
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        engine.resize(width, height);
      }
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  // Document sync: re-render when document changes
  useEffect(() => {
    const scheduler = createDocumentSyncScheduler(() => engineRef.current);
    const unsub = useDocumentStore.subscribe(() => {
      scheduler.schedule();
    });
    return () => {
      unsub();
      scheduler.dispose();
    };
  }, []);

  // Page sync: re-render when active page changes
  useEffect(() => {
    let prevPageId = useCanvasStore.getState().activePageId;
    const unsub = useCanvasStore.subscribe((state) => {
      if (state.activePageId !== prevPageId) {
        prevPageId = state.activePageId;
        engineRef.current?.syncFromDocument();
      }
    });
    return unsub;
  }, []);

  // Selection sync: re-render when selection changes
  useEffect(() => {
    let prevIds = useCanvasStore.getState().selection.selectedIds;
    const unsub = useCanvasStore.subscribe((state) => {
      if (state.selection.selectedIds !== prevIds) {
        prevIds = state.selection.selectedIds;
        engineRef.current?.markDirty();
      }
    });
    return unsub;
  }, []);

  // Wheel: zoom + pan
  useEffect(() => {
    const canvasEl = canvasRef.current;
    if (!canvasEl) return;

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const engine = engineRef.current;
      if (!engine) return;

      if (e.ctrlKey || e.metaKey) {
        let delta = -e.deltaY;
        if (e.deltaMode === 1) delta *= 40;
        const factor = Math.pow(1.005, delta);
        const newZoom = engine.zoom * factor;
        engine.zoomToPoint(e.clientX, e.clientY, newZoom);
      } else {
        let dx = -e.deltaX;
        let dy = -e.deltaY;
        if (e.deltaMode === 1) {
          dx *= 40;
          dy *= 40;
        }
        engine.pan(dx, dy);
      }
    };

    canvasEl.addEventListener('wheel', handleWheel, { passive: false });
    return () => canvasEl.removeEventListener('wheel', handleWheel);
  }, []);

  // Mouse/keyboard interactions (select, move, resize, draw, hover, etc.)
  useEffect(() => {
    const canvasEl = canvasRef.current;
    if (!canvasEl) return;

    const manager = new SkiaInteractionManager(
      engineRef,
      canvasEl,
      setEditingText,
      setPathContextMenu,
    );
    return manager.attach();
  }, []);

  return (
    <div ref={containerRef} className="flex-1 relative overflow-hidden bg-muted">
      <canvas ref={canvasRef} className="absolute inset-0 w-full h-full" />
      {editingText && (
        <textarea
          autoFocus
          defaultValue={editingText.content}
          style={{
            position: 'absolute',
            left: editingTextOverlay?.left,
            top: editingTextOverlay?.top,
            width: editingTextOverlay?.width,
            minHeight: editingTextOverlay?.minHeight,
            fontSize: editingTextOverlay?.fontSize,
            fontFamily: editingText.fontFamily,
            fontWeight: editingText.fontWeight,
            textAlign: editingText.textAlign as CanvasTextAlign,
            color: editingText.color,
            lineHeight: editingText.lineHeight,
            background: 'rgba(255,255,255,0.9)',
            border: '2px solid #0d99ff',
            borderRadius: 2,
            outline: 'none',
            resize: 'none',
            padding: '0 1px',
            margin: 0,
            overflow: 'hidden',
            zIndex: 10,
            boxSizing: 'border-box',
          }}
          onBlur={(e) => {
            const newContent = e.target.value;
            if (newContent !== editingText.content) {
              useDocumentStore
                .getState()
                .updateNode(editingText.nodeId, { content: newContent } as Partial<PenNode>);
            }
            setEditingText(null);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              setEditingText(null);
            }
            e.stopPropagation();
          }}
        />
      )}

      {pathContextMenu && (
        <PathAnchorContextMenu
          x={pathContextMenu.x}
          y={pathContextMenu.y}
          onAction={(action) => {
            updatePathAnchor(pathContextMenu, action);
            closePathContextMenu();
          }}
          onClose={closePathContextMenu}
        />
      )}

      {error && (
        <div className="absolute inset-0 flex items-center justify-center text-destructive">
          Failed to load CanvasKit: {error}
        </div>
      )}
    </div>
  );
}

function PathAnchorContextMenu({
  x,
  y,
  onAction,
  onClose,
}: {
  x: number;
  y: number;
  onAction: (action: PenPathPointType | 'reset') => void;
  onClose: () => void;
}) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleMouseDown = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };

    document.addEventListener('mousedown', handleMouseDown);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleMouseDown);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [onClose]);

  const items: Array<{ action: PenPathPointType | 'reset'; label: string }> = [
    { action: 'corner', label: 'Corner Point' },
    { action: 'mirrored', label: 'Symmetric Curve' },
    { action: 'independent', label: 'Free Curve' },
    { action: 'reset', label: 'Reset Handles' },
  ];

  return (
    <div
      ref={menuRef}
      className="fixed z-50 min-w-[160px] rounded-md border border-border bg-popover py-1 shadow-md"
      style={{ left: x, top: y }}
    >
      {items.map((item) => (
        <button
          key={item.action}
          type="button"
          className="w-full px-3 py-1.5 text-left text-xs text-popover-foreground transition-colors hover:bg-accent"
          onClick={() => onAction(item.action)}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
