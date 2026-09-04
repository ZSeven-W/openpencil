import { pasteboard } from '@kit.BasicServicesKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import napi from './OpNative';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/**
 * The engine <-> system pasteboard bridge.
 *
 * OUTBOUND: engine copy buttons (collab invite / share address, MCP
 * config, chat copy) queue one string that the desktop runner drains into
 * the OS clipboard; `drainCopyText` lands the same queue on the system
 * pasteboard (writing needs no permission — only reading is gated).
 *
 * INBOUND: `pasteText` forwards clipboard text obtained through the
 * PasteButton security component's one-shot grant into whichever engine
 * input owns the keyboard.
 */
export class ClipboardBridge {
  /** Polled from the host pump; a `null` take is the common no-op case. */
  static drainCopyText(engine: number): void {
    if (engine === 0) {
      return;
    }
    const text: string | null = napi.editorTakeCopyText(engine);
    if (text === null || text.length === 0) {
      return;
    }
    const data: pasteboard.PasteData = pasteboard.createData(
      pasteboard.MIMETYPE_TEXT_PLAIN,
      text
    );
    pasteboard.getSystemPasteboard().setData(data).catch((error: Object) => {
      hilog.warn(DOMAIN, TAG, 'pasteboard write failed: %{public}s', JSON.stringify(error));
    });
  }

  /** Returns true when the engine accepted the paste (caller redraws). */
  static pasteText(engine: number, text: string): boolean {
    if (engine === 0 || text.length === 0) {
      return false;
    }
    napi.editorPasteText(engine, text);
    return true;
  }
}
