import { hilog } from '@kit.PerformanceAnalysisKit';
import napi, { OpStatus } from './OpNative';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/** Focus id of the hidden proxy input; `focusControl` addresses it by this. */
export const IME_PROXY_ID = 'op_ime_proxy';

/**
 * Turns a hidden ArkUI `TextInput`'s edits into engine text calls.
 *
 * WHY A PROXY EXISTS AT ALL: `inputMethod.getController().attach()` — the
 * route the mobile shells use — is REFUSED on a 2in1 whose only content is a
 * SURFACE `XComponent` (observed: `attach failed {"code":12800009}`, the
 * client-detached error, on every focus transition on the HarmonyOS 6 PC
 * emulator). The input framework will only serve a real editable component,
 * so the shell gives it one: a 1×1 transparent `TextInput` that takes ArkUI
 * focus whenever the ENGINE reports text focus. The IME then composes into a
 * component it accepts, and this class relays the result to the engine.
 *
 * COMPOSITION: ArkUI's `onChange` hands back the committed body text and the
 * in-progress `previewText` separately, which maps one-to-one onto the
 * engine's preedit ABI — so pinyin paints under the engine's own caret
 * instead of only inside the IME candidate window.
 *
 * The proxy is a CONDUIT, never a source of truth: the engine owns the text.
 * This class holds only the last body-text value it was shown, purely to
 * diff successive callbacks.
 */
export class ImeProxyBridge {
  /** Body text as of the previous `onChange`; the diff baseline. */
  private lastValue: string = '';
  /** A composition is on screen, so the next body growth is its commit. */
  private previewActive: boolean = false;
  /**
   * The IME is mid-composition and owns every key until it reports back.
   *
   * It cannot be read from the component: `getPreviewText` exists only on
   * RichEditor's controller, and the HarmonyOS PC IME never fires
   * `setPreviewText` at all (measured — pinyin lives entirely in its own
   * candidate window until commit). So composition is INFERRED: a printable
   * key with no engine binding starts one, and any `onChange` — the IME's
   * only report — ends it.
   */
  private composing: boolean = false;
  private changeLogBudget: number = 12;
  private keyLogBudget: number = 12;

  /** Engine focus moved; forget the previous editor's text entirely. */
  reset(): void {
    this.lastValue = '';
    this.previewActive = false;
    this.composing = false;
  }

  /**
   * One `TextInput.onChange`. `value` is the committed body text WITHOUT the
   * composition; `preview` carries the composition when one is live.
   */
  onChange(engine: number, value: string, preview?: PreviewText): void {
    if (this.changeLogBudget > 0) {
      this.changeLogBudget--;
      hilog.info(
        DOMAIN,
        TAG,
        'ime proxy change %{public}s',
        `value="${value}" last="${this.lastValue}" preview="${preview?.value ?? ''}"` +
          ` offset=${preview?.offset ?? -1}`
      );
    }
    if (engine === 0) {
      this.lastValue = value;
      return;
    }
    // The IME reported: whatever it was composing has either landed or
    // moved on, so the engine owns the next Backspace again.
    this.composing = false;

    const previewText: string = preview?.value ?? '';
    if (previewText.length > 0) {
      // Still composing. The body text does not move while a composition is
      // open, so nothing but the preedit needs to reach the engine.
      this.previewActive = true;
      this.composing = true;
      const caret: number = ImeProxyBridge.utf8Length(previewText, previewText.length);
      napi.editorImePreedit(engine, previewText, caret, caret);
      this.lastValue = value;
      return;
    }

    const added: string = ImeProxyBridge.addedSuffix(this.lastValue, value);
    this.lastValue = value;

    if (this.previewActive) {
      // The composition closed. Whatever the body gained is what the user
      // picked from the candidate window; committing REPLACES the preedit,
      // where a plain insert would leave the pinyin run behind it.
      this.previewActive = false;
      napi.editorImeCommit(engine, added);
      return;
    }
    if (added.length > 0) {
      napi.editorText(engine, added);
    }
    // A SHRINKING body is deliberately ignored. Deletion reaches the engine
    // from `handleKey` below, on the key itself — which is the only source
    // that still fires once the proxy's own buffer runs empty. Acting on the
    // diff as well would delete twice per Backspace.
  }

  /**
   * A key pressed while the proxy holds focus, seen BEFORE the IME and the
   * TextInput get it. Returns true to claim it for the engine.
   *
   * This is the only usable hook, and the reason is measured: while the proxy
   * has ArkUI focus the XComponent's native key callback is silent, and the
   * TextInput swallows Backspace outright — `onKeyEvent` never sees it, so a
   * focused engine input could not delete a character. `onKeyPreIme` runs
   * ahead of both.
   *
   * WHO OWNS A KEY:
   * * While the IME is COMPOSING, it owns everything. Backspace corrects the
   *   pinyin, Enter and Space commit a candidate, arrows move the candidate
   *   selection — claiming any of those for the engine would break the
   *   composition the user can see in the candidate window.
   * * Otherwise the engine takes every key that `map_key` recognises
   *   (Backspace / Delete / Enter / Escape / arrows / the Ctrl chords) and
   *   leaves the rest — the printable ones — to the IME.
   *
   * The mapping stays single-sourced in the engine's `map_key`, reached
   * through `keyEvent`; `InvalidArg` is its "no editor binding" answer, and
   * therefore also this shell's "that key is printable" test.
   */
  handlePreImeKey(engine: number, event: KeyEvent): boolean {
    if (engine === 0 || event.type !== KeyType.Down) {
      return false;
    }
    if (ImeProxyBridge.isModifierKey(event.keyCode)) {
      // A modifier's OWN down event carries no character and starts no
      // composition. Treating it as printable (it has no engine binding, so
      // it looks exactly like one) would arm `composing` and swallow the
      // Backspace that follows — observed on the emulator as "Meta pressed,
      // then delete stops working".
      return false;
    }
    if (this.composing) {
      return false;
    }
    const modifiers: number = ImeProxyBridge.modifierMask(event);
    const status: number = napi.keyEvent(engine, event.keyCode, modifiers);
    const claimed: boolean = status !== OpStatus.INVALID_ARG;
    if (!claimed) {
      // A printable key the engine has no binding for. The IME is about to
      // start (or extend) a composition with it; every key until the IME
      // reports back through `onChange` belongs to that composition.
      this.composing = true;
    }
    if (this.keyLogBudget > 0) {
      this.keyLogBudget--;
      hilog.info(
        DOMAIN,
        TAG,
        'ime proxy key %{public}s',
        `code=${event.keyCode} mods=${modifiers} status=${status} claimed=${claimed}`
      );
    }
    return claimed && status === OpStatus.OK;
  }

  /**
   * Whether `keyCode` is a bare modifier / lock key rather than a character.
   *
   * These are frozen OS values from `oh_key_code.h` — the same transcription
   * the native key channel keeps in `op-engine-napi/src/xcomponent.rs`, which
   * tracks them as a bitmask instead of forwarding them.
   */
  private static isModifierKey(keyCode: number): boolean {
    switch (keyCode) {
      case 2045: // ALT_LEFT
      case 2046: // ALT_RIGHT
      case 2047: // SHIFT_LEFT
      case 2048: // SHIFT_RIGHT
      case 2072: // CTRL_LEFT
      case 2073: // CTRL_RIGHT
      case 2074: // CAPS_LOCK
      case 2076: // META_LEFT
      case 2077: // META_RIGHT
      case 2078: // FUNCTION
        return true;
      default:
        return false;
    }
  }

  /** This crate's modifier bitmask: 1 shift, 2 ctrl, 4 alt, 8 meta. */
  private static modifierMask(event: KeyEvent): number {
    let mask = 0;
    try {
      if (event.getModifierKeyState?.(['Shift']) ?? false) {
        mask |= 1;
      }
      if (event.getModifierKeyState?.(['Ctrl']) ?? false) {
        mask |= 2;
      }
      if (event.getModifierKeyState?.(['Alt']) ?? false) {
        mask |= 4;
      }
      if (event.getModifierKeyState?.(['Meta']) ?? false) {
        mask |= 8;
      }
    } catch (error) {
      // Older devices may not expose modifier state; plain keys still work.
    }
    return mask;
  }

  /** The text `next` gained over `previous`, or `''` when it gained none. */
  private static addedSuffix(previous: string, next: string): string {
    if (next.length <= previous.length) {
      return '';
    }
    // The IME appends at the caret, which for this proxy is always the end.
    return next.startsWith(previous) ? next.substring(previous.length) : next;
  }

  /**
   * UTF-8 byte length of `text`'s first `utf16Units` UTF-16 units — the unit
   * the engine's preedit ABI counts in. Surrogate pairs are one 4-byte
   * codepoint, so a half-consumed pair rounds down to the pair's start
   * instead of reporting an offset inside it.
   */
  static utf8Length(text: string, utf16Units: number): number {
    const limit: number = Math.max(0, Math.min(utf16Units, text.length));
    let bytes = 0;
    let i = 0;
    while (i < limit) {
      const code: number = text.charCodeAt(i);
      if (code >= 0xd800 && code <= 0xdbff && i + 1 < text.length) {
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
}
