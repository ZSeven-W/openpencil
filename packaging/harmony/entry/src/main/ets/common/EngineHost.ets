import { http } from '@kit.NetworkKit';
import { deviceInfo } from '@kit.BasicServicesKit';
import { ClipboardBridge } from './ClipboardBridge';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { displaySync } from '@kit.ArkGraphics2D';
import { OpCallbacks } from 'libopenpencil.so';
import napi, { OpEngineMode, OpShellAction, OpStatus } from './OpNative';
import { ImeConduit, ImeConduitHost } from './ImeConduit';
import { PointerRouter, PointerRouterHost } from './PointerRouter';
import { ViewportBands } from './WindowBridge';
import { ViewportInputState } from './ViewportInputState';
import { SystemFontRegistry } from './SystemFontRegistry';
import { BackgroundGenerationCoordinator } from './BackgroundGenerationCoordinator';
import { MonotonicClock } from './MonotonicClock';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/** Cadence of the shell-action / IME / theme drain, in milliseconds. */
const PUMP_INTERVAL_MS = 100;
/** Remote image fetch budget. */
const REMOTE_IMAGE_TIMEOUT_MS = 15000;

import { ShellActionSink, WindowControlKind } from './EngineHostTypes';
export { ShellActionSink, WindowControlKind } from './EngineHostTypes';
/**
 * Owns the engine handle for one XComponent surface.
 *
 * Foreground frames are coalesced onto ArkUI display sync. Once suspended,
 * the frame pump is stopped and only the render-free generation coordinator
 * may tick editor services under a finite HarmonyOS transient-task grant.
 */
export class EngineHost implements PointerRouterHost, ImeConduitHost {
  private engine: number = 0;
  private editorMode: boolean = true;
  private attachedOnce: boolean = false;
  private framePumpTimer: number = -1;
  private frameSync: displaySync.DisplaySync | null = null;
  private framePending: boolean = true;
  private nextWakeAtMs: number = 0;
  private surfaceId: string = '';
  private destroyed: boolean = false;
  private suspended: boolean = false;
  private sink: ShellActionSink | null = null;

  private readonly inputState: ViewportInputState = new ViewportInputState(1);
  private readonly pointerRouter: PointerRouter = new PointerRouter(this, this.inputState);
  private readonly ime: ImeConduit = new ImeConduit(this);
  private readonly backgroundGeneration: BackgroundGenerationCoordinator =
    new BackgroundGenerationCoordinator();

  private bands: ViewportBands = new ViewportBands();
  private keyboardHeight: number = 0;
  private pumpTimer: number = -1;
  private imeFocused: boolean = false;
  private lightIconPreference: boolean | null = null;
  /** 2in1 / PC: desktop chrome, pointer-first input, no soft keyboard. */
  private readonly desktopClass: boolean = deviceInfo.deviceType === '2in1';

  // ---- Lifecycle ---------------------------------------------------------

  setSink(sink: ShellActionSink | null): void {
    this.sink = sink;
  }

  engineHandle(): number {
    return this.engine;
  }

  isEditorMode(): boolean {
    return this.editorMode;
  }

  /** 2in1 / PC: the shell signs in through the system browser, not a page. */
  isDesktopClass(): boolean {
    return this.desktopClass;
  }

  get router(): PointerRouter {
    return this.pointerRouter;
  }

  get imeConduit(): ImeConduit {
    return this.ime;
  }

  /**
   * Creates the engine for `surfaceId` and binds the native surface. The
   * document may be empty: the engine then opens its canonical starter
   * document, exactly like the Android editor build.
   */
  create(
    surfaceId: string,
    doc: ArrayBuffer,
    bands: ViewportBands,
    storageRoot: string,
    editorMode: boolean
  ): boolean {
    if (this.engine !== 0) {
      return true;
    }
    this.surfaceId = surfaceId;
    this.editorMode = editorMode;
    this.bands = bands.clone();
    this.inputState.stageDensity(bands.dpr);

    this.engine = napi.create(
      doc,
      // ABI contract (op_engine.h): create/resize take the LOGICAL viewport
      // plus the device-pixel ratio — ViewportBands' vp values verbatim.
      bands.width,
      bands.height,
      bands.dpr,
      this.callbacks(),
      storageRoot,
      editorMode ? OpEngineMode.EDITOR : OpEngineMode.VIEWER
    );
    if (this.engine === 0) {
      hilog.error(DOMAIN, TAG, 'create failed: %{public}s', napi.lastError(0));
      return false;
    }
    this.backgroundGeneration.bind(this.engine);
    hilog.info(
      DOMAIN,
      TAG,
      'engine created %{public}s',
      `${bands.width}x${bands.height} dpr=${bands.dpr}`
    );
    this.attach();
    // HarmonyOS public directories are not path-writable, so Save / Save As
    // go through `DocumentViewPicker.save` (see `DocumentShell.saveDocument`).
    if (editorMode) {
      napi.editorConfigureSavePicker(this.engine, true);
    }
    // Desktop-class devices get the engine's full desktop chrome (top bar,
    // side panels, pointer-first interactions); phones and tablets keep the
    // touch chrome.
    if (this.desktopClass) {
      napi.editorSetTouchChrome(this.engine, false);
      // No soft keyboard exists here; the conduit still attaches, for
      // composition (see `syncIme`).
      this.ime.setShowKeyboard(false);
    }
    // Until the IME proves it delivers, the native key channel owns
    // printable characters. See `ImeConduit`'s prove-then-suppress rule.
    napi.setImeConduitAttached(false);
    this.applyViewport(bands);
    SystemFontRegistry.registerCjkFonts(this.engine);
    this.startPump();
    return true;
  }


  /** Applies the persisted UI locale right after engine create. */
  applyLocale(tag: string): number {
    if (this.engine === 0 || !this.editorMode) {
      return OpStatus.CLOSING;
    }
    return napi.editorSetLocale(this.engine, tag);
  }

  localeCode(): string | null {
    if (this.engine === 0 || !this.editorMode) {
      return null;
    }
    return napi.editorLocaleCode(this.engine);
  }

  /**
   * Until the FIRST successful attach, GPU mode was never selected and
   * `resume` is invalid: retry `attachSurface` until it succeeds, then use
   * `resume` thereafter. Same ladder as `OpSurfaceView.attachOrResume`.
   */
  attach(): void {
    if (this.engine === 0) {
      return;
    }
    this.backgroundGeneration.enterForeground();
    this.suspended = false;
    if (!this.attachedOnce) {
      const status: number = napi.attachSurface(this.engine, this.surfaceId);
      if (status === OpStatus.OK) {
        this.attachedOnce = true;
        this.startFramePump();
      } else {
        hilog.warn(
          DOMAIN,
          TAG,
          'attach failed status=%{public}d: %{public}s',
          status,
          napi.lastError(this.engine)
        );
      }
      return;
    }
    napi.resume(this.engine, this.surfaceId);
    this.startFramePump();
  }

  /**
   * Main-thread frame pump. The engine's own dirty tracking makes an idle
   * tick cheap, mirroring the Android player's Choreographer barrier; a
   * dedicated native vsync pump can replace this later without touching the
   * shell contract.
   */
  private startFramePump(): void {
    if (this.frameSync !== null || this.framePumpTimer !== -1) {
      return;
    }
    this.framePending = true;
    try {
      // Real vsync alignment: one coalesced frame per display refresh at
      // most. `framePending` merges every redraw request (input, engine
      // callbacks) that arrived since the previous tick.
      const sync = displaySync.create();
      sync.setExpectedFrameRateRange({ expected: 120, min: 60, max: 120 });
      sync.on('frame', () => this.framePumpTick());
      sync.start();
      this.frameSync = sync;
    } catch (error) {
      hilog.warn(
        DOMAIN,
        TAG,
        'displaySync unavailable (%{public}s); falling back to a timer',
        String(error)
      );
      this.framePumpTimer = setInterval(() => this.framePumpTick(), 16);
    }
  }

  private framePumpTick(): void {
    if (this.engine === 0) {
      return;
    }
    // The frame pump feeds the engine's monotonic clock. Never `Date.now()`:
    // ArkUI touch timestamps are boot-uptime nanoseconds, so frame time,
    // the engine's global clock, and raw touch events must share one
    // active-uptime domain or the first touch would jump the clock by the
    // boot offset.
    const now: number = MonotonicClock.nowMs();
    const timedWakeDue: boolean = this.nextWakeAtMs > 0 && now >= this.nextWakeAtMs;
    if (!this.framePending && !timedWakeDue) {
      return;
    }
    this.framePending = false;
    if (timedWakeDue) {
      this.nextWakeAtMs = 0;
    }
    const frameStatus: number = napi.frame(this.engine, now);
    this.pump();
  }

  /** Coalesced redraw request: painted on the next vsync tick. */
  requestFrame(): void {
    this.framePending = true;
  }

  stopFramePump(): void {
    if (this.frameSync !== null) {
      this.frameSync.stop();
      this.frameSync = null;
    }
    if (this.framePumpTimer !== -1) {
      clearInterval(this.framePumpTimer);
      this.framePumpTimer = -1;
    }
  }

  /** Blocking barrier before the platform reclaims the surface. */
  suspend(): void {
    if (this.engine === 0) {
      return;
    }
    // Cancel first, suspend second: a live pointer ladder must never cross
    // the blocking barrier, and the same idempotent call clears the
    // router's active map and long-press timer.
    this.pointerRouter.cancelAll();
    this.backgroundGeneration.prepareForBackground();
    this.suspended = true;
    this.stopPump();
    this.stopFramePump();
    napi.suspend(this.engine);
    this.backgroundGeneration.enterBackground();
  }

  resume(): void {
    if (this.engine === 0) {
      return;
    }
    this.attach();
    this.startPump();
  }

  destroy(): void {
    this.destroyed = true;
    this.suspended = true;
    this.backgroundGeneration.destroy();
    this.stopPump();
    this.stopFramePump();
    this.ime.release();
    this.sink = null;
    if (this.engine === 0) {
      return;
    }
    // Same contract as suspend(): cancel any live stream while the handle
    // is still valid, then destroy.
    this.pointerRouter.cancelAll();
    const engine: number = this.engine;
    this.engine = 0;
    napi.destroy(engine);
  }

  // ---- Viewport ----------------------------------------------------------

  /**
   * Publishes bounds, density, and the four safe-area bands in ONE atomic
   * call so a delayed avoid-area callback can never expose new bounds
   * combined with stale insets. Keyboard occlusion stays a separate channel
   * (`setKeyboard`), which is what keeps the IME from resizing the editor.
   */
  applyViewport(bands: ViewportBands): void {
    if (this.engine === 0 || bands.width <= 0 || bands.height <= 0) {
      return;
    }
    const geometryChanged: boolean =
      bands.width !== this.bands.width ||
      bands.height !== this.bands.height ||
      bands.dpr !== this.bands.dpr ||
      bands.top !== this.bands.top ||
      bands.right !== this.bands.right ||
      bands.bottom !== this.bands.bottom ||
      bands.left !== this.bands.left;
    if (geometryChanged && this.inputState.beginGeometryUpdate()) {
      this.pointerRouter.cancelAll();
    }
    this.bands = bands.clone();
    this.inputState.stageDensity(bands.dpr);
    // ABI contract: logical viewport + dpr + logical safe-area insets.
    const status: number = napi.resizeWithSafeArea(
      this.engine,
      bands.width,
      bands.height,
      bands.dpr,
      bands.top,
      bands.right,
      bands.bottom,
      bands.left
    );
    this.inputState.commitIfSuccessful(status);
    this.setKeyboard(bands.keyboard);
    this.pump();
  }

  setKeyboard(height: number): void {
    if (this.engine === 0) {
      return;
    }
    const next: number = Number.isFinite(height) && height > 0 ? height : 0;
    if (next === this.keyboardHeight) {
      return;
    }
    this.keyboardHeight = next;
    napi.setKeyboard(this.engine, next);
  }

  // ---- Input -------------------------------------------------------------

  handleTouch(event: TouchEvent): void {
    // Engine input almost always dirties the scene; marking here saves one
    // callback round-trip of latency before the vsync tick paints.
    this.framePending = true;
    this.pointerRouter.handleTouch(event);
  }

  handleMouse(event: MouseEvent): void {
    this.framePending = true;
    this.pointerRouter.handleMouse(event.action, event.button, event.x, event.y);
  }

  /**
   * Mouse-wheel / trackpad axis input (2in1). Routed through the engine's
   * panel-aware wheel logic: panels under the cursor scroll, the canvas
   * pans, and Ctrl+wheel zooms around the cursor.
   */
  handleAxis(x: number, y: number, dx: number, dy: number, ctrlDown: boolean): void {
    if (this.engine === 0 || !this.editorMode) {
      return;
    }
    this.framePending = true;
    napi.editorWheel(this.engine, x, y, dx, dy, ctrlDown);
    this.pump();
  }

  viewportVpWidth(): number {
    return this.bands.width;
  }

  onPointerActivity(): void {
    this.pump();
  }

  /**
   * Long-press over a focused engine text input: the page presents the
   * PasteButton security component (its tap grants the one-shot pasteboard
   * read). Desktop-class shells keep the right-click path instead.
   */
  onEditorLongPressPasteMenu(x: number, y: number): boolean {
    if (this.engine === 0 || !this.editorMode || this.desktopClass) {
      return false;
    }
    const sink: ShellActionSink | null = this.sink;
    if (sink === null || !napi.editorImeFocused(this.engine)) {
      return false;
    }
    sink.onEditorPasteMenu(x, y);
    return true;
  }

  pasteText(text: string): void {
    if (this.editorMode && ClipboardBridge.pasteText(this.engine, text)) {
      this.requestFrame();
    }
  }

  onImeInput(): void {
    this.pump();
  }

  // ---- Shell action drain ------------------------------------------------

  private startPump(): void {
    if (this.pumpTimer >= 0 || this.destroyed) {
      return;
    }
    this.pumpTimer = setInterval(() => this.pump(), PUMP_INTERVAL_MS);
  }

  private stopPump(): void {
    if (this.pumpTimer < 0) {
      return;
    }
    clearInterval(this.pumpTimer);
    this.pumpTimer = -1;
  }

  /**
   * Drains every pending shell action, then re-syncs the system keyboard and
   * the system-glyph preference. Actions are consumed only after a blocking
   * NAPI call has returned, so a picker or dialog never opens inside an
   * engine callback or a touch-dispatch stack.
   */
  pump(): void {
    if (this.engine === 0 || !this.editorMode || this.suspended) {
      return;
    }
    for (let i = 0; i < 8; i++) {
      const action: number = napi.editorTakeShellAction(this.engine);
      if (action <= OpShellAction.NONE) {
        break;
      }
      this.dispatchShellAction(action);
    }
    ClipboardBridge.drainCopyText(this.engine);
    this.syncIme();
    this.syncSystemChromeAppearance();
    this.backgroundGeneration.observeForegroundWork();
  }

  takeBackgroundGenerationPaused(): boolean {
    return this.backgroundGeneration.takePausedNotice();
  }

  private dispatchShellAction(action: number): void {
    const sink: ShellActionSink | null = this.sink;
    switch (action) {
      case OpShellAction.OPEN_DOCUMENT:
        if (sink !== null) {
          sink.onOpenDocument();
        }
        break;
      case OpShellAction.OPEN_LOGIN_WEBVIEW: {
        const url: string | null = napi.editorTakeLoginUrl(this.engine);
        if (url === null || url.length === 0) {
          hilog.warn(DOMAIN, TAG, 'login action had no URL; canceling the flow');
          napi.editorCancelLogin(this.engine);
        } else if (sink !== null) {
          sink.onOpenLoginUi(url);
        }
        break;
      }
      case OpShellAction.CLOSE_LOGIN_WEBVIEW:
        if (sink !== null) {
          sink.onCloseLoginUi();
        }
        break;
      case OpShellAction.EXPORT_DOCUMENT:
        if (sink !== null) {
          sink.onExportDocument();
        }
        break;
      case OpShellAction.SAVE_DOCUMENT:
        if (sink !== null) {
          sink.onSaveDocument();
        }
        break;
      case OpShellAction.IMPORT_IMAGE_OR_SVG:
        if (sink !== null) {
          sink.onImportImageOrSvg();
        }
        break;
      case OpShellAction.OPEN_ACCOUNT_CENTER:
        if (sink !== null) {
          sink.onOpenAccountCenter();
        }
        break;
      case OpShellAction.REQUEST_LOGIN:
        if (sink !== null) {
          sink.onRequestLogin();
        }
        break;
      case OpShellAction.OPEN_LANGUAGE_PICKER:
        if (sink !== null) {
          sink.onOpenLanguagePicker();
        }
        break;
      case OpShellAction.WINDOW_CLOSE:
        if (sink !== null) {
          sink.onWindowControl(WindowControlKind.CLOSE);
        }
        break;
      case OpShellAction.WINDOW_MINIMIZE:
        if (sink !== null) {
          sink.onWindowControl(WindowControlKind.MINIMIZE);
        }
        break;
      case OpShellAction.WINDOW_ZOOM:
        if (sink !== null) {
          sink.onWindowControl(WindowControlKind.ZOOM);
        }
        break;
      default:
        hilog.warn(DOMAIN, TAG, 'unknown editor shell action=%{public}d', action);
        break;
    }
  }

  /**
   * The engine's host owns IME focus; keep the system IME in sync.
   *
   * The conduit attaches on EVERY form factor, desktop-class included: it is
   * the only route a composing IME (pinyin, and every other CJK layout) has
   * into the engine, and the native key channel's direct injection is a
   * US-QWERTY table with no composition. The two coexist through
   * `setImeConduitAttached`, which the conduit raises only once the input
   * framework has actually delivered something — so a 2in1 whose IME never
   * engages keeps typing through the native path instead of going dead.
   *
   * Desktop-class hosts attach WITHOUT asking for a soft keyboard: there is
   * no on-screen keyboard to raise on a 2in1, and requesting one would only
   * risk covering the canvas.
   */
  private syncIme(): void {
    const focused: boolean = napi.editorImeFocused(this.engine);
    if (focused === this.imeFocused) {
      return;
    }
    this.imeFocused = focused;
    hilog.info(
      DOMAIN,
      TAG,
      'engine ime focus %{public}s',
      `${focused} desktopClass=${this.desktopClass}`
    );
    const sink: ShellActionSink | null = this.sink;
    if (sink !== null) {
      sink.onImeFocusChanged(focused);
    }
    if (focused) {
      this.ime.show();
    } else {
      this.ime.hide();
    }
  }

  private syncSystemChromeAppearance(): void {
    const next: boolean = napi.prefersLightSystemIcons(this.engine);
    if (this.lightIconPreference === next) {
      return;
    }
    this.lightIconPreference = next;
    const sink: ShellActionSink | null = this.sink;
    if (sink !== null) {
      sink.onSystemChromeAppearance(next);
    }
  }

  // ---- Login / export / document passthroughs ----------------------------

  /**
   * Hands the engine's auth runtime its shell-owned storage directory, this
   * device's name, the app version, and the regional SSO deployment
   * (`OpAuthRegion`: 0 = China, 1 = Global). Called AT MOST ONCE per process
   * by `AuthRuntime` — the runtime locks its origin on the first call.
   */
  configureAuth(
    storageDir: string,
    deviceName: string,
    appVersion: string,
    region: number
  ): number {
    if (this.engine === 0 || !this.editorMode) {
      return OpStatus.CLOSING;
    }
    return napi.editorConfigureAuth(this.engine, storageDir, deviceName, appVersion, region);
  }

  /** Starts a device-login pairing; the URL arrives as shell action 2. */
  beginLogin(): number {
    if (this.engine === 0 || !this.editorMode) {
      return OpStatus.CLOSING;
    }
    return napi.editorBeginLogin(this.engine);
  }

  /** JSON profile snapshot, or null when the engine has no account. */
  accountSnapshot(): string | null {
    if (this.engine === 0 || !this.editorMode) {
      return null;
    }
    return napi.editorAccountSnapshot(this.engine);
  }

  signOut(): number {
    if (this.engine === 0 || !this.editorMode) {
      return OpStatus.CLOSING;
    }
    const status: number = napi.editorSignOut(this.engine);
    if (status === OpStatus.OK) {
      this.pump();
    }
    return status;
  }

  /** User close/back or an unsupported flow: cancel the single auth flow. */
  cancelLogin(): void {
    if (this.engine === 0 || !this.editorMode) {
      return;
    }
    const status: number = napi.editorCancelLogin(this.engine);
    if (status !== OpStatus.OK && status !== OpStatus.CLOSING) {
      hilog.info(DOMAIN, TAG, 'login cancel returned status=%{public}d', status);
    }
  }

  openDocument(bytes: ArrayBuffer, displayName: string): number {
    if (this.engine === 0 || !this.editorMode) {
      return OpStatus.CLOSING;
    }
    const status: number = napi.editorOpenDocument(this.engine, bytes, displayName);
    if (status === OpStatus.OK) {
      this.pump();
    }
    return status;
  }

  importImageOrSvg(bytes: ArrayBuffer, displayName: string): number {
    if (this.engine === 0 || !this.editorMode) {
      return OpStatus.CLOSING;
    }
    return napi.editorImportImageOrSvg(this.engine, bytes, displayName);
  }

  exportFileName(): string | null {
    if (this.engine === 0 || !this.editorMode) {
      return null;
    }
    return napi.editorExportFileName(this.engine);
  }

  exportToPath(path: string): number {
    if (this.engine === 0 || !this.editorMode) {
      return OpStatus.CLOSING;
    }
    return napi.editorExportToPath(this.engine, path);
  }

  cancelExport(): void {
    if (this.engine === 0 || !this.editorMode) {
      return;
    }
    const status: number = napi.editorCancelExport(this.engine);
    if (status !== OpStatus.OK && status !== OpStatus.CLOSING) {
      hilog.info(DOMAIN, TAG, 'export cancel returned status=%{public}d', status);
    }
  }

  lastError(): string {
    return napi.lastError(this.engine);
  }

  // ---- Engine upcalls ----------------------------------------------------

  private callbacks(): OpCallbacks {
    return {
      onNeedsRedraw: (hasNextWake: boolean, nextWakeMs: number): void => {
        // Coalesce: mark pending and let the vsync tick paint exactly one
        // frame per refresh. Painting here directly would double-draw on
        // top of the pump and saturate the main thread under fast input.
        this.framePending = true;
        if (hasNextWake && nextWakeMs > 0) {
          this.nextWakeAtMs = nextWakeMs;
        }
        this.pump();
      },
      onRuntimeError: (kind: number, message: string, source: string | null): void => {
        hilog.error(
          DOMAIN,
          TAG,
          'runtime error kind=%{public}d: %{public}s (%{public}s)',
          kind,
          message,
          source ?? ''
        );
      },
      onInputFocusChanged: (focused: boolean, inputKind: number, returnKeyHint: number): void => {
        // Editor mode polls `editorImeFocused` after every interaction; this
        // callback serves the viewer-mode text ABI.
        hilog.info(
          DOMAIN,
          TAG,
          'input focus changed: focused=%{public}s kind=%{public}d hint=%{public}d',
          `${focused}`,
          inputKind,
          returnKeyHint
        );
      },
      onRemoteImageRequest: (requestId: number, url: string): void => {
        this.fetchRemoteImage(requestId, url);
      },
      // Collaboration credentials need a hardware-backed store. HarmonyOS
      // has no OpenPencil credential store yet, so the engine is told the
      // device key does not exist and cannot be created (collaboration stays
      // unavailable rather than falling back to plaintext storage).
      onCredentialLoad: (): ArrayBuffer | null => null,
      onCredentialStoreIfAbsent: (_value: ArrayBuffer): boolean => false,
    };
  }

  /** Fetches a remote image URL and pushes the bytes back; empty = failure. */
  private fetchRemoteImage(requestId: number, url: string): void {
    const request: http.HttpRequest = http.createHttp();
    request
      .request(url, {
        method: http.RequestMethod.GET,
        expectDataType: http.HttpDataType.ARRAY_BUFFER,
        connectTimeout: REMOTE_IMAGE_TIMEOUT_MS,
        readTimeout: REMOTE_IMAGE_TIMEOUT_MS,
      })
      .then((response: http.HttpResponse) => {
        const ok: boolean = response.responseCode >= 200 && response.responseCode <= 299;
        const bytes: ArrayBuffer =
          ok && response.result instanceof ArrayBuffer ? response.result : new ArrayBuffer(0);
        if (this.engine !== 0) {
          napi.remoteImageResult(this.engine, requestId, bytes);
        }
      })
      .catch((error: Object) => {
        hilog.warn(DOMAIN, TAG, 'remote image failed: %{public}s', JSON.stringify(error));
        if (this.engine !== 0) {
          napi.remoteImageResult(this.engine, requestId, new ArrayBuffer(0));
        }
      })
      .finally(() => {
        request.destroy();
      });
  }
}
