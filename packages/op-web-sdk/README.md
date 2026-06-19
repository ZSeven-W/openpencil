# @zseven-w/op-web-sdk

Read-only OpenPencil `.op` file viewer SDK for the web, backed by a Rust/wasm renderer.

> **Editing is not supported.** This SDK provides a read-only view of `.op` documents.
> For full editing capability use the [OpenPencil app](https://openpencil.app).

---

## Installation

```bash
bun add @zseven-w/op-web-sdk
# or
npm install @zseven-w/op-web-sdk
```

After installing, copy the wasm assets into your project's public directory:

```bash
cp -r node_modules/@zseven-w/op-web-sdk/wasm ./public/op-wasm
```

Or run the sync script from a monorepo checkout (requires the Rust wasm toolchain):

```bash
bun run sync-wasm
```

---

## Quick start

```ts
import { createViewer } from '@zseven-w/op-web-sdk';

const canvas = document.getElementById('my-canvas') as HTMLCanvasElement;

// createViewer initialises the wasm module, binds it to the canvas, and returns
// a read-only OpViewer.  Pass `doc` to load a document immediately.
const viewer = await createViewer({
  canvas,
  // Optional: override the default wasm asset URL.
  wasmUrl: '/op-wasm/op_web_sdk_bg.wasm',
});

// Load a .op document from a fetch response:
const resp = await fetch('/designs/my-design.op');
const bytes = new Uint8Array(await resp.arrayBuffer());
viewer.load(bytes);

// Read document metadata:
const doc = viewer.document;        // PenDocument
const pages = viewer.pages;         // PenPage[]
console.log(doc.name, viewer.pageCount);

// Control the viewport:
viewer.setZoom(1.5);
viewer.panTo(200, 100);
viewer.zoomToFit(canvas.clientWidth, canvas.clientHeight);

// Export the current view to SVG:
const svgBytes = viewer.export({ format: 'svg' });
const blob = new Blob([svgBytes], { type: 'image/svg+xml' });

// Listen to events:
const unsub = viewer.on('viewportchange', () => {
  console.log('viewport:', viewer.viewport);
});

// Clean up:
viewer.destroy();
```

---

## API reference

### `createViewer(options): Promise<OpViewer>`

Initialises the wasm module and returns a bound `OpViewer`.

| Option | Type | Description |
|---|---|---|
| `canvas` | `HTMLCanvasElement` | Canvas to render into (required). |
| `doc` | `string \| Uint8Array` | Initial document to load (optional). |
| `wasmUrl` | `string` | Override the `.wasm` asset URL (optional). |

---

### `OpViewer`

#### Document

| Member | Signature | Description |
|---|---|---|
| `load` | `(src: string \| Uint8Array): void` | Load or reload a document from JSON string or binary blob. Fires the `'load'` event. |
| `document` | `PenDocument` | Snapshot of the full parsed document. |
| `pages` | `PenPage[]` | Snapshot of the pages array. |
| `pageCount` | `number` | Total number of pages in the document. |
| `activePage` | `number` | Zero-based index of the currently active page. |

> `setActivePage` is not in the v1 read-only surface (single-page rendering). Multi-page
> navigation is deferred to a future release.

#### Viewport

| Member | Signature | Description |
|---|---|---|
| `viewport` | `Viewport` | Current pan + zoom state (`{ panX, panY, zoom }`). |
| `setViewport` | `(v: Viewport): void` | Set pan and zoom simultaneously. |
| `setZoom` | `(z: number): void` | Change zoom level, keeping current pan. |
| `panTo` | `(panX: number, panY: number): void` | Pan to a position, keeping current zoom. |
| `zoomToFit` | `(w: number, h: number): void` | Zoom to fit the given canvas dimensions. |

#### Export

| Member | Signature | Description |
|---|---|---|
| `export` | `(opts: { format: 'svg' }): Uint8Array` | Export the current document. Only `'svg'` is supported in v1. |

#### Events

| Member | Signature | Description |
|---|---|---|
| `on` | `(event: ViewerEvent, cb: () => void): () => void` | Subscribe to a viewer event. Returns an unsubscribe function. |
| `off` | `(event: ViewerEvent, cb: () => void): void` | Unsubscribe a specific callback. |

**ViewerEvent values:** `'load'` | `'viewportchange'`

#### Lifecycle

| Member | Signature | Description |
|---|---|---|
| `destroy` | `(): void` | Remove event listeners, detach from the canvas, free the wasm instance. |

---

## Types

```ts
import type { PenDocument, PenPage, Viewport, ViewerEvent } from '@zseven-w/op-web-sdk';
```

`PenDocument` and `PenPage` are generated from the canonical Rust schema
(`crates/op-web-sdk/bindings/ops.ts` via ts-rs). They are vendored into
`src/ops-types.ts` and re-exported from the package entry point.

---

## Wasm assets note

The package ships a `wasm/` directory containing the compiled wasm bundle. You must
make these files accessible at a URL your browser can fetch. The default URL is
resolved relative to the page; override it via the `wasmUrl` option in `createViewer`.

To re-sync the wasm assets after a Rust build:

```bash
# From packages/op-web-sdk/:
bun run sync-wasm
```

This rebuilds `crates/op-web-sdk` (requires the Rust wasm toolchain + wasm-bindgen),
copies the outputs to `wasm/`, and refreshes `src/ops-types.ts` from the latest
generated bindings.

---

## Editing boundary

This SDK is intentionally **read-only**. It provides:

- Document parsing and typed access (`document`, `pages`)
- GPU-accelerated wasm rendering on a canvas
- Viewport control (pan, zoom, fit)
- SVG export
- Event subscriptions

It does **not** support:

- Node creation, deletion, or mutation
- Selection or drag interactions
- Layer panel, property panel, or toolbar UI
- AI / MCP integrations

For the full editing experience, including AI-powered design generation and
the complete node editing surface, use the [OpenPencil app](https://openpencil.app)
or the `@zseven-w/pen-sdk` package (internal monorepo SDK).

React and Vue adapter packages (`@zseven-w/op-web-sdk-react`,
`@zseven-w/op-web-sdk-vue`) are planned for a future release.

---

## License

MIT
