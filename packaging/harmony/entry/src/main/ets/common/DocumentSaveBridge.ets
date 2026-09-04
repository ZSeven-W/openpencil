import napi, { OpStatus } from './OpNative';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { EngineHost } from './EngineHost';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/**
 * The engine half of the picker-backed Save / Save As round trip.
 *
 * These are plain passthroughs and would sit on `EngineHost` next to the
 * export ones, except that file is at the repo's 800-line cap. `EngineHost`
 * still owns the capability declaration (`editorConfigureSavePicker`) and the
 * shell-action dispatch; everything the save flow itself calls lives here, so
 * `DocumentShell.saveDocument` reads as one coordinator.
 */
export class DocumentSaveBridge {
  /** Suggested `<stem>.op` the picker opens pre-filled with. */
  static fileName(host: EngineHost): string | null {
    if (!DocumentSaveBridge.usable(host)) {
      return null;
    }
    return napi.editorSaveFileName(host.engineHandle());
  }

  /**
   * The durable destination URI a plain Save should rewrite, or `null` when
   * the shell must present the picker.
   */
  static target(host: EngineHost): string | null {
    if (!DocumentSaveBridge.usable(host)) {
      return null;
    }
    return napi.editorSaveTarget(host.engineHandle());
  }

  /**
   * Writes the live document's canonical `.op` bytes into a NEW app-private
   * staging file. Does NOT mark the document saved.
   */
  static stageToPath(host: EngineHost, path: string): number {
    if (!DocumentSaveBridge.usable(host)) {
      return OpStatus.CLOSING;
    }
    return napi.editorStageSaveToPath(host.engineHandle(), path);
  }

  /** The staged bytes reached the picked file: bind it and mark it saved. */
  static commit(host: EngineHost, handle: string, displayName: string): number {
    if (!DocumentSaveBridge.usable(host)) {
      return OpStatus.CLOSING;
    }
    const status: number = napi.editorCommitSave(host.engineHandle(), handle, displayName);
    if (status === OpStatus.OK) {
      // Repaint with the new file name and the cleared dirty marker.
      host.pump();
    }
    return status;
  }

  /**
   * The picker was dismissed (`failed = false`) or the copy into the picked
   * file blew up (`true`). The document keeps its changes and its previous
   * binding either way.
   */
  static cancel(host: EngineHost, failed: boolean): void {
    if (!DocumentSaveBridge.usable(host)) {
      return;
    }
    const status: number = napi.editorCancelSave(host.engineHandle(), failed);
    if (status !== OpStatus.OK && status !== OpStatus.CLOSING) {
      hilog.info(DOMAIN, TAG, 'save cancel returned status=%{public}d', status);
    }
  }

  private static usable(host: EngineHost): boolean {
    return host.engineHandle() !== 0 && host.isEditorMode();
  }
}
