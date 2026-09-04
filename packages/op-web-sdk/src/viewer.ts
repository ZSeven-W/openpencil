// OpViewer wrapper around the wasm Viewer class, providing read-only document snapshots.
import { ensureWasm, WasmViewer, _export } from './wasm.js';
import type { CreateViewerOptions, Viewport } from './types.js';
import type { PenDocument, PenPage } from './ops-types.js';
import { Emitter } from './events.js';
import type { ViewerEvent } from './events.js';
import { viewerWheelInput } from './wheel.js';

let canvasSeq = 0;

/** Ensure the canvas element has an id, assigning one if absent. */
function ensureCanvasId(canvas: HTMLCanvasElement): string {
  if (!canvas.id) canvas.id = `op-web-sdk-canvas-${++canvasSeq}`;
  return canvas.id;
}

/** Convert a string-or-binary source to UTF-8 text for the wasm load_str call. */
function toText(src: string | Uint8Array): string {
  return typeof src === 'string' ? src : new TextDecoder().decode(src);
}

/** Read-only wrapper around the wasm Viewer. Returns typed PenDocument / PenPage snapshots. */
export class OpViewer {
  /** @internal */ wheelHandler?: (e: WheelEvent) => void;
  /** @internal */ canvas?: HTMLCanvasElement;
  private readonly emitter = new Emitter();
  private destroyed = false;

  /** @internal */ constructor(private readonly inner: InstanceType<typeof WasmViewer>) {}

  /** Throw if the viewer has already been destroyed. */
  private assertLive(): void {
    if (this.destroyed) throw new Error('op-web-sdk: viewer has been destroyed');
  }

  /** Subscribe to a viewer event. Returns an unsubscribe function. */
  on(event: ViewerEvent, cb: () => void): () => void {
    return this.emitter.on(event, cb);
  }

  /** Unsubscribe a specific callback from a viewer event. */
  off(event: ViewerEvent, cb: () => void): void {
    this.emitter.off(event, cb);
  }

  /** @internal Fire an event — used by createViewer for wheel-driven viewport changes. */
  emit(event: ViewerEvent): void {
    this.emitter.emit(event);
  }

  /** Load (or reload) a document from a JSON string or binary blob. */
  load(src: string | Uint8Array): void {
    this.assertLive();
    this.inner.load_str(toText(src));
    this.inner.push_scene();
    this.emitter.emit('load');
  }

  /** Parsed document object. Returns a typed PenDocument snapshot. */
  get document(): PenDocument {
    this.assertLive();
    return JSON.parse(this.inner.document_json()) as PenDocument;
  }

  /** Parsed pages array. Returns a typed PenPage[] snapshot. */
  get pages(): PenPage[] {
    this.assertLive();
    return JSON.parse(this.inner.pages_json()) as PenPage[];
  }

  /** Total number of pages in the loaded document. */
  get pageCount(): number {
    this.assertLive();
    return this.inner.page_count();
  }

  /** Zero-based index of the currently active page. */
  get activePage(): number {
    this.assertLive();
    return this.inner.active_page_index();
  }

  /** Current viewport state parsed from wasm (snake_case → camelCase). */
  get viewport(): Viewport {
    this.assertLive();
    const v = JSON.parse(this.inner.viewport_json()) as {
      pan_x: number;
      pan_y: number;
      zoom: number;
    };
    return { panX: v.pan_x, panY: v.pan_y, zoom: v.zoom };
  }

  /** Set viewport pan and zoom simultaneously. */
  setViewport(v: Viewport): void {
    this.assertLive();
    this.inner.set_viewport(v.panX, v.panY, v.zoom);
    this.emitter.emit('viewportchange');
  }

  /** Change zoom level while keeping current pan position. */
  setZoom(z: number): void {
    this.assertLive();
    const c = this.viewport;
    this.inner.set_viewport(c.panX, c.panY, z);
    this.emitter.emit('viewportchange');
  }

  /** Pan to a new position while keeping current zoom level. */
  panTo(panX: number, panY: number): void {
    this.assertLive();
    const c = this.viewport;
    this.inner.set_viewport(panX, panY, c.zoom);
    this.emitter.emit('viewportchange');
  }

  /** Zoom to fit the given width/height into the canvas viewport. */
  zoomToFit(w: number, h: number): void {
    this.assertLive();
    this.inner.zoom_to_fit(w, h);
    this.emitter.emit('viewportchange');
  }

  /** Export the current document to SVG bytes.
   *  Only 'svg' is supported in v1; any other format throws immediately. */
  export(opts: { format: 'svg' }): Uint8Array {
    this.assertLive();
    if (opts.format !== 'svg')
      throw new Error(`op-web-sdk: format "${opts.format}" not supported in v1 (use 'svg')`);
    return _export(this.inner, 'svg');
  }

  /** Tear down the viewer: remove the wheel listener, detach from the canvas,
   *  free the wasm instance, and clear all event subscriptions.
   *  Idempotent — safe to call more than once. */
  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    if (this.canvas && this.wheelHandler)
      this.canvas.removeEventListener('wheel', this.wheelHandler);
    this.inner.detach();
    this.inner.free();
    this.emitter.clear();
  }

  /** @internal Expose the underlying wasm instance for sub-class access. */
  get _inner() {
    return this.inner;
  }
}

/** Initialise the wasm module, create a Viewer bound to the given canvas,
 *  optionally load an initial document, and return an OpViewer. */
export async function createViewer(opts: CreateViewerOptions): Promise<OpViewer> {
  await ensureWasm(opts.wasmUrl);
  const inner = new WasmViewer();
  const viewer = new OpViewer(inner);
  if (opts.doc !== undefined) viewer.load(opts.doc);
  const id = ensureCanvasId(opts.canvas);
  await inner.attach_canvas(id);
  // Store canvas reference and attach a non-passive wheel listener to forward
  // scroll/pinch events to the wasm renderer with cursor-relative coordinates.
  viewer.canvas = opts.canvas;
  viewer.wheelHandler = (e: WheelEvent) => {
    e.preventDefault();
    const r = opts.canvas.getBoundingClientRect();
    const wheel = viewerWheelInput(e, r.width, r.height);
    inner.forward_wheel(
      wheel.deltaX,
      wheel.deltaY,
      wheel.zoom,
      e.clientX - r.left,
      e.clientY - r.top,
    );
    viewer.emit('viewportchange');
  };
  opts.canvas.addEventListener('wheel', viewer.wheelHandler, { passive: false });
  return viewer;
}
