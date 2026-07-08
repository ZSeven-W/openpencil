import { createElement, useEffect, useRef, useState, type ReactNode } from 'react';
import { createViewer, type OpViewer } from '@zseven-w/op-web-sdk';
import { DesignProvider } from './use-viewer.js';

export interface DesignViewProps {
  /** Serialized document JSON string or raw bytes to load on mount. */
  doc?: string | Uint8Array;
  /** URL to the WASM bundle; omit to use the bundled default. */
  wasmUrl?: string;
  /** Called once after the viewer is created and ready. */
  onLoad?: (viewer: OpViewer) => void;
  /** Optional CSS class applied to the wrapper div. */
  className?: string;
  children?: ReactNode;
}

/**
 * Renders a canvas, creates an OpViewer on mount, and exposes it to
 * descendant hooks via DesignProvider. Destroys the viewer on unmount.
 * Handles the async race: if unmount occurs before createViewer resolves,
 * the late-arriving viewer is destroyed immediately.
 */
export function DesignView(props: DesignViewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [viewer, setViewer] = useState<OpViewer | null>(null);

  useEffect(() => {
    let cancelled = false;
    // Track the viewer that was synchronously assigned so the cleanup
    // path can destroy it even if setViewer batching hasn't flushed.
    let made: OpViewer | null = null;
    const canvas = canvasRef.current;
    if (!canvas) return;

    createViewer({ canvas, doc: props.doc, wasmUrl: props.wasmUrl }).then((v) => {
      if (cancelled) {
        // Component unmounted before the promise resolved — destroy immediately.
        v.destroy();
        return;
      }
      made = v;
      setViewer(v);
      props.onLoad?.(v);
    });

    return () => {
      cancelled = true;
      // Destroy synchronously if the viewer was already assigned.
      made?.destroy();
    };
    // Re-create when doc identity or wasmUrl changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.doc, props.wasmUrl]);

  return createElement(
    'div',
    { className: props.className },
    createElement('canvas', { ref: canvasRef, style: { width: '100%', height: '100%', display: 'block' } }),
    // Only render children once the viewer is ready.
    viewer ? createElement(DesignProvider, { viewer }, props.children) : null,
  );
}
