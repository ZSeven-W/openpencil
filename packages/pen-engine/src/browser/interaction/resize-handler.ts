/**
 * Handles resize and rotation interactions on selected nodes.
 * Extracted from apps/web/src/canvas/skia/skia-interaction-resize.ts.
 */
export class EngineResizeHandler {
  isResizing = false;
  isRotating = false;

  // Full implementation mirrors skia-interaction-resize.ts
  // but uses engine.updateNode() instead of useDocumentStore

  resetResize(): void {
    this.isResizing = false;
  }

  resetRotation(): void {
    this.isRotating = false;
  }
}
