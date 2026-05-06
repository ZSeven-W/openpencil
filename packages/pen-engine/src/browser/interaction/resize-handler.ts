/**
 * Handles
 * 在选定节点上调整大小和旋转交互。 Extracted 来自 apps/web/src/canvas/skia/skia
 -interaction-resize.ts。
 */
export class EngineResizeHandler {
  isResizing = false;
  isRotating = false;

  // Full 实现镜像 skia-interaction-resize.ts 但使用
  // engine.updateNode() 而不是 useDocumentStore

  resetResize(): void {
    this.isResizing = false;
  }

  resetRotation(): void {
    this.isRotating = false;
  }
}
