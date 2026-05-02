import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export type TagV1Tone = 'default' | 'accent' | 'success' | 'warning' | 'error';

const VALID_TAG_TONES = new Set<string>(['default', 'accent', 'success', 'warning', 'error']);

export interface TagV1Params {
  label: string;
  /** Render the trailing × close icon. Default true. */
  removable?: boolean;
  /** Color tone. Default `'default'` (slate). */
  tone?: TagV1Tone;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_tag_v0.
   * - `'dark'`: identical — tone bg/fg pairs are status semantic colors (spec §3.4).
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

interface ToneSpec {
  bg: string;
  fg: string;
}

// Status/alert tones: builder-private literals per spec §3.4.
// Kept hardcoded across all theme modes — they are semantic status colors
// (success=green, warning=amber, error=red), not surface theme tokens.
const TONES: Record<TagV1Tone, ToneSpec> = {
  default: { bg: '#F1F5F9', fg: '#475569' },
  accent: { bg: '#DBEAFE', fg: '#2563EB' },
  success: { bg: '#DCFCE7', fg: '#166534' },
  warning: { bg: '#FEF3C7', fg: '#B45309' },
  error: { bg: '#FEE2E2', fg: '#B91C1C' },
};

/**
 * Single closable tag (v1) — theme-aware variant of buildTag.
 * Light mode is byte-equal to add_tag_v0.
 *
 * Tone bg/fg pairs are status semantic colors (spec §3.4), kept
 * hardcoded across all theme modes. All modes produce identical trees.
 */
export function buildTagV1(params: TagV1Params): ElementTree {
  const requestedTone = (params.tone ?? 'default') as string;
  if (!VALID_TAG_TONES.has(requestedTone)) {
    throw new Error(
      `add_tag_v1: invalid tone "${requestedTone}"; expected one of: default, accent, success, warning, error`,
    );
  }
  const tone = TONES[requestedTone as TagV1Tone];
  const removable = params.removable ?? true;
  const children: ElementTree[] = [
    {
      type: 'text',
      name: 'Label',
      role: 'tag-label',
      content: params.label,
      fontSize: 13,
      fontWeight: 500,
      fill: [{ type: 'solid', color: tone.fg }],
    },
  ];
  if (removable) {
    children.push({
      type: 'icon_font',
      name: 'Remove',
      role: 'tag-remove',
      iconFontName: 'x',
      iconFontFamily: 'lucide',
      width: 14,
      height: 14,
      fill: [{ type: 'solid', color: tone.fg }],
    });
  }
  return {
    type: 'frame',
    name: 'Tag',
    role: 'tag',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 6,
    padding: [4, 10],
    cornerRadius: 12,
    fill: [{ type: 'solid', color: tone.bg }],
    children,
  };
}
