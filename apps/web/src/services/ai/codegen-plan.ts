import type {
  ChunkContract,
  CodeExecutionPlan,
  CodePlanFromAI,
  ExecutableChunk,
  PenNode,
  PlannedChunk,
} from '@zseven-w/pen-types';

export function hydratePlan(plan: CodePlanFromAI, nodes: PenNode[]): CodeExecutionPlan {
  const nodeMap = new Map<string, PenNode>();
  function indexNodes(list: PenNode[]) {
    for (const node of list) {
      nodeMap.set(node.id, node);
      const children = (node as { children?: PenNode[] }).children;
      if (children) indexNodes(children);
    }
  }
  indexNodes(nodes);

  const orders = computeExecutionOrder(plan.chunks);
  const chunks: ExecutableChunk[] = plan.chunks
    .map((chunk) => {
      const resolved = chunk.nodeIds
        .map((id) => nodeMap.get(id))
        .filter((node): node is PenNode => node !== undefined);
      if (resolved.length === 0) return null;
      return {
        ...chunk,
        nodes: resolved,
        order: orders.get(chunk.id) ?? 0,
        depContracts: [] as ChunkContract[],
      };
    })
    .filter((chunk): chunk is ExecutableChunk => chunk !== null);

  return {
    chunks,
    sharedStyles: plan.sharedStyles,
    rootLayout: plan.rootLayout,
  };
}

export function computeExecutionOrder(chunks: PlannedChunk[]): Map<string, number> {
  const orders = new Map<string, number>();

  function resolve(id: string, visited: Set<string>): number {
    if (orders.has(id)) return orders.get(id)!;
    if (visited.has(id)) return 0;
    visited.add(id);

    const chunk = chunks.find((item) => item.id === id);
    if (!chunk || chunk.dependencies.length === 0) {
      orders.set(id, 0);
      return 0;
    }

    const maxDep = Math.max(...chunk.dependencies.map((depId) => resolve(depId, visited)));
    const order = maxDep + 1;
    orders.set(id, order);
    return order;
  }

  for (const chunk of chunks) {
    resolve(chunk.id, new Set());
  }

  return orders;
}
