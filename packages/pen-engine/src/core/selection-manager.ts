export interface SelectionManagerOptions {
  onChange?: (ids: string[]) => void;
  onHover?: (id: string | null) => void;
}

/**
 * Manages selection state with immutable snapshots.
 * Each select() call produces a new array reference.
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

  /** Returns the current selection (immutable reference). */
  getSelection(): string[] {
    return this.selectedIds;
  }

  /** Returns the active (primary) node ID, or null. */
  getActiveId(): string | null {
    return this.activeId;
  }

  /** Returns the hovered node ID, or null. */
  getHoveredId(): string | null {
    return this.hoveredId;
  }

  /** Set the selection. Creates a new array reference. */
  select(ids: string[], activeId?: string): void {
    this.selectedIds = [...ids];
    this.activeId = activeId ?? (ids.length === 1 ? ids[0] : null);
    this.onChangeCb?.(this.selectedIds);
  }

  /** Clear selection. */
  clearSelection(): void {
    this.selectedIds = [];
    this.activeId = null;
    this.onChangeCb?.(this.selectedIds);
  }

  /** Set the hovered node. */
  setHoveredId(id: string | null): void {
    this.hoveredId = id;
    this.onHoverCb?.(id);
  }
}
