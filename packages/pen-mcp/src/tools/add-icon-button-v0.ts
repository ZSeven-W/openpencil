import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddIconButtonV0Params {
  icon: string;
  size?: number;
  icon_size?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Icon-only button. Forces the 44×44 minimum-hit-target + flex-centered
 * icon pattern (Apple HIG + Material). Common failure on non-Claude
 * models: layout="none" with manually-placed icon x/y (the documented
 * anti-pattern in pen-ai-skills memory: layout=none + absolute
 * positioning renders unreliably).
 *
 * Correct: frame(layout=horizontal, justifyContent=center,
 * alignItems=center) + centered icon_font child. This tool encodes it.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddIconButtonV0(
  params: AddIconButtonV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const size = params.size ?? 44;
  const iconSize = params.icon_size ?? 24;
  const button = {
    type: 'frame',
    name: 'Icon Button',
    role: 'icon-button',
    width: size,
    height: size,
    layout: 'horizontal',
    justifyContent: 'center',
    alignItems: 'center',
    cornerRadius: 8,
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
  assignIdsRecursively(button);
  return insertElementTree({ binding: 'btn', tree: button, ...params });
}
