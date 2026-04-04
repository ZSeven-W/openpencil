import type { PenNode } from '@/types/pen';
import { extractStreamingNodes, extractJsonFromResponse } from './design-parser';
import { insertStreamingNode, expandRootFrameHeight } from './design-canvas-ops';
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
  private rootNodeId: string | null = null;
  private readonly animated: boolean;
  private finished = false;

  constructor(private options: RendererOptions) {
    this.animated = options.animated ?? true;
  }

  feedText(rawResponse: string): number {
    const { results, newOffset } = extractStreamingNodes(rawResponse, this.streamOffset);
    if (results.length === 0) return 0;
    this.streamOffset = newOffset;

    if (this.animated) startNewAnimationBatch();

    let inserted = 0;
    for (const { node, parentId } of results) {
      if (this.options.idPrefix) {
        ensureIdPrefix(node, this.options.idPrefix);
      }

      if (this.options.agentColor && this.options.agentName) {
        this.collectIdsRecursive(node);
        addAgentIndicatorRecursive(node, this.options.agentColor, this.options.agentName);
      }

      if (this.animated) {
        markNodesForAnimation([node]);
      }

      if (parentId !== null) {
        const target = this.options.idPrefix
          ? ensurePrefixStr(parentId, this.options.idPrefix)
          : parentId;
        insertStreamingNode(node, target);
      } else {
        const target = this.options.parentFrameId ?? null;
        insertStreamingNode(node, target);

        if (this.options.agentColor && this.options.agentName) {
          addAgentFrame(node.id, this.options.agentColor, this.options.agentName);
        }

        if (!this.rootNodeId) this.rootNodeId = node.id;
      }

      this.appliedIds.add(node.id);
      this.insertedNodes.push(node as PenNode);
      inserted++;
    }

    expandRootFrameHeight(this.options.parentFrameId);

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

export function ensureIdPrefix(
  node: { id: string; children?: unknown[] },
  prefix: string,
): void {
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
