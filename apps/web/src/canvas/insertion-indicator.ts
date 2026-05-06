// ---------------------------------------------------------------------------
// Shared 插入指示器 + 容器突出显示状态
// Used 由 layout-reorder.ts 和 drag-into-layout.ts 渲染，由
// use-layout-indicator.ts。
// ---------------------------------------------------------------------------

export interface InsertionIndicator {
  x: number;
  y: number;
  length: number;
  orientation: 'vertical' | 'horizontal';
}

export interface ContainerHighlight {
  x: number;
  y: number;
  w: number;
  h: number;
}

export let activeInsertionIndicator: InsertionIndicator | null = null;
export let activeContainerHighlight: ContainerHighlight | null = null;

export function setInsertionIndicator(v: InsertionIndicator | null) {
  activeInsertionIndicator = v;
}

export function setContainerHighlight(v: ContainerHighlight | null) {
  activeContainerHighlight = v;
}
