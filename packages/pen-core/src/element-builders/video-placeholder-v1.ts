import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface VideoPlaceholderV1Params {
  /** Width in px. Default 320. Min 80. */
  width?: number;
  /** Height in px. Default 180 (16:9). Min 60. */
  height?: number;
  /** Optional caption shown below the play icon. */
  label?: string;
  /** Corner radius. Default 12. */
  corner_radius?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_video_placeholder_v0.
   * - `'dark'`: identical — video placeholder uses intentionally dark surface (#334155)
   *   in all modes. The dark bg represents the video frame itself, not a surface token.
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

/**
 * Video placeholder (v1) — theme-aware variant of buildVideoPlaceholder.
 * Light mode is byte-equal to add_video_placeholder_v0.
 *
 * All colors are builder-private constants (spec §3.4):
 * - bg: #334155 (slate-700) — intentionally dark, represents the video frame
 * - icon: #FFFFFF — white on dark, constant
 * - caption: #FFFFFFB3 — white at 70% opacity, constant
 * All theme modes produce identical trees.
 */
export function buildVideoPlaceholderV1(params: VideoPlaceholderV1Params): ElementTree {
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
