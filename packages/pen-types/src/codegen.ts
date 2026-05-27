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
  | 'react-native'
  | 'uniapp';

export const FRAMEWORKS: Framework[] = [
  'react',
  'vue',
  'svelte',
  'html',
  'flutter',
  'swiftui',
  'compose',
  'react-native',
  'uniapp',
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
  outputFiles?: string[];
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

export type CodegenQualityStatus = 'passed' | 'repaired' | 'degraded' | 'failed';

export type CodegenQualityIssueSeverity = 'info' | 'warning' | 'error';

export interface CodegenQualityIssue {
  code: string;
  severity: CodegenQualityIssueSeverity;
  message: string;
  filePath?: string;
  nodeId?: string;
}

export interface CodegenQualityReport {
  status: CodegenQualityStatus;
  framework: Framework;
  issues: CodegenQualityIssue[];
  checkedAt: string;
  summary: {
    fileCount: number;
    errorCount: number;
    warningCount: number;
    missingTextCount: number;
    missingAssetCount: number;
  };
}

export type CodegenProviderCallStage = 'planning' | 'chunk' | 'assembly' | 'repair' | 'direct_generation';

export type CodegenPipelineMode = 'direct_generation' | 'full_pipeline' | 'unknown';

export interface CodegenProviderCallTiming {
  stage: CodegenProviderCallStage;
  durationMs: number;
  attempt: number;
  provider?: string;
  model?: string;
  chunkId?: string;
  error?: string;
}

export interface CodegenTimingBreakdown {
  planningMs?: number;
  chunkMs?: number;
  assemblyMs?: number;
  qualityCheckMs?: number;
  repairMs?: number;
  providerMs?: number;
  providerCallTotalMs?: number;
  providerCallCount?: number;
  providerCalls?: CodegenProviderCallTiming[];
  totalMs?: number;
}

export interface CodegenRepairAttempt {
  attempt: number;
  issues: CodegenQualityIssue[];
  code: string;
  report: CodegenQualityReport;
  durationMs: number;
}

export interface CodegenCheckpoint {
  stage: 'planning' | 'chunk' | 'assembly' | 'quality_check' | 'repair' | 'final_validation';
  status: 'succeeded' | 'failed';
  attempt: number;
  data: unknown;
  createdAt: string;
}

export type CodegenResumeMode = 'from_failed_stage' | 'quality_check' | 'repair' | 'chunk';

// === Progress 事件 ===

export type ChunkStatus = 'pending' | 'running' | 'done' | 'degraded' | 'failed' | 'skipped';

export type CodeGenProgress =
  | {
      step: 'planning';
      status: 'running' | 'done' | 'failed';
      plan?: CodePlanFromAI;
      designIR?: unknown;
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
  | {
      step: 'assembly';
      status: 'running' | 'done' | 'failed';
      code?: string;
      files?: unknown[];
      error?: string;
    }
  | {
      step: 'quality_check';
      status: 'running' | 'done' | 'failed';
      report?: CodegenQualityReport;
      error?: string;
    }
  | {
      step: 'repair';
      status: 'running' | 'done' | 'failed' | 'skipped';
      attempt?: number;
      report?: CodegenQualityReport;
      error?: string;
    }
  | { step: 'final_validation'; status: 'running' | 'done' | 'failed'; error?: string }
  | {
      step: 'complete';
      finalCode: string;
      degraded: boolean;
      qualityReport?: CodegenQualityReport;
      timing?: CodegenTimingBreakdown;
      repairAttempts?: CodegenRepairAttempt[];
      pipelineMode?: CodegenPipelineMode;
    }
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
