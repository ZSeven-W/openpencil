import { KeyCode } from '@kit.InputKit';
import napi, { OpKey } from './OpNative';

/** Modifier snapshot for one physical key event. */
export class KeyModifiers {
  readonly ctrl: boolean;
  readonly shift: boolean;
  readonly alt: boolean;

  constructor(ctrl: boolean, shift: boolean, alt: boolean) {
    this.ctrl = ctrl;
    this.shift = shift;
    this.alt = alt;
  }
}

/**
 * Physical-keyboard forwarding for tablets and 2in1 (PC) devices.
 *
 * The engine owns every editing decision; this only translates HarmonyOS key
 * codes into the editor ABI's `OpKey` values, exactly like Android's
 * `OpInputConnection.sendKeyEvent`. Printable characters are forwarded ONLY
 * when the IME conduit is detached — while it is attached the same keystroke
 * already arrives as an `insertText` event and forwarding both would type
 * every character twice.
 */
export class KeyForwarder {
  /** Returns true when the event was consumed by the engine. */
  static handleKeyDown(
    engine: number,
    keyCode: number,
    keyText: string,
    modifiers: KeyModifiers,
    imeAttached: boolean
  ): boolean {
    if (engine === 0) {
      return false;
    }

    // Desktop shortcuts first: the engine exposes them as dedicated keys.
    if (modifiers.ctrl) {
      switch (keyCode) {
        case KeyCode.KEYCODE_Z:
          napi.editorKey(engine, modifiers.shift ? OpKey.REDO : OpKey.UNDO);
          return true;
        case KeyCode.KEYCODE_Y:
          napi.editorKey(engine, OpKey.REDO);
          return true;
        case KeyCode.KEYCODE_D:
          napi.editorKey(engine, OpKey.DUPLICATE);
          return true;
        default:
          // Every other Ctrl chord belongs to the engine's own handling or
          // to the system; never inject it as literal text.
          return false;
      }
    }

    const mapped: OpKey | null = KeyForwarder.mapKey(keyCode);
    if (mapped !== null) {
      napi.editorKey(engine, mapped);
      return true;
    }

    if (imeAttached) {
      return false;
    }
    const text: string = KeyForwarder.printableText(keyText);
    if (text.length === 0) {
      return false;
    }
    napi.editorText(engine, text);
    return true;
  }

  /** Named editor keys shared with the Android/iOS shells. */
  static mapKey(keyCode: number): OpKey | null {
    switch (keyCode) {
      case KeyCode.KEYCODE_DEL:
        return OpKey.BACKSPACE;
      case KeyCode.KEYCODE_FORWARD_DEL:
        return OpKey.DELETE;
      case KeyCode.KEYCODE_ENTER:
      case KeyCode.KEYCODE_NUMPAD_ENTER:
        return OpKey.ENTER;
      case KeyCode.KEYCODE_ESCAPE:
        return OpKey.ESCAPE;
      case KeyCode.KEYCODE_DPAD_UP:
        return OpKey.ARROW_UP;
      case KeyCode.KEYCODE_DPAD_DOWN:
        return OpKey.ARROW_DOWN;
      case KeyCode.KEYCODE_DPAD_LEFT:
        return OpKey.ARROW_LEFT;
      case KeyCode.KEYCODE_DPAD_RIGHT:
        return OpKey.ARROW_RIGHT;
      default:
        return null;
    }
  }

  /** Filters control/dead keys out of ArkUI's `keyText`. */
  static printableText(keyText: string): string {
    if (keyText.length === 0 || keyText.length > 2) {
      return '';
    }
    const code: number = keyText.charCodeAt(0);
    if (code < 0x20 || code === 0x7f) {
      return '';
    }
    return keyText;
  }
}
