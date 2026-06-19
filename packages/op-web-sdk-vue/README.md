# @zseven-w/op-web-sdk-vue

Vue 3 adapter for the OpenPencil read-only web viewer SDK. Embed a live OpenPencil design canvas in any Vue 3 application.

> **Read-only boundary:** This package provides a viewer only — no editing. For the full editor experience, use the OpenPencil desktop or web app directly.

## Install

```bash
npm install @zseven-w/op-web-sdk @zseven-w/op-web-sdk-vue
# or
bun add @zseven-w/op-web-sdk @zseven-w/op-web-sdk-vue
```

## Peer dependencies

| Package | Required version |
|---------|-----------------|
| `vue`   | `^3.4.0`        |

## WASM note

The core SDK (`@zseven-w/op-web-sdk`) loads a WebAssembly binary at runtime. You must either:
- Pass the wasm URL explicitly via the `wasmUrl` prop, or
- Serve the wasm file from your own static assets and ensure it is reachable.

The wasm file ships alongside the core package at `@zseven-w/op-web-sdk/dist/op_web_sdk_bg.wasm`.

## Usage

### `<DesignView>` component

The simplest way to embed a design viewer:

```vue
<script setup lang="ts">
import { DesignView } from '@zseven-w/op-web-sdk-vue';
import type { OpViewer } from '@zseven-w/op-web-sdk';

function onLoad(viewer: OpViewer) {
  console.log('viewer ready', viewer.document);
}
</script>

<template>
  <DesignView
    :doc="docStringOrBytes"
    wasmUrl="/assets/op_web_sdk_bg.wasm"
    style="width: 800px; height: 600px"
    @load="onLoad"
  />
</template>
```

**Props:**
- `doc?: string | Uint8Array` — serialized OpenPencil document (JSON string or binary bytes). Omit to start with an empty document.
- `wasmUrl?: string` — URL to the core WASM binary. Required unless your bundler resolves the virtual `virtual:op_web_sdk_wasm` module.

**Events:**
- `load(viewer: OpViewer)` — fired once the viewer is ready.

### Manual provide / inject

For advanced layouts where you need the viewer in child components, call `provideViewer` yourself and use `useViewer` in descendants:

```vue
<!-- parent.vue -->
<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { createViewer } from '@zseven-w/op-web-sdk';
import { provideViewer } from '@zseven-w/op-web-sdk-vue';

const canvasRef = ref<HTMLCanvasElement | null>(null);
onMounted(async () => {
  const viewer = await createViewer({ canvas: canvasRef.value! });
  provideViewer(viewer);
});
</script>
```

```vue
<!-- child.vue -->
<script setup lang="ts">
import { useViewer } from '@zseven-w/op-web-sdk-vue';
const viewer = useViewer(); // throws if no provider above
</script>
```

> **Vue lifecycle note:** `provide()` must be called during `setup()`, not inside `onMounted`. The built-in `<DesignView>` calls `provideViewer` in `onMounted` (after the async `createViewer` resolves), which means synchronous children cannot `inject` the viewer in their own `setup()`. This is acceptable for the common standalone use-case. If you need synchronous child injection, implement your own wrapper that `provide`s a `shallowRef<OpViewer|null>` in `setup()` and fills it on mount.

## Composables

These composables must be called inside a component that is a descendant of a `provideViewer` call (or `<DesignView>`).

| Composable | Returns | Reactive event |
|------------|---------|----------------|
| `useViewer()` | `OpViewer` | — (static reference) |
| `useDocument()` | `Readonly<ShallowRef<PenDocument>>` | `'load'` |
| `useViewport()` | `Readonly<ShallowRef<{panX,panY,zoom}>>` | `'viewportchange'` |
| `useActivePage()` | `Readonly<ShallowRef<number>>` | `'load'` |

Each composable subscribes to the relevant viewer event and automatically unsubscribes when the component scope is disposed.

```vue
<script setup lang="ts">
import { useViewport, useDocument } from '@zseven-w/op-web-sdk-vue';

const viewport = useViewport();  // ShallowRef<{panX, panY, zoom}>
const doc = useDocument();        // ShallowRef<PenDocument>
</script>

<template>
  <p>Zoom: {{ viewport.zoom }}</p>
  <p>Pages: {{ doc.pages?.length }}</p>
</template>
```

## API reference

```ts
// Provider / inject
function provideViewer(viewer: OpViewer): void
function useViewer(): OpViewer         // throws if no provider

// Composables (reactive, auto-dispose)
function useDocument(): Readonly<ShallowRef<PenDocument>>
function useViewport(): Readonly<ShallowRef<{ panX: number; panY: number; zoom: number }>>
function useActivePage(): Readonly<ShallowRef<number>>

// Component
const DesignView: DefineComponent<{ doc?: string | Uint8Array; wasmUrl?: string }, {}, {}, {}, {}, {}, {}, { load: (viewer: OpViewer) => void }>

// Injection key (for advanced use)
const viewerKey: InjectionKey<OpViewer>
```

## License

MIT
