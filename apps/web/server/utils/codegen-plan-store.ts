import type {
  CodePlanFromAI,
  PlannedChunk,
  ChunkResult,
  ChunkContract,
  ChunkStatus,
  ContractValidationResult,
  Framework,
  ExecutableChunkPayload,
  ResolvedDepContract,
  NodeSnapshot,
  CodeGenProgress,
  PenNode,
} from '@zseven-w/pen-types';
import { randomUUID } from 'node:crypto';
import { validateContract } from '@zseven-w/pen-mcp';

// --- Internal state ---

export interface PlanState {
  plan: CodePlanFromAI;
  nodes: Map<string, PenNode>;
  order: Map<string, number>;
  results: Map<string, ChunkResult>;
  statuses: Map<string, ChunkStatus>;
  lastActivity: number;
}

const plans = new Map<string, PlanState>();

const TTL_MS = 30 * 60 * 1000; // 30 minutes

function cleanExpired(): void {
  const now = Date.now();
  for (const [id, state] of plans) {
    if (now - state.lastActivity > TTL_MS) plans.delete(id);
  }
}

function touch(planId: string): void {
  const state = plans.get(planId);
  if (state) state.lastActivity = Date.now();
}

// --- Validation ---

function validatePlan(plan: CodePlanFromAI, nodeIndex: Map<string, PenNode>): string[] {
  const errors: string[] = [];
  const ids = new Set<string>();

  // Duplicate chunkId
  for (const chunk of plan.chunks) {
    if (ids.has(chunk.id)) errors.push(`Duplicate chunkId: ${chunk.id}`);
    ids.add(chunk.id);
  }
  if (errors.length > 0) return errors;

  for (const chunk of plan.chunks) {
    if (!chunk.nodeIds || chunk.nodeIds.length === 0) {
      errors.push(`Chunk ${chunk.id} has no nodeIds`);
    }
    for (const depId of chunk.dependencies) {
      if (!ids.has(depId)) {
        errors.push(`Chunk ${chunk.id} depends on unknown chunk ${depId}`);
      }
    }
    for (const nodeId of chunk.nodeIds) {
      if (!nodeIndex.has(nodeId)) {
        errors.push(`Chunk ${chunk.id}: node ${nodeId} not found in document`);
      }
    }
  }

  // Circular dependency detection via topological sort
  if (errors.length === 0) {
    const inDegree = new Map<string, number>();
    const adj = new Map<string, string[]>();
    for (const chunk of plan.chunks) {
      inDegree.set(chunk.id, chunk.dependencies.length);
      for (const dep of chunk.dependencies) {
        const list = adj.get(dep) ?? [];
        list.push(chunk.id);
        adj.set(dep, list);
      }
    }
    const queue = plan.chunks.filter((c) => c.dependencies.length === 0).map((c) => c.id);
    let processed = 0;
    while (queue.length > 0) {
      const id = queue.shift()!;
      processed++;
      for (const next of adj.get(id) ?? []) {
        const deg = (inDegree.get(next) ?? 1) - 1;
        inDegree.set(next, deg);
        if (deg === 0) queue.push(next);
      }
    }
    if (processed < plan.chunks.length) {
      const cycleIds = plan.chunks.filter((c) => (inDegree.get(c.id) ?? 0) > 0).map((c) => c.id);
      errors.push(`Circular dependency: ${cycleIds.join(' → ')}`);
    }
  }

  return errors;
}

function detectWarnings(plan: CodePlanFromAI): string[] {
  const warnings: string[] = [];
  const nodeToChunks = new Map<string, string[]>();
  for (const chunk of plan.chunks) {
    for (const nodeId of chunk.nodeIds) {
      const list = nodeToChunks.get(nodeId) ?? [];
      list.push(chunk.id);
      nodeToChunks.set(nodeId, list);
    }
  }
  for (const [nodeId, chunkIds] of nodeToChunks) {
    if (chunkIds.length > 1) {
      warnings.push(`Node ${nodeId} claimed by chunks: ${chunkIds.join(', ')}`);
    }
  }
  return warnings;
}

// --- Node indexing ---

function indexNodes(nodes: PenNode[]): Map<string, PenNode> {
  const map = new Map<string, PenNode>();
  function walk(list: PenNode[]) {
    for (const n of list) {
      map.set(n.id, n);
      const children = (n as { children?: PenNode[] }).children;
      if (children) walk(children);
    }
  }
  walk(nodes);
  return map;
}

// --- Topological sort ---

function topoSort(chunks: PlannedChunk[]): PlannedChunk[] {
  const inDegree = new Map<string, number>();
  const adj = new Map<string, string[]>();
  const byId = new Map<string, PlannedChunk>();

  for (const c of chunks) {
    byId.set(c.id, c);
    inDegree.set(c.id, c.dependencies.length);
    for (const dep of c.dependencies) {
      const list = adj.get(dep) ?? [];
      list.push(c.id);
      adj.set(dep, list);
    }
  }

  const result: PlannedChunk[] = [];
  const queue = chunks.filter((c) => c.dependencies.length === 0).map((c) => c.id);

  while (queue.length > 0) {
    const id = queue.shift()!;
    result.push(byId.get(id)!);
    for (const next of adj.get(id) ?? []) {
      const deg = (inDegree.get(next) ?? 1) - 1;
      inDegree.set(next, deg);
      if (deg === 0) queue.push(next);
    }
  }
  return result;
}

// --- Chunk hydration ---

function hydrateChunk(
  chunk: PlannedChunk,
  order: number,
  nodeIndex: Map<string, PenNode>,
  depContracts: ResolvedDepContract[],
  fullHydrate: boolean,
): ExecutableChunkPayload {
  const nodes: NodeSnapshot[] = chunk.nodeIds
    .map((id) => nodeIndex.get(id))
    .filter((n): n is PenNode => n !== undefined)
    .map((n) =>
      fullHydrate
        ? (n as unknown as NodeSnapshot)
        : ({ ...n, children: '...' } as unknown as NodeSnapshot),
    );

  return {
    ...chunk,
    nodes,
    order,
    depContracts,
  };
}

// --- Public API ---

export interface CreatePlanResult {
  planId: string;
  executionPlan: ExecutableChunkPayload[];
  warnings: string[];
}

export function createPlan(plan: CodePlanFromAI, allNodes: PenNode[]): CreatePlanResult {
  cleanExpired();

  const nodeIndex = indexNodes(allNodes);
  const errors = validatePlan(plan, nodeIndex);
  if (errors.length > 0) {
    throw new Error(errors.join('; '));
  }

  const warnings = detectWarnings(plan);
  const sorted = topoSort(plan.chunks);
  const orderMap = new Map<string, number>();
  sorted.forEach((c, i) => orderMap.set(c.id, i));

  const planId = randomUUID();
  const state: PlanState = {
    plan,
    nodes: nodeIndex,
    order: orderMap,
    results: new Map(),
    statuses: new Map(),
    lastActivity: Date.now(),
  };
  for (const c of plan.chunks) {
    state.statuses.set(c.id, 'pending');
  }
  plans.set(planId, state);

  const executionPlan = sorted.map((chunk, i) => hydrateChunk(chunk, i, nodeIndex, [], i === 0));

  return { planId, executionPlan, warnings };
}

export function getPlan(planId: string): PlanState | undefined {
  return plans.get(planId);
}

export interface SubmitChunkResult {
  validation: ContractValidationResult;
  progress: CodeGenProgress[];
  nextChunk?: ExecutableChunkPayload;
}

export function submitChunkResult(
  planId: string,
  result: ChunkResult,
  statusOverride?: 'failed' | 'skipped',
): SubmitChunkResult {
  const state = plans.get(planId);
  if (!state) throw new Error(`Plan ${planId} not found`);
  touch(planId);

  const validation = validateContract(result);

  let status: ChunkStatus;
  if (statusOverride === 'failed' || statusOverride === 'skipped') {
    status = statusOverride;
  } else if (validation.valid) {
    status = 'done';
  } else {
    status = 'degraded';
  }

  state.results.set(result.chunkId, result);
  state.statuses.set(result.chunkId, status);

  const progress: CodeGenProgress[] = state.plan.chunks.map((c) => ({
    step: 'chunk' as const,
    chunkId: c.id,
    name: c.name,
    status: state.statuses.get(c.id) ?? 'pending',
    result: state.results.get(c.id),
  }));

  let nextChunk: ExecutableChunkPayload | undefined;
  const sorted = topoSort(state.plan.chunks);
  for (const chunk of sorted) {
    const chunkStatus = state.statuses.get(chunk.id);
    if (chunkStatus !== 'pending') continue;

    const depsReady = chunk.dependencies.every((depId) => {
      const depStatus = state.statuses.get(depId);
      return (
        depStatus === 'done' ||
        depStatus === 'degraded' ||
        depStatus === 'failed' ||
        depStatus === 'skipped'
      );
    });

    if (depsReady) {
      const depContracts: ResolvedDepContract[] = chunk.dependencies.map((depId) => {
        const depStatus = state.statuses.get(depId);
        if (depStatus === 'failed' || depStatus === 'skipped') return null;
        return state.results.get(depId)?.contract ?? null;
      });
      nextChunk = hydrateChunk(
        chunk,
        state.order.get(chunk.id) ?? 0,
        state.nodes,
        depContracts,
        true,
      );
      break;
    }
  }

  return { validation, progress, nextChunk };
}

export interface AssemblePlanResult {
  chunks: ChunkResult[];
  contracts: ChunkContract[];
  dependencyGraph: Record<string, string[]>;
  degraded: boolean;
}

export function assemblePlan(planId: string, _framework: Framework): AssemblePlanResult {
  const state = plans.get(planId);
  if (!state) throw new Error(`Plan ${planId} not found`);

  const sorted = topoSort(state.plan.chunks);
  const chunks: ChunkResult[] = [];
  const contracts: ChunkContract[] = [];
  const dependencyGraph: Record<string, string[]> = {};
  let degraded = false;

  for (const chunk of sorted) {
    const result = state.results.get(chunk.id);
    if (result) {
      chunks.push(result);
      contracts.push(result.contract);
    }
    dependencyGraph[chunk.id] = chunk.dependencies;
    const status = state.statuses.get(chunk.id);
    if (status !== 'done') degraded = true;
  }

  // Terminal operation — clear cache
  plans.delete(planId);

  return { chunks, contracts, dependencyGraph, degraded };
}

export function cleanPlan(planId: string): { ok: boolean; deleted: boolean } {
  const existed = plans.has(planId);
  if (existed) plans.delete(planId);
  return { ok: true, deleted: existed };
}
