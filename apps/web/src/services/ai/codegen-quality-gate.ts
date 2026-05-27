import type {
  CodegenQualityReport,
  CodegenTimingBreakdown,
} from '@zseven-w/pen-types';
import { buildCodegenFiles, validateCodegenFiles } from './codegen-files';
import { buildCodegenQualityReport } from './codegen-quality';
import { collectStreamText, runRepair } from './codegen-model-calls';
import {
  addTiming,
  elapsedSince,
  nowMs,
  withProviderTiming,
} from './codegen-timing';
import type {
  CodegenQualityGateInput,
  CodegenQualityGateResult,
  CodegenRepairPassInput,
  CodegenRepairPassResult,
} from './codegen-types';

export function hasCodegenQualityErrors(report: CodegenQualityReport): boolean {
  return report.summary.errorCount > 0;
}

export function runCodegenQualityGate(input: CodegenQualityGateInput): CodegenQualityGateResult {
  const files = input.files?.length
    ? input.files
    : buildCodegenFiles({ framework: input.framework, code: input.code });
  const qualityReport = buildCodegenQualityReport({
    framework: input.framework,
    files,
    designIR: input.designIR,
  });
  return {
    code: input.code,
    files,
    qualityReport,
    degraded: hasCodegenQualityErrors(qualityReport) || !validateCodegenFiles(input.framework, files).valid,
  };
}

export async function runCodegenRepairPass(
  input: CodegenRepairPassInput,
): Promise<CodegenRepairPassResult> {
  const files = input.files?.length
    ? input.files
    : buildCodegenFiles({ framework: input.framework, code: input.code });
  const timing: CodegenTimingBreakdown = {};
  const repairStart = nowMs();
  const collectText = input.collectText ?? collectStreamText;
  const repairedCode = await runRepair(
    {
      framework: input.framework,
      code: input.code,
      files,
      issues: input.qualityReport.issues.filter((issue) => issue.severity === 'error'),
      designIR: input.designIR ?? {
        version: 1,
        target: { width: 0, height: 0, platformHint: 'desktop' },
        summary: {
          nodeCount: 0,
          textCount: 0,
          imageCount: 0,
          assetCount: 0,
          textContent: [],
          semanticKinds: {},
        },
        assets: [],
        nodes: [],
      },
      exportedAssetPaths: input.exportedAssetPaths ?? [],
    },
    input.model,
    input.provider,
    input.abortSignal,
    withProviderTiming(timing, collectText, {
      stage: 'repair',
      attempt: input.attempt ?? 1,
      provider: input.provider,
      model: input.model,
    }),
  );
  const repairMs = elapsedSince(repairStart);
  addTiming(timing, 'repairMs', repairMs);
  const repairedFiles = buildCodegenFiles({ framework: input.framework, code: repairedCode });
  const qualityReport = buildCodegenQualityReport({
    framework: input.framework,
    files: repairedFiles,
    designIR: input.designIR,
    repaired: true,
  });
  return {
    code: repairedCode,
    files: repairedFiles,
    qualityReport,
    degraded:
      hasCodegenQualityErrors(qualityReport) ||
      !validateCodegenFiles(input.framework, repairedFiles).valid,
    timing,
    repairAttempt: {
      attempt: input.attempt ?? 1,
      issues: input.qualityReport.issues,
      code: repairedCode,
      report: qualityReport,
      durationMs: repairMs,
    },
  };
}
