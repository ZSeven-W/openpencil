/** Touch/mouse action classes this gate reasons about. */
export enum ViewportInputAction {
  DOWN = 0,
  UP = 1,
  MOVE = 2,
  CANCEL = 3,
}

/**
 * Keeps pointer input aligned with the last COMMITTED native viewport.
 *
 * Unlike Android, ArkUI already reports touch coordinates in vp (the same
 * logical unit the engine uses), so no density conversion happens here. The
 * remaining hazard is identical to the Android/iOS shells: a resize, fold,
 * rotation, or safe-area change may land between the platform's new geometry
 * and the engine's atomic viewport call. Input stays blocked until that call
 * succeeds, and the tail of the interrupted physical stream is suppressed so
 * a stale Up cannot release a gesture that was already cancelled.
 */
export class ViewportInputState {
  private pendingDensityValue: number;
  private committedDensityValue: number;
  private inputBlockedValue: boolean = true;
  private suppressPhysicalStream: boolean = false;

  constructor(initialDensity: number) {
    this.pendingDensityValue = ViewportInputState.validDensity(initialDensity);
    this.committedDensityValue = this.pendingDensityValue;
  }

  /** The device pixel ratio staged for the next atomic viewport call. */
  get pendingDensity(): number {
    return this.pendingDensityValue;
  }

  /** The device pixel ratio the engine currently renders with. */
  get committedDensity(): number {
    return this.committedDensityValue;
  }

  get inputBlocked(): boolean {
    return this.inputBlockedValue;
  }

  /** Returns true when the staged density actually changed. */
  stageDensity(density: number): boolean {
    const next: number = ViewportInputState.validDensity(density);
    if (this.pendingDensityValue === next) {
      return false;
    }
    this.pendingDensityValue = next;
    return true;
  }

  /** Returns true when a previously live gesture stream must be cancelled. */
  beginGeometryUpdate(): boolean {
    const cancelActiveStream: boolean = !this.inputBlockedValue;
    this.inputBlockedValue = true;
    this.suppressPhysicalStream = true;
    return cancelActiveStream;
  }

  /** Publishes density + unblocks input only when the resize succeeded. */
  commitIfSuccessful(status: number): boolean {
    if (status !== 0) {
      return false;
    }
    this.committedDensityValue = this.pendingDensityValue;
    this.inputBlockedValue = false;
    return true;
  }

  /**
   * Filters the tail of the physical stream that was active when geometry
   * changed. Commit may unblock coordinates, but only a fresh primary Down
   * after all old pointers lifted may start a new native gesture.
   */
  acceptsTouch(action: ViewportInputAction, pointerCount: number): boolean {
    if (this.inputBlockedValue) {
      return false;
    }
    if (!this.suppressPhysicalStream) {
      return true;
    }
    if (action === ViewportInputAction.UP || action === ViewportInputAction.CANCEL) {
      this.suppressPhysicalStream = false;
      return false;
    }
    // Some devices deliver a fresh stream as a plain Down without a prior
    // terminal callback after a window/configuration interruption.
    if (action === ViewportInputAction.DOWN && pointerCount === 1) {
      this.suppressPhysicalStream = false;
      return true;
    }
    return false;
  }

  private static validDensity(value: number): number {
    return Number.isFinite(value) && value > 0 ? value : 1;
  }
}
