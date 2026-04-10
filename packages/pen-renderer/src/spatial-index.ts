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
  /** Position in the render array — higher = rendered later = visually on top */
  zIndex: number;
}

/**
 * Spatial index for fast hit testing using R-tree.
 * Nodes are indexed with their render order so hit results
 * are sorted topmost-first (children before parents).
 */
export class SpatialIndex {
  private tree = new RBush<RTreeItem>();
  private items = new Map<string, RTreeItem>();

  /**
   * Rebuild the entire index from a list of render nodes.
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
   * Find all nodes that contain the given scene point.
   * Returns nodes sorted by z-order: topmost (highest zIndex) first.
   */
  hitTest(sceneX: number, sceneY: number): RenderNode[] {
    const candidates = this.tree.search({
      minX: sceneX,
      minY: sceneY,
      maxX: sceneX,
      maxY: sceneY,
    });

    // Sort by zIndex descending — children (rendered later) come first
    candidates.sort((a, b) => b.zIndex - a.zIndex);
    return candidates.map((c) => c.renderNode).filter((rn) => isPointHittableRenderNode(rn));
  }

  /**
   * Find all nodes that intersect with a rectangle (for marquee selection).
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
   * Get the render node for a specific node ID.
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
