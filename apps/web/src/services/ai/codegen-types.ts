import type {
  ChunkResult,
  ChunkStatus,
  CodegenQualityReport,
  CodegenRepairAttempt,
  CodegenTimingBreakdown,
  CodegenPipelineMode,
  Framework,
  PenNode,
  CodePlanFromAI,
} from '@zseven-w/pen-types';
import type { CodegenAssetFile } from './codegen-assets';
import type { buildCodegenFiles } from './codegen-files';
import type { CodegenDesignIR } from './codegen-design-ir';

export type CodegenTextCollector = (
  systemPrompt: string,
  userMessage: string,
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
) => Promise<string>;

export interface GenerateCodeOptions {
  collectText?: CodegenTextCollector;
  maxRepairAttempts?: number;
  fastPath?: 'auto' | 'never';
}

export interface GenerateCodeResult {
  code: string;
  degraded: boolean;
  assets: CodegenAssetFile[];
  files?: ReturnType<typeof buildCodegenFiles>;
  qualityReport?: CodegenQualityReport;
  timing?: CodegenTimingBreakdown;
  repairAttempts?: CodegenRepairAttempt[];
  designIR?: CodegenDesignIR;
  pipelineMode?: CodegenPipelineMode;
}

export interface CodegenQualityGateInput {
  framework: Framework;
  code: string;
  files?: ReturnType<typeof buildCodegenFiles>;
  designIR?: CodegenDesignIR;
}

export interface CodegenQualityGateResult {
  code: string;
  files: ReturnType<typeof buildCodegenFiles>;
  qualityReport: CodegenQualityReport;
  degraded: boolean;
}

export interface CodegenRepairPassInput extends CodegenQualityGateInput {
  qualityReport: CodegenQualityReport;
  model: string;
  provider: string | undefined;
  abortSignal?: AbortSignal;
  collectText?: CodegenTextCollector;
  exportedAssetPaths?: string[];
  attempt?: number;
}

export interface CodegenRepairPassResult extends CodegenQualityGateResult {
  repairAttempt: CodegenRepairAttempt;
  timing: CodegenTimingBreakdown;
}

export interface CodegenChunkCheckpointInput {
  chunkId: string;
  name: string;
  status: ChunkStatus;
  result?: ChunkResult;
  error?: string;
}

export interface CodegenChunkResumeInput {
  plan: CodePlanFromAI;
  nodes: PenNode[];
  framework: Framework;
  variables?: Record<string, unknown>;
  designIR?: CodegenDesignIR;
  chunkId: string;
  checkpoints: CodegenChunkCheckpointInput[];
  model: string;
  provider?: string;
  abortSignal?: AbortSignal;
  collectText?: CodegenTextCollector;
  maxRepairAttempts?: number;
}
