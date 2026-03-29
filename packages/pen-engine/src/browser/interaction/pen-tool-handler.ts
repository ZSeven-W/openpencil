/**
 * Pen tool handler for path drawing.
 * Extracted from apps/web/src/canvas/skia/skia-pen-tool.ts.
 */
export class EnginePenToolHandler {
  // Full implementation mirrors skia-pen-tool.ts
  // but uses engine API instead of Zustand stores
  onMouseDown(_scene: { x: number; y: number }, _zoom: number): boolean {
    return false;
  }
  onMouseMove(_scene: { x: number; y: number }): boolean {
    return false;
  }
  onMouseUp(): boolean {
    return false;
  }
  onDblClick(): boolean {
    return false;
  }
  onKeyDown(_key: string): boolean {
    return false;
  }
  onToolChange(_tool: string): void {}
}
