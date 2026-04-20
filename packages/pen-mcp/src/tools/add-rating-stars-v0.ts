import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddRatingStarsV0Params {
  filled: number;
  total?: number;
  size?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Star rating row (e.g. review stars 4 / 5). Emits `total` lucide `star`
 * icons in a horizontal fit_content frame; the first `filled` get
 * role='star-filled' and the remainder get role='star-empty' so a
 * follow-up batch_design U-op can apply semantic colors. Using two roles
 * instead of two different icon names is deliberate — lucide has no
 * pre-filled `star-solid` variant, and coloring by role stays Style-Guide
 * orthogonal (§D6).
 *
 * `filled` is clamped to `[0, total]`; fractional values floor toward the
 * nearest full star (no partial-star rendering — pen-core icon_font is a
 * single glyph).
 */
export async function handleAddRatingStarsV0(
  params: AddRatingStarsV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const total = Math.max(1, Math.floor(params.total ?? 5));
  const filled = Math.max(0, Math.min(total, Math.floor(params.filled)));
  const size = Math.max(8, Math.floor(params.size ?? 16));
  const children: Record<string, unknown>[] = [];
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
  const row = {
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
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'rating_stars', tree: row, ...params });
}
