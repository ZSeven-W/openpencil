import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddSegmentedControlV0Item {
  label: string;
  active?: boolean;
}

export interface AddSegmentedControlV0Params {
  items: AddSegmentedControlV0Item[];
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * iOS pill-style segmented control. Container 32px high, cornerRadius=8,
 * gray-100 fill. Each segment gets width='fill_container' so the pill
 * distributes equally (overflow-safe). Active segment floats white on
 * top via role 'segment-active'.
 *
 * roles: 'segmented-control' / 'segment' | 'segment-active'.
 * Use for "iOS segmented control", "pill tabs", "filter toggle group".
 */
export async function handleAddSegmentedControlV0(
  params: AddSegmentedControlV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const segments = params.items.map((item) => {
    const seg: Record<string, unknown> = {
      type: 'frame',
      name: `Segment (${item.label})`,
      role: item.active ? 'segment-active' : 'segment',
      width: 'fill_container',
      height: 'fill_container',
      cornerRadius: 6,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'Label',
          role: 'label',
          content: item.label,
          fontSize: 13,
          fontWeight: item.active ? 600 : 500,
        },
      ],
    };
    if (item.active) {
      seg.fill = [{ type: 'solid', color: '#FFFFFF' }];
    } else {
      seg.fill = [];
    }
    return seg;
  });
  const container = {
    type: 'frame',
    name: 'Segmented Control',
    role: 'segmented-control',
    width: 'fill_container',
    height: 32,
    cornerRadius: 8,
    fill: [{ type: 'solid', color: '#F3F4F6' }],
    layout: 'horizontal',
    alignItems: 'stretch',
    gap: 4,
    padding: [4],
    children: segments,
  };
  assignIdsRecursively(container);
  return insertElementTree({ binding: 'segmented', tree: container, ...params });
}
