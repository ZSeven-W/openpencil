import { common, ConfigurationConstant } from '@kit.AbilityKit';
import { pasteboard } from '@kit.BasicServicesKit';
import napi from '../common/OpNative';
import { fileIo as fs } from '@kit.CoreFileKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { promptAction } from '@kit.ArkUI';
import { AccountCenterSheet } from '../common/AccountCenterSheet';
import { AccountSnapshot } from '../common/AccountSnapshot';
import { AuthRuntime } from '../common/AuthRuntime';
import { BrowserLauncher } from '../common/BrowserLauncher';
import { ConfirmDialog } from '../common/ConfirmDialog';
import { DeviceLoginRequest } from '../common/DeviceLoginRequest';
import { DocumentShell, ShellOutcome } from '../common/DocumentShell';
import { DocumentSaveBridge } from '../common/DocumentSaveBridge';
import { EngineHost, ShellActionSink, WindowControlKind } from '../common/EngineHost';
import { EngineLanguage } from '../common/EngineLanguage';
import { IME_PROXY_ID, ImeProxyBridge } from '../common/ImeProxyBridge';
import { LanguagePickerDialog } from '../common/LanguagePickerDialog';
import { NativeLoginPage } from '../common/NativeLoginPage';
import { NoticeDialog } from '../common/NoticeDialog';
import { PinchZoomDelta } from '../common/PinchZoomDelta';
import { SsoRegion, SsoRegionStore } from '../common/SsoRegion';
import { ViewportBands, WindowBridge } from '../common/WindowBridge';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/** Must match the id handed to `attachSurface`; the native side resolves it. */
const SURFACE_ID = 'openpencil_surface';

/**
 * The whole application UI.
 *
 * The engine paints everything inside the XComponent surface. ArkTS forwards
 * lifecycle (create / resize / destroy), raw pointers, keys, and the IME —
 * it never interprets a gesture and never draws editor chrome. Frames are
 * pumped on the NATIVE side from `OH_NativeVSync`, so this page has no
 * render loop; it only publishes viewport tuples and drains shell actions.
 */
@Entry
@Component
struct Index {
  private host: EngineHost = new EngineHost();
  private context: common.UIAbilityContext = getContext(this) as common.UIAbilityContext;
  private bands: ViewportBands = new ViewportBands();
  private surfaceReady: boolean = false;
  private engineStarted: boolean = false;
  private documentOpenInProgress: boolean = false;
  private imageImportInProgress: boolean = false;
  private exportInProgress: boolean = false;
  private saveInProgress: boolean = false;

  private readonly imeProxy: ImeProxyBridge = new ImeProxyBridge();
  private readonly imeProxyController: TextInputController = new TextInputController();
  @State private imeAnchorX: number = 0;
  @State private imeAnchorY: number = 0;

  @State private showLanguagePicker: boolean = false;
  @State private currentLocale: string = '';
  @State private showNotice: boolean = false;
  @State private noticeMessage: ResourceStr = '';

  // ---- Long-press paste menu ---------------------------------------------

  @State private showPasteMenu: boolean = false;
  @State private pasteMenuX: number = 0;
  @State private pasteMenuY: number = 0;

  // ---- Sign-in state -----------------------------------------------------

  private readonly auth: AuthRuntime = new AuthRuntime(
    getContext(this) as common.UIAbilityContext
  );
  private loginRequestInFlight: boolean = false;
  @State private night: boolean = false;
  @State private showLogin: boolean = false;
  @State private loginOrigin: string = '';
  @State private loginPairingId: string = '';
  @State private loginUrl: string = '';
  @State private showAccountCenter: boolean = false;
  @State private accountSnapshot: AccountSnapshot | null = null;
  @State private showSignOutConfirm: boolean = false;
  @State private showRegionRestart: boolean = false;
  @State private switchedRegionName: string = '';

  aboutToAppear(): void {
    this.night = this.context.config.colorMode === ConfigurationConstant.ColorMode.COLOR_MODE_DARK;
    WindowBridge.shared.setListener((bands: ViewportBands) => {
      this.bands = bands;
      if (this.engineStarted) {
        this.host.applyViewport(bands);
      } else {
        this.startEngineIfReady();
      }
    });
    this.host.setSink(this.shellSink());
  }

  aboutToDisappear(): void {
    WindowBridge.shared.setListener(null);
    this.host.destroy();
    this.engineStarted = false;
  }

  onPageShow(): void {
    if (this.engineStarted) {
      this.host.resume();
      if (this.host.takeBackgroundGenerationPaused()) {
        this.presentNotice($r('app.string.background_generation_paused'));
      }
    }
  }

  onPageHide(): void {
    if (this.engineStarted) {
      this.host.suspend();
    }
  }

  build() {
    Stack() {
      XComponent({
        id: SURFACE_ID,
        type: XComponentType.SURFACE,
        libraryname: 'openpencil',
      })
        .width('100%')
        .height('100%')
        .focusable(true)
        .defaultFocus(true)
        .onLoad(() => {
          // The native surface exists. Everything after this point is
          // owned by the engine thread through OH_NativeXComponent.
          this.surfaceReady = true;
          if (this.engineStarted) {
            this.host.attach();
          } else {
            this.startEngineIfReady();
          }
        })
        .onDestroy(() => {
          // Blocking suspend BEFORE returning: the platform reclaims the
          // surface as soon as this callback completes.
          this.surfaceReady = false;
          this.host.suspend();
        })

      // SURFACE-type XComponents with registered native callbacks consume
      // raw input on the native channel, so ArkTS onTouch on the component
      // itself never fires. This transparent full-size overlay receives all
      // pointer/mouse/key input instead and forwards it to the engine host
      // (the native side deliberately registers no touch dispatch).
      Column()
        .width('100%')
        .height('100%')
        .onTouch((event: TouchEvent) => {
          this.trackImeAnchor(event);
          this.host.handleTouch(event);
        })
        .onMouse((event: MouseEvent) => {
          this.host.handleMouse(event);
        })
        .onHover((isHover: boolean) => {
          if (!isHover) {
            // Leaving the surface ends hover highlighting in the engine.
            this.host.router.cancelAll();
          }
        })
        .onAxisEvent((event: AxisEvent) => this.forwardAxis(event))
        // Trackpad pinch. Measured on the HarmonyOS 6 2in1 emulator: a
        // trackpad pinch DOES reach `onAxisEvent`, but with both scroll axes
        // at 0 — the magnitude only lives on the pinch axis, and
        // `AxisEvent.getPinchAxisScaleValue` is API 21+ while this module
        // compiles against API 12. The ArkUI gesture is therefore the
        // portable route, and `onAxisEvent` above stays for what it can
        // still see: wheel and two-finger scroll.
        //
        // Ignored while touch pointers are live: a two-finger TOUCH pinch is
        // already handled (with pan) by PointerRouter, and letting both run
        // would zoom twice per gesture.
        .gesture(
          PinchGesture({ fingers: 2 })
            .onActionStart((event: GestureEvent) => this.beginPinch(event))
            .onActionUpdate((event: GestureEvent) => this.updatePinch(event))
            .onActionEnd(() => this.endPinch())
            .onActionCancel(() => this.endPinch())
        )

      // Hidden IME proxy. A SURFACE XComponent is not an editable component,
      // so `inputMethod.attach` is refused on 2in1 (12800009) and no system
      // IME can ever compose for it. This 1×1 transparent TextInput IS an
      // editable component: it takes ArkUI focus whenever the ENGINE reports
      // text focus, the IME composes into it, and `ImeProxyBridge` relays
      // preedit + commits to the engine. It is anchored at the last press so
      // the IME's candidate window opens next to the caret the user sees.
      TextInput({ controller: this.imeProxyController })
        .id(IME_PROXY_ID)
        .width(1)
        .height(1)
        .padding(0)
        .borderWidth(0)
        .backgroundColor(Color.Transparent)
        .caretColor(Color.Transparent)
        .opacity(0)
        .position({ x: this.imeAnchorX, y: this.imeAnchorY })
        // A 2in1 has no soft keyboard to raise, and a phone's is driven by
        // the engine's own focus, not by this proxy gaining it.
        .enableKeyboardOnFocus(false)
        .onChange((value: string, preview?: PreviewText) => {
          this.imeProxy.onChange(this.host.engineHandle(), value, preview);
          this.host.requestFrame();
        })
        // Named keys and Ctrl chords, taken BEFORE the IME and the TextInput.
        // While the proxy holds ArkUI focus the XComponent's native key
        // callback is silent and the TextInput swallows Backspace outright,
        // so this is the only path those keys have into the engine.
        .onKeyPreIme((event: KeyEvent) => {
          const claimed: boolean = this.imeProxy.handlePreImeKey(
            this.host.engineHandle(),
            event
          );
          if (claimed) {
            this.host.pump();
            this.host.requestFrame();
          }
          return claimed;
        })

      // Long-press paste over a focused engine text input. The system
      // PasteButton security component grants the one-shot pasteboard read
      // without needing the READ_PASTEBOARD user-grant permission. New
      // touches on the scrim dismiss; the long-press finger's own Up stays
      // with the input overlay it started on and is release-suppressed.
      if (this.showPasteMenu) {
        Column()
          .width('100%')
          .height('100%')
          .onTouch((event: TouchEvent) => {
            if (event.type === TouchType.Down) {
              this.showPasteMenu = false;
            }
          })
        PasteButton()
          .position({ x: this.pasteMenuX, y: this.pasteMenuY })
          .onClick((event: ClickEvent, result: PasteButtonOnClickResult) => {
            this.showPasteMenu = false;
            if (result === PasteButtonOnClickResult.SUCCESS) {
              this.pasteFromSystemPasteboard();
            }
          })
      }

      LanguagePickerDialog({
        visible: $showLanguagePicker,
        currentCode: this.currentLocale,
        onSelect: (code: string): void => this.applyLocale(code),
      })

      NoticeDialog({
        visible: $showNotice,
        message: this.noticeMessage,
        onDismiss: (): void => {},
      })

      // PHONE / TABLET ONLY. A desktop-class (2in1) shell never builds this
      // page: `onOpenLoginUi` hands the engine's verification URL straight to
      // the system browser instead, and the engine's poll finishes the flow.
      if (this.showLogin) {
        NativeLoginPage({
          visible: $showLogin,
          night: this.night,
          origin: this.loginOrigin,
          pairingId: this.loginPairingId,
          verificationUrl: this.loginUrl,
          regions: this.auth.regionStore(),
          onCancel: (): void => this.host.cancelLogin(),
          onOpenBrowser: (url: string): void => this.openInBrowser(url),
          onRegionSwitched: (region: SsoRegion): void => this.presentRegionRestart(region),
        })
      }

      if (this.showAccountCenter) {
        AccountCenterSheet({
          visible: $showAccountCenter,
          night: this.night,
          // The PC gets a small centered card; touch chrome gets the sheet.
          compact: this.host.isDesktopClass(),
          snapshot: this.accountSnapshot,
          regions: this.auth.regionStore(),
          onManageInBrowser: (url: string): void => this.openInBrowser(url),
          onRequestSignOut: (): void => {
            this.showSignOutConfirm = true;
          },
          onRegionSwitched: (region: SsoRegion): void => this.presentRegionRestart(region),
        })
      }

      ConfirmDialog({
        visible: $showSignOutConfirm,
        title: '',
        message: $r('app.string.account_center_sign_out_confirm'),
        confirmLabel: $r('app.string.account_center_sign_out'),
        cancelLabel: $r('app.string.dialog_cancel'),
        onConfirm: (): void => this.signOut(),
      })

      // A region switch only takes effect on the next launch: the engine's
      // auth runtime locks its origin at the first `configureAuth`.
      ConfirmDialog({
        visible: $showRegionRestart,
        title: $r('app.string.sso_region_switched_title', this.switchedRegionName),
        message: $r('app.string.sso_region_restart_note'),
        confirmLabel: $r('app.string.sso_region_restart_now'),
        cancelLabel: $r('app.string.sso_region_later'),
        onConfirm: (): void => this.restartApp(),
      })
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#FFF5F5F7')
  }

  // ---- Engine lifecycle --------------------------------------------------

  private startEngineIfReady(): void {
    if (this.engineStarted || !this.surfaceReady) {
      return;
    }
    if (this.bands.width <= 0 || this.bands.height <= 0) {
      return;
    }
    const storageRoot: string = `${this.context.filesDir}/config`;
    try {
      fs.mkdirSync(storageRoot, true);
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'could not create the storage root');
    }
    // An empty document selects the engine's canonical starter document,
    // exactly like the Android editor build.
    const created: boolean = this.host.create(
      SURFACE_ID,
      new ArrayBuffer(0),
      this.bands,
      storageRoot,
      true
    );
    if (!created) {
      this.presentNotice($r('app.string.engine_start_failed'));
      return;
    }
    this.engineStarted = true;
    this.restorePersistedLocale();
    // LAZY CONFIGURE (mirrors `AndroidAuthRuntime.configure`): a returning
    // user is configured now so the session restores immediately, while a
    // fresh install defers until the first sign-in tap so the region can
    // still reflect the latest IP verdict and any user switch.
    this.auth.configureAtStartup(this.host).catch((error: Object) => {
      hilog.warn(DOMAIN, TAG, 'auth configure failed: %{public}s', JSON.stringify(error));
    });
  }

  private restorePersistedLocale(): void {
    EngineLanguage.storedPreference(this.context)
      .then((tag: string | null) => {
        if (tag !== null) {
          this.host.applyLocale(tag);
        }
      })
      .catch((error: Object) => {
        hilog.warn(DOMAIN, TAG, 'locale restore failed: %{public}s', JSON.stringify(error));
      });
  }

  private applyLocale(code: string): void {
    if (this.host.applyLocale(code) !== 0) {
      return;
    }
    this.currentLocale = code;
    EngineLanguage.savePreference(this.context, code);
  }

  // ---- Shell actions -----------------------------------------------------

  private shellSink(): ShellActionSink {
    return {
      onOpenDocument: (): void => this.openDocument(),
      onImportImageOrSvg: (): void => this.importImageOrSvg(),
      onExportDocument: (): void => this.exportDocument(),
      onSaveDocument: (): void => this.saveDocument(),
      onOpenLoginUi: (url: string): void => this.presentLoginUi(url),
      onRequestLogin: (): void => this.requestLogin(),
      onOpenAccountCenter: (): void => this.presentAccountCenter(),
      // Engine-terminal close: dismiss without cancelling the flow again.
      onCloseLoginUi: (): void => {
        this.showNotice = false;
        this.showLogin = false;
      },
      onOpenLanguagePicker: (): void => {
        this.currentLocale = this.host.localeCode() ?? '';
        this.showLanguagePicker = true;
      },
      onSystemChromeAppearance: (prefersLightIcons: boolean): void => {
        WindowBridge.shared.applySystemChromeAppearance(prefersLightIcons);
      },
      onEditorPasteMenu: (x: number, y: number): void => {
        // Anchor the bubble just above the finger, clamped into the view.
        this.pasteMenuX = Math.max(8, Math.min(x - 32, this.bands.width - 96));
        this.pasteMenuY = Math.max(8, y - 56);
        this.showPasteMenu = true;
      },
      onImeFocusChanged: (focused: boolean): void => {
        this.imeProxy.reset();
        if (!focused) {
          // Returning focus to the surface also ends any composition the IME
          // still had open against the proxy.
          this.imeProxyController.stopEditing();
          focusControl.requestFocus(SURFACE_ID);
          return;
        }
        const taken: boolean = focusControl.requestFocus(IME_PROXY_ID);
        hilog.info(DOMAIN, TAG, 'ime proxy focus requested, taken=%{public}s', `${taken}`);
      },
      // The engine paints the traffic lights but owns no window; the ability
      // does. Close terminates the ability rather than only hiding the
      // window, so the red dot means the same thing it does on desktop.
      onWindowControl: (kind: WindowControlKind): void => {
        hilog.info(DOMAIN, TAG, 'window control %{public}d', kind);
        switch (kind) {
          case WindowControlKind.MINIMIZE:
            WindowBridge.shared.minimize();
            break;
          case WindowControlKind.ZOOM:
            WindowBridge.shared.toggleMaximize();
            break;
          default:
            this.context.terminateSelf().catch((error: Object) => {
              hilog.warn(DOMAIN, TAG, 'terminate failed: %{public}s', JSON.stringify(error));
            });
            break;
        }
      },
    };
  }

  /** Reads the pasteboard after the PasteButton's one-shot grant. */
  private pasteFromSystemPasteboard(): void {
    pasteboard.getSystemPasteboard().getData()
      .then((data: pasteboard.PasteData) => {
        const text: string | undefined = data.getPrimaryText();
        if (text !== undefined && text.length > 0) {
          this.host.pasteText(text);
        }
      })
      .catch((error: Object) => {
        hilog.warn(DOMAIN, TAG, 'pasteboard read failed: %{public}s', JSON.stringify(error));
      });
  }

  private openDocument(): void {
    if (this.documentOpenInProgress) {
      return;
    }
    this.documentOpenInProgress = true;
    DocumentShell.openDocument(this.context, this.host)
      .then((outcome: ShellOutcome) => {
        this.documentOpenInProgress = false;
        if (outcome === ShellOutcome.UNSUPPORTED_TYPE) {
          this.presentNotice($r('app.string.document_type_unsupported'));
        } else if (outcome === ShellOutcome.FAILED) {
          this.presentNotice($r('app.string.document_open_failed'));
        }
      })
      .catch((error: Object) => {
        this.documentOpenInProgress = false;
        hilog.warn(DOMAIN, TAG, 'open document failed: %{public}s', JSON.stringify(error));
        this.presentNotice($r('app.string.document_open_failed'));
      });
  }

  private importImageOrSvg(): void {
    if (this.imageImportInProgress) {
      // The action is already consumed. Never stack a second picker or
      // resurrect it after the user dismisses the active one.
      return;
    }
    this.imageImportInProgress = true;
    DocumentShell.importImageOrSvg(this.context, this.host)
      .then((outcome: ShellOutcome) => {
        this.imageImportInProgress = false;
        if (outcome === ShellOutcome.UNSUPPORTED_TYPE) {
          this.presentNotice($r('app.string.image_import_type_unsupported'));
        } else if (outcome === ShellOutcome.FAILED) {
          this.presentNotice($r('app.string.image_import_failed'));
        }
        // CANCELLED is deliberately silent: action 12 is one-shot and has
        // no frozen engine payload that needs an explicit cancel call.
      })
      .catch((error: Object) => {
        this.imageImportInProgress = false;
        hilog.warn(DOMAIN, TAG, 'image import failed: %{public}s', JSON.stringify(error));
        this.presentNotice($r('app.string.image_import_failed'));
      });
  }

  private exportDocument(): void {
    if (this.exportInProgress) {
      // A second shell action must not race the save UI; discard the newly
      // frozen engine request instead.
      this.host.cancelExport();
      return;
    }
    this.exportInProgress = true;
    DocumentShell.exportDocument(this.context, this.host)
      .then((outcome: ShellOutcome) => {
        this.exportInProgress = false;
        if (outcome === ShellOutcome.FAILED) {
          this.presentNotice($r('app.string.document_export_failed'));
        }
      })
      .catch((error: Object) => {
        this.exportInProgress = false;
        hilog.warn(DOMAIN, TAG, 'export failed: %{public}s', JSON.stringify(error));
        this.presentNotice($r('app.string.document_export_failed'));
      });
  }

  private saveDocument(): void {
    if (this.saveInProgress) {
      // A second shell action must not race the picker; discard the newly
      // staged engine request instead of replacing the live one.
      DocumentSaveBridge.cancel(this.host, false);
      return;
    }
    this.saveInProgress = true;
    DocumentShell.saveDocument(this.context, this.host)
      .then((outcome: ShellOutcome) => {
        this.saveInProgress = false;
        if (outcome === ShellOutcome.FAILED) {
          this.presentNotice($r('app.string.document_save_failed'));
        }
      })
      .catch((error: Object) => {
        this.saveInProgress = false;
        hilog.warn(DOMAIN, TAG, 'save failed: %{public}s', JSON.stringify(error));
        // The promise died before any terminal report; make sure the engine
        // is not left believing a save is still in flight.
        DocumentSaveBridge.cancel(this.host, true);
        this.presentNotice($r('app.string.document_save_failed'));
      });
  }

  private presentNotice(message: ResourceStr): void {
    this.noticeMessage = message;
    this.showNotice = true;
  }

  // ---- Sign-in -----------------------------------------------------------

  /**
   * Shell action 6: configure the auth runtime for the resolved region if this
   * process has not yet, then start the device flow. A build without an
   * `op-auth` backend answers `NotReady` — say so and cancel, exactly like the
   * Android shell's `onUnavailable`.
   */
  private requestLogin(): void {
    if (this.loginRequestInFlight) {
      return;
    }
    this.loginRequestInFlight = true;
    this.auth
      .startLogin(this.host)
      .then((available: boolean) => {
        this.loginRequestInFlight = false;
        if (!available) {
          this.presentNotice($r('app.string.native_login_unavailable'));
          this.host.cancelLogin();
        }
      })
      .catch((error: Object) => {
        this.loginRequestInFlight = false;
        hilog.warn(DOMAIN, TAG, 'start login failed: %{public}s', JSON.stringify(error));
        this.presentNotice($r('app.string.native_login_load_failed'));
        this.host.cancelLogin();
      });
  }

  /**
   * Shell action 2: the engine produced a verification URL.
   *
   * DEVICE SPLIT. On a desktop-class (2in1) shell the URL goes to the SYSTEM
   * BROWSER — a PC always has one, it already holds the user's web session,
   * and the engine's background poll completes the flow and fires action 3.
   * Phones and tablets get the native login page instead.
   */
  private presentLoginUi(url: string): void {
    const request: DeviceLoginRequest | null = DeviceLoginRequest.parse(url);
    if (request === null) {
      hilog.warn(DOMAIN, TAG, 'rejected the login request (bad verification URL)');
      this.presentNotice($r('app.string.native_login_load_failed'));
      this.host.cancelLogin();
      return;
    }
    // The URL carries a pairing id; log the host only, never the full URL.
    hilog.info(DOMAIN, TAG, 'login url taken for origin %{public}s', request.origin);
    if (this.host.isDesktopClass()) {
      this.openBrowserForLogin(request);
      return;
    }
    this.loginOrigin = request.origin;
    this.loginPairingId = request.pairingId;
    this.loginUrl = request.verificationUrl;
    this.showLogin = true;
  }

  private openBrowserForLogin(request: DeviceLoginRequest): void {
    BrowserLauncher.open(this.context, request.verificationUrl)
      .then((opened: boolean) => {
        if (opened) {
          this.toast($r('app.string.native_login_browser_hint'));
          return;
        }
        this.presentNotice($r('app.string.native_login_load_failed'));
        this.host.cancelLogin();
      })
      .catch(() => {
        this.presentNotice($r('app.string.native_login_load_failed'));
        this.host.cancelLogin();
      });
  }

  /**
   * Shell action 5: the account center. The engine's snapshot decides — a
   * signed-out engine turns the request into a sign-in instead of showing an
   * empty profile.
   */
  private presentAccountCenter(): void {
    const snapshot: AccountSnapshot | null = AccountSnapshot.parse(this.host.accountSnapshot());
    if (snapshot === null || !snapshot.signedIn) {
      this.requestLogin();
      return;
    }
    this.accountSnapshot = snapshot;
    this.showAccountCenter = true;
  }

  private signOut(): void {
    this.showAccountCenter = false;
    this.accountSnapshot = null;
    this.host.signOut();
  }

  private openInBrowser(url: string): void {
    BrowserLauncher.open(this.context, url)
      .then((opened: boolean) => {
        if (!opened) {
          this.presentNotice($r('app.string.native_login_load_failed'));
        }
      })
      .catch(() => {});
  }

  private presentRegionRestart(region: SsoRegion): void {
    this.switchedRegionName = SsoRegionStore.displayName(this.context, region);
    this.showRegionRestart = true;
  }

  /**
   * `UIAbilityContext.restartApp` is API 22 while this module targets API 12,
   * so the shell terminates itself and the user relaunches — the region then
   * applies at the next `configureAuth`.
   */
  private restartApp(): void {
    this.context.terminateSelf().catch((error: Object) => {
      hilog.warn(DOMAIN, TAG, 'restart terminate failed: %{public}s', JSON.stringify(error));
    });
  }

  private toast(message: ResourceStr): void {
    try {
      promptAction.showToast({ message: message, duration: 3000 });
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'toast failed: %{public}s', JSON.stringify(error));
    }
  }


  /**
   * Keeps the hidden IME proxy under the pointer, so the system candidate
   * window opens beside the input the user just clicked rather than in the
   * window's corner. Offset upward by one line so it never covers the caret.
   */
  private trackImeAnchor(event: TouchEvent): void {
    if (event.type !== TouchType.Down || event.touches.length === 0) {
      return;
    }
    const touch: TouchObject = event.touches[0];
    this.imeAnchorX = touch.x;
    this.imeAnchorY = Math.max(0, touch.y - 20);
  }

  // ---- Trackpad pinch ----------------------------------------------------

  private pinchScale: number = 1;
  private pinchActive: boolean = false;

  private beginPinch(event: GestureEvent): void {
    // Fingers on glass already pinch through PointerRouter (which also
    // pans); only a pointer-less device — the trackpad — gets here.
    if (this.host.router.activePointers > 0) {
      this.pinchActive = false;
      return;
    }
    this.pinchActive = true;
    this.pinchScale = event.scale > 0 ? event.scale : 1;
  }

  private updatePinch(event: GestureEvent): void {
    if (!this.pinchActive || event.scale <= 0) {
      return;
    }
    const delta: number = PinchZoomDelta.wheelDelta(this.pinchScale, event.scale);
    this.pinchScale = event.scale;
    if (delta === 0) {
      return;
    }
    // `zoom = true` routes the delta into the engine's pinch-about-cursor
    // path, exactly like Ctrl+wheel.
    this.host.handleAxis(event.pinchCenterX, event.pinchCenterY, 0, delta, true);
  }

  private endPinch(): void {
    this.pinchActive = false;
    this.pinchScale = 1;
  }

  private forwardAxis(event: AxisEvent): void {
    const dy: number = event.getVerticalAxisValue();
    const dx: number = event.getHorizontalAxisValue();
    if (dx === 0 && dy === 0) {
      return;
    }
    const ctrl: boolean = (napi.keyModifiers() & 2) !== 0;
    this.host.handleAxis(event.x ?? 0, event.y ?? 0, -dx, -dy, ctrl);
  }

}
