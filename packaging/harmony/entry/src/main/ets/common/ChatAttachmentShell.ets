import { fileIo as fs } from '@kit.CoreFileKit';
import { image } from '@kit.ImageKit';
import { photoAccessHelper } from '@kit.MediaLibraryKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { ShellOutcome } from './DocumentShell';
import { EngineHost } from './EngineHost';
import { OpStatus } from './OpNative';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/** Mirrors `op_editor_core::chat::MAX_ATTACHMENT_BYTES`. */
const MAX_ATTACHMENT_BYTES = 5 * 1024 * 1024;
/** Raw picker input ceiling, matching the shell's other picker reads. */
const MAX_INPUT_BYTES = 32 * 1024 * 1024;
/** Longest edge tried first when a photo has to be re-encoded. */
const INITIAL_MAX_EDGE = 4096;
const MIN_MAX_EDGE = 512;
const JPEG_QUALITY = 85;

/** Engine-ready bytes plus the media type and display name they carry. */
export class PreparedAttachment {
  data: ArrayBuffer;
  mediaType: string;
  fileName: string;

  constructor(data: ArrayBuffer, mediaType: string, fileName: string) {
    this.data = data;
    this.mediaType = mediaType;
    this.fileName = fileName;
  }
}

/**
 * Shell action 13 (`OpShellAction.PICK_CHAT_ATTACHMENT`): the Studio Home
 * "Add screenshot" button and the chat attach button. Mirrors
 * `ChatAttachmentPicker.kt` / `ChatAttachmentCoordinator.swift`: present the
 * system photo picker for one image, normalise it for the engine (HEIC or
 * oversized photos become a JPEG under the 5 MiB cap), and stage it through
 * `editorAttachChatImage`. The engine already consumed the request, so a
 * dismissed picker is silent and needs no cancel call.
 */
export class ChatAttachmentShell {
  static async pickAndAttach(host: EngineHost): Promise<ShellOutcome> {
    let uri: string = '';
    try {
      const options = new photoAccessHelper.PhotoSelectOptions();
      options.MIMEType = photoAccessHelper.PhotoViewMIMETypes.IMAGE_TYPE;
      options.maxSelectNumber = 1;
      const viewPicker = new photoAccessHelper.PhotoViewPicker();
      const result: photoAccessHelper.PhotoSelectResult = await viewPicker.select(options);
      if (result.photoUris.length === 0) {
        return ShellOutcome.CANCELLED;
      }
      uri = result.photoUris[0];
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'photo picker failed: %{public}s', JSON.stringify(error));
      return ShellOutcome.FAILED;
    }

    let prepared: PreparedAttachment | null = null;
    try {
      prepared = await ChatAttachmentShell.prepare(
        ChatAttachmentShell.readBounded(uri),
        ChatAttachmentShell.displayNameOf(uri)
      );
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'could not read chat attachment: %{public}s', JSON.stringify(error));
    }
    if (prepared === null) {
      hilog.warn(DOMAIN, TAG, 'chat attachment is not a decodable image under the size cap');
      return ShellOutcome.FAILED;
    }

    const status: number = host.attachChatImage(prepared.data, prepared.mediaType, prepared.fileName);
    host.requestFrame();
    if (status === OpStatus.OK || status === OpStatus.CLOSING) {
      return status === OpStatus.OK ? ShellOutcome.OK : ShellOutcome.CANCELLED;
    }
    // Busy = the turn already holds the maximum attachments; InvalidArg =
    // the engine refused the bytes. Neither leaves partial state behind.
    hilog.warn(
      DOMAIN,
      TAG,
      'chat attachment rejected, status=%{public}d: %{public}s',
      status,
      host.lastError()
    );
    return ShellOutcome.CANCELLED;
  }

  /** The engine-recognised media type of `data`, from its magic bytes. */
  static sniffMediaType(data: ArrayBuffer): string | null {
    const bytes = new Uint8Array(data, 0, Math.min(12, data.byteLength));
    const startsWith = (prefix: number[]): boolean => {
      if (bytes.length < prefix.length) {
        return false;
      }
      for (let i = 0; i < prefix.length; i++) {
        if (bytes[i] !== prefix[i]) {
          return false;
        }
      }
      return true;
    };
    if (startsWith([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])) {
      return 'image/png';
    }
    if (startsWith([0xff, 0xd8, 0xff])) {
      return 'image/jpeg';
    }
    if (startsWith([0x47, 0x49, 0x46, 0x38, 0x37, 0x61]) || startsWith([0x47, 0x49, 0x46, 0x38, 0x39, 0x61])) {
      return 'image/gif';
    }
    if (bytes.length >= 12 && startsWith([0x52, 0x49, 0x46, 0x46]) &&
      bytes[8] === 0x57 && bytes[9] === 0x45 && bytes[10] === 0x42 && bytes[11] === 0x50) {
      return 'image/webp';
    }
    return null;
  }

  /** A display name the engine accepts, with an extension matching the bytes. */
  static fileName(suggested: string, mediaType: string): string {
    let stem: string = '';
    for (const ch of suggested) {
      const code: number = ch.charCodeAt(0);
      if (code >= 0x20 && code !== 0x7f && ch !== '/' && ch !== '\\') {
        stem += ch;
      }
    }
    stem = stem.trim();
    const dot: number = stem.lastIndexOf('.');
    if (dot > 0) {
      stem = stem.substring(0, dot);
    }
    if (stem.length === 0) {
      stem = 'photo';
    }
    let extension: string = 'png';
    if (mediaType === 'image/jpeg') {
      extension = 'jpg';
    } else if (mediaType === 'image/gif') {
      extension = 'gif';
    } else if (mediaType === 'image/webp') {
      extension = 'webp';
    }
    return `${stem.substring(0, 100)}.${extension}`;
  }

  private static async prepare(data: ArrayBuffer, suggestedName: string): Promise<PreparedAttachment | null> {
    const sniffed: string | null = ChatAttachmentShell.sniffMediaType(data);
    if (sniffed !== null && data.byteLength <= MAX_ATTACHMENT_BYTES) {
      return new PreparedAttachment(data, sniffed, ChatAttachmentShell.fileName(suggestedName, sniffed));
    }
    const jpeg: ArrayBuffer | null = await ChatAttachmentShell.reencodeAsJpeg(data);
    if (jpeg === null) {
      return null;
    }
    return new PreparedAttachment(jpeg, 'image/jpeg', ChatAttachmentShell.fileName(suggestedName, 'image/jpeg'));
  }

  /** Decodes HEIC / oversized photos and packs a JPEG under the cap. */
  private static async reencodeAsJpeg(data: ArrayBuffer): Promise<ArrayBuffer | null> {
    const source: image.ImageSource = image.createImageSource(data);
    const packer: image.ImagePacker = image.createImagePacker();
    try {
      const info: image.ImageInfo = await source.getImageInfo();
      const width: number = info.size.width;
      const height: number = info.size.height;
      if (width <= 0 || height <= 0) {
        return null;
      }
      let maxEdge: number = INITIAL_MAX_EDGE;
      while (maxEdge >= MIN_MAX_EDGE) {
        const longest: number = Math.max(width, height);
        const scale: number = longest > maxEdge ? maxEdge / longest : 1;
        const decodeOptions: image.DecodingOptions = {
          desiredSize: {
            width: Math.max(1, Math.round(width * scale)),
            height: Math.max(1, Math.round(height * scale)),
          },
        };
        const pixelMap: image.PixelMap = await source.createPixelMap(decodeOptions);
        const packOptions: image.PackingOption = { format: 'image/jpeg', quality: JPEG_QUALITY };
        let packed: ArrayBuffer;
        try {
          packed = await packer.packing(pixelMap, packOptions);
        } finally {
          await pixelMap.release();
        }
        if (packed.byteLength <= MAX_ATTACHMENT_BYTES) {
          return packed;
        }
        maxEdge = Math.floor(maxEdge * 3 / 4);
      }
      return null;
    } finally {
      await packer.release();
      await source.release();
    }
  }

  /** Reads the picker grant's file in one bounded read (see DocumentShell). */
  private static readBounded(uri: string): ArrayBuffer {
    const file = fs.openSync(uri, fs.OpenMode.READ_ONLY);
    try {
      const size: number = fs.statSync(file.fd).size;
      if (size <= 0 || size > MAX_INPUT_BYTES) {
        throw new Error('photo is empty or exceeds the 32 MiB mobile input limit');
      }
      const buffer = new ArrayBuffer(size);
      const read: number = fs.readSync(file.fd, buffer, { offset: 0, length: size });
      if (read !== size) {
        throw new Error('photo changed while it was being read');
      }
      return buffer;
    } finally {
      fs.closeSync(file);
    }
  }

  private static displayNameOf(uri: string): string {
    const slash: number = uri.lastIndexOf('/');
    const raw: string = slash >= 0 ? uri.substring(slash + 1) : uri;
    try {
      return decodeURIComponent(raw);
    } catch (error) {
      return raw;
    }
  }
}
