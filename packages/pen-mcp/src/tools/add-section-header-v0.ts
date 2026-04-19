import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddSectionHeaderV0Action {
  label: string;
  icon?: string;
}

export interface AddSectionHeaderV0Params {
  title: string;
  action?: AddSectionHeaderV0Action;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Dashboard / landing section header: big title on the left, optional
 * trailing action (e.g. "See all →", "View More", "Edit").
 * Forces horizontal + space_between + alignItems=center layout so the
 * action always sits flush-right regardless of title length.
 *
 * Common failure on non-Claude models: title and action stack vertically
 * (wrong layout direction) or action overlaps title (missing
 * space_between). Tool encodes the correct pattern.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddSectionHeaderV0(
  params: AddSectionHeaderV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  // CRITICAL no-overlap invariants:
  //   1. Title CAN NOT horizontally overflow the action — solved by putting
  //      the title inside a fill_container sibling that absorbs all
  //      remaining horizontal space; action stays fit_content on the right.
  //   2. Title wrap height MUST propagate to the header's fit_content
  //      height so the NEXT sibling in the parent vertical layout gets
  //      pushed down by the wrapped title. Per
  //      packages/pen-ai-skills/skills/phases/generation/overflow.md:
  //        "Text in VERTICAL layout: width=fill_container + textGrowth=
  //         fixed-width. In horizontal: width=fit_content."
  //      If we put fill_container+fixed-width text directly inside a
  //      HORIZONTAL parent the layout engine does not compute wrap height
  //      correctly, so header.height:fit_content stays at 1-line height
  //      and following content overlaps the wrapped lines. Fix: wrap the
  //      title in a vertical container so the documented rule applies
  //      and wrap height is measured correctly.
  const children: Record<string, unknown>[] = [
    {
      type: 'frame',
      name: 'Title Container',
      role: 'section-header-title',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      children: [
        {
          type: 'text',
          name: 'Title',
          role: 'heading',
          content: params.title,
          fontSize: 20,
          fontWeight: 700,
          width: 'fill_container',
          textGrowth: 'fixed-width',
        },
      ],
    },
  ];
  if (params.action) {
    children.push(buildActionGroup(params.action));
  }
  const header = {
    type: 'frame',
    name: 'Section Header',
    role: 'section-header',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 16, // guaranteed breathing room between title and action
    children,
  };
  assignIdsRecursively(header);
  return insertElementTree({ binding: 'header', tree: header, ...params });
}

function buildActionGroup(action: AddSectionHeaderV0Action): Record<string, unknown> {
  const children: Record<string, unknown>[] = [
    {
      type: 'text',
      name: 'Action Label',
      role: 'label',
      content: action.label,
      fontSize: 14,
      fontWeight: 500,
    },
  ];
  if (action.icon) {
    children.push({
      type: 'icon_font',
      name: 'Action Icon',
      iconFontName: action.icon,
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  return {
    type: 'frame',
    name: 'Action',
    role: 'section-header-action',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
