import type { PenNode, SizingBehavior } from '@zseven-w/pen-types';
import { mapFigmaFills } from '../figma-fill-mapper.js';
import { mapFigmaStroke } from '../figma-stroke-mapper.js';
import { mapFigmaEffects } from '../figma-effect-mapper.js';
import { decodeFigmaVectorPath, computeSvgPathBounds } from '../figma-vector-decoder.js';
import type { TreeNode } from '../figma-tree-builder.js';
import {
  commonProps,
  resolveWidth,
  resolveHeight,
  figmaFillColor,
  lookupIconByName,
  type ConversionContext,
} from './common.js';

export function convertVector(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma;
  const id = ctx.generateId();
  const name = figma.name ?? '';

  const iconMatch = lookupIconByName(name);
  if (iconMatch) {
    const iconW = resolveWidth(figma, parentStackMode, ctx);
    const iconH = resolveHeight(figma, parentStackMode, ctx);

    // Lucide/Feather icon paths use a 24×24 viewbox. When 图标会变小，按比例缩放
    // strokeWeight，这样线条就不会显得不成比例的粗。
    const iconSize = Math.min(
      typeof iconW === 'number' ? iconW : 24,
      typeof iconH === 'number' ? iconH : 24,
    );
    const iconScale = iconSize / 24;

    let stroke =
      iconMatch.style === 'stroke'
        ? (mapFigmaStroke(figma) ?? {
            thickness: 1.5,
            fill: [{ type: 'solid', color: figmaFillColor(figma) ?? '#000000' }],
            cap: 'round' as const,
            join: 'round' as const,
          })
        : mapFigmaStroke(figma);

    if (stroke && iconScale < 0.99) {
      const rawThickness = typeof stroke.thickness === 'number' ? stroke.thickness : 1.5;
      stroke = { ...stroke, thickness: Math.round(rawThickness * iconScale * 100) / 100 };
    }

    return {
      type: 'path',
      ...commonProps(figma, id),
      d: iconMatch.d,
      iconId: iconMatch.iconId,
      width: iconW,
      height: iconH,
      fill: iconMatch.style === 'fill' ? mapFigmaFills(figma.fillPaints) : undefined,
      stroke,
      effects: mapFigmaEffects(figma.effects),
    };
  }

  const pathD = decodeFigmaVectorPath(figma, ctx.blobs);
  if (pathD) {
    const props = commonProps(figma, id);
    let width: SizingBehavior = resolveWidth(figma, parentStackMode, ctx);
    let height: SizingBehavior = resolveHeight(figma, parentStackMode, ctx);

    // When Figma 报告矢量节点的零宽度或高度（常见于以原点为中心的 chevron/arrow
    // 图标），从解码的路径边界导出实际的视觉范围并调整位置。
    const sizeX = figma.size?.x ?? 0;
    const sizeY = figma.size?.y ?? 0;
    if ((sizeX < 0.01 || sizeY < 0.01) && typeof width === 'number' && typeof height === 'number') {
      const bounds = computeSvgPathBounds(pathD);
      if (bounds) {
        const pathW = bounds.maxX - bounds.minX;
        const pathH = bounds.maxY - bounds.minY;
        if (sizeX < 0.01 && pathW > 0.01) {
          width = Math.round(pathW * 100) / 100;
          props.x = Math.round((props.x + bounds.minX) * 100) / 100;
        }
        if (sizeY < 0.01 && pathH > 0.01) {
          height = Math.round(pathH * 100) / 100;
          props.y = Math.round((props.y + bounds.minY) * 100) / 100;
        }
      }
    }

    // Figma 的 strokeGeometry 是 EXPANDED 笔划轮廓（不是中心线）。
    const hasVisibleFills = figma.fillPaints?.some((p: any) => p.visible !== false);
    const hasVisibleStrokes = figma.strokePaints?.some((p: any) => p.visible !== false);
    const isStrokeOnlyOutline =
      !hasVisibleFills &&
      hasVisibleStrokes &&
      !figma.fillGeometry?.length &&
      figma.strokeGeometry?.length;

    if (isStrokeOnlyOutline) {
      const strokeAsFill = mapFigmaFills(figma.strokePaints!);
      return {
        type: 'path',
        ...props,
        d: pathD,
        width,
        height,
        fill: strokeAsFill,
        effects: mapFigmaEffects(figma.effects),
      };
    }

    return {
      type: 'path',
      ...props,
      d: pathD,
      width,
      height,
      fill: mapFigmaFills(figma.fillPaints),
      stroke: mapFigmaStroke(figma),
      effects: mapFigmaEffects(figma.effects),
    };
  }

  ctx.warnings.push(`Vector node "${figma.name}" converted as rectangle (path data not decodable)`);
  return {
    type: 'rectangle',
    ...commonProps(figma, id),
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
    fill: mapFigmaFills(figma.fillPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
  };
}
