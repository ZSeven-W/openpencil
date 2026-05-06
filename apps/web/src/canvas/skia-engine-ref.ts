/**
 * Module 级别对活动
 * SkiaEngine 实例的单例引用。 Set 在安装时由 SkiaCanvas 执行，在卸载时清除。 Allows
 * 外部代码（键盘快捷键、AI Orchestrator 等），用于调用
 * zoomToFitContent() 等引擎方法，无需进行 prop-drilling。
 */

import type { SkiaEngine } from './skia/skia-engine';

let _engine: SkiaEngine | null = null;

export function setSkiaEngineRef(engine: SkiaEngine | null) {
  _engine = engine;
}

export function getSkiaEngineRef(): SkiaEngine | null {
  return _engine;
}

/**
 * Zoom 并平移，使所有
 * 文档内容都适合可见的画布区域。 Delegates 到活动 SkiaEngine 实例。
 */
export function zoomToFitContent() {
  _engine?.zoomToFitContent();
}

/**
 * Returns
 * 画布元素尺寸（以 CSS 像素为单位）。如果未安装引擎，Falls 返回 800x600。
 */
export function getCanvasSize(): { width: number; height: number } {
  return _engine?.getCanvasSize() ?? { width: 800, height: 600 };
}

/**
 * No-op — 使用
 * Skia 引擎，文档存储始终保持同步。 Previously 需要 Fabric.js，其中画布对象占据权威位置。
 */
export function syncCanvasPositionsToStore() {
  // Skia 引擎在交互过程中将位置直接写入文档存储。保存前需要 No 同步。
}

/**
 * Flag 在下一个选择事
 * 件中跳过深度分辨率。 Used 通过图层面板以编程方式选择子项，而不将它们自动解析到其父组。
 *
 */
let _skipNextDepthResolve = false;
export function setSkipNextDepthResolve() {
  _skipNextDepthResolve = true;
}
export function consumeSkipNextDepthResolve(): boolean {
  const v = _skipNextDepthResolve;
  _skipNextDepthResolve = false;
  return v;
}
