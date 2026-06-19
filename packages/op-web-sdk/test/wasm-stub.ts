// Committed test stub for the wasm glue module.
// Vitest resolves '../wasm/op_web_sdk.js' to this file via the alias in
// vitest.config.ts so tests pass on a fresh clone with no wasm/ dir present.

/** Minimal stand-in for the wasm-bindgen Viewer class. */
export class Viewer {
  free(): void {}
  load_str(_src: string): void {}
  page_count(): number { return 0; }
  active_page_index(): number { return 0; }
  async attach_canvas(_canvas_id: string): Promise<void> {}
  detach(): void {}
  mark_dirty(): void {}
  push_scene(): void {}
  set_viewport(_pan_x: number, _pan_y: number, _zoom: number): void {}
  zoom_to_fit(_w: number, _h: number): void {}
  forward_wheel(_dx: number, _dy: number, _ctrl_or_meta: boolean, _cursor_x: number, _cursor_y: number): void {}
  document_json(): string { return '{}'; }
  pages_json(): string { return '[]'; }
  viewport_json(): string { return '{}'; }
}

/** Minimal stand-in for the wasm _export function. */
export function _export(_viewer: Viewer, _format: string): Uint8Array {
  return new Uint8Array();
}

/** Async no-op standing in for the wasm-bindgen init default export. */
export default async function init(_module_or_path?: unknown): Promise<Record<string, never>> {
  return {};
}
