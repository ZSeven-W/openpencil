import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface UploadDropzoneV1Params {
  /** Width in px. Default 480. Min 200. */
  width?: number;
  /** Height in px. Default 200. Min 120. */
  height?: number;
  /** Main instruction text. Default "Drop files to upload". */
  title?: string;
  /** Secondary hint. Default "or click to browse". */
  subtitle?: string;
  /** Lucide icon. Default "upload-cloud". */
  icon?: string;
  /** Corner radius. Default 12. */
  corner_radius?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_upload_dropzone_v0.
   * - `'dark'`: dark-mode fills for bg, border, icon, title, subtitle.
   * - `'system'`: emits `$color-*` ref strings.
   */
  theme?: V1Theme;
}

/**
 * File upload dropzone (v1) — theme-aware variant of buildUploadDropzone.
 * Light mode is byte-equal to add_upload_dropzone_v0.
 *
 * Color slots:
 * - Outer bg: #F8FAFC → bgDeep
 * - Dashed stroke: #CBD5E1 → border
 * - Icon fill: #64748B → textMuted
 * - Title fill: #334155 → textBody
 * - Subtitle fill: #64748B → textMuted
 */
export function buildUploadDropzoneV1(params: UploadDropzoneV1Params): ElementTree {
  const width = Math.max(200, Math.floor(params.width ?? 480));
  const height = Math.max(120, Math.floor(params.height ?? 200));
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 12));
  const title = params.title ?? 'Drop files to upload';
  const subtitle = params.subtitle ?? 'or click to browse';
  const icon = params.icon ?? 'upload-cloud';

  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);

  // Light mode: v0 literals for byte-parity
  const bg = theme === 'light' ? '#F8FAFC' : t.colors.bgDeep;
  const strokeColor = theme === 'light' ? '#CBD5E1' : t.colors.border;
  const iconColor = theme === 'light' ? '#64748B' : t.colors.textMuted;
  const titleColor = theme === 'light' ? '#334155' : t.colors.textBody;
  const subtitleColor = theme === 'light' ? '#64748B' : t.colors.textMuted;

  return {
    type: 'frame',
    name: 'Upload Dropzone',
    role: 'upload-dropzone',
    width,
    height,
    cornerRadius,
    layout: 'vertical',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 12,
    padding: 24,
    fill: [{ type: 'solid', color: bg }],
    stroke: {
      thickness: 1.5,
      fill: [{ type: 'solid', color: strokeColor }],
      strokeDashArray: [6, 4],
    },
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        role: 'upload-dropzone-icon',
        iconFontName: icon,
        iconFontFamily: 'lucide',
        width: 40,
        height: 40,
        fill: [{ type: 'solid', color: iconColor }],
      },
      {
        type: 'text',
        name: 'Title',
        role: 'upload-dropzone-title',
        content: title,
        fontSize: 14,
        fontWeight: 600,
        fill: [{ type: 'solid', color: titleColor }],
      },
      {
        type: 'text',
        name: 'Subtitle',
        role: 'upload-dropzone-subtitle',
        content: subtitle,
        fontSize: 12,
        fontWeight: 400,
        fill: [{ type: 'solid', color: subtitleColor }],
      },
    ],
  };
}
