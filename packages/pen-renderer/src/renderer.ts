import type { CanvasKit, Surface } from 'canvaskit-wasm';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import {
  getActivePageChildren,
  getAllChildren,
  getDefaultTheme,
  resolveNodeForCanvas,
  MIN_ZOOM,
  MAX_ZOOM,
  CANVAS_BACKGROUND_DARK,
  FRAME_LABEL_FONT_SIZE,
  FRAME_LABEL_OFFSET_Y,
  FRAME_LABEL_COLOR,
  setRootChildrenProvider,
} from '@zseven-w/pen-core';
import { SkiaNodeRenderer } from './node-renderer.js';
import { SpatialIndex } from './spatial-index.js';
import {
  flattenToRenderNodes,
  resolveRefs,
  premeasureTextHeights,
  collectReusableIds,
  collectInstanceIds,
} from './document-flattener.js';
import { parseColor } from './paint-utils.js';
import { viewportMatrix, screenToScene, zoomToPoint as vpZoomToPoint } from './viewport.js';
import type { RenderNode, PenRendererOptions, ViewportState } from './types.js';

/**
 * Standalone read-only renderer for OpenPencil (.op) design files.
 * No React, no Zustand, no TanStack — just pure TypeScript + CanvasKit.
 *
 * @example
 * ```ts
 * import { loadCanvasKit, PenRenderer } from '@zseven-w/pen-renderer'
 *
 * const ck = await loadCanvasKit('/canvaskit/')
 * const renderer = new PenRenderer(ck, { fontBasePath: '/fonts/' })
 * renderer.init(document.getElementById('canvas') as HTMLCanvasElement)
 * renderer.setDocument(myDocument)
 * renderer.zoomToFit()
 * ```
 */
export class PenRenderer {
  private ck: CanvasKit;
  private surface: Surface | null = null;
  private canvasEl: HTMLCanvasElement | null = null;
  private nodeRenderer: SkiaNodeRenderer;
  private spatialIndex = new SpatialIndex();
  private renderNodes: RenderNode[] = [];
  private options: PenRendererOptions;

  // Component/instance IDs for colored frame labels
  private reusableIds = new Set<string>();
  private instanceIds = new Set<string>();

  // Viewport
  private _zoom = 1;
  private _panX = 0;
  private _panY = 0;
  private dirty = true;
  private animFrameId = 0;

  // Document
  private document: PenDocument | null = null;
  private activePageId: string | null = null;

  constructor(ck: CanvasKit, options?: PenRendererOptions) {
    this.ck = ck;
    this.options = options ?? {};
    this.nodeRenderer = new SkiaNodeRenderer(ck, {
      fontBasePath: this.options.fontBasePath,
      googleFontsCssUrl: this.options.googleFontsCssUrl,
    });
    if (this.options.iconLookup) {
      this.nodeRenderer.setIconLookup(this.options.iconLookup);
    }
    if (this.options.devicePixelRatio !== undefined) {
      this.nodeRenderer.devicePixelRatio = this.options.devicePixelRatio;
    }
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  init(canvas: HTMLCanvasElement) {
    this.canvasEl = canvas;
    const dpr = this.options.devicePixelRatio ?? window.devicePixelRatio ?? 1;
    canvas.width = canvas.clientWidth * dpr;
    canvas.height = canvas.clientHeight * dpr;

    this.surface = this.ck.MakeWebGLCanvasSurface(canvas);
    if (!this.surface) this.surface = this.ck.MakeSWCanvasSurface(canvas);
    if (!this.surface) {
      console.error('PenRenderer: Failed to create surface');
      return;
    }

    this.nodeRenderer.init();
    this.nodeRenderer.setRedrawCallback(() => this.markDirty());
    (this.nodeRenderer as any).textRenderer._onFontLoaded = () => this.markDirty();

    // Pre-load default fonts
    const defaultFonts = this.options.defaultFonts ?? ['Inter', 'Noto Sans SC'];
    for (const font of defaultFonts) {
      this.nodeRenderer.fontManager.ensureFont(font).then(() => this.markDirty());
    }

    // Wire up root children provider for layout engine fill-width fallback
    setRootChildrenProvider(() => this.document?.children ?? []);

    this.startRenderLoop();
  }

  dispose() {
    if (this.animFrameId) cancelAnimationFrame(this.animFrameId);
    this.nodeRenderer.dispose();
    this.surface?.delete();
    this.surface = null;
  }

  resize(width: number, height: number) {
    if (!this.canvasEl) return;
    const dpr = this.options.devicePixelRatio ?? window.devicePixelRatio ?? 1;
    this.canvasEl.width = width * dpr;
    this.canvasEl.height = height * dpr;
    this.surface?.delete();
    this.surface = this.ck.MakeWebGLCanvasSurface(this.canvasEl);
    if (!this.surface) this.surface = this.ck.MakeSWCanvasSurface(this.canvasEl);
    this.markDirty();
  }

  // ---------------------------------------------------------------------------
  // Document
  // ---------------------------------------------------------------------------

  setDocument(doc: PenDocument) {
    this.document = doc;
    this.activePageId = doc.pages?.[0]?.id ?? null;
    this.syncFromDocument();
  }

  getDocument(): PenDocument | null {
    return this.document;
  }

  // ---------------------------------------------------------------------------
  // Pages
  // ---------------------------------------------------------------------------

  setPage(pageId: string) {
    this.activePageId = pageId;
    this.syncFromDocument();
  }

  getPageIds(): string[] {
    return this.document?.pages?.map((p) => p.id) ?? [];
  }

  getActivePageId(): string | null {
    return this.activePageId;
  }

  // ---------------------------------------------------------------------------
  // Viewport
  // ---------------------------------------------------------------------------

  setViewport(zoom: number, panX: number, panY: number) {
    this._zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, zoom));
    this._panX = panX;
    this._panY = panY;
    this.markDirty();
  }

  getViewport(): ViewportState {
    return { zoom: this._zoom, panX: this._panX, panY: this._panY };
  }

  zoomToFit(padding = 64) {
    if (!this.canvasEl || this.renderNodes.length === 0) return;
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;
    for (const rn of this.renderNodes) {
      if (rn.clipStack && rn.clipStack.length > 0) continue;
      minX = Math.min(minX, rn.absX);
      minY = Math.min(minY, rn.absY);
      maxX = Math.max(maxX, rn.absX + rn.absW);
      maxY = Math.max(maxY, rn.absY + rn.absH);
    }
    if (!isFinite(minX)) return;

    const contentW = maxX - minX;
    const contentH = maxY - minY;
    const canvasW = this.canvasEl.clientWidth;
    const canvasH = this.canvasEl.clientHeight;
    const zoom = Math.min(
      (canvasW - padding * 2) / contentW,
      (canvasH - padding * 2) / contentH,
      2,
    );
    const panX = (canvasW - contentW * zoom) / 2 - minX * zoom;
    const panY = (canvasH - contentH * zoom) / 2 - minY * zoom;
    this.setViewport(zoom, panX, panY);
  }

  zoomToPoint(screenX: number, screenY: number, newZoom: number) {
    if (!this.canvasEl) return;
    const rect = this.canvasEl.getBoundingClientRect();
    const vp = vpZoomToPoint(
      { zoom: this._zoom, panX: this._panX, panY: this._panY },
      screenX,
      screenY,
      rect,
      newZoom,
    );
    this.setViewport(vp.zoom, vp.panX, vp.panY);
  }

  pan(dx: number, dy: number) {
    this.setViewport(this._zoom, this._panX + dx, this._panY + dy);
  }

  // ---------------------------------------------------------------------------
  // Theme
  // ---------------------------------------------------------------------------

  setThemeVariant(variant: Record<string, string>) {
    this.options.themeVariant = variant;
    this.syncFromDocument();
  }

  // ---------------------------------------------------------------------------
  // Hit testing
  // ---------------------------------------------------------------------------

  hitTest(screenX: number, screenY: number): PenNode | null {
    if (!this.canvasEl) return null;
    const rect = this.canvasEl.getBoundingClientRect();
    const scene = screenToScene(screenX, screenY, rect, {
      zoom: this._zoom,
      panX: this._panX,
      panY: this._panY,
    });
    const hits = this.spatialIndex.hitTest(scene.x, scene.y);
    return hits.length > 0 ? hits[0].node : null;
  }

  getNodeBounds(nodeId: string): { x: number; y: number; w: number; h: number } | null {
    const rn = this.spatialIndex.get(nodeId);
    if (!rn) return null;
    return { x: rn.absX, y: rn.absY, w: rn.absW, h: rn.absH };
  }

  // ---------------------------------------------------------------------------
  // Internal: Document sync
  // ---------------------------------------------------------------------------

  private syncFromDocument() {
    if (!this.document) return;
    const pageChildren = getActivePageChildren(this.document, this.activePageId);
    const allNodes = getAllChildren(this.document);

    // Collect reusable/instance IDs
    this.reusableIds.clear();
    this.instanceIds.clear();
    collectReusableIds(pageChildren, this.reusableIds);
    collectInstanceIds(pageChildren, this.instanceIds);

    // Resolve refs
    const resolved = resolveRefs(pageChildren, allNodes);

    // Resolve design variables
    const variables = this.document.variables ?? {};
    const themes = this.document.themes;
    const activeTheme = this.options.themeVariant ?? getDefaultTheme(themes);
    const variableResolved = resolved.map((n) => resolveNodeForCanvas(n, variables, activeTheme));

    // Pre-measure text heights
    const measured = premeasureTextHeights(variableResolved);

    this.renderNodes = flattenToRenderNodes(measured);
    this.spatialIndex.rebuild(this.renderNodes);
    this.markDirty();
  }

  // ---------------------------------------------------------------------------
  // Render loop
  // ---------------------------------------------------------------------------

  private markDirty() {
    this.dirty = true;
  }

  private startRenderLoop() {
    const loop = () => {
      this.animFrameId = requestAnimationFrame(loop);
      if (!this.dirty || !this.surface) return;
      this.dirty = false;
      this.render();
    };
    this.animFrameId = requestAnimationFrame(loop);
  }

  render() {
    if (!this.surface || !this.canvasEl) return;
    const canvas = this.surface.getCanvas();
    const ck = this.ck;
    const dpr = this.options.devicePixelRatio ?? window.devicePixelRatio ?? 1;

    // Clear
    const bgColor = this.options.backgroundColor ?? CANVAS_BACKGROUND_DARK;
    canvas.clear(parseColor(ck, bgColor));

    // Apply viewport transform
    canvas.save();
    canvas.scale(dpr, dpr);
    canvas.concat(viewportMatrix({ zoom: this._zoom, panX: this._panX, panY: this._panY }));

    // Pass current zoom to renderer
    this.nodeRenderer.zoom = this._zoom;

    // Draw all render nodes. Per-node try/catch isolates failures so a
    // single bad node (malformed effects, NaN coordinates, missing fill)
    // doesn't abort the whole render loop and blank the canvas. The
    // 2026-05-10 user report (Bistro generation, layer panel populated,
    // canvas empty) was traced to NaN feeding RRectXY when an LLM-emitted
    // shadow omitted `spread` — the throw escaped the loop and every
    // sibling stopped rendering. The shadow path now coerces missing
    // numeric fields to 0; this catch is the structural backstop.
    //
    // Snapshot the canvas save count before each node so a throw mid-
    // way through drawNode (which pushes save() / clip() per ancestor
    // clipStack entry, plus more for rotation / flip — see node-
    // renderer.ts:548, 574, 583, 701, 1094, 1102) can roll the canvas
    // state back to a clean baseline. Without this, the leaked clip /
    // rotation state corrupts every subsequent node's draw — Codex
    // stop-hook 2026-05-10 caught the bare-catch leak.
    for (const rn of this.renderNodes) {
      const saveCount = canvas.getSaveCount();
      try {
        this.nodeRenderer.drawNode(canvas, rn);
      } catch (err) {
        try {
          canvas.restoreToCount(saveCount);
        } catch {
          // restoreToCount itself can throw if the snapshot count is
          // somehow above the current depth (shouldn't happen in
          // practice — drawNode only pushes saves, never pops below
          // its entry depth — but guard so the error reporter still
          // logs).
        }
        const id = rn.node.id ?? '<no-id>';
        console.error(`[pen-renderer] drawNode threw for node ${id}:`, err);
      }
    }

    // Draw frame labels for root frames + reusable + instances
    for (const rn of this.renderNodes) {
      if (!rn.node.name) continue;
      const isRootFrame =
        rn.node.type === 'frame' && (rn.clipStack === undefined || rn.clipStack.length === 0);
      const isReusable = this.reusableIds.has(rn.node.id);
      const isInstance = this.instanceIds.has(rn.node.id);
      if (!isRootFrame && !isReusable && !isInstance) continue;
      this.drawFrameLabel(canvas, rn.node.name, rn.absX, rn.absY);
    }

    canvas.restore();
    this.surface.flush();
  }

  /** Simple frame label drawing for read-only renderer. */
  private drawFrameLabel(
    canvas: ReturnType<Surface['getCanvas']>,
    name: string,
    x: number,
    y: number,
  ) {
    const ck = this.ck;
    const fontSize = FRAME_LABEL_FONT_SIZE / this._zoom;
    const offsetY = FRAME_LABEL_OFFSET_Y / this._zoom;

    // Use Canvas 2D to rasterize the label text
    const dpr = this.options.devicePixelRatio ?? window.devicePixelRatio ?? 1;
    const scale = Math.min(this._zoom * dpr, 4);
    const tmp = document.createElement('canvas');
    const textW = Math.ceil(name.length * fontSize * 0.7 * scale) + 4;
    const textH = Math.ceil(fontSize * 1.4 * scale) + 4;
    tmp.width = textW;
    tmp.height = textH;
    const ctx = tmp.getContext('2d')!;
    ctx.scale(scale, scale);
    ctx.font = `500 ${fontSize}px Inter, system-ui, sans-serif`;
    ctx.fillStyle = FRAME_LABEL_COLOR;
    ctx.textBaseline = 'top';
    ctx.fillText(name, 0, 0);

    const imageData = ctx.getImageData(0, 0, textW, textH);
    const img = ck.MakeImage(
      {
        width: textW,
        height: textH,
        alphaType: ck.AlphaType.Unpremul,
        colorType: ck.ColorType.RGBA_8888,
        colorSpace: ck.ColorSpace.SRGB,
      },
      imageData.data,
      textW * 4,
    );
    if (img) {
      const paint = new ck.Paint();
      paint.setAntiAlias(true);
      canvas.drawImageRect(
        img,
        ck.LTRBRect(0, 0, textW, textH),
        ck.LTRBRect(x, y - offsetY - fontSize * 1.2, x + textW / scale, y - offsetY),
        paint,
      );
      paint.delete();
      img.delete();
    }
  }
}
