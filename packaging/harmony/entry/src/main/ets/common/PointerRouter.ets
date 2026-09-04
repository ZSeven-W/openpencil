import napi, { OpPointerPhase } from './OpNative';
import { PinchZoomDelta } from './PinchZoomDelta';
import { ViewportInputAction, ViewportInputState } from './ViewportInputState';
import { MonotonicClock } from './MonotonicClock';

/** Long-press delay (ms) before a right-click context menu. */
const LONG_PRESS_MS = 500;
/** Movement (vp) that cancels a long-press candidate. */
const LONG_PRESS_SLOP = 8;

/** What the router needs from the engine host. */
export interface PointerRouterHost {
  /** The live engine handle, or 0 when there is none. */
  engineHandle(): number;
  /** Editor mode drives the desktop chrome ladder; viewer mode does not. */
  isEditorMode(): boolean;
  /** A pointer interaction landed: drain shell actions and redraw. */
  onPointerActivity(): void;
  /** The current logical (vp) viewport width, 0 before the first layout. */
  viewportVpWidth(): number;
  /**
   * A long-press landed while an engine text input holds the IME. The host
   * may present a paste menu at `(x, y)` vp and return true; false falls
   * back to the engine's long-press (right-click) path.
   */
  onEditorLongPressPasteMenu(x: number, y: number): boolean;
}

/** One tracked pointer in component-local vp. */
class PointerSample {
  x: number;
  y: number;

  constructor(x: number, y: number) {
    this.x = x;
    this.y = y;
  }
}

/**
 * Turns raw ArkUI pointers into engine calls.
 *
 * Gestures are interpreted BY THE ENGINE, not by this shell: a single-finger
 * tap selects the topmost node under the finger, a single-finger drag pans,
 * and a two-finger pinch zooms around the pinch midpoint. The shell only
 * forwards raw pointers plus the transform-ownership handshake, mirroring
 * `OpSurfaceView.editorTouch` on Android and `OpPlayerView` on iOS.
 *
 * ArkUI already reports touch coordinates in vp — the engine's logical unit
 * with a top-left origin. The ENGINE consumes surface pixels (the Android
 * player forwards physical px), so every coordinate is multiplied by the
 * committed density at this entry point before it crosses the NAPI.
 */
export class PointerRouter {
  private readonly host: PointerRouterHost;
  private readonly inputState: ViewportInputState;
  private readonly active: Map<number, PointerSample> = new Map<number, PointerSample>();

  private primaryPointerId: number = -1;
  private longPressTimer: number = -1;
  private longPressArmed: boolean = false;
  private longPressFired: boolean = false;
  private twoFingerActive: boolean = false;
  private releaseSuppressed: boolean = false;
  private lastMidX: number = 0;
  private lastMidY: number = 0;
  private lastPinchDistance: number = 0;
  private lastKnownX: number = 0;
  private lastKnownY: number = 0;
  private downX: number = 0;
  private downY: number = 0;

  constructor(host: PointerRouterHost, inputState: ViewportInputState) {
    this.host = host;
    this.inputState = inputState;
  }

  /** Entry point for `XComponent.onTouch`. */
  handleTouch(event: TouchEvent): void {
    const engine: number = this.host.engineHandle();
    if (engine === 0) {
      return;
    }

    const action: ViewportInputAction = PointerRouter.actionFor(event.type);
    // A window resize/fold/rotation may have updated ArkUI geometry before
    // the atomic native viewport tuple. Do not mutate the document in that
    // split state; the old stream was cancelled when the transition began.
    if (!this.inputState.acceptsTouch(action, event.touches.length)) {
      return;
    }
    const tMs: number = Math.floor(event.timestamp / 1000000);
    const changed: TouchObject[] = event.changedTouches;
    // ABI contract: editor input is LOGICAL (vp) full-drawable coordinates,
    // which is exactly what ArkUI's TouchObject already reports.
    const px: number = 1;

    switch (action) {
      case ViewportInputAction.DOWN:
        for (const touch of changed) {
          this.onDown(engine, touch.id, touch.x * px, touch.y * px, tMs);
        }
        break;
      case ViewportInputAction.MOVE:
        for (const touch of changed) {
          const sample: PointerSample | undefined = this.active.get(touch.id);
          if (sample !== undefined) {
            sample.x = touch.x * px;
            sample.y = touch.y * px;
          }
        }
        this.onMove(engine, changed, tMs);
        break;
      case ViewportInputAction.UP:
        for (const touch of changed) {
          this.onUp(engine, touch.id, touch.x * px, touch.y * px, tMs);
        }
        break;
      default:
        this.onCancel(engine, tMs);
        break;
    }
  }

  /** Entry point for `XComponent.onMouse` (2in1 / PC and tablets). */
  handleMouse(action: MouseAction, button: MouseButton, x: number, y: number): void {
    const engine: number = this.host.engineHandle();
    if (engine === 0 || !this.host.isEditorMode() || this.inputState.inputBlocked) {
      return;
    }
    this.lastKnownX = x;
    this.lastKnownY = y;
    switch (action) {
      case MouseAction.Press:
        // Left press/release ride the platform's synthesized touch stream
        // (same path as fingers), because forwarding both the mouse event
        // AND its synthesized touch double-fires every press and toggles
        // chrome buttons straight back closed. Only the right button has no
        // touch twin.
        if (button === MouseButton.Right) {
          napi.editorRightPress(engine, x, y);
          this.host.onPointerActivity();
        }
        break;
      case MouseAction.Release:
        break;
      default:
        // Un-pressed cursor motion drives hover highlighting through the
        // capture-free hover channel; a pressed drag already flows through
        // the synthesized touch stream's editorMove.
        napi.editorHover(engine, x, y);
        break;
    }
  }

  /**
   * How many touch pointers are live. `0` while a trackpad drives the
   * surface, which is how the page's PinchGesture tells a two-finger
   * TRACKPAD pinch (no touches; the gesture is the only signal) apart from a
   * two-finger TOUCH pinch (handled right here, with pan, and therefore not
   * to be zoomed twice).
   */
  get activePointers(): number {
    return this.active.size;
  }

  /** Cancels every live stream; used on geometry transitions and teardown. */
  cancelAll(): void {
    const engine: number = this.host.engineHandle();
    this.clearLongPress();
    this.active.clear();
    if (engine === 0) {
      this.reset();
      return;
    }
    if (this.host.isEditorMode()) {
      napi.editorCancelGestureAt(engine, MonotonicClock.nowMs());
    } else {
      // Generic pointer Cancel is global in the engine; coordinates and
      // pointer id are intentionally ignored for cancellation. This cancel
      // is synthetic (no single event's TouchObject owns it), so it
      // carries the shared monotonic engine clock, never a zero timestamp.
      napi.pointer(engine, 0, OpPointerPhase.CANCEL, 0, 0, MonotonicClock.nowMs());
    }
    this.reset();
  }

  // ---- Touch ladder ------------------------------------------------------

  private onDown(engine: number, id: number, x: number, y: number, tMs: number): void {
    this.active.set(id, new PointerSample(x, y));
    if (!this.host.isEditorMode()) {
      napi.pointer(engine, id, OpPointerPhase.DOWN, x, y, tMs);
      return;
    }
    if (this.active.size === 1) {
      this.primaryPointerId = id;
      this.longPressArmed = true;
      this.longPressFired = false;
      this.downX = x;
      this.downY = y;
      this.lastKnownX = x;
      this.lastKnownY = y;
      this.armLongPress();
      napi.editorPressAt(engine, x, y, tMs);
      this.host.onPointerActivity();
      return;
    }
    if (this.active.size === 2) {
      // Two fingers: pan + pinch take over. The first pointer already
      // entered the editor press ladder; cancel that capture before
      // multi-touch starts so no marquee/node drag survives the takeover.
      // The takeover is synthetic (no single event's TouchObject owns it),
      // so it carries the shared monotonic engine clock — the same
      // boot-uptime domain as TouchEvent.timestamp.
      this.clearLongPress();
      napi.editorCancelGestureAt(engine, MonotonicClock.nowMs());
      this.releaseSuppressed = true;
      this.twoFingerActive = true;
      this.lastMidX = this.midpointX();
      this.lastMidY = this.midpointY();
      this.lastPinchDistance = this.pinchDistance();
      napi.editorBeginTransform(engine, this.lastMidX, this.lastMidY);
    }
  }

  private onMove(engine: number, changed: TouchObject[], tMs: number): void {
    if (!this.host.isEditorMode()) {
      for (const touch of changed) {
        napi.pointer(engine, touch.id, OpPointerPhase.MOVE, touch.x, touch.y, tMs);
      }
      return;
    }
    if (this.twoFingerActive && this.active.size >= 2) {
      const midX: number = this.midpointX();
      const midY: number = this.midpointY();
      const distance: number = this.pinchDistance();
      const dx: number = midX - this.lastMidX;
      const dy: number = midY - this.lastMidY;
      const pinchDelta: number = PinchZoomDelta.wheelDelta(this.lastPinchDistance, distance);
      this.lastMidX = midX;
      this.lastMidY = midY;
      this.lastPinchDistance = distance;
      this.lastKnownX = midX;
      this.lastKnownY = midY;
      if (dx !== 0 || dy !== 0) {
        napi.editorPan(engine, midX, midY, dx, dy);
      }
      if (pinchDelta !== 0) {
        napi.editorPinch(engine, midX, midY, pinchDelta);
      }
      return;
    }
    if (this.primaryPointerId < 0) {
      return;
    }
    const sample: PointerSample | undefined = this.active.get(this.primaryPointerId);
    if (sample === undefined) {
      return;
    }
    this.lastKnownX = sample.x;
    this.lastKnownY = sample.y;
    napi.editorMoveAt(engine, sample.x, sample.y, tMs);
    if (this.longPressArmed) {
      const deltaX: number = sample.x - this.downX;
      const deltaY: number = sample.y - this.downY;
      if (deltaX * deltaX + deltaY * deltaY > LONG_PRESS_SLOP * LONG_PRESS_SLOP) {
        this.clearLongPress();
      }
    }
  }

  private onUp(engine: number, id: number, x: number, y: number, tMs: number): void {
    this.active.delete(id);
    if (!this.host.isEditorMode()) {
      napi.pointer(engine, id, OpPointerPhase.UP, x, y, tMs);
      return;
    }
    if (this.twoFingerActive) {
      // End transform ownership before the remaining pointer is re-armed;
      // its eventual Up must never release the press ladder cancelled at
      // second-finger Down.
      napi.editorCancelGestureAt(engine, MonotonicClock.nowMs());
      this.twoFingerActive = false;
      this.longPressArmed = false;
      const remaining: number[] = Array.from(this.active.keys());
      this.primaryPointerId = remaining.length > 0 ? remaining[0] : -1;
    }
    if (this.active.size > 0) {
      return;
    }
    this.clearLongPress();
    if (!this.longPressFired && !this.releaseSuppressed) {
      napi.editorReleaseAt(engine, x, y, tMs);
    }
    this.reset();
    this.host.onPointerActivity();
  }

  private onCancel(engine: number, tMs: number): void {
    // A platform cancellation must never run the release ladder: release may
    // commit a deferred tap, drag/drop, or history.
    this.active.clear();
    if (this.host.isEditorMode()) {
      napi.editorCancelGestureAt(engine, tMs);
    } else {
      napi.pointer(engine, 0, OpPointerPhase.CANCEL, 0, 0, tMs);
    }
    this.clearLongPress();
    this.reset();
  }

  // ---- Long press --------------------------------------------------------

  private armLongPress(): void {
    this.clearLongPress();
    this.longPressArmed = true;
    this.longPressTimer = setTimeout(() => {
      this.longPressTimer = -1;
      this.fireLongPress();
    }, LONG_PRESS_MS);
  }

  private fireLongPress(): void {
    const engine: number = this.host.engineHandle();
    if (engine === 0 || !this.longPressArmed) {
      return;
    }
    this.longPressArmed = false;
    this.longPressFired = true;
    // The Down at press time already ran the engine's press ladder, so the
    // engine's IME focus reflects THIS touch: focused means the finger is
    // holding an editable text field — offer Paste instead of the
    // right-click context menu.
    if (this.host.onEditorLongPressPasteMenu(this.lastKnownX, this.lastKnownY)) {
      // The press capture opened at Down must not leak while the release
      // is suppressed by `longPressFired`.
      napi.editorCancelGestureAt(engine, MonotonicClock.nowMs());
      this.host.onPointerActivity();
      return;
    }
    napi.editorRightPress(engine, this.lastKnownX, this.lastKnownY);
    this.host.onPointerActivity();
  }

  private clearLongPress(): void {
    this.longPressArmed = false;
    if (this.longPressTimer >= 0) {
      clearTimeout(this.longPressTimer);
      this.longPressTimer = -1;
    }
  }

  private reset(): void {
    this.primaryPointerId = -1;
    this.longPressArmed = false;
    this.longPressFired = false;
    this.twoFingerActive = false;
    this.releaseSuppressed = false;
    this.lastMidX = 0;
    this.lastMidY = 0;
    this.lastPinchDistance = 0;
    this.downX = 0;
    this.downY = 0;
  }

  // ---- Geometry ----------------------------------------------------------

  private midpointX(): number {
    let sum = 0;
    this.active.forEach((sample: PointerSample) => {
      sum += sample.x;
    });
    return this.active.size > 0 ? sum / this.active.size : 0;
  }

  private midpointY(): number {
    let sum = 0;
    this.active.forEach((sample: PointerSample) => {
      sum += sample.y;
    });
    return this.active.size > 0 ? sum / this.active.size : 0;
  }

  private pinchDistance(): number {
    const samples: PointerSample[] = Array.from(this.active.values());
    if (samples.length < 2) {
      return 0;
    }
    const dx: number = samples[0].x - samples[1].x;
    const dy: number = samples[0].y - samples[1].y;
    return Math.sqrt(dx * dx + dy * dy);
  }

  private static actionFor(type: TouchType): ViewportInputAction {
    switch (type) {
      case TouchType.Down:
        return ViewportInputAction.DOWN;
      case TouchType.Up:
        return ViewportInputAction.UP;
      case TouchType.Move:
        return ViewportInputAction.MOVE;
      default:
        return ViewportInputAction.CANCEL;
    }
  }
}
