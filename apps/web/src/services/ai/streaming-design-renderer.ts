import type { PenNode } from '@/types/pen';
import { extractStreamingNodes, extractJsonFromResponse } from './design-parser';
import {
  insertStreamingNode,
  expandRootFrameHeight,
  getGenerationRootFrameId,
} from './design-canvas-ops';
import { startNewAnimationBatch, markNodesForAnimation } from './design-animation';
import {
  addAgentIndicatorRecursive,
  removeAgentIndicator,
  getActiveAgentFrames,
  addAgentFrame,
} from '@/canvas/agent-indicator';

export interface RendererOptions {
  agentColor?: string;
  agentName?: string;
  idPrefix?: string;
  parentFrameId?: string;
  animated?: boolean;
}

export class StreamingDesignRenderer {
  private streamOffset = 0;
  private appliedIds = new Set<string>();
  private indicatedIds = new Set<string>();
  private insertedNodes: PenNode[] = [];
  /** Nodes 等待先插入其父级。 */
  private pendingNodes: Array<{ node: any; parentId: string | null }> = [];
  private rootNodeId: string | null = null;
  private readonly animated: boolean;
  private finished = false;

  constructor(private options: RendererOptions) {
    this.animated = options.animated ?? true;
  }

  feedText(rawResponse: string): number {
    const { results, newOffset } = extractStreamingNodes(rawResponse, this.streamOffset);
    if (results.length === 0) {
      // Even 没有新节点，请重试挂起的节点 - 该批次中较早的节点可能已使其父节点可用。
      return this.flushPending();
    }
    this.streamOffset = newOffset;

    // Add 将新节点加入待处理队列
    for (const { node, parentId } of results) {
      if (this.options.idPrefix) {
        ensureIdPrefix(node, this.options.idPrefix);
      }
      const resolvedParent =
        parentId !== null && this.options.idPrefix
          ? ensurePrefixStr(parentId, this.options.idPrefix)
          : parentId;
      this.pendingNodes.push({ node, parentId: resolvedParent });
    }

    // Flush：插入已应用父级（或根）的节点。 Retry 循环处理同一批次内的依赖链。
    return this.flushPending();
  }

  /** Try 插入父节点可用的挂起节点。 */
  private flushPending(): number {
    if (this.pendingNodes.length === 0) return 0;

    if (this.animated) startNewAnimationBatch();

    let totalInserted = 0;
    let progress = true;
    while (progress) {
      progress = false;
      for (let i = this.pendingNodes.length - 1; i >= 0; i--) {
        const { node, parentId } = this.pendingNodes[i];
        // Can 插入如果：根节点（无父节点），或父节点已在画布上
        if (parentId === null || parentId === undefined || this.appliedIds.has(parentId)) {
          this.insertNode(node, parentId);
          this.pendingNodes.splice(i, 1);
          totalInserted++;
          progress = true;
        }
      }
    }

    if (totalInserted > 0) {
      expandRootFrameHeight(this.options.parentFrameId);
    }

    return totalInserted;
  }

  /** Insert 将单个节点放入画布中，并带有指示器和动画。 */
  private insertNode(node: any, parentId: string | null): void {
    if (this.options.agentColor && this.options.agentName) {
      this.collectIdsRecursive(node);
      addAgentIndicatorRecursive(node, this.options.agentColor, this.options.agentName);
    }

    if (this.animated) {
      markNodesForAnimation([node]);
    }

    if (parentId !== null) {
      insertStreamingNode(node, parentId);
    } else {
      const target = this.options.parentFrameId ?? null;
      insertStreamingNode(node, target);

      // insertStreamingNode 可以重新映射根帧 ID （例如，用 DEFAULT_FRAME_ID
      // 替换默认的空帧）。 Register 画布使用的实际 ID 下的徽章，而不是原始的 node.id。
      const effectiveId =
        getGenerationRootFrameId() !== node.id ? getGenerationRootFrameId() : node.id;

      if (this.options.agentColor && this.options.agentName) {
        addAgentFrame(effectiveId, this.options.agentColor, this.options.agentName);
      }

      if (!this.rootNodeId) this.rootNodeId = effectiveId;

      // Track 有效（可能重新映射）ID，因此 finish() 可以清理帧标记。重新映射后，node.id 可能与
      // effectiveId 不同。
      this.appliedIds.add(effectiveId);
    }

    // Always 跟踪原始 node.id — 待处理队列父依赖关系解析所需（子级通过原始 id 引用父级）。
    this.appliedIds.add(node.id);
    this.insertedNodes.push(node as PenNode);
  }

  /** Force - 插入其父节点从未到达的任何剩余待处理节点。 Called 在流结束（完成事件）以避免丢失孤立节点。
   *  */
  forceFlushPending(): number {
    if (this.pendingNodes.length === 0) return 0;
    if (this.animated) startNewAnimationBatch();
    let inserted = 0;
    for (const { node, parentId } of this.pendingNodes) {
      // Try 声明的父级，回退到根框架
      const target = parentId ?? this.rootNodeId ?? this.options.parentFrameId ?? null;
      this.insertNode(node, target);
      inserted++;
    }
    this.pendingNodes.length = 0;
    if (inserted > 0) expandRootFrameHeight(this.options.parentFrameId);
    return inserted;
  }

  flushRemaining(rawResponse: string): number {
    if (this.appliedIds.size > 0) return 0;

    const fallbackNodes = extractJsonFromResponse(rawResponse);
    if (!fallbackNodes || fallbackNodes.length === 0) return 0;

    if (this.animated) startNewAnimationBatch();

    let inserted = 0;
    for (const node of fallbackNodes) {
      if (this.options.idPrefix) ensureIdPrefix(node, this.options.idPrefix);
      if (this.options.agentColor && this.options.agentName) {
        this.collectIdsRecursive(node);
        addAgentIndicatorRecursive(node, this.options.agentColor, this.options.agentName);
      }
      if (this.animated) markNodesForAnimation([node]);

      const target = this.rootNodeId ?? this.options.parentFrameId ?? null;
      insertStreamingNode(node, target);
      if (!this.rootNodeId) this.rootNodeId = node.id;

      this.appliedIds.add(node.id);
      this.insertedNodes.push(node as PenNode);
      inserted++;
    }

    expandRootFrameHeight(this.options.parentFrameId);

    return inserted;
  }

  finish(delayMs = 0): void {
    if (this.finished) return;
    this.finished = true;

    const doCleanup = () => {
      for (const id of this.indicatedIds) {
        removeAgentIndicator(id);
      }

      const frames = getActiveAgentFrames();
      setTimeout(() => {
        for (const id of this.appliedIds) {
          frames.delete(id);
        }
      }, 2000);
    };

    if (delayMs > 0) {
      setTimeout(doCleanup, delayMs);
    } else {
      doCleanup();
    }
  }

  setIdentity(color: string, name: string): void {
    this.options.agentColor = color;
    this.options.agentName = name;
  }

  getAppliedIds(): ReadonlySet<string> {
    return this.appliedIds;
  }

  getInsertedNodes(): PenNode[] {
    return this.insertedNodes;
  }

  getRootId(): string | null {
    return this.rootNodeId;
  }

  private collectIdsRecursive(node: { id: string; children?: unknown[] }): void {
    this.indicatedIds.add(node.id);
    if (Array.isArray(node.children)) {
      for (const child of node.children) {
        this.collectIdsRecursive(child as { id: string; children?: unknown[] });
      }
    }
  }
}

export function ensureIdPrefix(node: { id: string; children?: unknown[] }, prefix: string): void {
  if (!node.id.startsWith(`${prefix}-`)) {
    node.id = `${prefix}-${node.id}`;
  }
  if (Array.isArray(node.children)) {
    for (const child of node.children) {
      ensureIdPrefix(child as { id: string; children?: unknown[] }, prefix);
    }
  }
}

export function ensurePrefixStr(id: string, prefix: string): string {
  if (id.startsWith(`${prefix}-`)) return id;
  return `${prefix}-${id}`;
}
