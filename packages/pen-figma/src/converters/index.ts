export type { ConversionContext, IconLookupResult } from './common.js';
export {
  SKIPPED_TYPES,
  collectImageBlobs,
  commonProps,
  extractPosition,
  extractRotation,
  extractFlip,
  normalizeAngle,
  mapCornerRadius,
  resolveWidth,
  resolveHeight,
  scaleTreeChildren,
  figmaFillColor,
  setIconLookup,
  lookupIconByName,
} from './common.js';
export {
  convertFrame,
  convertGroup,
  convertComponent,
  convertInstance,
} from './frame-converter.js';
export { convertRectangle, convertEllipse, convertLine } from './shape-converter.js';
export { convertText } from './text-converter.js';
export { convertVector } from './path-converter.js';

import type { TreeNode } from '../figma-tree-builder.js';
import type { ConversionContext } from './common.js';
import { SKIPPED_TYPES } from './common.js';
import {
  convertFrame,
  convertGroup,
  convertComponent,
  convertInstance,
  setConvertChildren,
} from './frame-converter.js';
import { convertRectangle, convertEllipse, convertLine } from './shape-converter.js';
import { convertText } from './text-converter.js';
import { convertVector } from './path-converter.js';
import type { PenNode } from '@zseven-w/pen-types';

/** Convert all children of a parent TreeNode. */
export function convertChildren(parent: TreeNode, ctx: ConversionContext): PenNode[] {
  const parentStackMode = ctx.layoutMode === 'preserve' ? undefined : parent.figma.stackMode;
  const result: PenNode[] = [];

  for (const child of parent.children) {
    if (child.figma.visible === false) continue;
    // Skip fully transparent nodes — their children are also invisible and
    // the Skia renderer does not propagate parent opacity to descendants.
    if (child.figma.opacity !== undefined && child.figma.opacity <= 0) continue;
    const node = convertNode(child, parentStackMode, ctx);
    if (node) result.push(node);
  }

  return result;
}

// Inject convertChildren into frame-converter to avoid circular imports
setConvertChildren(convertChildren);

/**
 * Main dispatcher: convert a Figma TreeNode to a PenNode.
 * Returns null for unsupported/skipped node types.
 */
export function convertNode(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode | null {
  const figma = treeNode.figma;
  if (!figma.type || SKIPPED_TYPES.has(figma.type)) return null;

  switch (figma.type) {
    case 'FRAME':
    case 'SECTION':
      return convertFrame(treeNode, parentStackMode, ctx);

    case 'GROUP':
      return convertGroup(treeNode, parentStackMode, ctx);

    case 'SYMBOL':
      return convertComponent(treeNode, parentStackMode, ctx);

    case 'INSTANCE':
      return convertInstance(treeNode, parentStackMode, ctx);

    case 'RECTANGLE':
    case 'ROUNDED_RECTANGLE':
      return convertRectangle(treeNode, parentStackMode, ctx);

    case 'ELLIPSE':
      return convertEllipse(treeNode, parentStackMode, ctx);

    case 'LINE':
      return convertLine(treeNode, ctx);

    case 'VECTOR':
    case 'STAR':
    case 'REGULAR_POLYGON':
    case 'BOOLEAN_OPERATION':
      return convertVector(treeNode, parentStackMode, ctx);

    case 'TEXT':
      return convertText(treeNode, parentStackMode, ctx);

    default: {
      if (treeNode.children.length > 0) {
        return convertFrame(treeNode, parentStackMode, ctx);
      }
      ctx.warnings.push(`Skipped unsupported node type: ${figma.type} (${figma.name})`);
      return null;
    }
  }
}
