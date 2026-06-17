/* tslint:disable */
/* eslint-disable */

/**
 * Long-lived shell handle. The smoke HTML must keep this alive (e.g.
 * `window.__opShell = mount("op")`) so closures stored on the shell
 * remain reachable for the page lifetime.
 *
 * The stub variant (without `skia` feature) carries no fields and exists
 * only so the wasm32-unknown-unknown CI baseline can compile-check the
 * public surface.
 */
export class WebShell {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
}

/**
 * Smoke entry retained for FFI validation (renders AA text + a fill).
 */
export function ck_smoke(canvas_id: string): Promise<void>;

/**
 * Stub mount used by the kickoff §1.2 wasm32-clean compile guard CI.
 * Returns a fields-less `WebShell` after verifying the host has a
 * canvas with the given id; never paints. Real rendering needs the
 * `skia` feature.
 */
export function mount(canvas_id: string): WebShell;

/**
 * Mount the full editor chrome on `canvas_id`, rendered via CanvasKit on the
 * GPU, with mouse / wheel / keyboard interactivity. Builds the shared
 * `WidgetHost` (skia-free under this feature) and drives it through
 * `CanvasKitBackend`, behind the same `RenderBackend` the desktop host uses.
 */
export function mount_ck(canvas_id: string): Promise<void>;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly ck_smoke: (a: number, b: number) => any;
    readonly mount_ck: (a: number, b: number) => any;
    readonly __wbg_webshell_free: (a: number, b: number) => void;
    readonly mount: (a: number, b: number) => [number, number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h6de093649961a993: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h39c63ed10843f7f5: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h64e255db661e882a: (a: number, b: number, c: any) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
