import { useRef, useEffect, useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import { attachCanvas, attachInteraction } from '@zseven-w/pen-engine/browser';
import type { CanvasBinding } from '@zseven-w/pen-engine/browser';
import { useDesignEngine } from '../hooks/use-design-engine.js';

export interface DesignCanvasProps {
  className?: string;
  onReady?: (engine: ReturnType<typeof useDesignEngine>) => void;
  loadingFallback?: ReactNode;
}

function DefaultLoadingSpinner() {
  return (
    <div className="absolute inset-0 flex items-center justify-center bg-background/50">
      <div className="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin" />
    </div>
  );
}

/**
 * Canvas component that uses pen-engine browser adapter for rendering and interaction.
 *
 * Uses:
 * - `attachCanvas(engine, canvasElement)` for CanvasKit/Skia GPU rendering
 * - `attachInteraction(engine, canvasElement)` for mouse/keyboard event binding
 *
 * Does NOT call engine.init() directly — all canvas/DOM binding goes through
 * the browser adapter functions.
 */
export function DesignCanvas({ className, onReady, loadingFallback }: DesignCanvasProps) {
  const engine = useDesignEngine();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const bindingRef = useRef<CanvasBinding | null>(null);
  const [loading, setLoading] = useState(true);

  // Attach CanvasKit rendering via browser adapter
  useEffect(() => {
    let disposed = false;
    const canvas = canvasRef.current;
    if (!canvas) return;

    attachCanvas(engine, canvas).then((binding) => {
      if (disposed) {
        binding.dispose();
        return;
      }
      bindingRef.current = binding;
      setLoading(false);
      binding.render();

      // Zoom to fit document content after initial render.
      // getContentBounds() returns the bounding box of all nodes on the
      // active page (or null if empty). We pass this to zoomToRect along
      // with the container dimensions so the viewport centers on the content.
      const container = containerRef.current;
      if (container) {
        const { width, height } = container.getBoundingClientRect();
        const bounds = engine.getContentBounds();
        if (bounds) {
          engine.zoomToRect(bounds.x, bounds.y, bounds.w, bounds.h, width, height);
        }
      }
      onReady?.(engine);
    });

    return () => {
      disposed = true;
      bindingRef.current?.dispose();
      bindingRef.current = null;
    };
  }, [engine]);

  // Resize observer -> browser adapter binding
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        bindingRef.current?.resize(width, height);
      }
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  // Attach mouse/keyboard interaction via browser adapter
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    return attachInteraction(engine, canvas);
  }, [engine]);

  // Wheel zoom/pan -> engine core API (pure math)
  const handleWheel = useCallback(
    (e: WheelEvent) => {
      e.preventDefault();
      if (e.ctrlKey || e.metaKey) {
        let delta = -e.deltaY;
        if (e.deltaMode === 1) delta *= 40;
        const factor = Math.pow(1.005, delta);
        const rect = canvasRef.current?.getBoundingClientRect();
        const mx = e.clientX - (rect?.left ?? 0);
        const my = e.clientY - (rect?.top ?? 0);
        const newZoom = engine.zoom * factor;
        const newPanX = mx - (mx - engine.panX) * (newZoom / engine.zoom);
        const newPanY = my - (my - engine.panY) * (newZoom / engine.zoom);
        engine.setViewport(newZoom, newPanX, newPanY);
      } else {
        let dx = -e.deltaX;
        let dy = -e.deltaY;
        if (e.deltaMode === 1) {
          dx *= 40;
          dy *= 40;
        }
        engine.setViewport(engine.zoom, engine.panX + dx, engine.panY + dy);
      }
    },
    [engine],
  );

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    canvas.addEventListener('wheel', handleWheel, { passive: false });
    return () => canvas.removeEventListener('wheel', handleWheel);
  }, [handleWheel]);

  const containerClass = className
    ? `relative overflow-hidden ${className}`
    : 'relative overflow-hidden';

  return (
    <div ref={containerRef} className={containerClass}>
      <canvas ref={canvasRef} className="absolute inset-0 w-full h-full" />
      {loading && (loadingFallback ?? <DefaultLoadingSpinner />)}
    </div>
  );
}
