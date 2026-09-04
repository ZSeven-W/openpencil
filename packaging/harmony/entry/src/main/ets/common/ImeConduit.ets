import { inputMethod } from '@kit.IMEKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import napi, { OpKey } from './OpNative';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/** What the conduit needs from the engine host. */
export interface ImeConduitHost {
  /** The live engine handle, or 0 when there is none. */
  engineHandle(): number;
  /** A text mutation landed; drain shell actions and force a redraw. */
  onImeInput(): void;
}

/**
 * Bridges the system IME to the editor-mode engine.
 *
 * There is no visible text field: the engine owns the text and decides WHICH
 * input has IME focus (canvas text, property field, chat, rename…) through
 * its focus ladder. This class is only the conduit, exactly like Android's
 * `OpInputConnection` and iOS's offscreen `UITextView`. It attaches an
 * `InputMethodController` when the engine reports IME focus and detaches
 * when it drops.
 *
 * EMPTY-CONDUIT BACKSPACE CONTRACT (regression-pinned):
 * the conduit never holds any text, so a naive implementation that deletes
 * from a local buffer would swallow every backspace — the exact bug that was
 * fixed on iOS (`deleteBackward` override) and on Android
 * (`deleteSurroundingText` forwarding). `deleteLeft` here therefore ALWAYS
 * forwards `OpKey.BACKSPACE` to the engine, with no buffer check and no
 * early return. `getLeftTextOfCursor` additionally reports placeholder
 * surrounding text so IMEs that suppress backspace when they believe the
 * field is empty still emit the event.
 */
export class ImeConduit {
  private readonly host: ImeConduitHost;
  private controller: inputMethod.InputMethodController = inputMethod.getController();
  private attached: boolean = false;
  private subscribed: boolean = false;
  private busy: boolean = false;
  /**
   * Desktop-class hosts pass `false`: a 2in1 has no soft keyboard to raise,
   * and asking for one would either no-op or cover the canvas. The
   * subscription — which is what carries pinyin composition — is identical
   * either way.
   */
  private showKeyboard: boolean = true;
  /** A composition is on screen; the next `insertText` COMMITS it. */
  private previewActive: boolean = false;
  /**
   * The IME has actually delivered text at least once.
   *
   * PROVE-THEN-SUPPRESS: the native key channel is the only text path that is
   * known to work on 2in1 (a SURFACE XComponent eats hardware keys before the
   * input framework sees them), so it is NOT disabled merely because
   * `attach()` resolved — an `attach` that resolves against an IME which
   * never calls back would leave the editor unable to type at all. Native
   * injection is suppressed at the moment the IME proves it delivers, and not
   * before.
   */
  private imeProven: boolean = false;

  constructor(host: ImeConduitHost) {
    this.host = host;
  }

  get isAttached(): boolean {
    return this.attached;
  }

  /** Desktop-class hosts suppress the soft keyboard; see `showKeyboard`. */
  setShowKeyboard(show: boolean): void {
    this.showKeyboard = show;
  }

  /** Attaches the controller and starts forwarding IME events to the engine. */
  async show(): Promise<void> {
    if (this.attached || this.busy) {
      return;
    }
    this.busy = true;
    try {
      this.subscribe();
      await this.controller.attach(this.showKeyboard, {
        inputAttribute: {
          textInputType: inputMethod.TextInputType.TEXT,
          enterKeyType: inputMethod.EnterKeyType.NONE,
        },
      });
      this.attached = true;
      hilog.info(
        DOMAIN,
        TAG,
        'IME attached %{public}s',
        `showKeyboard=${this.showKeyboard} proven=${this.imeProven}`
      );
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'IME attach failed: %{public}s', JSON.stringify(error));
    } finally {
      this.busy = false;
    }
  }

  /**
   * First real delivery from the input framework: from here the IME owns
   * printable characters and the native channel must stop injecting them.
   */
  private markProven(source: string): void {
    if (this.imeProven) {
      return;
    }
    this.imeProven = true;
    napi.setImeConduitAttached(true);
    hilog.info(DOMAIN, TAG, 'IME proven by %{public}s; native text path off', source);
  }

  /** Hides the keyboard and detaches; subscriptions are torn down too. */
  async hide(): Promise<void> {
    if (!this.attached || this.busy) {
      return;
    }
    this.busy = true;
    try {
      await this.controller.detach();
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'IME detach failed: %{public}s', JSON.stringify(error));
    } finally {
      this.attached = false;
      this.previewActive = false;
      this.imeProven = false;
      napi.setImeConduitAttached(false);
      this.busy = false;
    }
  }

  /** Teardown for engine destruction; never throws. */
  release(): void {
    this.unsubscribe();
    if (this.attached) {
      this.controller.detach().catch((error: Object) => {
        hilog.warn(DOMAIN, TAG, 'IME release failed: %{public}s', JSON.stringify(error));
      });
      this.attached = false;
    }
    this.previewActive = false;
    this.imeProven = false;
    napi.setImeConduitAttached(false);
  }

  private subscribe(): void {
    if (this.subscribed) {
      return;
    }
    this.subscribed = true;

    // Committed text. HarmonyOS's InputMethodController does not surface
    // preedit in API 12 (the IME keeps its composition in its own candidate
    // window and delivers only the commit), so this is the plain-insertion
    // path — the same branch iOS takes when nothing is marked.
    this.controller.on('insertText', (text: string) => {
      const engine: number = this.host.engineHandle();
      if (engine === 0 || text.length === 0) {
        return;
      }
      this.markProven('insertText');
      if (this.previewActive) {
        // A composition is on screen: this text REPLACES it. Appending with
        // `editorText` instead would leave the pinyin run in the document
        // ahead of the characters the user actually chose.
        this.previewActive = false;
        napi.editorImeCommit(engine, text);
      } else {
        napi.editorText(engine, text);
      }
      this.host.onImeInput();
    });

    // Composition (preedit). The candidate window lives in the IME process;
    // these two events are how the in-progress pinyin run reaches the engine
    // so it can paint under the caret. API 17+, hence the guarded subscribe:
    // an older device simply never composes and keeps the commit-only path.
    try {
      this.controller.on('setPreviewText', (text: string, range: inputMethod.Range) => {
        const engine: number = this.host.engineHandle();
        if (engine === 0) {
          return;
        }
        hilog.info(
          DOMAIN,
          TAG,
          'IME setPreviewText %{public}s',
          `len=${text.length} start=${range.start} end=${range.end}`
        );
        this.markProven('setPreviewText');
        this.previewActive = text.length > 0;
        // The engine's preedit ABI takes BYTE offsets into the preedit text;
        // `range` counts UTF-16 units, so a CJK run would otherwise report a
        // caret in the middle of a codepoint.
        const caret: number = ImeConduit.utf8Length(text, range.end);
        napi.editorImePreedit(engine, text, caret, caret);
        this.host.onImeInput();
      });
      this.controller.on('finishTextPreview', () => {
        const engine: number = this.host.engineHandle();
        if (engine === 0) {
          return;
        }
        hilog.info(DOMAIN, TAG, 'IME finishTextPreview');
        // Only clears an ABANDONED composition. A committed one already
        // cleared itself through `insertText` above, and an empty preedit is
        // how the engine is told to drop the composition either way.
        if (this.previewActive) {
          this.previewActive = false;
          napi.editorImePreedit(engine, '', 0, 0);
          this.host.onImeInput();
        }
      });
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'IME preview events unavailable: %{public}s', JSON.stringify(error));
    }

    this.controller.on('sendKeyboardStatus', (status: inputMethod.KeyboardStatus) => {
      hilog.info(DOMAIN, TAG, 'IME keyboard status=%{public}d', status);
    });

    // Backspace. NO buffer check: the conduit is always empty, so the only
    // correct behaviour is to forward the key straight through.
    this.controller.on('deleteLeft', (length: number) => {
      const engine: number = this.host.engineHandle();
      if (engine === 0) {
        return;
      }
      this.markProven('deleteLeft');
      const count: number = Math.max(1, Math.min(length, ImeConduit.MAX_DELETE_RUN));
      for (let i = 0; i < count; i++) {
        napi.editorKey(engine, OpKey.BACKSPACE);
      }
      this.host.onImeInput();
    });

    this.controller.on('deleteRight', (length: number) => {
      const engine: number = this.host.engineHandle();
      if (engine === 0) {
        return;
      }
      const count: number = Math.max(1, Math.min(length, ImeConduit.MAX_DELETE_RUN));
      for (let i = 0; i < count; i++) {
        napi.editorKey(engine, OpKey.DELETE);
      }
      this.host.onImeInput();
    });

    this.controller.on('sendFunctionKey', (_key: inputMethod.FunctionKey) => {
      const engine: number = this.host.engineHandle();
      if (engine === 0) {
        return;
      }
      napi.editorKey(engine, OpKey.ENTER);
      this.host.onImeInput();
    });

    this.controller.on('moveCursor', (direction: inputMethod.Direction) => {
      const engine: number = this.host.engineHandle();
      if (engine === 0) {
        return;
      }
      napi.editorKey(engine, ImeConduit.arrowKeyFor(direction));
      this.host.onImeInput();
    });

    // Surrounding-text probes. The engine owns the real text; reporting a
    // placeholder run keeps IMEs from concluding the field is empty and
    // suppressing `deleteLeft`.
    this.controller.on('getLeftTextOfCursor', (length: number) => {
      return ImeConduit.placeholder(length);
    });
    this.controller.on('getRightTextOfCursor', (_length: number) => {
      return '';
    });
    this.controller.on('getTextIndexAtCursor', () => {
      return ImeConduit.PLACEHOLDER_LENGTH;
    });
  }

  private unsubscribe(): void {
    if (!this.subscribed) {
      return;
    }
    this.subscribed = false;
    try {
      this.controller.off('setPreviewText');
      this.controller.off('finishTextPreview');
      this.controller.off('sendKeyboardStatus');
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'IME preview unsubscribe failed: %{public}s', JSON.stringify(error));
    }
    try {
      this.controller.off('insertText');
      this.controller.off('deleteLeft');
      this.controller.off('deleteRight');
      this.controller.off('sendFunctionKey');
      this.controller.off('moveCursor');
      this.controller.off('getLeftTextOfCursor');
      this.controller.off('getRightTextOfCursor');
      this.controller.off('getTextIndexAtCursor');
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'IME unsubscribe failed: %{public}s', JSON.stringify(error));
    }
  }

  private static arrowKeyFor(direction: inputMethod.Direction): OpKey {
    switch (direction) {
      case inputMethod.Direction.CURSOR_UP:
        return OpKey.ARROW_UP;
      case inputMethod.Direction.CURSOR_DOWN:
        return OpKey.ARROW_DOWN;
      case inputMethod.Direction.CURSOR_LEFT:
        return OpKey.ARROW_LEFT;
      default:
        return OpKey.ARROW_RIGHT;
    }
  }

  /**
   * UTF-8 byte length of `text`'s first `utf16Units` UTF-16 units — the
   * conversion the engine's preedit ABI needs. Surrogate pairs count as one
   * 4-byte codepoint, so a half-consumed pair is rounded down to the pair's
   * start rather than reported as a byte offset inside it.
   */
  private static utf8Length(text: string, utf16Units: number): number {
    const limit: number = Math.max(0, Math.min(utf16Units, text.length));
    let bytes = 0;
    let i = 0;
    while (i < limit) {
      const code: number = text.charCodeAt(i);
      if (code >= 0xd800 && code <= 0xdbff && i + 1 < text.length) {
        // High surrogate: only counted once its low half is inside the range.
        if (i + 1 >= limit) {
          break;
        }
        bytes += 4;
        i += 2;
        continue;
      }
      if (code < 0x80) {
        bytes += 1;
      } else if (code < 0x800) {
        bytes += 2;
      } else {
        bytes += 3;
      }
      i += 1;
    }
    return bytes;
  }

  private static placeholder(length: number): string {
    const count: number = Math.max(0, Math.min(length, ImeConduit.PLACEHOLDER_LENGTH));
    return ' '.repeat(count);
  }

  /** A backspace burst is clamped so a runaway length cannot wipe a document. */
  private static readonly MAX_DELETE_RUN: number = 64;
  /** Reported surrounding-text length; long enough for word-level IMEs. */
  private static readonly PLACEHOLDER_LENGTH: number = 32;
}
