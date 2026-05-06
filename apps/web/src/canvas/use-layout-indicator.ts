/**
 * Layout 重新排序插
 *
 * 入指示器覆盖。 Previously 通过 Fabric.js
 * `after:rende
 r` 钩子渲染。 Now 无操作 — 布局指示器由 Skia 覆盖系统呈现。
 */
export function useLayoutIndicator() {
  // No-op：布局指示器由 SkiaEngine 覆盖渲染器处理。
}
