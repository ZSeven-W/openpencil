import type { ViewportState } from '@zseven-w/pen-types';
import { MIN_ZOOM } from '@zseven-w/pen-core';

/** Maximum 视口的缩放级别 (64x)。 */
const VIEWPORT_MAX_ZOOM = 64;

export interface ViewportControllerOptions {
  onChange?: (state: ViewportState) => void;
}

/**
 * Pure 数学视口控制器
 * ——无 canvas/DOM 依赖性。 Manages 缩放、平移和坐标变换。 Extracted 来自
 * apps/web/src/canvas/skia/skia-engine.ts 视口逻辑。
 */
export class ViewportController {
  private _zoom = 1;
  private _panX = 0;
  private _panY = 0;
  private onChangeCb?: (state: ViewportState) => void;

  constructor(options?: ViewportControllerOptions) {
    this.onChangeCb = options?.onChange;
  }

  get zoom(): number {
    return this._zoom;
  }
  get panX(): number {
    return this._panX;
  }
  get panY(): number {
    return this._panY;
  }

  /** Set 具有缩放限制的视口状态。 */
  setViewport(zoom: number, panX: number, panY: number): void {
    this._zoom = Math.max(MIN_ZOOM, Math.min(VIEWPORT_MAX_ZOOM, zoom));
    this._panX = panX;
    this._panY = panY;
    this.onChangeCb?.({ zoom: this._zoom, panX: this._panX, panY: this._panY });
  }

  /**
   * Convert
   * 屏幕坐标到场景坐标。 For 在没有画布矩形的情况下使用（假设原点为 0,0）。
   */
  screenToScene(screenX: number, screenY: number): { x: number; y: number } {
    return {
      x: (screenX - this._panX) / this._zoom,
      y: (screenY - this._panY) / this._zoom,
    };
  }

  /**
   * Convert 场景坐标到屏幕坐标。
   */
  sceneToScreen(sceneX: number, sceneY: number): { x: number; y: number } {
    return {
      x: sceneX * this._zoom + this._panX,
      y: sceneY * this._zoom + this._panY,
    };
  }

  /**
   * Zoom 在容器内放置一
   * 个矩形。 Does 不会缩放超过 1 倍（避免过度缩放小内容）。
   */
  zoomToRect(
    x: number,
    y: number,
    w: number,
    h: number,
    containerW: number,
    containerH: number,
    padding = 0,
  ): void {
    if (w <= 0 || h <= 0) return;
    const scaleX = (containerW - padding * 2) / w;
    const scaleY = (containerH - padding * 2) / h;
    let zoom = Math.min(scaleX, scaleY, 1);
    zoom = Math.max(MIN_ZOOM, Math.min(VIEWPORT_MAX_ZOOM, zoom));
    const centerX = x + w / 2;
    const centerY = y + h / 2;
    this.setViewport(zoom, containerW / 2 - centerX * zoom, containerH / 2 - centerY * zoom);
  }

  /** Get 将视口状态作为普通对象。 */
  getState(): ViewportState {
    return { zoom: this._zoom, panX: this._panX, panY: this._panY };
  }
}
