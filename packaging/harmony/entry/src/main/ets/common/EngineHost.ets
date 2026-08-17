import { http } from '@kit.NetworkKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { OpCallbacks } from 'libopenpencil.so';
import napi, { OpEngineMode, OpShellAction, OpStatus } from './OpNative';
import { ImeConduit, ImeConduitHost } from './ImeConduit';
import { KeyForwarder, KeyModifiers } from './KeyForwarder';
import { PointerRouter, PointerRouterHost } from './PointerRouter';
import { ViewportBands } from './WindowBridge';
import { ViewportInputState } from './ViewportInputState';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/** Cadence of the shell-action / IME / theme drain, in milliseconds. */
const PUMP_INTERVAL_MS = 100;
/** Remote image fetch budget. */
const REMOTE_IMAGE_TIMEOUT_MS = 15000;

/** Main-thread handlers the page installs for engine-requested shell work. */
export interface ShellActionSink {
  onOpenDocument(): void;
  onExportDocument(): void;
  onOpenAccountCenter(): void;
  onRequestLogin(): void;
  onOpenLanguagePicker(): void;
  /** The engine produced a device-login verification URL. */
  onOpenLoginUi(url: string): void;
  /** The engine terminated the login flow itself. */
  onCloseLoginUi(): void;
  /** The engine's backdrop needs light system glyphs. */
  onSystemChromeAppearance(prefersLightIcons: boolean): void;
}

/**
 * Owns the engine handle for one XComponent surface.
 *
 * FRAME PUMP: unlike Android (Choreographer) and iOS (CADisplayLink), the
 * HarmonyOS build pumps frames on the NATIVE side from `OH_NativeVSync`,
 * because the native layer already owns the surface through
 * `OH_NativeXComponent`. ArkTS therefore forwards lifecycle and input only
 * and never calls `frame()` on a timer. What still has to happen on the
 * ArkTS side is draining the engine's shell-action queue, keeping the system
 * keyboard in sync with the engine's IME focus, and following the engine's
 * system-glyph preference; `pump()` does exactly that, driven by input
 * dispatch plus a low-frequency safety-net timer.
 */
export class EngineHost implements PointerRouterHost, ImeConduitHost {
  private engine: number = 0;
  private editorMode: boolean = true;
  private attachedOnce: boolean = false;
  private surfaceId: string = '';
  private destroyed: boolean = false;
  private sink: ShellActionSink | null = null;

  private readonly inputState: ViewportInputState = new ViewportInputState(1);
  private readonly pointerRouter: PointerRouter = new PointerRouter(this, this.inputState);
  private readonly ime: ImeConduit = new ImeConduit(this);

  private bands: ViewportBands = new ViewportBands();
  private keyboardHeight: number = 0;
  private pumpTimer: number = -1;
  private imeFocused: boolean = false;
  private lightIconPreference: boolean | null = null;

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
    hilog.info(
      DOMAIN,
      TAG,
      'engine created (%{public}fx%{public}f dpr=%{public}f)',
      bands.width,
      bands.height,
      bands.dpr
    );
    this.attach();
    this.applyViewport(bands);
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
    if (!this.attachedOnce) {
      const status: number = napi.attachSurface(this.engine, this.surfaceId);
      if (status === OpStatus.OK) {
        this.attachedOnce = true;
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
  }

  /** Blocking barrier before the platform reclaims the surface. */
  suspend(): void {
    if (this.engine === 0) {
      return;
    }
    this.stopPump();
    napi.suspend(this.engine);
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
    this.stopPump();
    this.ime.release();
    this.sink = null;
    if (this.engine === 0) {
      return;
    }
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
    this.pointerRouter.handleTouch(event);
  }

  handleMouse(event: MouseEvent): void {
    this.pointerRouter.handleMouse(event.action, event.button, event.x, event.y);
  }

  /** Returns true when the engine consumed the key. */
  handleKey(keyCode: number, keyText: string, modifiers: KeyModifiers): boolean {
    if (this.engine === 0 || !this.editorMode) {
      return false;
    }
    const handled: boolean = KeyForwarder.handleKeyDown(
      this.engine,
      keyCode,
      keyText,
      modifiers,
      this.ime.isAttached
    );
    if (handled) {
      this.pump();
    }
    return handled;
  }

  onPointerActivity(): void {
    this.pump();
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
    if (this.engine === 0 || !this.editorMode) {
      return;
    }
    for (let i = 0; i < 8; i++) {
      const action: number = napi.editorTakeShellAction(this.engine);
      if (action <= OpShellAction.NONE) {
        break;
      }
      this.dispatchShellAction(action);
    }
    this.syncIme();
    this.syncSystemChromeAppearance();
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
      default:
        hilog.warn(DOMAIN, TAG, 'unknown editor shell action=%{public}d', action);
        break;
    }
  }

  /** The engine's host owns IME focus; keep the system keyboard in sync. */
  private syncIme(): void {
    const focused: boolean = napi.editorImeFocused(this.engine);
    if (focused === this.imeFocused) {
      return;
    }
    this.imeFocused = focused;
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
      onNeedsRedraw: (_hasNextWake: boolean, _nextWakeMs: number): void => {
        // The native side owns the vsync pump, so no frame is scheduled
        // here. A mutation may still have queued a shell action, so drain.
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
