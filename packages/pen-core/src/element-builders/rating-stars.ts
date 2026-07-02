import type { ElementTree } from './helpers.js';

export interface RatingStarsParams {
  filled: number;
  total?: number;
  size?: number;
}

/**
 * Star rating row (e.g. review 4/5). Emits `total` lucide `star`
 * icons; first `filled` get role='star-filled' + rest 'star-empty'
 * so batch_design U-op can apply semantic colors. `filled` clamped
 * to [0, total]; fractions floor (no partial-star glyph in lucide).
 */
export function buildRatingStars(params: RatingStarsParams): ElementTree {
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
