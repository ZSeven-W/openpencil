// Single seam to the Plan-1 wasm bundle. All SDK code imports the wasm
// Viewer through here so unit tests can mock this module.
import init, { Viewer as WasmViewer, _export } from 'virtual:op_web_sdk_wasm';

let started: Promise<unknown> | null = null;

/** Initialise the wasm module exactly once. `url` overrides the .wasm asset URL. */
export async function ensureWasm(url?: string): Promise<void> {
  if (!started) started = init(url ? { module_or_path: url } : undefined);
  await started;
}

export { WasmViewer, _export };
