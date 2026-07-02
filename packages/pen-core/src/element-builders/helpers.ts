import { generateId } from '../id.js';

/**
 * Common shape produced by all element-tool builders. Intentionally
 * loose (Record<string, unknown>) — the exact schema is validated by
 * the downstream insert pipeline (pen-mcp handleBatchDesign /
 * apps/web document-store.addNode).
 */
export type ElementTree = Record<string, unknown>;

/**
 * Walk a node subtree and stamp every node with a fresh id.
 * Mirrors the invariant pen-mcp's batch_design relies on — the
 * top-level insert gets an id but nested children don't, so builders
 * that emit multi-level trees must assign ids themselves.
 */
export function assignIdsRecursively(node: ElementTree): void {
  if (typeof node.id !== 'string') node.id = generateId();
  const children = node.children;
  if (Array.isArray(children)) {
    for (const child of children) {
      if (child && typeof child === 'object') {
        assignIdsRecursively(child as ElementTree);
      }
    }
  }
}

/**
 * Canonical scroll-row wrapper taught in
 * `packages/pen-ai-skills/skills/phases/generation/overflow.md`
 * §HORIZONTAL SCROLL ROWS: outer wrapper (fill_container + clipContent +
 * vertical) > inner row (fit_content + horizontal + gap + padding=[0,20]) >
 * children.
 *
 * Shared by add_card_row_v0 / add_metric_row_v0 / add_nav_chip_row_v0.
 * Each tool only differs in the per-item node builder; the wrapper
 * is identical and must stay so — it's what makes the overflow
 * behavior predictable on weak models.
 */
export function buildScrollWrapper(opts: {
  rowName: string;
  innerChildren: ElementTree[];
  gap: number;
}): ElementTree {
  return {
    type: 'frame',
    name: opts.rowName,
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
        gap: opts.gap,
        padding: [0, 20],
        children: opts.innerChildren,
      },
    ],
  };
}
