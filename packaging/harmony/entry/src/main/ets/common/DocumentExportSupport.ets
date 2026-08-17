/**
 * Pure helpers for the frozen-export save flow. Mirrors the Android
 * `DocumentExportSupport` and the iOS `DocumentExportCoordinator` filename
 * discipline so all three shells accept exactly the engine-derived names and
 * nothing else.
 */
export class DocumentExportSupport {
  /** Matches the engine-side 4 KiB export file-name ABI cap. */
  static readonly MAX_FILENAME_BYTES: number = 4 * 1024;

  /**
   * Extensions the engine can produce on this platform, in the order the
   * save picker offers them.
   *
   * WebP is deliberately ABSENT: like iOS, the pinned mobile Skia archive
   * has no WebP encoder, so the format stays hidden instead of failing at
   * save time.
   */
  static readonly SAVE_SUFFIXES: string[] = ['.png', '.jpg', '.jpeg', '.svg', '.pdf'];

  /**
   * Returns the filename when it is safe to place inside an app-private
   * staging directory: no traversal, no path separators, no control
   * characters, a non-empty stem and extension, and within the ABI cap.
   */
  static validatedFilename(filename: string | null): string | null {
    if (filename === null || filename.length === 0) {
      return null;
    }
    if (filename === '.' || filename === '..') {
      return null;
    }
    if (filename.includes('/') || filename.includes('\\')) {
      return null;
    }
    for (let i = 0; i < filename.length; i++) {
      const code: number = filename.charCodeAt(i);
      if (code < 0x20 || code === 0x7f) {
        return null;
      }
    }
    if (DocumentExportSupport.utf8Length(filename) > DocumentExportSupport.MAX_FILENAME_BYTES) {
      return null;
    }
    const dot: number = filename.lastIndexOf('.');
    if (dot <= 0 || dot === filename.length - 1) {
      return null;
    }
    return filename;
  }

  /** The lower-case extension including the leading dot, or '' when absent. */
  static suffixOf(filename: string): string {
    const dot: number = filename.lastIndexOf('.');
    if (dot < 0 || dot === filename.length - 1) {
      return '';
    }
    return filename.substring(dot).toLowerCase();
  }

  /** Save-picker MIME for the staged file's extension. */
  static mimeTypeFor(filename: string): string {
    switch (DocumentExportSupport.suffixOf(filename)) {
      case '.png':
        return 'image/png';
      case '.jpg':
      case '.jpeg':
        return 'image/jpeg';
      case '.svg':
        return 'image/svg+xml';
      case '.pdf':
        return 'application/pdf';
      default:
        return 'application/octet-stream';
    }
  }

  /**
   * The suffix list handed to the save picker. The engine already decided
   * the format, so the picker offers that one suffix; an unknown suffix
   * falls back to the full supported set.
   */
  static saveSuffixesFor(filename: string): string[] {
    const suffix: string = DocumentExportSupport.suffixOf(filename);
    if (DocumentExportSupport.SAVE_SUFFIXES.includes(suffix)) {
      return [suffix];
    }
    return DocumentExportSupport.SAVE_SUFFIXES;
  }

  private static utf8Length(value: string): number {
    let total = 0;
    for (let i = 0; i < value.length; i++) {
      const code: number = value.charCodeAt(i);
      if (code < 0x80) {
        total += 1;
      } else if (code < 0x800) {
        total += 2;
      } else if (code >= 0xd800 && code <= 0xdbff) {
        // A surrogate pair encodes one 4-byte code point.
        total += 4;
        i++;
      } else {
        total += 3;
      }
    }
    return total;
  }
}
