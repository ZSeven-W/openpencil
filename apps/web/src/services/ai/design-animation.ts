import type { PenNode } from '@/types/pen';

// ---------------------------------------------------------------------------
// Shared 协调集 — 由 canvas-sync 检查以触发淡入
// ---------------------------------------------------------------------------

/** 创建 Fabric 对象时应淡入的节点的 IDs 。 */
export const pendingAnimationNodes = new Set<string>();

// ---------------------------------------------------------------------------
// Sequential 显示队列 — 所有节点都遵循单个有序队列。
// Each 节点：边框在预定时间出现，内容淡入
// BORDER_LEAD 毫秒后。 No 两个节点同时显示。
// ---------------------------------------------------------------------------

/** Interval 每个节点的边界显示之间（毫秒）。 */
const REVEAL_INTERVAL = 300;
/** Delay 边框出现和内容淡入之间的时间（毫秒）。 */
const BORDER_LEAD = 400;

/** Maps nodeId → 边界应该出现的绝对时间戳。 */
const nodeRevealTime = new Map<string, number>();

/** The 下一个可用的显示时间戳（随着节点排队而前进）。 */
let nextRevealAt = 0;

/**
 * Mark 树中的所有节点
 * IDs 用于淡入动画。 Assigns 通过 BFS 顺序（父→子）顺序显示时间戳。 New
 * 节点始终调度为 AFTER 先前排队的节点，即使它们到达稍后的流块中。
 *
 */
export function markNodesForAnimation(nodes: PenNode[]): void {
  // Ensure 新节点启动时间不早于现在
  const now = Date.now();
  if (nextRevealAt < now) nextRevealAt = now;

  // BFS 确保父级在子级之前，逐级确保
  const queue: PenNode[] = [...nodes];
  while (queue.length > 0) {
    const node = queue.shift()!;
    pendingAnimationNodes.add(node.id);
    nodeRevealTime.set(node.id, nextRevealAt);
    nextRevealAt += REVEAL_INTERVAL;
    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) {
        queue.push(child);
      }
    }
  }
}

/**
 * Start 一批新的动画。 No-op — 保持队列连续性。
 */
export function startNewAnimationBatch(): void {
  // 故意不执行队列连续性
}

/**
 * Get 该节点的内容开始
 * 淡入之前的总延迟（毫秒）。= 边界显示之前的时间 + BORDER_LEAD
 */
export function getNextStaggerDelay(nodeId?: string): number {
  if (!nodeId) return 0;
  const revealAt = nodeRevealTime.get(nodeId);
  if (revealAt === undefined) return 0;
  const now = Date.now();
  const waitForBorder = Math.max(0, revealAt - now);
  return waitForBorder + BORDER_LEAD;
}

/**
 * Check 如果节点的边
 * 框应该可见。 Returns 当当前时间已达到节点的预定显示时间时为 true。
 */
export function isNodeBorderReady(nodeId: string): boolean {
  const revealAt = nodeRevealTime.get(nodeId);
  if (revealAt === undefined) return false;
  return Date.now() >= revealAt;
}

/** Get 节点的计划显示时间戳（如果未排队则未定义）。 */
export function getNodeRevealTime(nodeId: string): number | undefined {
  return nodeRevealTime.get(nodeId);
}

/** Reset 所有动画状态。 Call 在一代开始时出现过一次。 */
export function resetAnimationState(): void {
  pendingAnimationNodes.clear();
  nodeRevealTime.clear();
  nextRevealAt = 0;
}
