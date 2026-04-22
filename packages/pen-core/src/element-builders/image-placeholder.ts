import type { ElementTree } from './helpers.js';

export interface ImagePlaceholderParams {
  /** Width in px. Default 200. Min 40. */
  width?: number;
  /** Height in px. Default 140. Min 40. */
  height?: number;
  /** Optional caption shown below the icon (e.g. "Upload", "Hero image"). */
  label?: string;
  /**
   * Lucide icon name shown centered. Default "image" (generic).
   * Common alternatives: "image-plus" (uploadable), "video" (video
   * placeholder), "camera" (capture). Not constrained here — the
   * lookup happens at render time.
   */
  icon?: string;
  /**
   * Corner radius. Default 8 (typical card/image affordance).
   * Set to `width/2` externally if you want a round avatar-style
   * placeholder, but `add_avatar_v0` is a cleaner fit for that.
   */
  corner_radius?: number;
}

/**
 * Image placeholder — gray box with centered icon + optional label.
 * The "this will be an image later" affordance. Covers a gap the
 * N-tool family had before (G() handled real image fetch, but when
 * the caller wants a visual slot for a future image they had to
 * fall back to batch_design).
 *
 * Shape:
 *   frame(w×h, cornerRadius, fill=#F1F5F9 slate-100, layout=vertical,
 *         alignItems=center, justifyContent=center, gap=8)
 *     └ icon_font(40×40, role='image-placeholder-icon')
 *     └ optional text(role='image-placeholder-label', fontSize=13,
 *                     fontWeight=500, color=#64748B slate-500)
 *
 * Uses `frame + fill`, NEVER image node — an empty image (src='')
 * renders as a broken image indicator in the canvas, which is
 * wrong for "future image" semantics. When you want an actual
 * image that may not have a src yet (e.g. G() output pre-enrich),
 * use the `image` type directly via batch_design.
 */
export function buildImagePlaceholder(params: ImagePlaceholderParams): ElementTree {
  const width = Math.max(40, Math.floor(params.width ?? 200));
  const height = Math.max(40, Math.floor(params.height ?? 140));
  const icon = params.icon ?? 'image';
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 8));

  const children: ElementTree[] = [
    {
      type: 'icon_font',
      name: 'Placeholder Icon',
      role: 'image-placeholder-icon',
      iconFontName: icon,
      iconFontFamily: 'lucide',
      width: 40,
      height: 40,
      fill: [{ type: 'solid', color: '#94A3B8' }],
    },
  ];
  if (params.label) {
    children.push({
      type: 'text',
      name: 'Label',
      role: 'image-placeholder-label',
      content: params.label,
      fontSize: 13,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#64748B' }],
    });
  }

  return {
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
    fill: [{ type: 'solid', color: '#F1F5F9' }],
    children,
  };
}
