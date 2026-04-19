import { handleBatchDesign } from './batch-design';
import { generateId } from '../utils/id';
import { ensureParentExists } from './element-tool-helpers';

export interface AddBottomNavV0Item {
  title: string;
  icon: string;
  active?: boolean;
}

export interface AddBottomNavV0Params {
  items: AddBottomNavV0Item[];
  height?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * MVP element tool — bottom navigation bar with fixed schema.
 *
 * Solves two failure modes taught in
 * `packages/pen-ai-skills/skills/phases/generation/layout.md`:
 *   - NO FIXED-POSITION (don't emit empty spacer siblings after the nav)
 *   - bottom-nav is inline part of the page flow, not floating
 *
 * The tool emits a single frame with role='bottom-tab-bar'; caller
 * inserts it as the LAST child of their page (no spacer needed).
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddBottomNavV0(
  params: AddBottomNavV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const height = params.height ?? 62;
  const nav = buildNav(params, height);
  assignIdsRecursively(nav);
  const parentRef = params.parent_id ? `"${params.parent_id}"` : 'null';
  const dsl = `nav=I(${parentRef}, ${JSON.stringify(nav)})`;
  return handleBatchDesign({
    operations: dsl,
    filePath: params.filePath,
    pageId: params.pageId,
    postProcess: false,
  });
}

function buildNav(params: AddBottomNavV0Params, height: number): Record<string, unknown> {
  const tabs = params.items.map((item) => buildTab(item));
  return {
    type: 'frame',
    name: 'Bottom Tab Bar',
    role: 'bottom-tab-bar',
    width: 'fill_container',
    height,
    layout: 'horizontal',
    justifyContent: 'space_around',
    alignItems: 'center',
    children: tabs,
  };
}

function buildTab(item: AddBottomNavV0Item): Record<string, unknown> {
  return {
    type: 'frame',
    name: `Tab (${item.title})`,
    role: item.active ? 'nav-item-active' : 'nav-item',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'vertical',
    alignItems: 'center',
    gap: 4,
    padding: [4, 12],
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
        content: item.title,
        fontSize: 11,
        fontWeight: item.active ? 600 : 500,
      },
    ],
  };
}

function assignIdsRecursively(node: Record<string, unknown>): void {
  if (typeof node.id !== 'string') node.id = generateId();
  const children = node.children;
  if (Array.isArray(children)) {
    for (const child of children) {
      if (child && typeof child === 'object') {
        assignIdsRecursively(child as Record<string, unknown>);
      }
    }
  }
}
