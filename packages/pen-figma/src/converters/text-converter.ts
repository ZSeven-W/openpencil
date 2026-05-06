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

  // Reconcile textGrowth 具有已解析的宽度： 1. Layout
  // 大小调整字符串 (fill_container、fit_content) — 容器规定宽度，因此文本必须使用固定宽度模式 (Textbox) 进行换行。 2.
  // textAutoResize 缺失（.fig 二进制文件中未定义）——Figma
  // 默认为固定维度；视为固定宽度，因此文本以存储的宽度换行。
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
