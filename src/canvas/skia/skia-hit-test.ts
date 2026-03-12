import RBush from 'rbush'
import type { RenderNode } from './skia-renderer'

interface RTreeItem {
  minX: number
  minY: number
  maxX: number
  maxY: number
  nodeId: string
  renderNode: RenderNode
  /** Position in the render array — higher = rendered later = visually on top */
  zIndex: number
}

/**
 * Spatial index for fast hit testing using R-tree.
 * Nodes are indexed with their render order so hit results
 * are sorted topmost-first (children before parents).
 */
export class SpatialIndex {
  private tree = new RBush<RTreeItem>()
  private items = new Map<string, RTreeItem>()

  /**
   * Rebuild the entire index from a list of render nodes.
   */
  rebuild(nodes: RenderNode[]) {
    this.tree.clear()
    this.items.clear()

    const items: RTreeItem[] = []
    for (let i = 0; i < nodes.length; i++) {
      const rn = nodes[i]
      if (('visible' in rn.node ? rn.node.visible : undefined) === false) continue
      if (('locked' in rn.node ? rn.node.locked : undefined) === true) continue

      const item: RTreeItem = {
        minX: rn.absX,
        minY: rn.absY,
        maxX: rn.absX + rn.absW,
        maxY: rn.absY + rn.absH,
        nodeId: rn.node.id,
        renderNode: rn,
        zIndex: i,
      }
      items.push(item)
      this.items.set(rn.node.id, item)
    }

    this.tree.load(items)
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
    })

    // Sort by zIndex descending — children (rendered later) come first
    candidates.sort((a, b) => b.zIndex - a.zIndex)
    return candidates.map((c) => c.renderNode)
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
    })
    return candidates.map((c) => c.renderNode)
  }

  /**
   * Get the render node for a specific node ID.
   */
  get(nodeId: string): RenderNode | undefined {
    return this.items.get(nodeId)?.renderNode
  }
}
