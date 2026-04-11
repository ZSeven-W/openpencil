// packages/pen-renderer/src/render-node-thumbnail.ts
//
// Offscreen thumbnail helper for individual PenNodes. Used by the git conflict
// UI to render side-by-side ours/theirs previews without mounting a full
// PenRenderer instance.
//
// Design goals:
//   - Accept full document context so ref-type nodes resolve correctly.
//   - Return a data-URL string on success, null on any failure (graceful).
//   - Never throw — all errors are caught and converted to null.
//   - CanvasKit is not available in tests / SSR: detect and fall back to null.
//   - The output shape (data URL | null) is stable for test assertions.

import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import { getAllChildren, getDefaultTheme, resolveNodeForCanvas } from '@zseven-w/pen-core';
import { flattenToRenderNodes, resolveRefs } from './document-flattener.js';

export interface ThumbnailContext {
  /** Full document used for ref node resolution. */
  document: PenDocument;
  /** Page id used to locate the node in a multi-page doc (unused today; reserved for future per-page context). */
  pageId: string | null;
  /** Output canvas size in logical pixels (square, default: 128). */
  size?: number;
}

/**
 * Render a single PenNode into a data URL at the requested size.
 *
 * Returns a data URL string when rendering succeeded, or `null` when:
 *   - CanvasKit / OffscreenCanvas is unavailable (Node.js / test env)
 *   - The node or document is invalid
 *   - Any rendering step throws
 *
 * Callers MUST handle the `null` case and display a placeholder.
 */
export async function renderNodeThumbnail(
  node: PenNode,
  ctx: ThumbnailContext,
): Promise<string | null> {
  try {
    // Guard: need a valid node object.
    if (!node || typeof node !== 'object') return null;

    const size = ctx.size ?? 128;
    if (!Number.isFinite(size) || size <= 0) return null;

    // Resolve ref nodes using the root document tree as the component registry.
    // resolveRefs walks the node tree and substitutes `ref` nodes with their
    // component originals. For non-ref nodes it is a shallow identity pass.
    // getAllChildren handles both single-page (document.children) and multi-page
    // (document.pages[i].children) layouts — refs can cross pages so we need all.
    const rootNodes: PenNode[] = ctx.document ? getAllChildren(ctx.document) : [];
    let resolvedNodes: PenNode[];
    try {
      resolvedNodes = resolveRefs([node], rootNodes);
    } catch {
      // If ref resolution fails (e.g. circular ref), fall back to raw node.
      resolvedNodes = [node];
    }

    const resolvedNode = resolvedNodes[0] ?? node;

    // Resolve design $variable references so fill colors, stroke widths, etc.
    // render with their concrete values rather than the raw "$color-primary"
    // strings.  Mirrors the same step in renderer.ts (line ~279).
    const variables = ctx.document?.variables ?? {};
    const themes = ctx.document?.themes;
    const activeTheme = getDefaultTheme(themes);
    const variableResolved = resolveNodeForCanvas(resolvedNode, variables, activeTheme);

    // Flatten to RenderNode array so we have absolute coordinates, auto-layout
    // positions, and text pre-measurements for all descendants.
    let renderNodes;
    try {
      renderNodes = flattenToRenderNodes([variableResolved]);
    } catch {
      return null;
    }

    if (!renderNodes || renderNodes.length === 0) return null;

    // Detect CanvasKit availability — not available in tests or SSR.
    let ck: import('canvaskit-wasm').CanvasKit | null = null;
    try {
      const { getCanvasKit } = await import('./init.js');
      ck = getCanvasKit();
    } catch {
      return null;
    }

    if (!ck) {
      // CanvasKit not initialised yet.
      return null;
    }

    // OffscreenCanvas guard — not available in Node.js.
    if (typeof OffscreenCanvas === 'undefined') return null;

    // Determine scaling: fit the node's bounding box into the requested size.
    const rootRenderNode = renderNodes[0];
    const nodeW = rootRenderNode.absW > 0 ? rootRenderNode.absW : size;
    const nodeH = rootRenderNode.absH > 0 ? rootRenderNode.absH : size;
    const scale = Math.min(size / nodeW, size / nodeH);

    const canvasW = Math.max(1, Math.round(nodeW * scale));
    const canvasH = Math.max(1, Math.round(nodeH * scale));

    // Software SkSurface (no WebGL required — safe for offscreen use).
    const skSurface = ck.MakeSurface(canvasW, canvasH);
    if (!skSurface) return null;

    try {
      const skCanvas = skSurface.getCanvas();
      skCanvas.clear(ck.TRANSPARENT);
      skCanvas.scale(scale, scale);

      const { SkiaNodeRenderer } = await import('./node-renderer.js');
      const renderer = new SkiaNodeRenderer(ck);
      for (const rn of renderNodes) {
        renderer.drawNode(skCanvas, rn);
      }

      skSurface.flush();
      const imgSnapshot = skSurface.makeImageSnapshot();
      if (!imgSnapshot) return null;

      const pngBytes = imgSnapshot.encodeToBytes();
      if (!pngBytes) return null;

      // Convert raw PNG bytes to a data URL via Blob + FileReader.
      const blob = new Blob([pngBytes as Uint8Array<ArrayBuffer>], { type: 'image/png' });
      const dataUrl = await blobToDataUrl(blob);
      return dataUrl;
    } finally {
      skSurface.delete();
    }
  } catch {
    // Any unexpected error → graceful null
    return null;
  }
}

/** Convert a Blob to a data URL using FileReader. */
async function blobToDataUrl(blob: Blob): Promise<string | null> {
  return new Promise<string | null>((resolve) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      resolve(typeof result === 'string' ? result : null);
    };
    reader.onerror = () => resolve(null);
    reader.readAsDataURL(blob);
  });
}
