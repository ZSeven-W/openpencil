import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ImagePlaceholderV1Params {
  /** Width in px. Default 200. Min 40. */
  width?: number;
  /** Height in px. Default 140. Min 40. */
  height?: number;
  /** Optional caption shown below the icon (e.g. "Upload", "Hero image"). */
  label?: string;
  /**
   * Lucide icon name shown centered. Default "image" (generic).
   * Common alternatives: "image-plus" (uploadable), "video" (video
   * placeholder), "camera" (capture).
   */
  icon?: string;
  /**
   * Corner radius. Default 8.
   */
  corner_radius?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_image_placeholder_v0.
   * - `'dark'`: bg → bgDeep, icon → textMuted, label → textMuted.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
  /**
   * 2-3 English keywords to search for when the auto-search pass replaces
   * this placeholder with a real photo (e.g. "burger fries", "modern
   * office"). Stamped onto the frame as `imageSearchQuery`; consumed by
   * the web app's `image-search-pipeline`. Without it the pipeline falls
   * back to `label` then a generic "placeholder" query.
   */
  image_search_query?: string;
}

/**
 * Image placeholder — theme-aware variant of buildImagePlaceholder.
 * Light mode is byte-equal to add_image_placeholder_v0.
 *
 * Color mapping:
 *   frame bg (#F1F5F9 slate-100) → bgDeep
 *   icon   (#94A3B8 slate-400)   → textMuted
 *   label  (#64748B slate-500)   → textMuted
 */
export function buildImagePlaceholderV1(params: ImagePlaceholderV1Params): ElementTree {
  const width = Math.max(40, Math.floor(params.width ?? 200));
  const height = Math.max(40, Math.floor(params.height ?? 140));
  const icon = params.icon ?? 'image';
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 8));
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const frameBg = isLight ? '#F1F5F9' : t.colors.bgDeep;
  const iconColor = isLight ? '#94A3B8' : t.colors.textMuted;
  const labelColor = isLight ? '#64748B' : t.colors.textMuted;

  const children: ElementTree[] = [
    {
      type: 'icon_font',
      name: 'Placeholder Icon',
      role: 'image-placeholder-icon',
      iconFontName: icon,
      iconFontFamily: 'lucide',
      width: 40,
      height: 40,
      fill: [{ type: 'solid', color: iconColor }],
    },
  ];
  // Conditional emit: builders never invent optional content, and an
  // empty-content text node still consumes flex gap so it is not
  // layout-neutral with a "no label" output. Callers wanting the
  // image-placeholder-label role must pass a non-empty `label`.
  if (params.label) {
    children.push({
      type: 'text',
      name: 'Label',
      role: 'image-placeholder-label',
      content: params.label,
      fontSize: 13,
      fontWeight: 500,
      fill: [{ type: 'solid', color: labelColor }],
    });
  }

  const frame: ElementTree = {
    type: 'frame',
    name: 'Image Placeholder',
    role: 'image-placeholder',
    width,
    height,
    cornerRadius,
    layout: 'vertical',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
    fill: [{ type: 'solid', color: frameBg }],
    children,
  };
  if (typeof params.image_search_query === 'string' && params.image_search_query.length > 0) {
    (frame as ElementTree & { imageSearchQuery?: string }).imageSearchQuery =
      params.image_search_query;
  }
  return frame;
}
