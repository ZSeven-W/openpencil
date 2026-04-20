import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddCarouselDotsV0Params {
  total: number;
  current?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Carousel / pager pagination dots. `total` dots laid out horizontally;
 * the dot at `current` (0-indexed, clamped) is stretched into an
 * elongated pill (16×6 with cornerRadius=3) so the active state reads at
 * a glance — the dominant iOS/Material pattern. Inactive dots are 6×6
 * circles.
 *
 * Emitted as `frame+cornerRadius=width/2` (not ellipse) so cornerRadius
 * handles the rounding without the renderer caveats around ellipse +
 * sibling layout (see pen-ai-skills/skills/phases/generation/layout.md
 * §RING / CIRCLE WITH CENTER CONTENT).
 */
export async function handleAddCarouselDotsV0(
  params: AddCarouselDotsV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const total = Math.max(1, Math.floor(params.total));
  const current = Math.max(0, Math.min(total - 1, Math.floor(params.current ?? 0)));
  const children: Record<string, unknown>[] = [];
  for (let i = 0; i < total; i += 1) {
    const isActive = i === current;
    children.push({
      type: 'frame',
      name: isActive ? 'Dot Active' : 'Dot',
      role: isActive ? 'dot-active' : 'dot',
      width: isActive ? 16 : 6,
      height: 6,
      cornerRadius: 3,
      fill: [{ type: 'solid', color: isActive ? '#111827' : '#D1D5DB' }],
    });
  }
  const row = {
    type: 'frame',
    name: 'Carousel Dots',
    role: 'carousel-dots',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 6,
    children,
  };
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'carousel_dots', tree: row, ...params });
}
