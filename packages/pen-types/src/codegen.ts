import type { PenNode } from './pen.js';

// === Canonical 框架类型 ===

export type Framework =
  | 'react'
  | 'vue'
  | 'svelte'
  | 'html'
  | 'flutter'
  | 'swiftui'
  | 'compose'
  | 'react-native';

export const FRAMEWORKS: Framework[] = [
  'react',
  'vue',
  'svelte',
  'html',
  'flutter',
  'swiftui',
  'compose',
  'react-native',
];

// === Step 1 输出：AI 规划器返回此（无节点数据，最小标记）===

export interface PlannedChunk {
  id: string;
  name: string;
  nodeIds: string[];
  role: string;
  suggestedComponentName: string;
  dependencies: string[];
  exposedSlots?: string[];
}

export interface CodePlanFromAI {
  chunks: PlannedChunk[];
  sharedStyles: { name: string; description: string }[];
  rootLayout: { direction: string; gap: number; responsive: boolean };
}

// === Runtime：与节点数据水合+执行顺序===

export interface ExecutableChunk extends PlannedChunk {
  nodes: PenNode[];
  order: number;
  depContracts: ChunkContract[];
}

export interface CodeExecutionPlan {
  chunks: ExecutableChunk[];
  sharedStyles: { name: string; description: string }[];
  rootLayout: { direction: string; gap: number; responsive: boolean };
}

// === Chunk 合约：每个块的结构化元数据输出 ===

export interface ChunkContract {
  chunkId: string;
  componentName: string;
  exportedProps: PropDef[];
  slots: SlotDef[];
  cssClasses: string[];
  cssVariables: string[];
  imports: ImportDef[];
}

export interface PropDef {
  name: string;
  type: string;
  required: boolean;
}

export interface SlotDef {
  name: string;
  description: string;
}

export interface ImportDef {
  source: string;
  specifiers: string[];
}

// === Chunk 生成输出 ===

export interface ChunkResult {
  chunkId: string;
  code: string;
  contract: ChunkContract;
}

// === Progress 事件 ===

export type ChunkStatus = 'pending' | 'running' | 'done' | 'degraded' | 'failed' | 'skipped';

export type CodeGenProgress =
  | {
      step: 'planning';
      status: 'running' | 'done' | 'failed';
      plan?: CodePlanFromAI;
      error?: string;
    }
  | {
      step: 'chunk';
      chunkId: string;
      name: string;
      status: ChunkStatus;
      result?: ChunkResult;
      error?: string;
    }
  | { step: 'assembly'; status: 'running' | 'done' | 'failed'; error?: string }
  | { step: 'complete'; finalCode: string; degraded: boolean }
  | { step: 'error'; message: string; chunkId?: string };

// === Contract 验证 ===

export interface ContractValidationResult {
  valid: boolean;
  issues: string[];
}

// === Wire DTO 类型（MCP/CLI 响应）===

/**
 * Depth-用于电汇的有
 * 限节点快照。 When 深度已耗尽，`children` 是字符串 `"..."` 而不是
 NodeSnapshot[]。
 */
export type NodeSnapshot = Omit<PenNode, 'children'> & {
  children?: NodeSnapshot[] | '...';
};

/**
 * Hydrated
 * codegen_plan 和 codegen_submit_chunk 返回的块有效负载。 Uses NodeSnapshot（深度有限）而不是
 * PenNode[]。当跳过依赖块 failed/was 时，depContracts 条目可能为空。
 */
export interface ExecutableChunkPayload extends Omit<ExecutableChunk, 'nodes' | 'depContracts'> {
  nodes: NodeSnapshot[];
  depContracts: ResolvedDepContract[];
}

/**
 * 如果上游块失败或被跳过，则可能不存在依赖契约。
 */
export type ResolvedDepContract = ChunkContract | null;
