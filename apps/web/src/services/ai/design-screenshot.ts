/**
 * Screenshot capture utilities for design validation.
 *
 * Backed by SkiaEngine.captureRegion() which does a CanvasKit readPixels
 * on the live canvas. Only usable from the web side (not from pen-mcp —
 * see Phase 2 for the RPC-based external API).
 */

import { getSkiaEngineRef } from '@/canvas/skia-engine-ref';
import { useDocumentStore } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

function uint8ToBase64(bytes: Uint8Array): string {
  let binary = '';
  const len = bytes.length;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function computeBounds(node: PenNode): { x: number; y: number; w: number; h: number } {
  const n = node as unknown as { x?: number; y?: number; width?: number; height?: number };
  return {
    x: n.x ?? 0,
    y: n.y ?? 0,
    w: n.width ?? 100,
    h: n.height ?? 100,
  };
}

/**
 * Capture a screenshot of a specific node. Returns a base64 PNG data URL,
 * or null if the canvas isn't ready or the node doesn't exist.
 */
export async function captureNodeScreenshot(nodeId: string): Promise<string | null> {
  const engine = getSkiaEngineRef();
  if (!engine) return null;

  const node = useDocumentStore.getState().getNodeById(nodeId);
  if (!node) return null;

  const bounds = computeBounds(node);
  const png = await engine.captureRegion(bounds);
  if (!png) return null;

  return `data:image/png;base64,${uint8ToBase64(png)}`;
}

/**
 * Capture a screenshot of the entire document root frame.
 * Returns a base64 PNG data URL, or null if canvas isn't ready.
 */
export async function captureRootFrameScreenshot(): Promise<string | null> {
  const engine = getSkiaEngineRef();
  if (!engine) return null;
  const png = await engine.captureRegion('root');
  if (!png) return null;
  return `data:image/png;base64,${uint8ToBase64(png)}`;
}
