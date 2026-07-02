import type { DesignEngine } from '../core/design-engine.js';
import { CanvasRenderer } from './canvas-renderer.js';
import { loadCanvasKit } from '@zseven-w/pen-renderer';

export interface CanvasBinding {
  render(): void;
  resize(width: number, height: number): void;
  renderToImageData(width: number, height: number): Promise<Uint8Array>;
  dispose(): void;
}

export interface AttachCanvasOptions {
  canvasKitPath?: string | ((file: string) => string);
  devicePixelRatio?: number;
  backgroundColor?: string;
  fontBasePath?: string;
  googleFontsCssUrl?: string;
  onProgress?: (loaded: number, total: number) => void;
}

/**
 * Initialize CanvasKit WASM and bind engine to a canvas element for GPU rendering.
 * Returns a CanvasBinding for render lifecycle management.
 *
 * The binding subscribes to engine events (document:change, selection:change, etc.)
 * and automatically re-renders when state changes.
 */
export async function attachCanvas(
  engine: DesignEngine,
  canvas: HTMLCanvasElement | OffscreenCanvas,
  options?: AttachCanvasOptions,
): Promise<CanvasBinding> {
  const ck = await loadCanvasKit({
    locateFile: options?.canvasKitPath,
    onProgress: options?.onProgress,
  });

  const renderer = new CanvasRenderer(ck, engine, {
    devicePixelRatio: options?.devicePixelRatio,
    backgroundColor: options?.backgroundColor,
    fontBasePath: options?.fontBasePath,
    googleFontsCssUrl: options?.googleFontsCssUrl,
  });

  renderer.init(canvas);
  renderer.syncFromDocument();

  // Subscribe to engine events for auto-rerender
  const unsubs: (() => void)[] = [];

  unsubs.push(
    engine.on('document:change', () => {
      renderer.syncFromDocument();
    }),
  );

  unsubs.push(
    engine.on('selection:change', () => {
      renderer.markDirty();
    }),
  );

  unsubs.push(
    engine.on('viewport:change', () => {
      renderer.markDirty();
    }),
  );

  unsubs.push(
    engine.on('page:change', () => {
      renderer.syncFromDocument();
    }),
  );

  unsubs.push(
    engine.on('node:hover', () => {
      renderer.markDirty();
    }),
  );

  return {
    render() {
      renderer.render();
    },
    resize(width: number, height: number) {
      renderer.resize(width, height);
    },
    async renderToImageData(width: number, height: number): Promise<Uint8Array> {
      return renderer.renderToImageData(width, height);
    },
    dispose() {
      for (const unsub of unsubs) unsub();
      renderer.dispose();
    },
  };
}
