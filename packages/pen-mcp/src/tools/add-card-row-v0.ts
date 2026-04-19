import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  buildScrollWrapper,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddCardRowV0Item {
  title: string;
  subtitle?: string;
  icon?: string;
}

export interface AddCardRowV0Params {
  items: AddCardRowV0Item[];
  card_width?: number;
  gap?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Horizontal scroll row of CARDS (title + subtitle + optional icon).
 * Each card: 140×160 frame, cornerRadius=20, padding=16, vertical layout.
 *
 * Narrow tool (no children_type union). Use this when the spec mentions
 * "workout cards", "feature cards", "swipeable cards with image/label",
 * or anything where each item has a title plus descriptive subtext.
 *
 * Structural wrapper is shared with add_metric_row_v0 / add_nav_chip_row_v0
 * via buildScrollWrapper — all three produce the same overflow-safe
 * outer shape but differ in child node shape.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddCardRowV0(
  params: AddCardRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const cardWidth = params.card_width ?? 140;
  const gap = params.gap ?? 12;
  const cards = params.items.map((item) => buildCard(item, cardWidth));
  const wrapper = buildScrollWrapper({
    rowName: 'Card Row',
    innerChildren: cards,
    gap,
  });
  assignIdsRecursively(wrapper);
  return insertElementTree({ binding: 'row', tree: wrapper, ...params });
}

function buildCard(item: AddCardRowV0Item, cardWidth: number): Record<string, unknown> {
  const children: Record<string, unknown>[] = [];
  if (item.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: item.icon,
      iconFontFamily: 'lucide',
      width: 24,
      height: 24,
    });
  }
  children.push({
    type: 'text',
    name: 'Title',
    role: 'heading',
    content: item.title,
    fontSize: 16,
    fontWeight: 600,
    width: 'fill_container',
  });
  if (item.subtitle) {
    children.push({
      type: 'text',
      name: 'Subtitle',
      role: 'body',
      content: item.subtitle,
      fontSize: 13,
      fontWeight: 400,
      width: 'fill_container',
    });
  }
  return {
    type: 'frame',
    name: 'Card',
    role: 'card',
    width: cardWidth,
    height: 160,
    cornerRadius: 20,
    padding: 16,
    layout: 'vertical',
    gap: 8,
    children,
  };
}
