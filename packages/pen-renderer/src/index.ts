/**
 * @zseven-w/pen-renderer — Standalone CanvasKit/Skia renderer for OpenPencil (.op) files
 *
 * @example
 * ```ts
 * import { loadCanvasKit, PenRenderer } from '@zseven-w/pen-renderer'
 *
 * const ck = await loadCanvasKit('/canvaskit/')
 * const renderer = new PenRenderer(ck, { fontBasePath: '/fonts/' })
 * renderer.init(canvas)
 * renderer.setDocument(doc)
 * renderer.zoomToFit()
 * ```
 */

// ---- Primary API ----
export { loadCanvasKit, getCanvasKit } from './init.js';
export type { LoadCanvasKitOptions } from './init.js';
export { PenRenderer } from './renderer.js';

// ---- Types ----
export type { RenderNode, ViewportState, PenRendererOptions, IconLookupFn } from './types.js';

// ---- Low-level utilities (for apps/web editor re-use) ----
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

// ---- Thumbnail helper (Phase 7c) ----
export { renderNodeThumbnail } from './render-node-thumbnail.js';
export type { ThumbnailContext } from './render-node-thumbnail.js';
