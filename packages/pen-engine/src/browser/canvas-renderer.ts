import type { CanvasKit, Surface } from 'canvaskit-wasm';
import type { DesignEngine } from '../core/design-engine.js';
import { SkiaNodeRenderer, type RenderNode } from '@zseven-w/pen-renderer';
import {
  flattenToRenderNodes,
  resolveRefs,
  premeasureTextHeights,
  collectReusableIds,
  collectInstanceIds,
  parseColor,
  viewportMatrix,
  getViewportBounds,
  isRectInViewport,
} from '@zseven-w/pen-renderer';
import {
  getActivePageChildren,
  getAllChildren,
  getDefaultTheme,
  resolveNodeForCanvas,
} from '@zseven-w/pen-core';
import type { DesignEngineOptions } from '@zseven-w/pen-types';

/**
 * Manages CanvasKit rendering for a DesignEngine instance.
 * Created by attachCanvas(), not directly.
 */
export class CanvasRenderer {
  private ck: CanvasKit;
  private surface: Surface | null = null;
  private canvasEl: HTMLCanvasElement | OffscreenCanvas | null = null;
  private nodeRenderer: SkiaNodeRenderer;
  private engine: DesignEngine;
  private renderNodes: RenderNode[] = [];
  private reusableIds = new Set<string>();
  private instanceIds = new Set<string>();
  private dirty = true;
  private animFrameId = 0;
  private options: DesignEngineOptions;

  constructor(ck: CanvasKit, engine: DesignEngine, options?: DesignEngineOptions) {
    this.ck = ck;
    this.engine = engine;
    this.options = options ?? {};
    this.nodeRenderer = new SkiaNodeRenderer(ck, {
      fontBasePath: this.options.fontBasePath,
      googleFontsCssUrl: this.options.googleFontsCssUrl,
    });
    if (this.options.iconLookup) {
      this.nodeRenderer.setIconLookup(this.options.iconLookup);
    }
  }

  init(canvasEl: HTMLCanvasElement | OffscreenCanvas): void {
    this.canvasEl = canvasEl;
    const dpr =
      this.options.devicePixelRatio ??
      (typeof window !== 'undefined' ? window.devicePixelRatio : 1);

    if (canvasEl instanceof HTMLCanvasElement) {
      canvasEl.width = canvasEl.clientWidth * dpr;
      canvasEl.height = canvasEl.clientHeight * dpr;
    }

    this.surface = this.ck.MakeWebGLCanvasSurface(canvasEl as HTMLCanvasElement);
    if (!this.surface) {
      this.surface = this.ck.MakeSWCanvasSurface(canvasEl as HTMLCanvasElement);
    }
    if (!this.surface) {
      console.error('CanvasRenderer: Failed to create surface');
      return;
    }

    this.nodeRenderer.init();
    this.nodeRenderer.setRedrawCallback(() => this.markDirty());
    this.nodeRenderer.fontManager.ensureFont('Inter').then(() => this.markDirty());
    this.nodeRenderer.fontManager.ensureFont('Noto Sans SC').then(() => this.markDirty());
    this.startRenderLoop();
  }

  dispose(): void {
    if (this.animFrameId) cancelAnimationFrame(this.animFrameId);
    this.nodeRenderer.dispose();
    this.surface?.delete();
    this.surface = null;
  }

  resize(width: number, height: number): void {
    if (!this.canvasEl || !(this.canvasEl instanceof HTMLCanvasElement)) return;
    const dpr = this.options.devicePixelRatio ?? window.devicePixelRatio ?? 1;
    this.canvasEl.width = width * dpr;
    this.canvasEl.height = height * dpr;
    this.surface?.delete();
    this.surface = this.ck.MakeWebGLCanvasSurface(this.canvasEl);
    if (!this.surface) {
      this.surface = this.ck.MakeSWCanvasSurface(this.canvasEl);
    }
    this.render();
  }

  markDirty(): void {
    this.dirty = true;
  }

  syncFromDocument(): void {
    try {
      const doc = this.engine.getDocument();
      const activePageId = this.engine.getActivePage();
      const pageChildren = getActivePageChildren(doc, activePageId);
      const allNodes = getAllChildren(doc);

      this.reusableIds.clear();
      this.instanceIds.clear();
      collectReusableIds(pageChildren, this.reusableIds);
      collectInstanceIds(pageChildren, this.instanceIds);

      const resolved = resolveRefs(pageChildren, allNodes);
      const variables = doc.variables ?? {};
      const themes = doc.themes;
      const defaultTheme = getDefaultTheme(themes);
      const variableResolved = resolved.map((n) =>
        resolveNodeForCanvas(n, variables, defaultTheme),
      );
      const measured = premeasureTextHeights(variableResolved);
      this.renderNodes = flattenToRenderNodes(measured);
      this.engine.getSpatialIndex().rebuild(this.renderNodes);
    } catch (err) {
      console.error('[CanvasRenderer] syncFromDocument failed:', err);
    }
    this.markDirty();
  }

  render(): void {
    if (!this.surface || !this.canvasEl) return;
    const canvas = this.surface.getCanvas();
    const ck = this.ck;
    const dpr =
      this.options.devicePixelRatio ??
      (typeof window !== 'undefined' ? window.devicePixelRatio : 1);
    const selectedIds = new Set(this.engine.getSelection());
    const bgColor = this.options.backgroundColor ?? '#1a1a1a';
    const zoom = this.engine.zoom;
    const panX = this.engine.panX;
    const panY = this.engine.panY;

    canvas.clear(parseColor(ck, bgColor));
    canvas.save();
    canvas.scale(dpr, dpr);
    canvas.concat(viewportMatrix({ zoom, panX, panY }));
    this.nodeRenderer.zoom = zoom;

    const cw = this.canvasEl instanceof HTMLCanvasElement ? this.canvasEl.clientWidth : 800;
    const ch = this.canvasEl instanceof HTMLCanvasElement ? this.canvasEl.clientHeight : 600;
    const vpBounds = getViewportBounds({ zoom, panX, panY }, cw, ch, 64 / zoom);

    for (const rn of this.renderNodes) {
      if (!isRectInViewport({ x: rn.absX, y: rn.absY, w: rn.absW, h: rn.absH }, vpBounds)) continue;
      this.nodeRenderer.drawNode(canvas, rn);
      if (selectedIds.has(rn.node.id)) {
        this.drawSelectionBorder(canvas, rn.absX, rn.absY, rn.absW, rn.absH);
      }
    }

    canvas.restore();
    this.surface.flush();
  }

  async renderToImageData(width: number, height: number): Promise<Uint8Array> {
    // Render to an offscreen surface and extract pixels
    const surface = this.ck.MakeSurface(width, height);
    if (!surface) throw new Error('Failed to create offscreen surface');
    const canvas = surface.getCanvas();
    canvas.clear(this.ck.WHITE);
    // Render at 1:1 scale
    for (const rn of this.renderNodes) {
      this.nodeRenderer.drawNode(canvas, rn);
    }
    surface.flush();
    const image = surface.makeImageSnapshot();
    const bytes = image.encodeToBytes();
    image.delete();
    surface.delete();
    return bytes ?? new Uint8Array();
  }

  private drawSelectionBorder(
    canvas: import('canvaskit-wasm').Canvas,
    x: number,
    y: number,
    w: number,
    h: number,
  ): void {
    const ck = this.ck;
    const paint = new ck.Paint();
    paint.setStyle(ck.PaintStyle.Stroke);
    paint.setAntiAlias(true);
    paint.setStrokeWidth(1 / this.engine.zoom);
    paint.setColor(parseColor(ck, '#3b82f6'));
    canvas.drawRect(ck.LTRBRect(x, y, x + w, y + h), paint);
    paint.delete();
  }

  private startRenderLoop(): void {
    const loop = () => {
      this.animFrameId = requestAnimationFrame(loop);
      if (!this.dirty || !this.surface) return;
      this.dirty = false;
      this.render();
    };
    this.animFrameId = requestAnimationFrame(loop);
  }
}
