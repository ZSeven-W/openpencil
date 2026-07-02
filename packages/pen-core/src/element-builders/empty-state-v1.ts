import type { ElementTree } from './helpers.js';
import { type V1Theme } from './resolve-theme.js';

export interface EmptyStateV1Params {
  title: string;
  subtitle?: string;
  icon?: string;
  cta_label?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_empty_state_v0.
   * - `'dark'` / `'system'`: identical output (no hardcoded colors in v0).
   * Accepts theme param for API consistency across all v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Empty-state block — theme-aware version of buildEmptyState.
 * No color fills are hardcoded in v0, so all three modes produce
 * identical output (byte-parity with v0 in all modes). The `theme`
 * parameter is accepted for API consistency.
 */
export function buildEmptyStateV1(params: EmptyStateV1Params): ElementTree {
  const children: ElementTree[] = [];
  if (params.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      role: 'empty-state-icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 48,
      height: 48,
    });
  }
  children.push({
    type: 'text',
    name: 'Title',
    role: 'empty-state-title',
    content: params.title,
    fontSize: 18,
    fontWeight: 600,
  });
  if (params.subtitle) {
    children.push({
      type: 'text',
      name: 'Subtitle',
      role: 'empty-state-subtitle',
      content: params.subtitle,
      fontSize: 14,
      fontWeight: 400,
    });
  }
  if (params.cta_label) {
    children.push({
      type: 'frame',
      name: 'CTA',
      role: 'button',
      cornerRadius: 24,
      padding: [12, 24],
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'CTA Label',
          role: 'label',
          content: params.cta_label,
          fontSize: 14,
          fontWeight: 500,
        },
      ],
    });
  }
  return {
    type: 'frame',
    name: 'Empty State',
    role: 'empty-state',
    width: 'fill_container',
    layout: 'vertical',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 16,
    padding: [48, 24],
    children,
  };
}
