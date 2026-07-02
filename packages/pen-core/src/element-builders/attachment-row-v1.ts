import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface AttachmentRowV1Params {
  /** File name (shown bold). */
  filename: string;
  /** Optional size label (e.g. "1.2 MB", "340 KB"). Rendered muted. */
  size?: string;
  /**
   * File-type icon. Default "file". Caller picks from lucide's
   * file-* set (file, file-text, file-image, file-video, file-audio,
   * file-archive, file-spreadsheet, file-code).
   */
  icon?: string;
  /**
   * When true, appends a small × icon on the right as a remove affordance.
   * Default true.
   */
  removable?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_attachment_row_v0.
   * - `'dark'`: dark surface + dark-mode text fills.
   * - `'system'`: emits `$color-*` ref strings for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * File attachment row — theme-aware version of buildAttachmentRow.
 * Light mode is byte-equal to add_attachment_row_v0.
 */
export function buildAttachmentRowV1(params: AttachmentRowV1Params): ElementTree {
  const removable = params.removable !== false;
  const icon = params.icon ?? 'file';
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  // Light-mode constants (v0 byte-parity)
  const filenameFill = isLight ? '#0F172A' : t.colors.textPrimary;
  const sizeFill = isLight ? '#64748B' : t.colors.textMuted;
  const iconFill = isLight ? '#64748B' : t.colors.textMuted;
  const removeFill = isLight ? '#94A3B8' : t.colors.textSubtle;
  const surfaceFill = isLight ? '#F8FAFC' : t.colors.bgDeep;

  const metaChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Filename',
      role: 'attachment-filename',
      content: params.filename,
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: filenameFill }],
    },
  ];
  if (params.size) {
    metaChildren.push({
      type: 'text',
      name: 'Size',
      role: 'attachment-size',
      content: params.size,
      fontSize: 12,
      fontWeight: 400,
      fill: [{ type: 'solid', color: sizeFill }],
    });
  }

  const rowChildren: ElementTree[] = [
    {
      type: 'icon_font',
      name: 'Type Icon',
      role: 'attachment-icon',
      iconFontName: icon,
      iconFontFamily: 'lucide',
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: iconFill }],
    },
    {
      type: 'frame',
      name: 'Meta',
      role: 'attachment-meta',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      gap: 4,
      children: metaChildren,
    },
  ];

  if (removable) {
    rowChildren.push({
      type: 'icon_font',
      name: 'Remove',
      role: 'attachment-remove',
      iconFontName: 'x',
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
      fill: [{ type: 'solid', color: removeFill }],
    });
  }

  return {
    type: 'frame',
    name: 'Attachment',
    role: 'attachment-row',
    width: 'fill_container',
    height: 'fit_content',
    cornerRadius: 8,
    layout: 'horizontal',
    alignItems: 'center',
    gap: 10,
    padding: [10, 12],
    fill: [{ type: 'solid', color: surfaceFill }],
    children: rowChildren,
  };
}
