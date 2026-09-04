import { common } from '@kit.AbilityKit';
import { fileIo as fs, picker } from '@kit.CoreFileKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { DocumentExportSupport } from './DocumentExportSupport';
import { DocumentSaveBridge } from './DocumentSaveBridge';
import { EngineHost } from './EngineHost';
import { OpStatus } from './OpNative';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/**
 * NAPI, the UTF-8 ABI copy, and document decoding temporarily coexist. Keep
 * the picker limit below the engine's generic 256 MiB cap to protect mobile
 * memory — the same 32 MiB ceiling the Android and iOS shells enforce.
 */
const MAX_DOCUMENT_BYTES = 32 * 1024 * 1024;

/** Terminal state of one picker-backed shell action. */
export enum ShellOutcome {
  OK = 0,
  CANCELLED = 1,
  UNSUPPORTED_TYPE = 2,
  FAILED = 3,
}

/**
 * Bridges the engine's Open/Export shell actions to the HarmonyOS file
 * pickers. Mirrors `MainActivity.launchDocumentPicker` /
 * `beginDocumentExport` on Android: the engine validates the document, the
 * shell only moves bytes.
 */
export class DocumentShell {
  /** Shell action 1: pick an `.op` / `.pen` file and hand it to the engine. */
  static async openDocument(context: common.UIAbilityContext, host: EngineHost): Promise<ShellOutcome> {
    let uri: string = '';
    try {
      const viewPicker = new picker.DocumentViewPicker(context);
      const options = new picker.DocumentSelectOptions();
      options.maxSelectNumber = 1;
      // `.op` and `.pen` have no system-wide MIME registration; the suffix
      // filter is a hint, so the display name is validated again below.
      options.fileSuffixFilters = ['.op', '.pen'];
      const result: string[] = await viewPicker.select(options);
      if (result.length === 0) {
        return ShellOutcome.CANCELLED;
      }
      uri = result[0];
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'document picker failed: %{public}s', JSON.stringify(error));
      return ShellOutcome.CANCELLED;
    }

    const displayName: string = DocumentShell.displayNameOf(uri);
    if (!DocumentShell.hasSupportedExtension(displayName)) {
      return ShellOutcome.UNSUPPORTED_TYPE;
    }

    let bytes: ArrayBuffer;
    try {
      bytes = DocumentShell.readBoundedFile(uri, 'document');
    } catch (error) {
      hilog.warn(
        DOMAIN,
        TAG,
        "could not read document '%{public}s': %{public}s",
        displayName,
        JSON.stringify(error)
      );
      return ShellOutcome.FAILED;
    }

    const status: number = host.openDocument(bytes, displayName);
    if (status === OpStatus.OK) {
      hilog.info(DOMAIN, TAG, "opened document '%{public}s'", displayName);
      return ShellOutcome.OK;
    }
    if (status === OpStatus.CLOSING) {
      hilog.warn(DOMAIN, TAG, 'document open ignored because the engine is closing');
      return ShellOutcome.CANCELLED;
    }
    hilog.warn(
      DOMAIN,
      TAG,
      "could not open document '%{public}s', status=%{public}d: %{public}s",
      displayName,
      status,
      host.lastError()
    );
    return ShellOutcome.FAILED;
  }

  /**
   * Shell action 12: pick exactly one raster image or editable SVG and
   * return its bounded bytes plus display name to Rust. An empty picker
   * result is terminal and silent: the action was already consumed by the
   * shell-action drain, and image import has no pending engine artifact to
   * cancel. The SDK documents no exception code for dismissal, so rejected
   * picker promises remain visible failures rather than being swallowed.
   */
  static async importImageOrSvg(
    context: common.UIAbilityContext,
    host: EngineHost
  ): Promise<ShellOutcome> {
    let uri: string = '';
    try {
      const viewPicker = new picker.DocumentViewPicker(context);
      const options = new picker.DocumentSelectOptions();
      options.maxSelectNumber = 1;
      options.fileSuffixFilters = ['.png', '.jpg', '.jpeg', '.gif', '.webp', '.svg'];
      const result: string[] = await viewPicker.select(options);
      if (result.length === 0) {
        return ShellOutcome.CANCELLED;
      }
      uri = result[0];
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'image picker failed: %{public}s', JSON.stringify(error));
      return ShellOutcome.FAILED;
    }

    const displayName: string = DocumentShell.displayNameOf(uri);
    if (!DocumentShell.hasSupportedImageExtension(displayName)) {
      return ShellOutcome.UNSUPPORTED_TYPE;
    }

    let bytes: ArrayBuffer;
    try {
      bytes = DocumentShell.readBoundedFile(uri, 'image');
    } catch (error) {
      hilog.warn(
        DOMAIN,
        TAG,
        "could not read image '%{public}s': %{public}s",
        displayName,
        JSON.stringify(error)
      );
      return ShellOutcome.FAILED;
    }

    const status: number = host.importImageOrSvg(bytes, displayName);
    if (status === OpStatus.OK || status === OpStatus.BUSY) {
      // Busy means Rust painted the collaboration race explanation itself.
      host.requestFrame();
    }
    if (status === OpStatus.OK) {
      hilog.info(DOMAIN, TAG, "imported image '%{public}s'", displayName);
      return ShellOutcome.OK;
    }
    if (status === OpStatus.CLOSING || status === OpStatus.BUSY) {
      // Closing needs no stale page alert. Busy is the collaboration race
      // gate, whose precise explanation is already painted by Rust.
      return ShellOutcome.CANCELLED;
    }
    hilog.warn(
      DOMAIN,
      TAG,
      "could not import image '%{public}s', status=%{public}d: %{public}s",
      displayName,
      status,
      host.lastError()
    );
    return ShellOutcome.FAILED;
  }

  /**
   * Shell action 4: name the frozen export, let the user place it, then
   * stage it app-privately and copy it into the chosen file. Every terminal
   * path either consumes or explicitly cancels the frozen artifact so the
   * next export is not refused as busy.
   */
  static async exportDocument(
    context: common.UIAbilityContext,
    host: EngineHost
  ): Promise<ShellOutcome> {
    const filename: string | null = DocumentExportSupport.validatedFilename(host.exportFileName());
    if (filename === null) {
      host.cancelExport();
      return ShellOutcome.FAILED;
    }

    let destination: string = '';
    try {
      const viewPicker = new picker.DocumentViewPicker(context);
      const options = new picker.DocumentSaveOptions();
      options.newFileNames = [filename];
      // The engine-derived suffix covers rendered media and Code-panel
      // source/bundle artifacts. WebP stays hidden because the pinned mobile
      // Skia archive has no WebP encoder.
      options.fileSuffixChoices = DocumentExportSupport.saveSuffixesFor(filename);
      const result: string[] = await viewPicker.save(options);
      if (result.length === 0) {
        host.cancelExport();
        return ShellOutcome.CANCELLED;
      }
      destination = result[0];
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'save picker failed: %{public}s', JSON.stringify(error));
      host.cancelExport();
      return ShellOutcome.CANCELLED;
    }

    const stagingDir: string = `${context.cacheDir}/exports/${Date.now()}`;
    const stagedPath: string = `${stagingDir}/${filename}`;
    try {
      fs.mkdirSync(stagingDir, true);
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'could not create export staging directory');
      host.cancelExport();
      return ShellOutcome.FAILED;
    }

    // Rust writes the payload into the app-private staging file so large
    // PNG/PDF exports never cross NAPI as a byte array.
    const status: number = host.exportToPath(stagedPath);
    if (status !== OpStatus.OK) {
      hilog.warn(
        DOMAIN,
        TAG,
        "could not stage export '%{public}s', status=%{public}d: %{public}s",
        filename,
        status,
        host.lastError()
      );
      // The failed write leaves the artifact frozen; discard it.
      host.cancelExport();
      DocumentShell.removeStaging(stagingDir);
      return ShellOutcome.FAILED;
    }

    try {
      const dest = fs.openSync(destination, fs.OpenMode.READ_WRITE | fs.OpenMode.TRUNC);
      try {
        fs.copyFileSync(stagedPath, dest.fd);
      } finally {
        fs.closeSync(dest);
      }
      return ShellOutcome.OK;
    } catch (error) {
      hilog.warn(
        DOMAIN,
        TAG,
        "could not save export '%{public}s': %{public}s",
        filename,
        JSON.stringify(error)
      );
      return ShellOutcome.FAILED;
    } finally {
      DocumentShell.removeStaging(stagingDir);
    }
  }

  /**
   * Shell action 11: place the document where the user's file manager can
   * reach it.
   *
   * HarmonyOS public directories are not path-writable, so the engine cannot
   * save anywhere visible on its own — and `.op` is a document format, not an
   * export format, so there is no other way out either. The engine stages the
   * canonical bytes into an app-private file and this method copies them into
   * the picked destination.
   *
   * A plain Save on an already-placed document skips the picker entirely: the
   * engine hands back the bound file URI and the copy goes straight there. If
   * that URI can no longer be written (the file was deleted, or the grant did
   * not survive), the round trip falls back to asking for a new destination
   * rather than reporting a failed save.
   *
   * Every terminal path either commits or cancels the engine's pending save
   * and removes the staging directory.
   */
  static async saveDocument(
    context: common.UIAbilityContext,
    host: EngineHost
  ): Promise<ShellOutcome> {
    const filename: string | null =
      DocumentExportSupport.validatedFilename(DocumentSaveBridge.fileName(host));
    if (filename === null) {
      hilog.warn(DOMAIN, TAG, 'engine offered no usable save file name');
      DocumentSaveBridge.cancel(host, true);
      return ShellOutcome.FAILED;
    }

    const stagingDir: string = `${context.cacheDir}/saves/${Date.now()}`;
    const stagedPath: string = `${stagingDir}/${filename}`;
    try {
      fs.mkdirSync(stagingDir, true);
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'could not create save staging directory');
      DocumentSaveBridge.cancel(host, true);
      return ShellOutcome.FAILED;
    }

    // Rust writes the canonical `.op` bytes so a multi-megabyte document
    // never crosses NAPI as a byte array.
    const status: number = DocumentSaveBridge.stageToPath(host, stagedPath);
    if (status !== OpStatus.OK) {
      hilog.warn(
        DOMAIN,
        TAG,
        "could not stage save '%{public}s', status=%{public}d: %{public}s",
        filename,
        status,
        host.lastError()
      );
      DocumentSaveBridge.cancel(host, true);
      DocumentShell.removeStaging(stagingDir);
      return ShellOutcome.FAILED;
    }

    try {
      const bound: string | null = DocumentSaveBridge.target(host);
      if (bound !== null && bound.length > 0 && DocumentShell.copyInto(stagedPath, bound)) {
        DocumentSaveBridge.commit(host, bound, filename);
        return ShellOutcome.OK;
      }
      // No binding yet, or the bound destination refused the write: ask the
      // user where this document should live.
      const destination: string | null = await DocumentShell.pickSaveDestination(context, filename);
      if (destination === null) {
        DocumentSaveBridge.cancel(host, false);
        return ShellOutcome.CANCELLED;
      }
      if (!DocumentShell.copyInto(stagedPath, destination)) {
        DocumentSaveBridge.cancel(host, true);
        return ShellOutcome.FAILED;
      }
      DocumentSaveBridge.commit(host, destination, DocumentShell.displayNameOf(destination));
      return ShellOutcome.OK;
    } finally {
      DocumentShell.removeStaging(stagingDir);
    }
  }

  /** `DocumentViewPicker.save`, returning the picked URI or null. */
  private static async pickSaveDestination(
    context: common.UIAbilityContext,
    filename: string
  ): Promise<string | null> {
    try {
      const viewPicker = new picker.DocumentViewPicker(context);
      const options = new picker.DocumentSaveOptions();
      options.newFileNames = [filename];
      options.fileSuffixChoices = ['.op'];
      const result: string[] = await viewPicker.save(options);
      return result.length === 0 ? null : result[0];
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'save picker failed: %{public}s', JSON.stringify(error));
      return null;
    }
  }

  /** Truncating copy of the staged file into a picked URI. */
  private static copyInto(stagedPath: string, destination: string): boolean {
    try {
      const dest = fs.openSync(destination, fs.OpenMode.READ_WRITE | fs.OpenMode.TRUNC);
      try {
        fs.copyFileSync(stagedPath, dest.fd);
      } finally {
        fs.closeSync(dest);
      }
      return true;
    } catch (error) {
      hilog.warn(
        DOMAIN,
        TAG,
        'could not write the save destination: %{public}s',
        JSON.stringify(error)
      );
      return false;
    }
  }

  private static readBoundedFile(uri: string, kind: string): ArrayBuffer {
    const file = fs.openSync(uri, fs.OpenMode.READ_ONLY);
    try {
      const size: number = fs.statSync(file.fd).size;
      if (size <= 0) {
        throw new Error(`${kind} is empty`);
      }
      if (size > MAX_DOCUMENT_BYTES) {
        throw new Error(`${kind} exceeds the 32 MiB mobile input limit`);
      }
      const buffer = new ArrayBuffer(size);
      // `ReadOptions.offset` is the FILE offset, and every read fills the
      // destination from index 0 — so a partial-read loop would overwrite
      // itself. Regular files satisfy one full read; anything else is a
      // truncated/racing document and must be rejected rather than opened.
      const read: number = fs.readSync(file.fd, buffer, { offset: 0, length: size });
      if (read !== size) {
        throw new Error(`${kind} changed while it was being read`);
      }
      return buffer;
    } finally {
      fs.closeSync(file);
    }
  }

  private static removeStaging(directory: string): void {
    try {
      fs.rmdirSync(directory);
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'could not remove export staging directory');
    }
  }

  private static displayNameOf(uri: string): string {
    const slash: number = uri.lastIndexOf('/');
    const raw: string = slash >= 0 ? uri.substring(slash + 1) : uri;
    let name: string = raw;
    try {
      name = decodeURIComponent(raw);
    } catch (error) {
      name = raw;
    }
    const trimmed: string = name.trim();
    return trimmed.length > 0 ? trimmed : 'document.op';
  }

  private static hasSupportedExtension(name: string): boolean {
    const dot: number = name.lastIndexOf('.');
    if (dot < 0 || dot === name.length - 1) {
      // No extension at all: let the engine decide, like Android does.
      return true;
    }
    const extension: string = name.substring(dot + 1).toLowerCase();
    return extension === 'op' || extension === 'pen';
  }

  private static hasSupportedImageExtension(name: string): boolean {
    const dot: number = name.lastIndexOf('.');
    if (dot < 0 || dot === name.length - 1) {
      return false;
    }
    const extension: string = name.substring(dot + 1).toLowerCase();
    return extension === 'png' || extension === 'jpg' || extension === 'jpeg' ||
      extension === 'gif' || extension === 'webp' || extension === 'svg';
  }
}
