import { SpatialIndex } from '@zseven-w/pen-renderer';
import type { RenderNode } from '@zseven-w/pen-renderer';
import type { PenNode } from '@zseven-w/pen-types';

/**
 * Engine-level spatial index wrapper.
 * Wraps pen-renderer's SpatialIndex to provide engine-level API.
 */
export class EngineSpatialIndex {
  private inner = new SpatialIndex();

  /** Rebuild the index from a list of render nodes. */
  rebuild(nodes: RenderNode[]): void {
    this.inner.rebuild(nodes);
  }

  /** Hit test: find all nodes containing the given scene point, topmost first. */
  hitTest(sceneX: number, sceneY: number): RenderNode[] {
    return this.inner.hitTest(sceneX, sceneY);
  }

  /** Search rect: find all nodes intersecting with a rectangle (x, y, width, height). */
  searchRect(x: number, y: number, w: number, h: number): RenderNode[] {
    return this.inner.searchRect(x, y, x + w, y + h);
  }

  /** Get the render node for a specific node ID. */
  get(nodeId: string): RenderNode | undefined {
    return this.inner.get(nodeId);
  }

  /** Find a PenNode by point. Returns the topmost hit, or null. */
  hitTestNode(sceneX: number, sceneY: number): PenNode | null {
    const hits = this.inner.hitTest(sceneX, sceneY);
    return hits.length > 0 ? hits[0].node : null;
  }

  /** Find all PenNodes in a rect. */
  searchRectNodes(x: number, y: number, w: number, h: number): PenNode[] {
    return this.searchRect(x, y, w, h).map((rn) => rn.node);
  }
}
