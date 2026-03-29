import type { CanvasKit } from 'canvaskit-wasm';

let ckInstance: CanvasKit | null = null;
let ckPromise: Promise<CanvasKit> | null = null;

export interface LoadCanvasKitOptions {
  locateFile?: string | ((file: string) => string);
  onProgress?: (loaded: number, total: number) => void;
}

/**
 * Load CanvasKit WASM singleton. Returns the same instance on subsequent calls.
 *
 * @param locateFileOrOptions - Base path string (e.g. '/canvaskit/'), a resolver
 *   function `(file: string) => string`, or a `LoadCanvasKitOptions` object.
 *   Defaults to '/canvaskit/'.
 */
export async function loadCanvasKit(
  locateFileOrOptions?: string | ((file: string) => string) | LoadCanvasKitOptions,
): Promise<CanvasKit> {
  if (ckInstance) return ckInstance;
  if (ckPromise) return ckPromise;

  let resolver: (file: string) => string;
  let onProgress: ((loaded: number, total: number) => void) | undefined;

  if (
    typeof locateFileOrOptions === 'object' &&
    locateFileOrOptions !== null &&
    !('call' in locateFileOrOptions)
  ) {
    const opts = locateFileOrOptions as LoadCanvasKitOptions;
    resolver =
      typeof opts.locateFile === 'function'
        ? opts.locateFile
        : (file: string) => `${opts.locateFile ?? '/canvaskit/'}${file}`;
    onProgress = opts.onProgress;
  } else {
    const locateFile = locateFileOrOptions as string | ((file: string) => string) | undefined;
    resolver =
      typeof locateFile === 'function'
        ? locateFile
        : (file: string) => `${locateFile ?? '/canvaskit/'}${file}`;
  }

  ckPromise = (async () => {
    // canvaskit-wasm is a CJS module (module.exports = CanvasKitInit).
    // Depending on bundler interop, the init function may be on .default or the module itself.
    const mod = await import('canvaskit-wasm');
    const CanvasKitInit =
      typeof mod.default === 'function'
        ? mod.default
        : (mod as unknown as (opts?: {
            locateFile?: (file: string) => string;
          }) => Promise<CanvasKit>);
    const ck = await CanvasKitInit({
      locateFile: resolver,
    });
    ckInstance = ck;
    // Fire final progress (best effort)
    onProgress?.(1, 1);
    return ck;
  })();

  return ckPromise;
}

/**
 * Get the already-loaded CanvasKit instance. Returns null if not yet loaded.
 */
export function getCanvasKit(): CanvasKit | null {
  return ckInstance;
}
