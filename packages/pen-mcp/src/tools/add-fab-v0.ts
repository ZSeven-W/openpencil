import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddFabV0Params {
  icon: string;
  size?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Floating action button — circular 56×56 (Material FAB default) with a
 * centered icon. Positioning (bottom-right of screen) is caller's job since
 * pen-core layout has no reliable absolute positioning. Tool just emits
 * the visual.
 *
 * role='fab'.
 */
export async function handleAddFabV0(
  params: AddFabV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const size = params.size ?? 56;
  const iconSize = Math.round(size * 0.43);
  const fab = {
    type: 'frame',
    name: 'FAB',
    role: 'fab',
    width: size,
    height: size,
    cornerRadius: size / 2,
    fill: [{ type: 'solid', color: '#2563EB' }],
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: params.icon,
        iconFontFamily: 'lucide',
        width: iconSize,
        height: iconSize,
      },
    ],
  };
  assignIdsRecursively(fab);
  return insertElementTree({ binding: 'fab', tree: fab, ...params });
}
