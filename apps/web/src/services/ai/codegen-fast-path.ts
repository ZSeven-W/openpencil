import type {
  CodeGenProgress,
  CodegenRepairAttempt,
  CodegenTimingBreakdown,
  Framework,
  PenNode,
} from '@zseven-w/pen-types';
import type { CodegenAssetFile } from './codegen-assets';
import { buildCodegenFiles, validateCodegenFiles } from './codegen-files';
import type { CodegenDesignIR } from './codegen-design-ir';
import { buildCodegenQualityReport } from './codegen-quality';
import { runDirectGeneration } from './codegen-model-calls';
import { hasCodegenQualityErrors, runCodegenRepairPass } from './codegen-quality-gate';
import {
  addTiming,
  elapsedSince,
  finalizeTiming,
  mergeProviderCallTiming,
  nowMs,
  withProviderTiming,
} from './codegen-timing';
import type { CodegenTextCollector, GenerateCodeOptions, GenerateCodeResult } from './codegen-types';

function getChildren(node: PenNode): PenNode[] {
  return 'children' in node && Array.isArray(node.children) ? node.children : [];
}

function countNodes(nodes: PenNode[]): number {
  return nodes.reduce((total, node) => total + 1 + countNodes(getChildren(node)), 0);
}

export function shouldUseDirectFastPath(input: {
  nodes: PenNode[];
  framework: Framework;
  options: GenerateCodeOptions;
}): boolean {
  if (input.options.fastPath === 'never') return false;
  if (!['html', 'vue', 'uniapp'].includes(input.framework)) return false;
  if (input.nodes.length !== 1) return false;
  const root = input.nodes[0];
  if (root.type !== 'frame') return false;
  return countNodes(input.nodes) <= 80;
}

export async function generateCodeWithDirectFastPath(input: {
  nodes: PenNode[];
  assets: CodegenAssetFile[];
  exportedAssetPaths: string[];
  framework: Framework;
  variables: Record<string, unknown> | undefined;
  onProgress: (event: CodeGenProgress) => void;
  model: string;
  provider: string | undefined;
  abortSignal?: AbortSignal;
  options: GenerateCodeOptions;
  designIR: CodegenDesignIR;
  collectText: CodegenTextCollector;
  totalStart: number;
}): Promise<GenerateCodeResult> {
  const timing: CodegenTimingBreakdown = {};
  input.onProgress({ step: 'assembly', status: 'running' });
  const directStart = nowMs();
  let finalCode = await runDirectGeneration(
    {
      nodes: input.nodes,
      framework: input.framework,
      variables: input.variables,
      exportedAssetPaths: input.exportedAssetPaths,
      designIR: input.designIR,
    },
    input.model,
    input.provider,
    input.abortSignal,
    withProviderTiming(timing, input.collectText, {
      stage: 'direct_generation',
      attempt: 1,
      provider: input.provider,
      model: input.model,
    }),
  );
  addTiming(timing, 'assemblyMs', elapsedSince(directStart));

  let files = buildCodegenFiles({ framework: input.framework, code: finalCode });
  input.onProgress({ step: 'assembly', status: 'done', code: finalCode, files });

  const repairAttempts: CodegenRepairAttempt[] = [];
  input.onProgress({ step: 'quality_check', status: 'running' });
  const qualityStart = nowMs();
  let qualityReport = buildCodegenQualityReport({
    framework: input.framework,
    files,
    designIR: input.designIR,
  });
  addTiming(timing, 'qualityCheckMs', elapsedSince(qualityStart));
  input.onProgress({
    step: 'quality_check',
    status: hasCodegenQualityErrors(qualityReport) ? 'failed' : 'done',
    report: qualityReport,
    error: hasCodegenQualityErrors(qualityReport)
      ? qualityReport.issues
          .filter((issue) => issue.severity === 'error')
          .map((issue) => issue.message)
          .join('; ')
      : undefined,
  });

  const maxRepairAttempts = input.options.maxRepairAttempts ?? 2;
  for (let attempt = 1; hasCodegenQualityErrors(qualityReport) && attempt <= maxRepairAttempts; attempt++) {
    if (input.abortSignal?.aborted) throw new Error('Aborted');
    input.onProgress({ step: 'repair', status: 'running', attempt, report: qualityReport });
    const repair = await runCodegenRepairPass({
      framework: input.framework,
      code: finalCode,
      files,
      designIR: input.designIR,
      qualityReport,
      model: input.model,
      provider: input.provider,
      abortSignal: input.abortSignal,
      collectText: input.collectText,
      exportedAssetPaths: input.exportedAssetPaths,
      attempt,
    });
    mergeProviderCallTiming(timing, repair.timing);
    addTiming(timing, 'repairMs', repair.timing.repairMs ?? 0);
    finalCode = repair.code;
    files = repair.files;
    qualityReport = repair.qualityReport;
    repairAttempts.push(repair.repairAttempt);
    input.onProgress({
      step: 'repair',
      status: hasCodegenQualityErrors(qualityReport) ? 'failed' : 'done',
      attempt,
      report: qualityReport,
      error: hasCodegenQualityErrors(qualityReport)
        ? qualityReport.issues
            .filter((issue) => issue.severity === 'error')
            .map((issue) => issue.message)
            .join('; ')
        : undefined,
    });
  }

  let degraded = false;
  if (!hasCodegenQualityErrors(qualityReport)) {
    input.onProgress({ step: 'final_validation', status: 'running' });
    input.onProgress({ step: 'final_validation', status: 'done' });
  } else {
    degraded = true;
    input.onProgress({
      step: 'final_validation',
      status: 'failed',
      error: qualityReport.issues
        .filter((issue) => issue.severity === 'error')
        .map((issue) => issue.message)
        .join('; '),
    });
  }

  if (!validateCodegenFiles(input.framework, files).valid) degraded = true;

  finalizeTiming(timing, elapsedSince(input.totalStart));
  input.onProgress({
    step: 'complete',
    finalCode,
    degraded,
    qualityReport,
    timing,
    repairAttempts,
    pipelineMode: 'direct_generation',
  });
  return {
    code: finalCode,
    degraded,
    assets: input.assets,
    files,
    qualityReport,
    timing,
    repairAttempts,
    designIR: input.designIR,
    pipelineMode: 'direct_generation',
  };
}
