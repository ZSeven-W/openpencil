/**
 * @zseven-w/pen-renderer — OpenPencil (.op) 文件的 Standalone CanvasKit/Skia 渲染器
 *
 * @example
 * ```ts
 * import { loadCanvasKit, PenRenderer } from '@zseven-w/pen-renderer'
 *
 * const ck = await loadCanvasKit('/canvaskit/')
 * const renderer = new PenRenderer(ck, { fontBasePath: '/fonts/' })
 * 渲染器.init(canvas)
 * 渲染器.setDocument(doc)
 * 渲染器.zoomToFit()
 * ```
 */

// ---- Primary API ----
export { loadCanvasKit, getCanvasKit } from './init.js';
export type { LoadCanvasKitOptions } from './init.js';
export { PenRenderer } from './renderer.js';

// ---- Types ----
export type { RenderNode, ViewportState, PenRendererOptions, IconLookupFn } from './types.js';

// ---- Low 级实用程序（用于 apps/web 编辑器重用） ----
export { SkiaNodeRenderer } from './node-renderer.js';
export { SkiaTextRenderer } from './text-renderer.js';
export { SkiaFontManager, BUNDLED_FONT_FAMILIES } from './font-manager.js';
export type {
  FontManagerOptions,
  NativeFontPermission as LocalFontPermission,
} from './font-manager.js';
export { SkiaImageLoader } from './image-loader.js';
export { SpatialIndex } from './spatial-index.js';
export {
  flattenToRenderNodes,
  resolveRefs,
  remapIds,
  premeasureTextHeights,
  collectReusableIds,
  collectInstanceIds,
} from './document-flattener.js';
export {
  viewportMatrix,
  screenToScene,
  sceneToScreen,
  zoomToPoint,
  getViewportBounds,
  isRectInViewport,
} from './viewport.js';
export {
  parseColor,
  cornerRadiusValue,
  cornerRadii,
  resolveFillColor,
  resolveStrokeColor,
  resolveStrokeWidth,
  wrapLine,
  cssFontFamily,
} from './paint-utils.js';
export { sanitizeSvgPath, hasInvalidNumbers, tryManualPathParse } from './path-utils.js';

// ---- Thumbnail 帮助程序 (Phase 7c) ----
export { renderNodeThumbnail } from './render-node-thumbnail.js';
export type { ThumbnailContext } from './render-node-thumbnail.js';
