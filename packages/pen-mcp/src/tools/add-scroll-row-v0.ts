import { handleBatchDesign } from './batch-design';
import { generateId } from '../utils/id';

export interface AddScrollRowV0Item {
  title: string;
  subtitle?: string;
  icon?: string;
}

export type AddScrollRowV0ChildrenType = 'card' | 'metric_tile' | 'nav_item';

export interface AddScrollRowV0Params {
  children_type: AddScrollRowV0ChildrenType;
  items: AddScrollRowV0Item[];
  card_width?: number;
  gap?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * MVP element tool (D1=A sugar route). Builds a horizontal scroll row that
 * strictly follows the pattern taught in
 * `packages/pen-ai-skills/skills/phases/generation/overflow.md` §HORIZONTAL
 * SCROLL ROWS: outer wrapper (fill_container + clipContent) > inner row
 * (fit_content, horizontal, gap, padding) > children with fixed widths.
 *
 * This is the exact structure LLMs (especially non-Claude) get wrong when
 * using batch_design directly. By exposing a narrow tool we force the
 * correct shape at schema level.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7.1
 */
export async function handleAddScrollRowV0(
  params: AddScrollRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  const gap = params.gap ?? 12;
  const cardWidth = params.card_width ?? defaultCardWidth(params.children_type);
  const wrapper = buildWrapperNode(params, gap, cardWidth);
  // batch_design only assigns an id to the top-level inserted node; nested
  // children come through untouched. Without ids on nested nodes, later
  // tree traversal / update / delete / move all fail. Assign ids to the
  // entire subtree here (batch_design harmlessly overwrites the top id).
  assignIdsRecursively(wrapper);
  const parentRef = params.parent_id ? `"${params.parent_id}"` : 'null';
  const dsl = `row=I(${parentRef}, ${JSON.stringify(wrapper)})`;
  return handleBatchDesign({
    operations: dsl,
    filePath: params.filePath,
    pageId: params.pageId,
    postProcess: false,
  });
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

function defaultCardWidth(kind: AddScrollRowV0ChildrenType): number {
  switch (kind) {
    case 'card':
      return 140;
    case 'metric_tile':
      return 120;
    case 'nav_item':
      return 72;
  }
}

function buildWrapperNode(
  params: AddScrollRowV0Params,
  gap: number,
  cardWidth: number,
): Record<string, unknown> {
  const cards = params.items.map((item) => buildChildNode(item, params.children_type, cardWidth));
  return {
    type: 'frame',
    name: `Scroll Row (${params.children_type})`,
    role: 'scroll-row-wrapper',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    clipContent: true,
    children: [
      {
        type: 'frame',
        name: 'Scroll Inner Row',
        role: 'scroll-row',
        width: 'fit_content',
        height: 'fit_content',
        layout: 'horizontal',
        gap,
        padding: [0, 20],
        children: cards,
      },
    ],
  };
}

function buildChildNode(
  item: AddScrollRowV0Item,
  kind: AddScrollRowV0ChildrenType,
  cardWidth: number,
): Record<string, unknown> {
  switch (kind) {
    case 'card':
      return buildCard(item, cardWidth);
    case 'metric_tile':
      return buildMetricTile(item, cardWidth);
    case 'nav_item':
      return buildNavItem(item, cardWidth);
  }
}

function buildCard(item: AddScrollRowV0Item, cardWidth: number): Record<string, unknown> {
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

function buildMetricTile(item: AddScrollRowV0Item, cardWidth: number): Record<string, unknown> {
  const children: Record<string, unknown>[] = [];
  if (item.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: item.icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  children.push({
    type: 'text',
    name: 'Label',
    role: 'body',
    content: item.title,
    fontSize: 12,
    fontWeight: 500,
    width: 'fill_container',
  });
  if (item.subtitle) {
    children.push({
      type: 'text',
      name: 'Value',
      role: 'heading',
      content: item.subtitle,
      fontSize: 28,
      fontWeight: 700,
      width: 'fill_container',
    });
  }
  return {
    type: 'frame',
    name: 'Metric Tile',
    role: 'metric-tile',
    width: cardWidth,
    height: 100,
    cornerRadius: 16,
    padding: 16,
    layout: 'vertical',
    gap: 4,
    children,
  };
}

function buildNavItem(item: AddScrollRowV0Item, cardWidth: number): Record<string, unknown> {
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
    name: 'Label',
    role: 'label',
    content: item.title,
    fontSize: 11,
    fontWeight: 500,
  });
  return {
    type: 'frame',
    name: 'Nav Item',
    role: 'nav-item',
    width: cardWidth,
    height: 'fit_content',
    cornerRadius: 12,
    padding: [8, 12],
    layout: 'vertical',
    gap: 4,
    alignItems: 'center',
    children,
  };
}
