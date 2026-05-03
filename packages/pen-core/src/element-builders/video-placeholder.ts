import type { ElementTree } from './helpers.js';

export interface VideoPlaceholderParams {
  /** Width in px. Default 320. Min 80. */
  width?: number;
  /** Height in px. Default 180 (16:9). Min 60. */
  height?: number;
  /** Optional caption shown below the play icon (e.g. "Coming soon"). */
  label?: string;
  /** Corner radius. Default 12. */
  corner_radius?: number;
}

/**
 * Video placeholder — dark gray box with a centered play triangle
 * icon + optional caption. The "future video embed" affordance.
 *
 * Structurally close to `add_image_placeholder_v0` but visually
 * distinct: darker background (suggests the video frame itself is
 * dark), icon is `play` by default (not `image`), aspect-ratio
 * biased towards 16:9 (320×180 default instead of 200×140). A
 * separate tool because the semantic fit is different — an image
 * slot reads as "picture coming", a video slot reads as "play me
 * later" — and the AI benefits from the narrower shape-pocket.
 *
 * Structure:
 *   frame(w×h, cornerRadius=12, fill=#334155 slate-700,
 *         layout=vertical, center justified, gap=8)
 *     └ icon_font("play", 48×48, role='video-placeholder-icon', white)
 *     └ optional text(label, 13/500, white/70 opacity, role='video-placeholder-label')
 */
export function buildVideoPlaceholder(params: VideoPlaceholderParams): ElementTree {
  const width = Math.max(80, Math.floor(params.width ?? 320));
  const height = Math.max(60, Math.floor(params.height ?? 180));
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 12));

  const children: ElementTree[] = [
    {
      type: 'icon_font',
      name: 'Play Icon',
      role: 'video-placeholder-icon',
      iconFontName: 'play',
      iconFontFamily: 'lucide',
      width: 48,
      height: 48,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    },
  ];
  if (params.label) {
    children.push({
      type: 'text',
      name: 'Label',
      role: 'video-placeholder-label',
      content: params.label,
      fontSize: 13,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#FFFFFFB3' }], // white @ 70%
    });
  }

  return {
    type: 'frame',
    name: 'Video Placeholder',
    role: 'video-placeholder',
    width,
    height,
    cornerRadius,
    layout: 'vertical',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
    fill: [{ type: 'solid', color: '#334155' }],
    children,
  };
}
