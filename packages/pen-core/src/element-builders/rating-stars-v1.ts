import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface RatingStarsV1Params {
  filled: number;
  total?: number;
  size?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_rating_stars_v0.
   * - `'dark'`: identical (no hardcoded colors in v0 — icons inherit fill).
   * - `'system'`: identical (no color refs needed).
   */
  theme?: V1Theme;
}

/**
 * Star rating row (v1) — theme-aware variant of buildRatingStars.
 * No hardcoded colors in v0 (icon_font nodes with no explicit fill),
 * so all theme modes are identical and byte-parity with v0 is guaranteed.
 * Accepts theme param for API consistency.
 *
 * `filled` clamped to [0, total]; fractions floor (no partial-star glyph).
 */
export function buildRatingStarsV1(params: RatingStarsV1Params): ElementTree {
  const total = Math.max(1, Math.floor(params.total ?? 5));
  const filled = Math.max(0, Math.min(total, Math.floor(params.filled)));
  const size = Math.max(8, Math.floor(params.size ?? 16));
  const children: ElementTree[] = [];
  for (let i = 0; i < total; i += 1) {
    const isFilled = i < filled;
    children.push({
      type: 'icon_font',
      name: isFilled ? 'Star Filled' : 'Star Empty',
      role: isFilled ? 'star-filled' : 'star-empty',
      iconFontName: 'star',
      iconFontFamily: 'lucide',
      width: size,
      height: size,
    });
  }
  return {
    type: 'frame',
    name: 'Rating Stars',
    role: 'rating-stars',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 2,
    children,
  };
}
