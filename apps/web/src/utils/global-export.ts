/**
 * Global 画布导出
 * — 将整个页面或整个文档渲染为 PNG / JPEG / WEBP / PDF。 Reuses 活动 SkiaEngine 的
 *
 * CanvasKit 实例 + 节点渲染器，以便导出在视
 * 觉上与用户在屏幕上看到的
 * 内容相匹配。 PDF 输出是最小光栅 PDF（每个设计页一个 JPEG 页）——无需外部库。
 *
 */

import type { PenNode, PenDocument } from '@zseven-w/pen-types';
import { resolveRefs, premeasureTextHeights, flattenToRenderNodes } from '@zseven-w/pen-renderer';
import { resolveNodeForCanvas, getDefaultTheme } from '@zseven-w/pen-core';
import { getSkiaEngineRef } from '@/canvas/skia-engine-ref';

export type ImageExportFormat = 'png' | 'jpeg' | 'webp';
export type GlobalExportFormat = ImageExportFormat | 'pdf';

interface PageRender {
  /** Page 名称（用于文件名）。 */
  name: string;
  /**
   * Encoded
   * 图像字节（已从 WASM 堆中复制出来）。 Typed 与 `<ArrayBuffer>` 因此它满足 TS
   * 5.7+ 中的 `BlobPart` 约束，该约束拒绝更宽的 `Uint8Array<ArrayBufferLike>`。
   */
  bytes: Uint8Array<ArrayBuffer>;
  /** Pixel 编码图像的尺寸。 */
  width: number;
  height: number;
  /** Logical 设计单位尺寸（用作 PDF MediaBox）。 */
  logicalWidth: number;
  logicalHeight: number;
}

interface RenderPageOptions {
  multiplier: number;
  format: ImageExportFormat;
  /** When '白色'，用白色而不是透明清除表面。 */
  background?: 'transparent' | 'white';
}

/** Decode 将 URL 的 Base64 数据像 `data:image/jpeg;base64,...` 一样转换为原始字节。 */
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
 * Render 单页编码图像字节。
 * Returns null 如果 SkiaEngine 不可用，则页面为空，
 * 或表面 allocation/encoding 失败。
 *
 * Uses `MakeSWCanvasSurface` 在临时 `<canvas>` 元素上而不是
 * `MakeSurface(w,h)` 因为当 WebGL 时编辑器会回退到 SW 路径
 * 不可用，因此它是最可靠的交叉构建选项。 Encoding 去
 * 通过浏览器的本机 `canvas.toDataURL` （它始终支持
 * PNG/JPEG/WEBP) 而不是 `Image.encodeToBytes`，据观察，
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

  // Mirror SkiaEngine.syncFromDocument 因此导出与屏幕渲染相匹配。
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

  // 来自根级节点的 Bounding 框（那些没有继承的 clipRect 的节点）。
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

  // Create 一个临时画布 + Skia 由它支持的软件表面。 After flush()，渲染的像素位于画布的 2d
  // 上下文中，可通过 toDataURL 访问。
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

  // Encode 通过浏览器的本机画布编码器。
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

/** List 文档的页面 — 回退到单个旧页面。 */
function listPages(doc: PenDocument): { id: string; name: string; children: PenNode[] }[] {
  if (doc.pages && doc.pages.length > 0) {
    return doc.pages.map((p) => ({ id: p.id, name: p.name || 'Page', children: p.children }));
  }
  return [{ id: '__legacy__', name: 'Page 1', children: doc.children }];
}

/**
 * Sanitize
 * 用作文件名的字符串。 Allows 字母、数字、连字符、下划线和 CJK 字符；将其他所有内容折叠为下划线。
 */
export function sanitizeFilename(name: string, fallback = 'untitled'): string {
  const safe = (name || '').replace(/[^\p{L}\p{N}_-]+/gu, '_').replace(/^_+|_+$/g, '');
  return safe || fallback;
}

/**
 * Export 将活动页面
 * 作为图像 (PNG/JPEG/WEBP)。 Returns 失败时为 null。
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
 * Export 将整个文档
 * 作为多页光栅 PDF。 Each 页面呈现为 JPEG 并嵌入为 /XObject /Image 和
 * /DCTDecode 过滤器 — 无需外部 PDF 库。
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
 * Build 一个最小的
 *
 * PDF，每页嵌入一个 JPEG 图像。 Object 布局： 1：Catalog 2：Pages 根
 * 3、4、5：第 1
 * 页（Page、Image、C
 * ontents）6、7、
 * 8：第 2 页 ...
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

  // Per-页面对象
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
