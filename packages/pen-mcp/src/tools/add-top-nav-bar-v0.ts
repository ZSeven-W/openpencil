import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddTopNavBarV0Params {
  title: string;
  leading_icon?: string;
  trailing_icon?: string;
  height?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Mobile top navigation bar: optional leading icon (usually back/menu)
 * + centered title + optional trailing icon (search / more).
 *
 * Dual of add_bottom_nav_v0. Forces the fill_container + fixed height
 * + horizontal space_between + padding=[0,16] pattern that's consistent
 * across iOS-style and Material-style app bars.
 *
 * Title always centered. Leading/trailing icons occupy 44×44 hit targets
 * per Apple HIG + Material guidelines.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddTopNavBarV0(
  params: AddTopNavBarV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const height = params.height ?? 56;
  const bar = {
    type: 'frame',
    name: 'Top Nav Bar',
    role: 'top-nav-bar',
    width: 'fill_container',
    height,
    layout: 'horizontal',
    justifyContent: 'space_between',
    alignItems: 'center',
    padding: [0, 16],
    children: [
      buildIconSlot(params.leading_icon, 'leading'),
      {
        type: 'text',
        name: 'Title',
        role: 'heading',
        content: params.title,
        fontSize: 17,
        fontWeight: 600,
      },
      buildIconSlot(params.trailing_icon, 'trailing'),
    ],
  };
  assignIdsRecursively(bar);
  return insertElementTree({ binding: 'nav', tree: bar, ...params });
}

function buildIconSlot(
  icon: string | undefined,
  position: 'leading' | 'trailing',
): Record<string, unknown> {
  if (!icon) {
    // Empty spacer with the same 44x44 footprint so the title stays
    // visually centered even with an asymmetric slot.
    return {
      type: 'frame',
      name: `${position} Spacer`,
      role: 'nav-spacer',
      width: 44,
      height: 44,
      layout: 'none',
      children: [],
    };
  }
  return {
    type: 'frame',
    name: `${position} Icon Button`,
    role: 'icon-button',
    width: 44,
    height: 44,
    layout: 'horizontal',
    justifyContent: 'center',
    alignItems: 'center',
    cornerRadius: 8,
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: icon,
        iconFontFamily: 'lucide',
        width: 24,
        height: 24,
      },
    ],
  };
}
