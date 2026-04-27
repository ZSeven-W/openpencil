import type { ElementTree } from './helpers.js';

export type TagTone = 'default' | 'accent' | 'success' | 'warning' | 'error';

export interface TagParams {
  label: string;
  /** Render the trailing × close icon. Default true — that's what makes
   *  this tool a `tag` (closable filter / selection chip) rather than a
   *  static `badge`. Set false for read-only category tags that still
   *  want the larger surface than badge_v0 provides. */
  removable?: boolean;
  /** Color tone. Default `'default'` (slate). */
  tone?: TagTone;
}

interface ToneSpec {
  bg: string;
  fg: string;
}

const TONES: Record<TagTone, ToneSpec> = {
  default: { bg: '#F1F5F9', fg: '#475569' },
  accent: { bg: '#DBEAFE', fg: '#2563EB' },
  success: { bg: '#DCFCE7', fg: '#166534' },
  warning: { bg: '#FEF3C7', fg: '#B45309' },
  error: { bg: '#FEE2E2', fg: '#B91C1C' },
};

/**
 * Single closable tag — filter / selection / applied-criteria chip.
 * Pill body with label and (by default) a trailing × close icon.
 *
 * Distinct from `add_badge_v0` (read-only label, no close affordance,
 * smaller font) and `add_chip_input_v0` (multi-tag input field with
 * inline caret). Use this when the user can drop the criterion by
 * clicking the ×: "Status: Active ×", "Plan: Pro ×", "Tag: design ×".
 */
export function buildTag(params: TagParams): ElementTree {
  const tone = TONES[params.tone ?? 'default'];
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
