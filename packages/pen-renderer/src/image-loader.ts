import type { CanvasKit, Image as SkImage } from 'canvaskit-wasm';

const MAX_IMAGE_DIMENSION = 4096;
const MAX_IMAGE_PIXELS = MAX_IMAGE_DIMENSION * MAX_IMAGE_DIMENSION;

export interface ResolvedImageSource {
  cacheKey: string;
  loadUrl: string | null;
}

export interface ImageLoadStatus {
  state: 'loading' | 'loaded' | 'missing' | 'error';
}

/**
 * Async Canvas
 * Kit 的图像加载器。 Loads 图像通过浏览器的本机 Image 元素（支持所有浏览器支持的格式），光栅化为 Canvas 2d，然后转换为
 * CanvasKit Image 以进行 GPU 渲染。
 */
export class SkiaImageLoader {
  private ck: CanvasKit;
  private cache = new Map<string, SkImage | null>();
  private loading = new Set<string>();
  /** In-航班负载承诺（与 `loading` URL 集分开，用于 flushPending） */
  private pendingPromises = new Set<Promise<unknown>>();
  private status = new Map<string, ImageLoadStatus>();
  private onLoaded: (() => void) | null = null;
  private sourceResolver: (src: string) => ResolvedImageSource = (src) => ({
    cacheKey: src,
    loadUrl: src,
  });

  constructor(ck: CanvasKit) {
    this.ck = ck;
  }

  /** Set 回调，用于在图像加载完成时触发重新渲染。 */
  setOnLoaded(cb: () => void) {
    this.onLoaded = cb;
  }

  setSourceResolver(resolver: (src: string) => ResolvedImageSource) {
    this.sourceResolver = resolver;
  }

  /** Get 缓存的图像，如果未加载/失败则为 null。如果尚未请求，则 Returns 未定义。 */
  get(src: string): SkImage | null | undefined {
    const resolved = this.sourceResolver(src);
    return this.cache.get(resolved.cacheKey);
  }

  getStatus(src: string): ImageLoadStatus | undefined {
    const resolved = this.sourceResolver(src);
    return this.status.get(resolved.cacheKey);
  }

  /** Start 加载图像（如果尚未缓存或正在进行中）。 */
  request(src: string) {
    const resolved = this.sourceResolver(src);
    if (this.cache.has(resolved.cacheKey) || this.loading.has(resolved.cacheKey)) return;

    if (!resolved.loadUrl) {
      this.cache.set(resolved.cacheKey, null);
      this.status.set(resolved.cacheKey, { state: 'missing' });
      this.onLoaded?.();
      return;
    }

    this.loading.add(resolved.cacheKey);
    this.status.set(resolved.cacheKey, { state: 'loading' });
    const pending = this.loadAsync(resolved);
    this.pendingPromises.add(pending);
    pending.finally(() => this.pendingPromises.delete(pending));
  }

  /** Number 飞行中图像加载承诺。 */
  pendingCount(): number {
    return this.pendingPromises.size;
  }

  /**
   * Wait 对于每个当前待
   * 处理的图像加载进行解决。 Used 通过 SkiaEngine.waitForSettled
   来协调读回时序。
   */
  async flushPending(): Promise<void> {
    const snapshot = Array.from(this.pendingPromises);
    await Promise.all(snapshot.map((p) => p.catch(() => undefined)));
  }

  dispose() {
    for (const img of this.cache.values()) {
      img?.delete();
    }
    this.cache.clear();
    this.loading.clear();
    this.pendingPromises.clear();
    this.status.clear();
  }

  private async loadAsync(source: ResolvedImageSource) {
    try {
      // Use 浏览器 Image 元素 — 支持所有浏览器支持的格式
      const htmlImg = await this.loadHtmlImage(source.loadUrl!);
      const skImg = this.htmlImageToSkia(htmlImg);
      this.cache.set(source.cacheKey, skImg);
      this.loading.delete(source.cacheKey);
      this.status.set(source.cacheKey, { state: skImg ? 'loaded' : 'error' });
      this.onLoaded?.();
    } catch (e) {
      console.warn('Failed to load image:', source.loadUrl?.slice(0, 80), e);
      this.cache.set(source.cacheKey, null);
      this.loading.delete(source.cacheKey);
      this.status.set(source.cacheKey, { state: 'error' });
      this.onLoaded?.();
    }
  }

  private loadHtmlImage(src: string): Promise<HTMLImageElement> {
    return new Promise((resolve, reject) => {
      const img = new Image();
      if (/^https?:\/\//i.test(src)) {
        img.crossOrigin = 'anonymous';
      }
      img.onload = () => resolve(img);
      img.onerror = (e) => reject(new Error(`Image load failed: ${e}`));
      img.src = src;
    });
  }

  /** Rasterize an HTML Image to Canvas 2D, then convert to CanvasKit Image. */
  private htmlImageToSkia(htmlImg: HTMLImageElement): SkImage | null {
    const sourceW = htmlImg.naturalWidth || htmlImg.width;
    const sourceH = htmlImg.naturalHeight || htmlImg.height;
    if (sourceW <= 0 || sourceH <= 0) return null;

    const { width, height } = this.getSafeRasterSize(sourceW, sourceH);

    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    ctx.imageSmoothingEnabled = width !== sourceW || height !== sourceH;
    ctx.drawImage(htmlImg, 0, 0, width, height);
    const imageData = ctx.getImageData(0, 0, width, height);

    return (
      this.ck.MakeImage(
        {
          width,
          height,
          alphaType: this.ck.AlphaType.Unpremul,
          colorType: this.ck.ColorType.RGBA_8888,
          colorSpace: this.ck.ColorSpace.SRGB,
        },
        imageData.data,
        width * 4,
      ) ?? null
    );
  }

  private getSafeRasterSize(sourceW: number, sourceH: number): { width: number; height: number } {
    let scale = 1;
    const maxDimension = Math.max(sourceW, sourceH);
    if (maxDimension > MAX_IMAGE_DIMENSION) {
      scale = Math.min(scale, MAX_IMAGE_DIMENSION / maxDimension);
    }

    const totalPixels = sourceW * sourceH;
    if (totalPixels > MAX_IMAGE_PIXELS) {
      scale = Math.min(scale, Math.sqrt(MAX_IMAGE_PIXELS / totalPixels));
    }

    return {
      width: Math.max(1, Math.round(sourceW * scale)),
      height: Math.max(1, Math.round(sourceH * scale)),
    };
  }
}
