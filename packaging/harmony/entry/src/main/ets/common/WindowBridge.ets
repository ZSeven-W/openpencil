import { window, display } from '@kit.ArkUI';
import { hilog } from '@kit.PerformanceAnalysisKit';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/** One atomic viewport tuple in engine units (vp) plus keyboard occlusion. */
export class ViewportBands {
  width: number = 0;
  height: number = 0;
  dpr: number = 1;
  top: number = 0;
  right: number = 0;
  bottom: number = 0;
  left: number = 0;
  keyboard: number = 0;

  clone(): ViewportBands {
    const copy = new ViewportBands();
    copy.width = this.width;
    copy.height = this.height;
    copy.dpr = this.dpr;
    copy.top = this.top;
    copy.right = this.right;
    copy.bottom = this.bottom;
    copy.left = this.left;
    copy.keyboard = this.keyboard;
    return copy;
  }

  equals(other: ViewportBands): boolean {
    return (
      this.width === other.width &&
      this.height === other.height &&
      this.dpr === other.dpr &&
      this.top === other.top &&
      this.right === other.right &&
      this.bottom === other.bottom &&
      this.left === other.left &&
      this.keyboard === other.keyboard
    );
  }
}

/** Notified whenever the window publishes a new viewport tuple. */
export type ViewportListener = (bands: ViewportBands) => void;

/**
 * Owns the main window's edge-to-edge configuration and turns platform
 * geometry into the engine's viewport tuple.
 *
 * This mirrors `EdgeToEdgeWindow.kt`: the drawable surface spans the full
 * window (including the status bar, navigation area, and any display
 * cutout) and the safe-area bands are forwarded SEPARATELY so only the
 * engine's interactive chrome avoids system UI — the canvas backdrop stays
 * visually continuous. System bars remain visible and transparent; they are
 * never hidden. The IME is a separate occlusion channel (Android's
 * `adjustNothing`): showing the keyboard must not resize the editor.
 */
export class WindowBridge {
  static readonly shared: WindowBridge = new WindowBridge();

  private mainWindow: window.Window | null = null;
  private bands: ViewportBands = new ViewportBands();
  private listener: ViewportListener | null = null;
  private lastLightIconPreference: boolean | null = null;

  /** Applies the edge-to-edge window setup and starts observing geometry. */
  async attach(win: window.Window): Promise<void> {
    this.mainWindow = win;
    try {
      // Extend the content below the system bars without hiding them.
      await win.setWindowLayoutFullScreen(true);
      await win.setWindowSystemBarEnable(['status', 'navigation']);
      await win.setWindowSystemBarProperties({
        statusBarColor: '#00000000',
        navigationBarColor: '#00000000',
        // The first frame has not resolved the document/editor theme yet.
        // Start from the viewer's neutral light surface; the frame pump
        // updates this only when the engine-reported preference changes.
        statusBarContentColor: '#FF000000',
        navigationBarContentColor: '#FF000000',
      });
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'edge-to-edge setup failed: %{public}s', JSON.stringify(error));
    }

    try {
      win.on('windowSizeChange', () => this.refresh());
      win.on('avoidAreaChange', () => this.refresh());
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'window observers failed: %{public}s', JSON.stringify(error));
    }
    this.refresh();
  }

  detach(): void {
    const win: window.Window | null = this.mainWindow;
    this.mainWindow = null;
    this.listener = null;
    if (win === null) {
      return;
    }
    try {
      win.off('windowSizeChange');
      win.off('avoidAreaChange');
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'window observer teardown failed: %{public}s', JSON.stringify(error));
    }
  }

  /** The page subscribes here; the current tuple is replayed immediately. */
  setListener(listener: ViewportListener | null): void {
    this.listener = listener;
    if (listener !== null && this.bands.width > 0) {
      listener(this.bands.clone());
    }
  }

  current(): ViewportBands {
    return this.bands.clone();
  }

  /**
   * Keeps system glyphs legible over the theme-colored edge-to-edge bands.
   * HarmonyOS takes the CONTENT color directly, so — unlike Android's
   * "light bars" naming — the engine preference is used as-is.
   */
  applySystemChromeAppearance(prefersLightIcons: boolean): void {
    if (this.lastLightIconPreference === prefersLightIcons) {
      return;
    }
    this.lastLightIconPreference = prefersLightIcons;
    const win: window.Window | null = this.mainWindow;
    if (win === null) {
      return;
    }
    const content: string = prefersLightIcons ? '#FFFFFFFF' : '#FF000000';
    win
      .setWindowSystemBarProperties({
        statusBarColor: '#00000000',
        navigationBarColor: '#00000000',
        statusBarContentColor: content,
        navigationBarContentColor: content,
      })
      .catch((error: Object) => {
        hilog.warn(DOMAIN, TAG, 'system bar update failed: %{public}s', JSON.stringify(error));
      });
  }

  /** Re-reads window bounds, avoid areas, and density into one tuple. */
  refresh(): void {
    const win: window.Window | null = this.mainWindow;
    if (win === null) {
      return;
    }
    let dpr = 1;
    try {
      const info: display.Display = display.getDefaultDisplaySync();
      if (Number.isFinite(info.densityPixels) && info.densityPixels > 0) {
        dpr = info.densityPixels;
      }
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'density read failed: %{public}s', JSON.stringify(error));
    }

    const next = new ViewportBands();
    next.dpr = dpr;
    try {
      const rect = win.getWindowProperties().windowRect;
      next.width = rect.width / dpr;
      next.height = rect.height / dpr;
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'window rect read failed: %{public}s', JSON.stringify(error));
      return;
    }

    // The safe area is the union of the system bars, the display cutout, and
    // the navigation indicator — the exact set Android reads as
    // `systemBars() | displayCutout()`.
    const system = this.avoidArea(win, window.AvoidAreaType.TYPE_SYSTEM);
    const cutout = this.avoidArea(win, window.AvoidAreaType.TYPE_CUTOUT);
    const indicator = this.avoidArea(win, window.AvoidAreaType.TYPE_NAVIGATION_INDICATOR);
    next.top = Math.max(system.top, cutout.top, indicator.top) / dpr;
    next.right = Math.max(system.right, cutout.right, indicator.right) / dpr;
    next.bottom = Math.max(system.bottom, cutout.bottom, indicator.bottom) / dpr;
    next.left = Math.max(system.left, cutout.left, indicator.left) / dpr;

    // The keyboard is a local occluder, never part of the safe area.
    const keyboard = this.avoidArea(win, window.AvoidAreaType.TYPE_KEYBOARD);
    next.keyboard = keyboard.bottom / dpr;

    if (next.equals(this.bands)) {
      return;
    }
    this.bands = next;
    const listener: ViewportListener | null = this.listener;
    if (listener !== null) {
      listener(next.clone());
    }
  }

  private avoidArea(win: window.Window, type: window.AvoidAreaType): AvoidBand {
    try {
      const area: window.AvoidArea = win.getWindowAvoidArea(type);
      return new AvoidBand(
        area.topRect.height,
        area.rightRect.width,
        area.bottomRect.height,
        area.leftRect.width
      );
    } catch (error) {
      return new AvoidBand(0, 0, 0, 0);
    }
  }
}

/** Physical-pixel inset band read from one avoid area. */
class AvoidBand {
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
  readonly left: number;

  constructor(top: number, right: number, bottom: number, left: number) {
    this.top = top;
    this.right = right;
    this.bottom = bottom;
    this.left = left;
  }
}
