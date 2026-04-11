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
  /** Nodes waiting for their parent to be inserted first. */
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
      // Even with no new nodes, retry pending ones — earlier nodes
      // in this batch might have made their parents available.
      return this.flushPending();
    }
    this.streamOffset = newOffset;

    // Add new nodes to pending queue
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

    // Flush: insert nodes whose parent is already applied (or root).
    // Retry loop handles dependency chains within the same batch.
    return this.flushPending();
  }

  /** Try to insert pending nodes whose parents are available. */
  private flushPending(): number {
    if (this.pendingNodes.length === 0) return 0;

    if (this.animated) startNewAnimationBatch();

    let totalInserted = 0;
    let progress = true;
    while (progress) {
      progress = false;
      for (let i = this.pendingNodes.length - 1; i >= 0; i--) {
        const { node, parentId } = this.pendingNodes[i];
        // Can insert if: root node (no parent), or parent already on canvas
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

  /** Insert a single node into the canvas with indicators and animation. */
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

      // insertStreamingNode may remap root frame ID (e.g. replaces the
      // default empty frame with DEFAULT_FRAME_ID). Register the badge
      // under the actual ID the canvas uses, not the original node.id.
      const effectiveId =
        getGenerationRootFrameId() !== node.id ? getGenerationRootFrameId() : node.id;

      if (this.options.agentColor && this.options.agentName) {
        addAgentFrame(effectiveId, this.options.agentColor, this.options.agentName);
      }

      if (!this.rootNodeId) this.rootNodeId = effectiveId;

      // Track the effective (possibly remapped) ID so finish() can clean up
      // the frame badge. node.id may differ from effectiveId after remapping.
      this.appliedIds.add(effectiveId);
    }

    // Always track the original node.id — needed for pending queue parent
    // dependency resolution (children reference parent by original id).
    this.appliedIds.add(node.id);
    this.insertedNodes.push(node as PenNode);
  }

  /** Force-insert any remaining pending nodes whose parents never arrived.
   *  Called at stream end (done event) to avoid losing orphaned nodes. */
  forceFlushPending(): number {
    if (this.pendingNodes.length === 0) return 0;
    if (this.animated) startNewAnimationBatch();
    let inserted = 0;
    for (const { node, parentId } of this.pendingNodes) {
      // Try the declared parent, fall back to root frame
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
