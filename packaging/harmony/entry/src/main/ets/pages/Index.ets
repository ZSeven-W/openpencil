import { common } from '@kit.AbilityKit';
import { fileIo as fs } from '@kit.CoreFileKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { DocumentShell, ShellOutcome } from '../common/DocumentShell';
import { EngineHost, ShellActionSink } from '../common/EngineHost';
import { EngineLanguage } from '../common/EngineLanguage';
import { KeyModifiers } from '../common/KeyForwarder';
import { LanguagePickerDialog } from '../common/LanguagePickerDialog';
import { NoticeDialog } from '../common/NoticeDialog';
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
  private exportInProgress: boolean = false;

  @State private showLanguagePicker: boolean = false;
  @State private currentLocale: string = '';
  @State private showNotice: boolean = false;
  @State private noticeMessage: ResourceStr = '';

  aboutToAppear(): void {
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
        .onTouch((event: TouchEvent) => {
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
        .onKeyEvent((event: KeyEvent) => {
          if (event.type !== KeyType.Down) {
            return;
          }
          this.host.handleKey(event.keyCode, event.keyText, Index.modifiersOf(event));
        })

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
      onExportDocument: (): void => this.exportDocument(),
      // Sign-in, the account center, and the device-login flow all need the
      // private `op-auth` runtime, which has no HarmonyOS build yet. Tell
      // the user, then cancel the engine-side flow through the SAME call the
      // Android shell uses when it rejects a login request.
      onOpenLoginUi: (_url: string): void => {
        this.presentNotice($r('app.string.native_login_unavailable'));
        this.host.cancelLogin();
      },
      onRequestLogin: (): void => {
        this.presentNotice($r('app.string.native_login_unavailable'));
        this.host.cancelLogin();
      },
      onOpenAccountCenter: (): void => {
        this.presentNotice($r('app.string.native_login_unavailable'));
      },
      onCloseLoginUi: (): void => {
        this.showNotice = false;
      },
      onOpenLanguagePicker: (): void => {
        this.currentLocale = this.host.localeCode() ?? '';
        this.showLanguagePicker = true;
      },
      onSystemChromeAppearance: (prefersLightIcons: boolean): void => {
        WindowBridge.shared.applySystemChromeAppearance(prefersLightIcons);
      },
    };
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

  private presentNotice(message: ResourceStr): void {
    this.noticeMessage = message;
    this.showNotice = true;
  }

  private static modifiersOf(event: KeyEvent): KeyModifiers {
    let ctrl = false;
    let shift = false;
    let alt = false;
    try {
      ctrl = event.getModifierKeyState?.(['Ctrl']) ?? false;
      shift = event.getModifierKeyState?.(['Shift']) ?? false;
      alt = event.getModifierKeyState?.(['Alt']) ?? false;
    } catch (error) {
      // Older devices may not expose modifier state; plain keys still work.
    }
    return new KeyModifiers(ctrl, shift, alt);
  }
}
