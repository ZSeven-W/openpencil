/**
 * Pen 用于路径绘制的工
 * 具处理程序。 Extracted 来自 apps/web/src/c
 anvas/skia/skia-pen-tool.ts。
 */
export class EnginePenToolHandler {
  // Full 实现镜像 skia-pen-tool.ts 但使用引擎 API 而不是
  // Zustand 存储
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
