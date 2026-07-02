import type { ElementTree } from './helpers.js';

export interface SkeletonParams {
  /** Number of skeleton rows to emit (clamped 1..20). Default 3. */
  rows?: number;
  /** Height per row in px (clamped 4..48). Default 16. */
  row_height?: number;
  /** Gap between rows in px (clamped 0..32). Default 12. */
  row_gap?: number;
  /**
   * When true, the LAST row is 60% width (simulates an unfinished
   * paragraph line — the classic "shimmer" pattern). Default true.
   */
  last_row_short?: boolean;
}

/**
 * Loading skeleton: N stacked gray rectangles mimicking text lines
 * while content fetches. Each row is `rectangle + fill_container +
 * height` — avoids the frame-with-fill route so post-processing
 * won't treat it as a card. cornerRadius=4 (subtle rounding, matches
 * typical shimmer libraries).
 *
 * Last row defaults to 60% width (`fit_content` + px width) to
 * suggest a paragraph that ends mid-line — more organic than a
 * grid of uniform bars.
 */
export function buildSkeleton(params: SkeletonParams): ElementTree {
  const rows = Math.max(1, Math.min(20, params.rows ?? 3));
  const rowHeight = Math.max(4, Math.min(48, params.row_height ?? 16));
  const rowGap = Math.max(0, Math.min(32, params.row_gap ?? 12));
  const lastShort = params.last_row_short ?? true;

  const children: ElementTree[] = [];
  for (let i = 0; i < rows; i++) {
    const isLast = i === rows - 1;
    if (isLast && lastShort && rows > 1) {
      // Short last row: fixed-px width wrapped in a container that
      // pushes it to start. Using a 60%-suggestive hard px here
      // would vary by canvas; wrap in a fill_container row and
      // let the rectangle take ~60% via fit_content + width=60%-of-
      // typical-mobile (220px ≈ 60% of 375 mobile frame).
      children.push({
        type: 'rectangle',
        role: 'skeleton-row',
        width: 220,
        height: rowHeight,
        cornerRadius: 4,
        fill: [{ type: 'solid', color: '#E2E8F0' }],
      });
    } else {
      children.push({
        type: 'rectangle',
        role: 'skeleton-row',
        width: 'fill_container',
        height: rowHeight,
        cornerRadius: 4,
        fill: [{ type: 'solid', color: '#E2E8F0' }],
      });
    }
  }

  return {
    type: 'frame',
    name: 'Skeleton',
    role: 'skeleton',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: rowGap,
    alignItems: 'start',
    children,
  };
}
