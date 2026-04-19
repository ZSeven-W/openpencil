import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  buildScrollWrapper,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddNavChipRowV0Item {
  label: string;
  icon: string;
  active?: boolean;
}

export interface AddNavChipRowV0Params {
  items: AddNavChipRowV0Item[];
  chip_width?: number;
  gap?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Horizontal scroll row of NAV CHIPS (icon + small label, with optional active state).
 * Each chip: 72×fit_content frame, cornerRadius=12, padding=[8,12], vertical layout,
 * alignItems=center. Active chips get role='nav-chip-active' + bolder label.
 *
 * Narrow tool. Use when the spec mentions "category filter chips", "quick
 * access shortcuts", "swipeable nav items", "horizontal tab chips".
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddNavChipRowV0(
  params: AddNavChipRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const chipWidth = params.chip_width ?? 72;
  const gap = params.gap ?? 12;
  const chips = params.items.map((item) => buildChip(item, chipWidth));
  const wrapper = buildScrollWrapper({
    rowName: 'Nav Chip Row',
    innerChildren: chips,
    gap,
  });
  assignIdsRecursively(wrapper);
  return insertElementTree({ binding: 'row', tree: wrapper, ...params });
}

function buildChip(item: AddNavChipRowV0Item, chipWidth: number): Record<string, unknown> {
  return {
    type: 'frame',
    name: `Chip (${item.label})`,
    role: item.active ? 'nav-chip-active' : 'nav-chip',
    width: chipWidth,
    height: 'fit_content',
    cornerRadius: 12,
    padding: [8, 12],
    layout: 'vertical',
    alignItems: 'center',
    gap: 4,
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: item.icon,
        iconFontFamily: 'lucide',
        width: 24,
        height: 24,
      },
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: item.label,
        fontSize: 11,
        fontWeight: item.active ? 600 : 500,
      },
    ],
  };
}
