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

  constructor(host: ImeConduitHost) {
    this.host = host;
  }

  get isAttached(): boolean {
    return this.attached;
  }

  /** Shows the keyboard and starts forwarding IME events to the engine. */
  async show(): Promise<void> {
    if (this.attached || this.busy) {
      return;
    }
    this.busy = true;
    try {
      this.subscribe();
      await this.controller.attach(true, {
        inputAttribute: {
          textInputType: inputMethod.TextInputType.TEXT,
          enterKeyType: inputMethod.EnterKeyType.NONE,
        },
      });
      this.attached = true;
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'IME attach failed: %{public}s', JSON.stringify(error));
    } finally {
      this.busy = false;
    }
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
      napi.editorText(engine, text);
      this.host.onImeInput();
    });

    // Backspace. NO buffer check: the conduit is always empty, so the only
    // correct behaviour is to forward the key straight through.
    this.controller.on('deleteLeft', (length: number) => {
      const engine: number = this.host.engineHandle();
      if (engine === 0) {
        return;
      }
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

  private static placeholder(length: number): string {
    const count: number = Math.max(0, Math.min(length, ImeConduit.PLACEHOLDER_LENGTH));
    return ' '.repeat(count);
  }

  /** A backspace burst is clamped so a runaway length cannot wipe a document. */
  private static readonly MAX_DELETE_RUN: number = 64;
  /** Reported surrounding-text length; long enough for word-level IMEs. */
  private static readonly PLACEHOLDER_LENGTH: number = 32;
}
