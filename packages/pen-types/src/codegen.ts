import type { PenNode } from './pen.js';

// === Canonical framework type ===

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

// === Step 1 output: AI planner returns this (no node data, minimal tokens) ===

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

// === Runtime: hydrated with node data + execution order ===

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

// === Chunk contract: structured metadata output from each chunk ===

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

// === Chunk generation output ===

export interface ChunkResult {
  chunkId: string;
  code: string;
  contract: ChunkContract;
}

// === Progress events ===

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

// === Contract validation ===

export interface ContractValidationResult {
  valid: boolean;
  issues: string[];
}

// === Wire DTO types (MCP/CLI responses) ===

/**
 * Depth-limited node snapshot for wire transfer.
 * When depth is exhausted, `children` is the string `"..."` instead of NodeSnapshot[].
 */
export type NodeSnapshot = Omit<PenNode, 'children'> & {
  children?: NodeSnapshot[] | '...';
};

/**
 * Hydrated chunk payload returned by codegen_plan and codegen_submit_chunk.
 * Uses NodeSnapshot (depth-limited) instead of PenNode[].
 * depContracts entries may be null when a dependency chunk failed/was skipped.
 */
export interface ExecutableChunkPayload extends Omit<ExecutableChunk, 'nodes' | 'depContracts'> {
  nodes: NodeSnapshot[];
  depContracts: ResolvedDepContract[];
}

/**
 * A dependency contract that may be absent if the upstream chunk failed or was skipped.
 */
export type ResolvedDepContract = ChunkContract | null;
