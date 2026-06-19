// Type declarations for the virtual wasm glue module.
// The virtual specifier 'virtual:op_web_sdk_wasm' is resolved by the Vite plugin
// in vitest.config.ts (to the test stub) and in tsup.config.ts (to the real bundle).
// Using a virtual specifier avoids requiring a physical wasm/ file for tsc or Vitest.
declare module 'virtual:op_web_sdk_wasm' {
  export class Viewer {
    constructor();
    free(): void;
    load_str(src: string): void;
    page_count(): number;
    active_page_index(): number;
    attach_canvas(canvas_id: string): Promise<void>;
    detach(): void;
    mark_dirty(): void;
    push_scene(): void;
    set_viewport(pan_x: number, pan_y: number, zoom: number): void;
    zoom_to_fit(w: number, h: number): void;
    forward_wheel(dx: number, dy: number, ctrl_or_meta: boolean, cursor_x: number, cursor_y: number): void;
    document_json(): string;
    pages_json(): string;
    viewport_json(): string;
  }
  export function _export(viewer: Viewer, format: string): Uint8Array;
  export default function init(module_or_path?: unknown): Promise<unknown>;
}
