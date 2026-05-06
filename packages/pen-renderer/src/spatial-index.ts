import RBush from 'rbush';
import type { RenderNode } from './types.js';
import type { PenEffect, PenFill, PenNode, PenStroke } from '@zseven-w/pen-types';

interface RTreeItem {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
  nodeId: string;
  renderNode: RenderNode;
  /** 渲染数组中的 Position — 较高 = 稍后渲染 = 视觉上位于顶部 */
  zIndex: number;
}

/**
 * Spatial 使用 r
 * 树进行快速命中测试的索引。 Nodes 按其渲染顺序进行索引，因此命中结果按最上面的顺序排序（子
 * 项在父项之前）。
 */
export class SpatialIndex {
  private tree = new RBush<RTreeItem>();
  private items = new Map<string, RTreeItem>();

  /**
   * Rebuild 渲染节点列表中的整个索引。
   */
  rebuild(nodes: RenderNode[]) {
    this.tree.clear();
    this.items.clear();

    const items: RTreeItem[] = [];
    for (let i = 0; i < nodes.length; i++) {
      const rn = nodes[i];
      if (('visible' in rn.node ? rn.node.visible : undefined) === false) continue;
      if (('locked' in rn.node ? rn.node.locked : undefined) === true) continue;

      const item: RTreeItem = {
        minX: rn.absX,
        minY: rn.absY,
        maxX: rn.absX + rn.absW,
        maxY: rn.absY + rn.absH,
        nodeId: rn.node.id,
        renderNode: rn,
        zIndex: i,
      };
      items.push(item);
      this.items.set(rn.node.id, item);
    }

    this.tree.load(items);
  }

  /**
   * Find 包含给定场景点
   * 的所有节点。 Returns 节点按 z 顺序排序：最上面（最高的 zIndex）首先。
   */
  hitTest(sceneX: number, sceneY: number): RenderNode[] {
    const candidates = this.tree.search({
      minX: sceneX,
      minY: sceneY,
      maxX: sceneX,
      maxY: sceneY,
    });

    // Sort by zIndex 降序 — 子项（稍后渲染）先行
    candidates.sort((a, b) => b.zIndex - a.zIndex);
    return candidates.map((c) => c.renderNode).filter((rn) => isPointHittableRenderNode(rn));
  }

  /**
   * Find 与矩形相交的所有节点（用于选取框选择）。
   */
  searchRect(left: number, top: number, right: number, bottom: number): RenderNode[] {
    const candidates = this.tree.search({
      minX: Math.min(left, right),
      minY: Math.min(top, bottom),
      maxX: Math.max(left, right),
      maxY: Math.max(top, bottom),
    });
    return candidates.map((c) => c.renderNode);
  }

  /**
   * Get 特定节点 ID 的渲染节点。
   */
  get(nodeId: string): RenderNode | undefined {
    return this.items.get(nodeId)?.renderNode;
  }
}

function isPointHittableRenderNode(renderNode: RenderNode): boolean {
  const node = renderNode.node;
  if (resolveNodeOpacity(node.opacity) <= 0) return false;

  if (node.type === 'frame' || node.type === 'group' || node.type === 'rectangle') {
    const hasExplicitAppearance =
      (Array.isArray(node.fill) && node.fill.length > 0) ||
      !!node.stroke ||
      (Array.isArray(node.effects) && node.effects.length > 0);
    if (!hasExplicitAppearance) {
      if (node.type === 'frame' || node.type === 'group') {
        return false;
      }
      return true;
    }
    return (
      hasVisibleFill(node.fill) || hasVisibleStroke(node.stroke) || hasVisibleEffects(node.effects)
    );
  }

  return true;
}

function hasVisibleFill(fill: PenFill[] | undefined): boolean {
  if (!Array.isArray(fill) || fill.length === 0) return false;
  return fill.some((entry) => {
    const opacity = resolveNodeOpacity(entry.opacity);
    if (opacity <= 0) return false;

    switch (entry.type) {
      case 'solid':
        return hasVisibleColor(entry.color);
      case 'linear_gradient':
      case 'radial_gradient':
        return entry.stops.some((stop) => hasVisibleColor(stop.color));
      case 'image':
        return !!entry.url;
      default:
        return false;
    }
  });
}

function hasVisibleStroke(stroke: PenStroke | undefined): boolean {
  if (!stroke) return false;
  const thickness = resolveStrokeThickness(stroke);
  if (thickness <= 0) return false;
  return hasVisibleFill(stroke.fill);
}

function hasVisibleEffects(effects: PenEffect[] | undefined): boolean {
  if (!Array.isArray(effects) || effects.length === 0) return false;
  return effects.some((effect) => {
    if (effect.type === 'shadow') {
      return (
        hasVisibleColor(effect.color) &&
        (effect.blur > 0 || effect.spread !== 0 || effect.offsetX !== 0 || effect.offsetY !== 0)
      );
    }

    return effect.radius > 0;
  });
}

function hasVisibleColor(color: string | undefined): boolean {
  if (!color) return false;
  return resolveColorAlpha(color) > 0;
}

function resolveColorAlpha(color: string): number {
  const normalized = color.trim().toLowerCase();
  if (!normalized) return 0;
  if (normalized === 'transparent') return 0;

  const hex = normalized.match(/^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i)?.[1];
  if (hex) {
    if (hex.length === 4) return parseInt(hex[3] + hex[3], 16) / 255;
    if (hex.length === 8) return parseInt(hex.slice(6, 8), 16) / 255;
    return 1;
  }

  const rgbaMatch = normalized.match(/^rgba?\(([^)]+)\)$/);
  if (rgbaMatch) {
    const parts = rgbaMatch[1].split(',').map((part) => part.trim());
    if (parts.length >= 4) {
      const alpha = Number.parseFloat(parts[3]);
      return Number.isFinite(alpha) ? alpha : 1;
    }
    return 1;
  }

  return 1;
}

function resolveNodeOpacity(opacity: PenNode['opacity'] | PenFill['opacity']): number {
  if (typeof opacity === 'number') return opacity;
  if (typeof opacity === 'string') {
    const parsed = Number.parseFloat(opacity);
    if (Number.isFinite(parsed)) return parsed;
  }
  return 1;
}

function resolveStrokeThickness(stroke: PenStroke): number {
  if (typeof stroke.thickness === 'number') return stroke.thickness;
  if (Array.isArray(stroke.thickness)) {
    return Math.max(...stroke.thickness);
  }
  return 0;
}
