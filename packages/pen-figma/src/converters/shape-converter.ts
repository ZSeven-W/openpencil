import type { PenNode } from '@zseven-w/pen-types';
import { mapFigmaFills } from '../figma-fill-mapper.js';
import { mapFigmaStroke } from '../figma-stroke-mapper.js';
import { mapFigmaEffects } from '../figma-effect-mapper.js';
import type { TreeNode } from '../figma-tree-builder.js';
import {
  commonProps,
  resolveWidth,
  resolveHeight,
  mapCornerRadius,
  normalizeAngle,
  extractPosition,
  extractRotation,
  type ConversionContext,
} from './common.js';

export function convertRectangle(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma;
  const id = ctx.generateId();

  return {
    type: 'rectangle',
    ...commonProps(figma, id),
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
    cornerRadius: mapCornerRadius(figma),
    fill: mapFigmaFills(figma.fillPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
  };
}

export function convertEllipse(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma;
  const id = ctx.generateId();

  // Convert Figma arcData (radians) to PenNode arc properties (degrees)
  const arc = figma.arcData;
  const arcProps = arc ? mapFigmaArcData(arc) : {};
  const props = commonProps(figma, id);

  // For arc ellipses, absorb flipX/flipY into the arc angles instead of
  // relying on canvas-level flip (SVG path flip doesn't work well in Fabric.js).
  if (
    arcProps.sweepAngle !== undefined ||
    arcProps.startAngle !== undefined ||
    arcProps.innerRadius !== undefined
  ) {
    const start = arcProps.startAngle ?? 0;
    const sweep = arcProps.sweepAngle ?? 360;
    if (props.flipX) {
      arcProps.startAngle = normalizeAngle(180 - start - sweep);
      arcProps.sweepAngle = sweep;
      delete props.flipX;
    }
    if (props.flipY) {
      arcProps.startAngle = normalizeAngle(360 - start - sweep);
      arcProps.sweepAngle = sweep;
      delete props.flipY;
    }
  }

  return {
    type: 'ellipse',
    ...props,
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
    ...arcProps,
    fill: mapFigmaFills(figma.fillPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
  };
}

/** Convert Figma arcData (radians, endAngle) to PenNode arc props (degrees, sweepAngle). */
function mapFigmaArcData(arc: {
  startingAngle?: number;
  endingAngle?: number;
  innerRadius?: number;
}): {
  startAngle?: number;
  sweepAngle?: number;
  innerRadius?: number;
} {
  const startRad = arc.startingAngle ?? 0;
  const endRad = arc.endingAngle ?? Math.PI * 2;
  const inner = arc.innerRadius ?? 0;

  let actualStartRad: number;
  let sweepRad: number;

  if (endRad >= startRad) {
    actualStartRad = startRad;
    sweepRad = endRad - startRad;
  } else {
    actualStartRad = endRad;
    sweepRad = startRad - endRad;
  }

  const startDeg = (actualStartRad * 180) / Math.PI;
  const sweepDeg = (sweepRad * 180) / Math.PI;

  const result: { startAngle?: number; sweepAngle?: number; innerRadius?: number } = {};
  if (Math.abs(startDeg) > 0.1) result.startAngle = Math.round(startDeg * 100) / 100;
  if (Math.abs(sweepDeg - 360) > 0.1) result.sweepAngle = Math.round(sweepDeg * 100) / 100;
  if (inner > 0.001) result.innerRadius = Math.round(inner * 1000) / 1000;
  return result;
}

export function convertLine(treeNode: TreeNode, ctx: ConversionContext): PenNode {
  const figma = treeNode.figma;
  const id = ctx.generateId();
  const { x, y } = extractPosition(figma);
  const w = figma.size?.x ?? 100;

  return {
    type: 'line',
    id,
    name: figma.name || undefined,
    x,
    y,
    x2: x + w,
    y2: y,
    rotation: extractRotation(figma.transform),
    opacity: figma.opacity !== undefined && figma.opacity < 1 ? figma.opacity : undefined,
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
  };
}
