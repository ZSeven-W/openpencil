# @zseven-w/op-web-sdk-react

React adapter for the OpenPencil read-only web viewer SDK. Embed a live OpenPencil design canvas in any React 19 application.

> **Read-only boundary:** This package provides a viewer only — no editing. For the full editor experience, use the OpenPencil desktop or web app directly.

## Install

```bash
npm install @zseven-w/op-web-sdk @zseven-w/op-web-sdk-react
# or
bun add @zseven-w/op-web-sdk @zseven-w/op-web-sdk-react
```

## Peer dependencies

| Package      | Required version |
|--------------|-----------------|
| `react`      | `^19.0.0`       |
| `react-dom`  | `^19.0.0`       |

## WASM note

The core SDK (`@zseven-w/op-web-sdk`) loads a WebAssembly binary at runtime. You must either:
- Pass the wasm URL explicitly via the `wasmUrl` prop, or
- Serve the wasm file from your own static assets and ensure it is reachable.

The wasm file ships alongside the core package at `@zseven-w/op-web-sdk/dist/op_web_sdk_bg.wasm`.

## Usage

### `<DesignView>` component

The simplest way to embed a design viewer:

```tsx
import { DesignView } from '@zseven-w/op-web-sdk-react';
import type { OpViewer } from '@zseven-w/op-web-sdk';

export function MyPage() {
  function handleLoad(viewer: OpViewer) {
    console.log('viewer ready', viewer.document);
  }

  return (
    <DesignView
      doc={docStringOrBytes}
      wasmUrl="/assets/op_web_sdk_bg.wasm"
      onLoad={handleLoad}
      style={{ width: 800, height: 600 }}
    />
  );
}
```

**Props (`DesignViewProps`):**
- `doc?: string | Uint8Array` — serialized OpenPencil document (JSON string or binary bytes). Omit to start with an empty document.
- `wasmUrl?: string` — URL to the core WASM binary.
- `onLoad?: (viewer: OpViewer) => void` — called once the viewer is ready.
- `className?: string` — CSS class applied to the wrapper div.
- `children?: ReactNode` — child components that can call viewer hooks (rendered only after the viewer is ready, inside `DesignProvider`).

### Manual provider

For advanced layouts, wrap your subtree with `DesignProvider` and use `useViewer` in descendants:

```tsx
import { useState, useEffect, useRef } from 'react';
import { createViewer } from '@zseven-w/op-web-sdk';
import { DesignProvider } from '@zseven-w/op-web-sdk-react';

export function MyLayout() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [viewer, setViewer] = useState(null);

  useEffect(() => {
    let cancelled = false;
    createViewer({ canvas: canvasRef.current! }).then((v) => {
      if (cancelled) { v.destroy(); return; }
      setViewer(v);
    });
    return () => { cancelled = true; viewer?.destroy(); };
  }, []);

  return (
    <>
      <canvas ref={canvasRef} />
      {viewer && (
        <DesignProvider viewer={viewer}>
          <MyChildComponents />
        </DesignProvider>
      )}
    </>
  );
}
```

## Hooks

These hooks must be called inside a component that is a descendant of `DesignProvider` (or `<DesignView>`).

| Hook | Returns | Reactive event |
|------|---------|----------------|
| `useViewer()` | `OpViewer` | — (stable reference) |
| `useDocument()` | `PenDocument` | `'load'` |
| `useViewport()` | `{ panX: number; panY: number; zoom: number }` | `'viewportchange'` |
| `useActivePage()` | `number` | `'load'` |

Each hook uses `useSyncExternalStore` internally and re-renders only when the relevant viewer event fires.

```tsx
import { useViewport, useDocument } from '@zseven-w/op-web-sdk-react';

export function ZoomDisplay() {
  const viewport = useViewport();  // { panX, panY, zoom }
  const doc = useDocument();        // PenDocument

  return (
    <div>
      <p>Zoom: {viewport.zoom}</p>
      <p>Pages: {doc.pages?.length}</p>
    </div>
  );
}
```

## API reference

```ts
// Provider / context
function DesignProvider(props: { viewer: OpViewer; children?: ReactNode }): JSX.Element
function useViewer(): OpViewer         // throws if no provider above

// Hooks (re-render on event)
function useDocument(): PenDocument
function useViewport(): { panX: number; panY: number; zoom: number }
function useActivePage(): number

// Component
function DesignView(props: DesignViewProps): JSX.Element

interface DesignViewProps {
  doc?: string | Uint8Array;
  wasmUrl?: string;
  onLoad?: (viewer: OpViewer) => void;
  className?: string;
  children?: ReactNode;
}
```

## License

MIT
