export interface SelectionManagerOptions {
  onChange?: (ids: string[]) => void;
  onHover?: (id: string | null) => void;
}

/**
 * Manages
 * 具有不可变快照的选择状态。 Each select() 调用产生一个新的数组引用。
 */
export class SelectionManager {
  private selectedIds: string[] = [];
  private activeId: string | null = null;
  private hoveredId: string | null = null;
  private onChangeCb?: (ids: string[]) => void;
  private onHoverCb?: (id: string | null) => void;

  constructor(options?: SelectionManagerOptions) {
    this.onChangeCb = options?.onChange;
    this.onHoverCb = options?.onHover;
  }

  /** Returns 当前选择（不可变引用）。 */
  getSelection(): string[] {
    return this.selectedIds;
  }

  /** Returns 活动（主）节点 ID，或 null。 */
  getActiveId(): string | null {
    return this.activeId;
  }

  /** Returns 悬停节点 ID，或 null。 */
  getHoveredId(): string | null {
    return this.hoveredId;
  }

  /** Set 选择。 Creates 一个新的数组引用。 */
  select(ids: string[], activeId?: string): void {
    this.selectedIds = [...ids];
    this.activeId = activeId ?? (ids.length === 1 ? ids[0] : null);
    this.onChangeCb?.(this.selectedIds);
  }

  /** Clear 选择。 */
  clearSelection(): void {
    this.selectedIds = [];
    this.activeId = null;
    this.onChangeCb?.(this.selectedIds);
  }

  /** Set 悬停节点。 */
  setHoveredId(id: string | null): void {
    this.hoveredId = id;
    this.onHoverCb?.(id);
  }
}
