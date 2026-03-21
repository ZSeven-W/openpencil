import type { CanvasKit } from 'canvaskit-wasm'

let ckInstance: CanvasKit | null = null
let ckPromise: Promise<CanvasKit> | null = null

/**
 * Load CanvasKit WASM singleton. Returns the same instance on subsequent calls.
 *
 * @param locateFile - Base path string (e.g. '/canvaskit/') or a function
 *   `(file: string) => string` that resolves WASM file URLs. Defaults to '/canvaskit/'.
 */
export async function loadCanvasKit(
  locateFile?: string | ((file: string) => string),
): Promise<CanvasKit> {
  if (ckInstance) return ckInstance
  if (ckPromise) return ckPromise

  const resolver: (file: string) => string = typeof locateFile === 'function'
    ? locateFile
    : (file: string) => `${locateFile ?? '/canvaskit/'}${file}`

  ckPromise = (async () => {
    // canvaskit-wasm is a CJS module (module.exports = CanvasKitInit).
    // Depending on bundler interop, the init function may be on .default or the module itself.
    const mod = await import('canvaskit-wasm')
    const CanvasKitInit = typeof mod.default === 'function'
      ? mod.default
      : (mod as unknown as (opts?: { locateFile?: (file: string) => string }) => Promise<CanvasKit>)
    const ck = await CanvasKitInit({
      locateFile: resolver,
    })
    ckInstance = ck
    return ck
  })()

  return ckPromise
}

/**
 * Get the already-loaded CanvasKit instance. Returns null if not yet loaded.
 */
export function getCanvasKit(): CanvasKit | null {
  return ckInstance
}
