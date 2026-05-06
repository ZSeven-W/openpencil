import { SpatialIndex } from '@zseven-w/pen-renderer';
import type { RenderNode } from '@zseven-w/pen-renderer';
import type { PenNode } from '@zseven-w/pen-types';

/**
 * Engine 级空间索引
 * 包装器。 Wraps 笔渲染器的 SpatialIndex
 提供引擎级别的 API。
 */
export class EngineSpatialIndex {
  private inner = new SpatialIndex();

  /** Rebuild 渲染节点列表中的索引。 */
  rebuild(nodes: RenderNode[]): void {
    this.inner.rebuild(nodes);
  }

  /** Hit 测试：查找包含给定场景点的所有节点，从最顶层开始。 */
  hitTest(sceneX: number, sceneY: number): RenderNode[] {
    return this.inner.hitTest(sceneX, sceneY);
  }

  /** Search rect：查找与矩形（x，y，宽度，高度）相交的所有节点。 */
  searchRect(x: number, y: number, w: number, h: number): RenderNode[] {
    return this.inner.searchRect(x, y, x + w, y + h);
  }

  /** Get 特定节点 ID 的渲染节点。 */
  get(nodeId: string): RenderNode | undefined {
    return this.inner.get(nodeId);
  }

  /** Find 一个 PenNode 按点。 Returns 最上面的命中，或 null。 */
  hitTestNode(sceneX: number, sceneY: number): PenNode | null {
    const hits = this.inner.hitTest(sceneX, sceneY);
    return hits.length > 0 ? hits[0].node : null;
  }

  /** Find 所有 PenNodes 在一个矩形中。 */
  searchRectNodes(x: number, y: number, w: number, h: number): PenNode[] {
    return this.searchRect(x, y, w, h).map((rn) => rn.node);
  }
}
