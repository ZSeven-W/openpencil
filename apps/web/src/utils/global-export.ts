/**
 * Global canvas export — render whole pages or the entire document
 * to PNG / JPEG / WEBP / PDF.
 *
 * Reuses the active SkiaEngine's CanvasKit instance + node renderer so
 * the export visually matches what the user sees on screen. PDF output
 * is a minimal raster PDF (one JPEG page per design page) — no external
 * library required.
 */

import type { PenNode, PenDocument } from '@zseven-w/pen-types';
import { resolveRefs, premeasureTextHeights, flattenToRenderNodes } from '@zseven-w/pen-renderer';
import { resolveNodeForCanvas, getDefaultTheme } from '@zseven-w/pen-core';
import { getSkiaEngineRef } from '@/canvas/skia-engine-ref';

export type ImageExportFormat = 'png' | 'jpeg' | 'webp';
export type GlobalExportFormat = ImageExportFormat | 'pdf';

interface PageRender {
  /** Page name (used in filenames). */
  name: string;
  /**
   * Encoded image bytes (already copied out of the WASM heap).
   * Typed with `<ArrayBuffer>` so it satisfies the `BlobPart` constraint
   * in TS 5.7+, which rejects the wider `Uint8Array<ArrayBufferLike>`.
   */
  bytes: Uint8Array<ArrayBuffer>;
  /** Pixel dimensions of the encoded image. */
  width: number;
  height: number;
  /** Logical dimensions in design units (used as PDF MediaBox). */
  logicalWidth: number;
  logicalHeight: number;
}

interface RenderPageOptions {
  multiplier: number;
  format: ImageExportFormat;
  /** When 'white', clears the surface with white instead of transparent. */
  background?: 'transparent' | 'white';
}

/** Decode a base64 data URL like `data:image/jpeg;base64,...` into raw bytes. */
function dataUrlToBytes(dataUrl: string): Uint8Array<ArrayBuffer> | null {
  const comma = dataUrl.indexOf(',');
  if (comma < 0) return null;
  const base64 = dataUrl.slice(comma + 1);
  try {
    const bin = atob(base64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  } catch (err) {
    console.error('[global-export] Failed to decode data URL:', err);
    return null;
  }
}

/**
 * Render a single page to encoded image bytes.
 * Returns null if the SkiaEngine isn't available, the page is empty,
 * or surface allocation/encoding fails.
 *
 * Uses `MakeSWCanvasSurface` on a temporary `<canvas>` element rather than
 * `MakeSurface(w,h)` because the editor falls back to the SW path when WebGL
 * is unavailable, so it's the most reliable cross-build option. Encoding goes
 * through the browser's native `canvas.toDataURL` (which always supports
 * PNG/JPEG/WEBP) instead of `Image.encodeToBytes`, which has been observed to
 * return null in some CanvasKit builds.
 */
function renderPageToImage(
  pageChildren: PenNode[],
  doc: PenDocument,
  opts: RenderPageOptions,
): PageRender | null {
  const engine = getSkiaEngineRef();
  if (!engine) {
    console.error('[global-export] SkiaEngine not available');
    return null;
  }
  const ck = engine.ck;

  // Mirror SkiaEngine.syncFromDocument so the export matches the on-screen render.
  const allNodes: PenNode[] =
    doc.pages && doc.pages.length > 0 ? doc.pages.flatMap((p) => p.children) : doc.children;
  const resolved = resolveRefs(pageChildren, allNodes);
  const variables = doc.variables ?? {};
  const defaultTheme = getDefaultTheme(doc.themes);
  const variableResolved = resolved.map((n) => resolveNodeForCanvas(n, variables, defaultTheme));
  const measured = premeasureTextHeights(variableResolved);
  const renderNodes = flattenToRenderNodes(measured);
  if (renderNodes.length === 0) {
    console.warn('[global-export] Page has no visible nodes');
    return null;
  }

  // Bounding box from root-level nodes (those without an inherited clipRect).
  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;
  for (const rn of renderNodes) {
    if (rn.clipRect) continue;
    if (rn.absX < minX) minX = rn.absX;
    if (rn.absY < minY) minY = rn.absY;
    if (rn.absX + rn.absW > maxX) maxX = rn.absX + rn.absW;
    if (rn.absY + rn.absH > maxY) maxY = rn.absY + rn.absH;
  }
  if (!isFinite(minX)) {
    console.warn('[global-export] Could not compute page bounding box');
    return null;
  }

  const logicalW = Math.max(1, Math.ceil(maxX - minX));
  const logicalH = Math.max(1, Math.ceil(maxY - minY));
  const outW = Math.max(1, Math.ceil(logicalW * opts.multiplier));
  const outH = Math.max(1, Math.ceil(logicalH * opts.multiplier));

  // Create a temporary canvas + Skia software surface backed by it.
  // After flush(), the rendered pixels live in the canvas's 2D context,
  // accessible via toDataURL.
  const offCanvas = document.createElement('canvas');
  offCanvas.width = outW;
  offCanvas.height = outH;

  const surface = ck.MakeSWCanvasSurface(offCanvas);
  if (!surface) {
    console.error('[global-export] MakeSWCanvasSurface failed');
    return null;
  }

  try {
    const canvas = surface.getCanvas();
    const wantsBg = opts.background === 'white' || opts.format === 'jpeg';
    canvas.clear(wantsBg ? ck.WHITE : ck.TRANSPARENT);

    canvas.save();
    canvas.scale(opts.multiplier, opts.multiplier);
    canvas.translate(-minX, -minY);
    for (const rn of renderNodes) {
      engine.renderer.drawNode(canvas, rn);
    }
    canvas.restore();
    surface.flush();
  } finally {
    surface.delete();
  }

  // Encode via the browser's native canvas encoder.
  const mimeType =
    opts.format === 'jpeg' ? 'image/jpeg' : opts.format === 'webp' ? 'image/webp' : 'image/png';
  const quality = opts.format === 'png' ? undefined : 0.92;
  let dataUrl: string;
  try {
    dataUrl = offCanvas.toDataURL(mimeType, quality);
  } catch (err) {
    console.error('[global-export] toDataURL failed:', err);
    return null;
  }
  if (!dataUrl || dataUrl === 'data:,') {
    console.error('[global-export] toDataURL returned empty result');
    return null;
  }
  const bytes = dataUrlToBytes(dataUrl);
  if (!bytes) return null;

  return {
    name: '',
    bytes,
    width: outW,
    height: outH,
    logicalWidth: logicalW,
    logicalHeight: logicalH,
  };
}

/** List the pages of a document — falls back to a single legacy page. */
function listPages(doc: PenDocument): { id: string; name: string; children: PenNode[] }[] {
  if (doc.pages && doc.pages.length > 0) {
    return doc.pages.map((p) => ({ id: p.id, name: p.name || 'Page', children: p.children }));
  }
  return [{ id: '__legacy__', name: 'Page 1', children: doc.children }];
}

/**
 * Sanitize a string for use as a filename. Allows letters, digits, hyphen,
 * underscore, and CJK characters; collapses everything else to underscores.
 */
export function sanitizeFilename(name: string, fallback = 'untitled'): string {
  const safe = (name || '').replace(/[^\p{L}\p{N}_-]+/gu, '_').replace(/^_+|_+$/g, '');
  return safe || fallback;
}

/**
 * Export the active page as an image (PNG/JPEG/WEBP).
 * Returns null on failure.
 */
export function exportActivePageImage(
  doc: PenDocument,
  activePageId: string | null,
  format: ImageExportFormat,
  multiplier = 1,
): { blob: Blob; ext: string; baseName: string } | null {
  const pages = listPages(doc);
  const page = pages.find((p) => p.id === activePageId) ?? pages[0];
  if (!page) return null;

  const result = renderPageToImage(page.children, doc, { multiplier, format });
  if (!result) return null;

  const mime = format === 'jpeg' ? 'image/jpeg' : format === 'webp' ? 'image/webp' : 'image/png';
  const ext = format === 'jpeg' ? 'jpg' : format;
  return {
    blob: new Blob([result.bytes], { type: mime }),
    ext,
    baseName: sanitizeFilename(page.name, 'page'),
  };
}

/**
 * Export the entire document as a multi-page raster PDF.
 * Each page renders to JPEG and is embedded as an /XObject /Image with
 * /DCTDecode filter — no external PDF library needed.
 */
export function exportDocumentPdf(doc: PenDocument, multiplier = 2): Blob | null {
  const pages = listPages(doc);
  const renders: PageRender[] = [];
  for (const p of pages) {
    const r = renderPageToImage(p.children, doc, {
      multiplier,
      format: 'jpeg',
      background: 'white',
    });
    if (r) {
      r.name = p.name;
      renders.push(r);
    }
  }
  if (renders.length === 0) return null;
  return buildRasterPdf(renders);
}

/**
 * Build a minimal PDF embedding one JPEG image per page.
 *
 * Object layout:
 *   1: Catalog
 *   2: Pages root
 *   3, 4, 5: page 1 (Page, Image, Contents)
 *   6, 7, 8: page 2 ...
 */
function buildRasterPdf(pages: PageRender[]): Blob {
  const enc = new TextEncoder();
  const chunks: Uint8Array[] = [];
  let length = 0;
  const offsets: number[] = [];

  const push = (data: Uint8Array | string) => {
    const u = typeof data === 'string' ? enc.encode(data) : data;
    chunks.push(u);
    length += u.length;
  };
  const startObj = (id: number) => {
    offsets[id] = length;
    push(`${id} 0 obj\n`);
  };
  const endObj = () => push('\nendobj\n');

  // Header — version + binary marker (raw bytes, not UTF-8).
  push('%PDF-1.4\n');
  push(new Uint8Array([0x25, 0xff, 0xff, 0xff, 0xff, 0x0a]));

  // 1: Catalog
  startObj(1);
  push('<< /Type /Catalog /Pages 2 0 R >>');
  endObj();

  // Allocate page object IDs first so the Pages root can reference them.
  const ids = pages.map((_, i) => ({
    page: 3 + i * 3,
    img: 3 + i * 3 + 1,
    content: 3 + i * 3 + 2,
  }));
  const totalObjects = 2 + pages.length * 3;

  // 2: Pages root
  startObj(2);
  push(
    `<< /Type /Pages /Count ${pages.length} /Kids [${ids.map((id) => `${id.page} 0 R`).join(' ')}] >>`,
  );
  endObj();

  // Per-page objects
  for (let i = 0; i < pages.length; i++) {
    const p = pages[i];
    const W = p.logicalWidth;
    const H = p.logicalHeight;
    const { page: pageId, img: imgId, content: contentId } = ids[i];

    // Page
    startObj(pageId);
    push(
      `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${W} ${H}] ` +
        `/Resources << /XObject << /Im0 ${imgId} 0 R >> >> ` +
        `/Contents ${contentId} 0 R >>`,
    );
    endObj();

    // Image XObject — JPEG via DCTDecode
    startObj(imgId);
    push(
      `<< /Type /XObject /Subtype /Image /Width ${p.width} /Height ${p.height} ` +
        `/ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode ` +
        `/Length ${p.bytes.length} >>\nstream\n`,
    );
    push(p.bytes);
    push('\nendstream');
    endObj();

    // Contents — draw the image at full page size.
    // PDF user space origin is bottom-left, so the CTM "W 0 0 H 0 0 cm" places
    // the unit-square image at (0,0) sized W×H. JPEGs in DCTDecode are decoded
    // top-down by PDF, so this gives the expected orientation.
    const contentStr = `q ${W} 0 0 ${H} 0 0 cm /Im0 Do Q`;
    const contentBytes = enc.encode(contentStr);
    startObj(contentId);
    push(`<< /Length ${contentBytes.length} >>\nstream\n`);
    push(contentBytes);
    push('\nendstream');
    endObj();
  }

  // xref
  const xrefOffset = length;
  push(`xref\n0 ${totalObjects + 1}\n`);
  push('0000000000 65535 f \n');
  for (let id = 1; id <= totalObjects; id++) {
    const off = offsets[id] ?? 0;
    push(`${off.toString().padStart(10, '0')} 00000 n \n`);
  }

  // Trailer
  push(`trailer << /Size ${totalObjects + 1} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`);

  // Concatenate
  const out = new Uint8Array(length);
  let pos = 0;
  for (const c of chunks) {
    out.set(c, pos);
    pos += c.length;
  }
  return new Blob([out], { type: 'application/pdf' });
}

/** Trigger a download of a Blob with the given filename. */
export function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
