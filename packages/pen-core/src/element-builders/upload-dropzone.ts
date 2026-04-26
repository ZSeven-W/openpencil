import type { ElementTree } from './helpers.js';

export interface UploadDropzoneParams {
  /** Width in px. Default 480. Min 200. */
  width?: number;
  /** Height in px. Default 200. Min 120. */
  height?: number;
  /** Main instruction text. Default "Drop files to upload". */
  title?: string;
  /** Secondary hint (file types, size limits, etc). Default "or click to browse". */
  subtitle?: string;
  /** Lucide icon shown above the title. Default "upload-cloud". */
  icon?: string;
  /** Corner radius. Default 12. */
  corner_radius?: number;
}

/**
 * File upload dropzone — dashed-border tile with cloud icon +
 * two-line instruction. The "drop files here OR click to browse"
 * pattern from every modern upload UI.
 *
 * Structurally distinct from `add_empty_chart_v0` (also dashed
 * border + icon + title/subtitle) in two ways: bigger footprint
 * (480×200 default vs chart's 320×200), and the icon is semantic
 * (`upload-cloud` / `upload` / `file-up`) rather than chart-typed.
 * The AI picks between them by intent — "where users upload
 * something" → dropzone; "where a chart will render when data
 * arrives" → empty-chart.
 *
 * Structure:
 *   frame(w×h, cornerRadius=12, dashed stroke, bg=#F8FAFC,
 *         layout=vertical, center/center, gap=12, role='upload-dropzone')
 *     ├ icon_font(icon, 40×40, slate-500, role='upload-dropzone-icon')
 *     ├ text(title, 14/600 slate-700, role='upload-dropzone-title')
 *     └ text(subtitle, 12/400 slate-500, role='upload-dropzone-subtitle')
 */
export function buildUploadDropzone(params: UploadDropzoneParams): ElementTree {
  const width = Math.max(200, Math.floor(params.width ?? 480));
  const height = Math.max(120, Math.floor(params.height ?? 200));
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 12));
  const title = params.title ?? 'Drop files to upload';
  const subtitle = params.subtitle ?? 'or click to browse';
  const icon = params.icon ?? 'upload-cloud';

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
    paddingTop: 24,
    paddingBottom: 24,
    paddingLeft: 24,
    paddingRight: 24,
    fill: [{ type: 'solid', color: '#F8FAFC' }],
    stroke: {
      thickness: 1.5,
      fill: [{ type: 'solid', color: '#CBD5E1' }],
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
        fill: [{ type: 'solid', color: '#64748B' }],
      },
      {
        type: 'text',
        name: 'Title',
        role: 'upload-dropzone-title',
        content: title,
        fontSize: 14,
        fontWeight: 600,
        fill: [{ type: 'solid', color: '#334155' }],
      },
      {
        type: 'text',
        name: 'Subtitle',
        role: 'upload-dropzone-subtitle',
        content: subtitle,
        fontSize: 12,
        fontWeight: 400,
        fill: [{ type: 'solid', color: '#64748B' }],
      },
    ],
  };
}
