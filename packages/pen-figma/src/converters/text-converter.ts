import type { PenNode } from '@zseven-w/pen-types';
import { mapFigmaFills } from '../figma-fill-mapper.js';
import { mapFigmaEffects } from '../figma-effect-mapper.js';
import { mapFigmaTextProps } from '../figma-text-mapper.js';
import type { TreeNode } from '../figma-tree-builder.js';
import { commonProps, resolveWidth, resolveHeight, type ConversionContext } from './common.js';

export function convertText(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma;
  const id = ctx.generateId();
  const textProps = mapFigmaTextProps(figma);
  const width = resolveWidth(figma, parentStackMode, ctx);

  // Reconcile textGrowth with the resolved width:
  // 1. Layout sizing string (fill_container, fit_content) — container dictates width,
  //    so text must use fixed-width mode (Textbox) for wrapping.
  // 2. textAutoResize missing (undefined in .fig binary) — Figma defaults to fixed
  //    dimensions; treat as fixed-width so text wraps at the stored width.
  if (textProps.textGrowth === undefined) {
    if (typeof width === 'string' || !figma.textAutoResize) {
      textProps.textGrowth = 'fixed-width';
    }
  } else if (textProps.textGrowth === 'auto' && typeof width === 'string') {
    textProps.textGrowth = 'fixed-width';
  }

  return {
    type: 'text',
    ...commonProps(figma, id),
    width,
    height: resolveHeight(figma, parentStackMode, ctx),
    ...textProps,
    fill: mapFigmaFills(figma.fillPaints),
    effects: mapFigmaEffects(figma.effects),
  };
}
