import type { CanvasKit, Surface } from 'canvaskit-wasm';
import type { EllipseNode, PathNode } from '@/types/pen';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore, getActivePageChildren, getAllChildren } from '@/stores/document-store';
import { resolveNodeForCanvas, getDefaultTheme } from '@/variables/resolve-variables';
import { getCanvasBackground, MIN_ZOOM, MAX_ZOOM } from '../canvas-constants';
import { setRootChildrenProvider } from '../canvas-layout-engine';
import { SkiaRenderer, type RenderNode } from './skia-renderer';
import {
  SpatialIndex,
  parseColor,
  viewportMatrix,
  zoomToPoint as vpZoomToPoint,
  flattenToRenderNodes,
  resolveRefs,
  premeasureTextHeights,
  collectReusableIds,
  collectInstanceIds,
  getViewportBounds,
  isRectInViewport,
} from '@zseven-w/pen-renderer';
import { getActiveAgentIndicators, getActiveAgentFrames, isPreviewNode } from '../agent-indicator';
import { isNodeBorderReady, getNodeRevealTime } from '@/services/ai/design-animation';
import { lookupIconByName } from '@/services/ai/icon-resolver';
import { getEditablePathState } from './path-editing';
import { resolveRuntimeAssetSource } from '@/utils/document-assets';
import { fitSceneBoundsToViewport, getFocusBounds } from './focus-fit';

// Re-export for use by canvas component
export { screenToScene } from '@zseven-w/pen-renderer';
export { SpatialIndex } from '@zseven-w/pen-renderer';

// ---------------------------------------------------------------------------
// SkiaEngine — ties rendering, viewport, hit testing together
// ---------------------------------------------------------------------------

export class SkiaEngine {
  ck: CanvasKit;
  surface: Surface | null = null;
  renderer: SkiaRenderer;
  spatialIndex = new SpatialIndex();
  renderNodes: RenderNode[] = [];

  // Component/instance IDs for colored frame labels
  private reusableIds = new Set<string>();
  private instanceIds = new Set<string>();

  // Agent animation: track start time so glow only pulses ~2 times
  private agentAnimStart = 0;

  private canvasEl: HTMLCanvasElement | null = null;
  private animFrameId = 0;
  private dirty = true;

  // Ref counter — when > 0, render() skips the agent-overlay self-loop
  // (further down). captureRegion increments this for the duration of
  // waitForSettled + makeImageSnapshot so the loop terminates even when
  // AI agents are actively painting indicators on the canvas. A counter
  // (rather than a boolean) is used so that overlapping/nested captures
  // don't re-enable the self-loop while an outer capture is still settling.
  // Agent overlays are still drawn into each captured frame — we just
  // don't schedule the next animation tick during capture.
  private _captureRefcount = 0;

  // Viewport
  zoom = 1;
  panX = 0;
  panY = 0;

  // Drag suppression — prevents syncFromDocument during drag
  // so the layout engine doesn't override visual positions
  dragSyncSuppressed = false;

  // Interaction state
  hoveredNodeId: string | null = null;
  marquee: { x1: number; y1: number; x2: number; y2: number } | null = null;
  previewShape: {
    type: 'rectangle' | 'ellipse' | 'frame' | 'line' | 'polygon';
    x: number;
    y: number;
    w: number;
    h: number;
  } | null = null;
  penPreview: import('./skia-overlays').PenPreviewData | null = null;

  constructor(ck: CanvasKit) {
    this.ck = ck;
    this.renderer = new SkiaRenderer(ck);
    // Wire up icon lookup for icon_font nodes
    this.renderer.setIconLookup(lookupIconByName);
    if (
      typeof (this.renderer as { setImageSourceResolver?: unknown }).setImageSourceResolver ===
      'function'
    ) {
      this.renderer.setImageSourceResolver((src) => {
        const filePath = useDocumentStore.getState().filePath;
        const resolved = resolveRuntimeAssetSource(src, filePath);
        return {
          cacheKey: resolved.runtimeUrl ?? `missing:${filePath ?? ''}:${src}`,
          loadUrl: resolved.runtimeUrl,
        };
      });
    }
    // Wire up root children provider for layout engine fill-width fallback
    setRootChildrenProvider(() => useDocumentStore.getState().document.children);
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  init(canvasEl: HTMLCanvasElement) {
    this.canvasEl = canvasEl;
    const dpr = window.devicePixelRatio || 1;
    canvasEl.width = canvasEl.clientWidth * dpr;
    canvasEl.height = canvasEl.clientHeight * dpr;

    this.surface = this.createSurface(canvasEl);
    if (!this.surface) {
      console.error('SkiaEngine: Failed to create surface');
      return;
    }

    this.renderer.init();
    this.renderer.setRedrawCallback(() => this.markDirty());
    // Re-render when async font loading completes
    (this.renderer as any)._onFontLoaded = () => this.markDirty();
    // Pre-load default fonts for vector text rendering.
    // Noto Sans SC is loaded alongside Inter so CJK glyphs are always available
    // in the fallback chain — system CJK fonts (PingFang SC, Microsoft YaHei, etc.)
    // are skipped from Google Fonts, and without Noto Sans SC the fallback chain
    // would only contain Inter which has no CJK coverage, causing tofu.
    this.renderer.fontManager.ensureFont('Inter').then(() => this.markDirty());
    this.renderer.fontManager.ensureFont('Noto Sans SC').then(() => this.markDirty());
    this.startRenderLoop();
  }

  dispose() {
    if (this.animFrameId) cancelAnimationFrame(this.animFrameId);
    this.renderer.dispose();
    this.safeDeleteSurface(this.surface);
  }

  resize(width: number, height: number) {
    if (!this.canvasEl) return;
    const dpr = window.devicePixelRatio || 1;
    this.canvasEl.width = width * dpr;
    this.canvasEl.height = height * dpr;

    if (this.recreateSurface()) {
      // Render synchronously so the new surface is filled before the browser
      // paints. Setting canvas.width/height clears the pixel buffer to
      // transparent; ResizeObserver fires between layout and paint, so writing
      // pixels here lands in the same frame's composite. Without this, the
      // canvas flashes through to its container background (e.g. bg-muted)
      // for one frame whenever a sibling panel mounts and the flex layout shifts.
      this.dirty = false;
      this.render();
    }
  }

  // ---------------------------------------------------------------------------
  // Document sync
  // ---------------------------------------------------------------------------

  syncFromDocument() {
    if (this.dragSyncSuppressed) return;
    try {
      const docState = useDocumentStore.getState();
      const activePageId = useCanvasStore.getState().activePageId;
      const pageChildren = getActivePageChildren(docState.document, activePageId);
      const allNodes = getAllChildren(docState.document);

      // Collect reusable/instance IDs from raw tree (before ref resolution strips them)
      this.reusableIds.clear();
      this.instanceIds.clear();
      collectReusableIds(pageChildren, this.reusableIds);
      collectInstanceIds(pageChildren, this.instanceIds);

      // Resolve refs, variables, then flatten
      const resolved = resolveRefs(pageChildren, allNodes);

      // Resolve design variables
      const variables = docState.document.variables ?? {};
      const themes = docState.document.themes;
      const defaultTheme = getDefaultTheme(themes);
      const variableResolved = resolved.map((n) =>
        resolveNodeForCanvas(n, variables, defaultTheme),
      );

      // Only premeasure text HEIGHTS for fixed-width text (where wrapping
      // estimation may differ from Canvas 2D). Never touch widths or
      // container-relative sizing to maintain layout consistency with Fabric.js.
      const measured = premeasureTextHeights(variableResolved);

      this.renderNodes = flattenToRenderNodes(measured);

      this.spatialIndex.rebuild(this.renderNodes);
    } catch (err) {
      console.error('[SkiaEngine] syncFromDocument failed:', err);
    }
    this.markDirty();
  }

  // ---------------------------------------------------------------------------
  // Render loop
  // ---------------------------------------------------------------------------

  markDirty() {
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

  private render() {
    if (!this.surface || !this.canvasEl) return;
    try {
      const surface = this.surface;
      const canvas = surface.getCanvas();
      const ck = this.ck;

      const dpr = window.devicePixelRatio || 1;
      const selectedIds = new Set(useCanvasStore.getState().selection.selectedIds);

      // Clear
      const bgColor = getCanvasBackground();
      canvas.clear(parseColor(ck, bgColor));

      // Apply viewport transform
      canvas.save();
      canvas.scale(dpr, dpr);
      canvas.concat(viewportMatrix({ zoom: this.zoom, panX: this.panX, panY: this.panY }));

      // Pass current zoom to renderer for zoom-aware text rasterization
      this.renderer.zoom = this.zoom;

      const vpBounds = getViewportBounds(
        { zoom: this.zoom, panX: this.panX, panY: this.panY },
        this.canvasEl.clientWidth,
        this.canvasEl.clientHeight,
        64 / this.zoom,
      );
      // Draw all render nodes
      for (const rn of this.renderNodes) {
        // Skip nodes outside the viewport
        if (!isRectInViewport({ x: rn.absX, y: rn.absY, w: rn.absW, h: rn.absH }, vpBounds))
          continue;
        this.renderer.drawNodeWithSelection(canvas, rn, selectedIds);
      }

      // Draw agent indicators (glow, badges, node borders, preview fills)
      const agentIndicators = getActiveAgentIndicators();
      const agentFrames = getActiveAgentFrames();
      const hasAgentOverlays = agentIndicators.size > 0 || agentFrames.size > 0;

      if (!hasAgentOverlays) {
        this.agentAnimStart = 0;
      }

      if (hasAgentOverlays) {
        const now = Date.now();
        if (this.agentAnimStart === 0) this.agentAnimStart = now;
        const elapsed = now - this.agentAnimStart;
        // Frame glow: smooth fade-in → fade-out (single bell, ~1.2s)
        const GLOW_DURATION = 1200;
        const glowT = Math.min(1, elapsed / GLOW_DURATION);
        const breath = Math.sin(glowT * Math.PI); // 0 → 1 → 0

        // Agent node borders and preview fills (per-element fade-in → fade-out)
        const NODE_FADE_DURATION = 1000;
        for (const rn of this.renderNodes) {
          const indicator = agentIndicators.get(rn.node.id);
          if (!indicator) continue;
          if (!isNodeBorderReady(rn.node.id)) continue;

          const revealAt = getNodeRevealTime(rn.node.id);
          if (revealAt === undefined) continue;
          const nodeElapsed = now - revealAt;
          if (nodeElapsed > NODE_FADE_DURATION) continue;

          // Smooth bell curve: fade in then fade out
          const nodeT = Math.min(1, nodeElapsed / NODE_FADE_DURATION);
          const nodeBreath = Math.sin(nodeT * Math.PI);

          if (isPreviewNode(rn.node.id)) {
            this.renderer.drawAgentPreviewFill(
              canvas,
              rn.absX,
              rn.absY,
              rn.absW,
              rn.absH,
              indicator.color,
              now,
            );
          }

          this.renderer.drawAgentNodeBorder(
            canvas,
            rn.absX,
            rn.absY,
            rn.absW,
            rn.absH,
            indicator.color,
            nodeBreath,
            this.zoom,
          );
        }

        // Agent frame glow and badges
        for (const rn of this.renderNodes) {
          const frame = agentFrames.get(rn.node.id);
          if (!frame) continue;

          this.renderer.drawAgentGlow(
            canvas,
            rn.absX,
            rn.absY,
            rn.absW,
            rn.absH,
            frame.color,
            breath,
            this.zoom,
          );
          this.renderer.drawAgentBadge(
            canvas,
            frame.name,
            rn.absX,
            rn.absY,
            rn.absW,
            frame.color,
            this.zoom,
            now,
          );
        }
      }

      // Hover outline
      if (this.hoveredNodeId && !selectedIds.has(this.hoveredNodeId)) {
        const hovered = this.spatialIndex.get(this.hoveredNodeId);
        if (hovered) {
          this.renderer.drawHoverOutline(
            canvas,
            hovered.absX,
            hovered.absY,
            hovered.absW,
            hovered.absH,
          );
        }
      }

      // Arc handles for selected ellipse
      if (selectedIds.size === 1) {
        const selId = selectedIds.values().next().value as string;
        const selRN = this.spatialIndex.get(selId);
        if (selRN && selRN.node.type === 'ellipse') {
          const eNode = selRN.node as EllipseNode;
          this.renderer.drawArcHandles(
            canvas,
            selRN.absX,
            selRN.absY,
            selRN.absW,
            selRN.absH,
            eNode.startAngle ?? 0,
            eNode.sweepAngle ?? 360,
            eNode.innerRadius ?? 0,
            this.zoom,
          );
        }
        if (selRN && selRN.node.type === 'path') {
          const pathState = getEditablePathState(selRN.node as PathNode, {
            x: selRN.absX,
            y: selRN.absY,
            width: selRN.absW,
            height: selRN.absH,
          });
          if (pathState) {
            this.renderer.drawPathEditor(
              canvas,
              pathState.sceneAnchors,
              this.zoom,
              pathState.closed,
            );
          }
        }
      }

      // Drawing preview shape
      if (this.previewShape) {
        this.renderer.drawPreview(canvas, this.previewShape);
      }

      // Pen tool preview
      if (this.penPreview) {
        this.renderer.drawPenPreview(canvas, this.penPreview, this.zoom);
      }

      // Selection marquee
      if (this.marquee) {
        this.renderer.drawSelectionMarquee(
          canvas,
          this.marquee.x1,
          this.marquee.y1,
          this.marquee.x2,
          this.marquee.y2,
        );
      }

      canvas.restore();

      // Draw frame labels outside viewport transform so fontSize stays constant
      // (avoids Math.ceil(12/zoom) integer-boundary jumps causing label size flicker)
      canvas.save();
      canvas.scale(dpr, dpr);
      for (const rn of this.renderNodes) {
        if (!rn.node.name) continue;
        const isRootFrame = rn.node.type === 'frame' && !rn.clipRect;
        const isReusable = this.reusableIds.has(rn.node.id);
        const isInstance = this.instanceIds.has(rn.node.id);
        if (!isRootFrame && !isReusable && !isInstance) continue;
        const sx = rn.absX * this.zoom + this.panX;
        const sy = rn.absY * this.zoom + this.panY;
        this.renderer.drawFrameLabelColored(
          canvas,
          rn.node.name,
          sx,
          sy,
          isReusable,
          isInstance,
          1,
        );
      }
      canvas.restore();

      surface.flush();

      // Keep animating while agent overlays are active (spinning dot + node flashes).
      // Suppressed during captureRegion so waitForSettled can reach a stable state.
      // Uses a ref counter so concurrent captures don't unblock each other.
      if (hasAgentOverlays && this._captureRefcount === 0) {
        this.markDirty();
      }
    } catch (error) {
      this.handleSurfaceFailure('render', error);
    }
  }

  private createSurface(canvasEl: HTMLCanvasElement): Surface | null {
    try {
      return (
        this.ck.MakeWebGLCanvasSurface(canvasEl) ?? this.ck.MakeSWCanvasSurface(canvasEl) ?? null
      );
    } catch (error) {
      console.error('[SkiaEngine] createSurface failed:', error);
      return null;
    }
  }

  private safeDeleteSurface(surface: Surface | null) {
    if (!surface) return;

    try {
      surface.delete();
    } catch (error) {
      console.warn('[SkiaEngine] Failed to delete surface:', error);
    } finally {
      if (surface === this.surface) {
        this.surface = null;
      }
    }
  }

  private recreateSurface(): boolean {
    if (!this.canvasEl) return false;

    this.safeDeleteSurface(this.surface);
    this.surface = this.createSurface(this.canvasEl);
    return !!this.surface;
  }

  private handleSurfaceFailure(stage: string, error: unknown) {
    console.error(`[SkiaEngine] ${stage} failed:`, error);
    if (this.recreateSurface()) {
      this.markDirty();
    }
  }

  // ---------------------------------------------------------------------------
  // Capture utilities
  // ---------------------------------------------------------------------------

  /**
   * Drive the engine to a stable rendered state suitable for readback.
   *
   * This must work regardless of whether the editor tab is focused. The
   * render loop is RAF-driven and `requestAnimationFrame` is heavily
   * throttled (or fully suspended) on background tabs, so a strategy that
   * waits for RAF will stall for the full timeout when the editor isn't
   * focused.
   *
   * Algorithm: each pass force-renders synchronously, then waits for any
   * async font/image loads triggered by that render to complete, then
   * checks whether anything is still pending. We need TWO consecutive
   * stable passes because resolving a pending font/image fires
   * `markDirty()` again, so the very next render must also produce no
   * new work before we can trust the surface contents.
   *
   *   loop:
   *     1. dirty := false; render()         (sync, no RAF)
   *     2. await flushPending(font + image) (resolves what render() just queued)
   *     3. if dirty + pendingCount all 0    → stablePasses++
   *        else                              → stablePasses = 0
   *
   * @param timeoutMs - max wait (default 5000ms). On timeout, logs a warning
   *                    and returns; the caller can still attempt readback.
   */
  async waitForSettled(timeoutMs = 5000): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    const REQUIRED_STABLE_PASSES = 2;
    let stablePasses = 0;
    let passes = 0;

    while (Date.now() < deadline && stablePasses < REQUIRED_STABLE_PASSES) {
      passes++;

      // 1. Force a synchronous render. This:
      //    - clears the dirty flag (we set it to false ourselves since
      //      render() itself does not touch the flag — only the RAF loop
      //      does, and the RAF loop is suspended on background tabs)
      //    - draws the current scene into the surface
      //    - causes node-renderer / text-renderer to issue any image/font
      //      requests they need (which become entries in pendingPromises /
      //      pendingFetches)
      this.dirty = false;
      this.render();

      // 2. Wait for any currently pending async loads to settle. These may
      //    include loads triggered by the render in step 1.
      await this.renderer.fontManager.flushPending();
      await this.renderer.imageLoader.flushPending();

      // 3. Resolving a font/image load fires markDirty() via the loader
      //    callbacks, so dirty may be true again here. Also new pending
      //    items may have been added if the manager wraps loads in chains.
      const isStable =
        !this.dirty &&
        this.renderer.fontManager.pendingCount() === 0 &&
        this.renderer.imageLoader.pendingCount() === 0;

      if (isStable) {
        stablePasses++;
      } else {
        // The previous render either left dirty true (font/image just
        // resolved) or queued new pending work. Restart the stable counter
        // and run another pass.
        stablePasses = 0;
      }

      // Yield to the microtask queue so any chained .then() callbacks
      // (e.g. font load → markDirty) get a chance to run before the next
      // pass observes the dirty flag.
      await Promise.resolve();
    }

    if (stablePasses < REQUIRED_STABLE_PASSES) {
      console.warn(
        `[SkiaEngine.waitForSettled] Timed out after ${timeoutMs}ms (${passes} passes) — capture may have pending loads`,
      );
    }
  }

  /**
   * Capture a region of the live canvas as PNG bytes.
   *
   * @param bounds - scene-space bbox, or 'root' for the whole canvas surface
   * @param opts.dpr - device pixel ratio override (default: window.devicePixelRatio)
   * @param opts.padding - extra pixels around bounds (default 0)
   * @param opts.waitForSettled - wait for async loads (default true)
   * @returns PNG bytes, or null if canvas not ready
   */
  async captureRegion(
    bounds: { x: number; y: number; w: number; h: number } | 'root',
    opts?: { dpr?: number; padding?: number; waitForSettled?: boolean },
  ): Promise<Uint8Array | null> {
    // Validate bounds upfront — this is a parameter contract check, not
    // dependent on any runtime state. Fail loud if the caller passed
    // declared sizing strings (`"fill_container"` / `"fit_content"`)
    // instead of computed pixel coordinates. Done before refcount/try so
    // we don't even bump the refcount on a clearly-invalid call.
    if (bounds !== 'root') {
      const ok =
        Number.isFinite(bounds.x) &&
        Number.isFinite(bounds.y) &&
        Number.isFinite(bounds.w) &&
        Number.isFinite(bounds.h);
      if (!ok) {
        throw new Error(
          `captureRegion: bounds must have numeric x/y/w/h, got ${JSON.stringify(bounds)}`,
        );
      }
    }

    // Block render()'s agent-overlay self-loop for the duration of capture.
    // Without this, an active AI agent painting node indicators would keep
    // dirtying the surface forever and waitForSettled would time out.
    // Ref-counted so overlapping/nested captures don't unblock each other.
    this._captureRefcount++;
    try {
      if (opts?.waitForSettled !== false) {
        // waitForSettled() drives force-renders + flushes pending font/image
        // loads in a loop until two consecutive stable passes. After it
        // returns, the surface is guaranteed to reflect the current scene
        // with all assets resolved (or the timeout warning has been logged).
        await this.waitForSettled();
      } else if (this.surface && this.canvasEl) {
        // Caller opted out of settling — still do one sync render so the
        // surface is at least up-to-date with the current scene state.
        this.dirty = false;
        this.render();
      }

      if (!this.surface || !this.canvasEl) return null;

      // Take a full-surface snapshot via makeImageSnapshot (works across all
      // CanvasKit versions; cropping happens via OffscreenCanvas if needed).
      const snapshot = this.surface.makeImageSnapshot();
      if (!snapshot) return null;

      // Encode to PNG bytes (full surface)
      const fullBytes = snapshot.encodeToBytes();
      snapshot.delete();
      if (!fullBytes) return null;

      // For 'root' or whole-surface capture, return as-is
      if (bounds === 'root') {
        return fullBytes;
      }

      // For specific bounds, crop using OffscreenCanvas
      const dpr = opts?.dpr ?? (typeof window !== 'undefined' ? window.devicePixelRatio : 1);
      const padding = opts?.padding ?? 0;
      const rect = {
        x: Math.max(0, Math.floor((bounds.x - padding) * dpr)),
        y: Math.max(0, Math.floor((bounds.y - padding) * dpr)),
        w: Math.max(1, Math.ceil((bounds.w + padding * 2) * dpr)),
        h: Math.max(1, Math.ceil((bounds.h + padding * 2) * dpr)),
      };

      try {
        const blob = new Blob([new Uint8Array(fullBytes) as unknown as ArrayBuffer], {
          type: 'image/png',
        });
        const bitmap = await createImageBitmap(blob);
        const off = new OffscreenCanvas(rect.w, rect.h);
        const ctx = off.getContext('2d');
        if (!ctx) {
          bitmap.close();
          return fullBytes; // degraded: return full frame rather than fail
        }
        ctx.drawImage(bitmap, rect.x, rect.y, rect.w, rect.h, 0, 0, rect.w, rect.h);
        bitmap.close();
        const croppedBlob = await off.convertToBlob({ type: 'image/png' });
        const croppedBuf = await croppedBlob.arrayBuffer();
        return new Uint8Array(croppedBuf);
      } catch (err) {
        console.warn('[SkiaEngine.captureRegion] crop failed, returning full frame', err);
        return fullBytes;
      }
    } finally {
      // Decrement the ref counter, even if waitForSettled threw or this
      // capture returned early. Concurrent/nested captures rely on the
      // counter never going below zero or skipping a decrement.
      this._captureRefcount = Math.max(0, this._captureRefcount - 1);
      // If this was the LAST in-flight capture and agent overlays are
      // still active, re-arm the dirty flag so the normal render-loop
      // animation resumes on the next RAF tick. Skipped while other
      // captures are still in progress — they'll re-arm when they finish.
      if (this._captureRefcount === 0) {
        const a = getActiveAgentIndicators();
        const f = getActiveAgentFrames();
        if (a.size > 0 || f.size > 0) {
          this.markDirty();
        }
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Viewport control
  // ---------------------------------------------------------------------------

  setViewport(zoom: number, panX: number, panY: number) {
    this.zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, zoom));
    this.panX = panX;
    this.panY = panY;
    useCanvasStore.getState().setZoom(this.zoom);
    useCanvasStore.getState().setPan(this.panX, this.panY);
    this.markDirty();
  }

  zoomToPoint(screenX: number, screenY: number, newZoom: number) {
    if (!this.canvasEl) return;
    const rect = this.canvasEl.getBoundingClientRect();
    const vp = vpZoomToPoint(
      { zoom: this.zoom, panX: this.panX, panY: this.panY },
      screenX,
      screenY,
      rect,
      newZoom,
    );
    this.setViewport(vp.zoom, vp.panX, vp.panY);
  }

  pan(dx: number, dy: number) {
    this.setViewport(this.zoom, this.panX + dx, this.panY + dy);
  }

  getCanvasRect(): DOMRect | null {
    return this.canvasEl?.getBoundingClientRect() ?? null;
  }

  getCanvasSize(): { width: number; height: number } {
    return {
      width: this.canvasEl?.clientWidth ?? 800,
      height: this.canvasEl?.clientHeight ?? 600,
    };
  }

  zoomToFitContent() {
    this.zoomToFitSelectionOrContent([]);
  }

  zoomToFitSelectionOrContent(selectedIds = useCanvasStore.getState().selection.selectedIds) {
    if (!this.canvasEl || this.renderNodes.length === 0) return;

    const selectionSet = new Set(selectedIds);
    const hasRenderableSelection = this.renderNodes.some((rn) => selectionSet.has(rn.node.id));
    const bounds = getFocusBounds(this.renderNodes, selectedIds);
    if (!bounds) return;

    const viewport = fitSceneBoundsToViewport(
      bounds,
      this.canvasEl.clientWidth,
      this.canvasEl.clientHeight,
      {
        padding: 64,
        maxZoom: hasRenderableSelection ? 8 : 1,
      },
    );
    if (!viewport) return;

    this.setViewport(viewport.zoom, viewport.panX, viewport.panY);
  }
}
