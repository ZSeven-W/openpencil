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
 * Initialize
 * CanvasKit WASM 并将引擎绑定到画布元素以进行 GPU 渲染。 Returns 和 CanvasBinding 用于渲染生命周期管理。 The
 *
 * 绑定订阅引擎事件（文档：更改、选择：更改等）并在状态更改时自动重新渲染。
 *
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

  // Subscribe 用于自动重新渲染的引擎事件
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
