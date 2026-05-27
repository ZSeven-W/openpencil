// apps/web/src/services/ai/code-generation-pipeline.ts

import type { PenNode } from '@zseven-w/pen-types';
import type {
  Framework,
  CodePlanFromAI,
  ChunkResult,
  ChunkContract,
  ChunkStatus,
  CodeGenProgress,
  CodegenQualityReport,
  CodegenTimingBreakdown,
  CodegenRepairAttempt,
} from '@zseven-w/pen-types';
import { sanitizeName } from '@zseven-w/pen-core';
import {
  collectChunkAssetHints,
  extractCodegenAssets,
} from './codegen-assets';
import { buildCodegenFiles, validateCodegenFiles } from './codegen-files';
import { buildCodegenDesignIR } from './codegen-design-ir';
import {
  collectStreamText,
  runAssembly,
  runChunkGeneration,
  runPlanning,
  runRepair,
} from './codegen-model-calls';
import { generateCodeWithDirectFastPath, shouldUseDirectFastPath } from './codegen-fast-path';
import {
  hasCodegenQualityErrors,
  runCodegenRepairPass,
} from './codegen-quality-gate';
import { buildCodegenQualityReport } from './codegen-quality';
import { hydratePlan } from './codegen-plan';
import { validateContract } from './codegen-response-parser';
import {
  addTiming,
  elapsedSince,
  finalizeTiming,
  mergeProviderCallTiming,
  nowMs,
  withProviderTiming,
} from './codegen-timing';
import type {
  CodegenChunkResumeInput,
  GenerateCodeOptions,
  GenerateCodeResult,
} from './codegen-types';

export { computeExecutionOrder, hydratePlan } from './codegen-plan';
export { parseChunkResponse } from './codegen-response-parser';
export type {
  CodegenChunkCheckpointInput,
  CodegenChunkResumeInput,
  CodegenQualityGateInput,
  CodegenQualityGateResult,
  CodegenRepairPassInput,
  CodegenRepairPassResult,
  CodegenTextCollector,
  GenerateCodeOptions,
  GenerateCodeResult,
} from './codegen-types';
export { hasCodegenQualityErrors, runCodegenQualityGate, runCodegenRepairPass } from './codegen-quality-gate';

export async function resumeCodegenFromChunkCheckpoint(
  input: CodegenChunkResumeInput,
): Promise<GenerateCodeResult & { chunkResult: ChunkResult; chunkName: string }> {
  const totalStart = nowMs();
  const timing: CodegenTimingBreakdown = {};
  const { nodes: sanitizedNodes, assets } = extractCodegenAssets(input.nodes);
  const designIR =
    input.designIR ??
    buildCodegenDesignIR(
      sanitizedNodes,
      assets.map((asset) => ({
        relativePath: asset.relativePath,
        sourceNodeId: asset.sourceNodeId,
        sourceNodeName: asset.sourceNodeName,
        sourceKind: asset.sourceKind,
      })),
    );
  const collectText = input.collectText ?? collectStreamText;
  const execPlan = hydratePlan(input.plan, sanitizedNodes);
  if (execPlan.chunks.length === 0) {
    throw new Error('Planning checkpoint produced no valid chunks');
  }
  const targetChunk = execPlan.chunks.find((chunk) => chunk.id === input.chunkId);
  if (!targetChunk) {
    throw new Error(`Chunk checkpoint ${input.chunkId} does not exist in the planning checkpoint.`);
  }

  const checkpointByChunk = new Map(input.checkpoints.map((checkpoint) => [checkpoint.chunkId, checkpoint]));
  const resultByChunk = new Map<string, ChunkResult>();
  const statusByChunk = new Map<string, ChunkStatus>();
  for (const chunk of execPlan.chunks) {
    const checkpoint = checkpointByChunk.get(chunk.id);
    if (checkpoint?.result) resultByChunk.set(chunk.id, checkpoint.result);
    statusByChunk.set(chunk.id, checkpoint?.status ?? 'pending');
  }

  const depContracts: ChunkContract[] = targetChunk.dependencies
    .map((depId) => resultByChunk.get(depId)?.contract)
    .filter((contract): contract is ChunkContract => Boolean(contract?.componentName));
  const assetHints = collectChunkAssetHints(targetChunk.nodes, assets);
  const chunkStart = nowMs();
  const chunkResult = await runChunkGeneration(
    targetChunk.nodes,
    input.framework,
    targetChunk.suggestedComponentName,
    depContracts,
    targetChunk.id,
    designIR,
    input.model,
    input.provider,
    input.abortSignal,
    assetHints,
    withProviderTiming(timing, collectText, {
      stage: 'chunk',
      attempt: 1,
      provider: input.provider,
      model: input.model,
      chunkId: targetChunk.id,
    }),
  );
  if (
    !chunkResult.contract.componentName ||
    !/^[A-Z][a-zA-Z0-9]*$/.test(chunkResult.contract.componentName)
  ) {
    chunkResult.contract.componentName = sanitizeName(targetChunk.suggestedComponentName);
  }
  const validation = validateContract(chunkResult);
  resultByChunk.set(targetChunk.id, chunkResult);
  statusByChunk.set(targetChunk.id, validation.valid ? 'done' : 'degraded');
  addTiming(timing, 'chunkMs', elapsedSince(chunkStart));

  const chunkInputs = execPlan.chunks.map((chunk) => {
    const status = statusByChunk.get(chunk.id);
    const result = resultByChunk.get(chunk.id);
    return {
      chunkId: chunk.id,
      name: chunk.name,
      code: result?.code ?? '',
      contract: result?.contract,
      status: (status === 'done' ? 'successful' : status === 'degraded' ? 'degraded' : 'failed') as
        | 'successful'
        | 'degraded'
        | 'failed',
    };
  });
  if (!chunkInputs.some((chunk) => chunk.code.length > 0)) {
    throw new Error('All chunks failed — no code to assemble');
  }

  const exportedAssetPaths = assets.map((asset) => asset.relativePath);
  let finalCode: string;
  let degraded =
    chunkInputs.some((chunk) => chunk.status !== 'successful') || !validation.valid;
  try {
    const assemblyStart = nowMs();
    finalCode = await runAssembly(
      chunkInputs,
      input.plan,
      input.framework,
      input.variables,
      input.model,
      input.provider,
      input.abortSignal,
      exportedAssetPaths,
      designIR,
      withProviderTiming(timing, collectText, {
        stage: 'assembly',
        attempt: 1,
        provider: input.provider,
        model: input.model,
      }),
    );
    addTiming(timing, 'assemblyMs', elapsedSince(assemblyStart));
  } catch {
    finalCode = chunkInputs
      .filter((chunk) => chunk.code)
      .map((chunk) => `// ── ${chunk.name} (${chunk.status}) ──\n\n${chunk.code}`)
      .join('\n\n');
    degraded = true;
  }

  let files = buildCodegenFiles({ framework: input.framework, code: finalCode });
  const repairAttempts: CodegenRepairAttempt[] = [];
  const qualityStart = nowMs();
  let qualityReport = buildCodegenQualityReport({
    framework: input.framework,
    files,
    designIR,
  });
  addTiming(timing, 'qualityCheckMs', elapsedSince(qualityStart));

  const maxRepairAttempts = input.maxRepairAttempts ?? 2;
  for (let attempt = 1; hasCodegenQualityErrors(qualityReport) && attempt <= maxRepairAttempts; attempt++) {
    const repair = await runCodegenRepairPass({
      framework: input.framework,
      code: finalCode,
      files,
      designIR,
      qualityReport,
      model: input.model,
      provider: input.provider,
      abortSignal: input.abortSignal,
      collectText,
      exportedAssetPaths,
      attempt,
    });
    mergeProviderCallTiming(timing, repair.timing);
    addTiming(timing, 'repairMs', repair.timing.repairMs ?? 0);
    finalCode = repair.code;
    files = repair.files;
    qualityReport = repair.qualityReport;
    repairAttempts.push(repair.repairAttempt);
  }

  if (hasCodegenQualityErrors(qualityReport)) degraded = true;
  if (!validateCodegenFiles(input.framework, files).valid) degraded = true;
  finalizeTiming(timing, elapsedSince(totalStart));

  return {
    code: finalCode,
    degraded,
    assets,
    files,
    qualityReport,
    timing,
    repairAttempts,
    designIR,
    pipelineMode: 'full_pipeline',
    chunkResult,
    chunkName: targetChunk.name,
  };
}

// ── Main 管道 ──

export async function generateCode(
  nodes: PenNode[],
  framework: Framework,
  variables: Record<string, unknown> | undefined,
  onProgress: (event: CodeGenProgress) => void,
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
  options: GenerateCodeOptions = {},
): Promise<GenerateCodeResult> {
  const totalStart = nowMs();
  const timing: CodegenTimingBreakdown = {};
  const { nodes: sanitizedNodes, assets } = extractCodegenAssets(nodes);
  const designIR = buildCodegenDesignIR(
    sanitizedNodes,
    assets.map((asset) => ({
      relativePath: asset.relativePath,
      sourceNodeId: asset.sourceNodeId,
      sourceNodeName: asset.sourceNodeName,
      sourceKind: asset.sourceKind,
    })),
  );
  const collectText = options.collectText ?? collectStreamText;
  const exportedAssetPaths = assets.map((asset) => asset.relativePath);
  if (shouldUseDirectFastPath({ nodes: sanitizedNodes, framework, options })) {
    return generateCodeWithDirectFastPath({
      nodes: sanitizedNodes,
      assets,
      exportedAssetPaths,
      framework,
      variables,
      onProgress,
      model,
      provider,
      abortSignal,
      options,
      designIR,
      collectText,
      totalStart,
    });
  }
  // ── Step 1: Planning ──
  onProgress({ step: 'planning', status: 'running' });

  let planFromAI: CodePlanFromAI;
  try {
    const started = nowMs();
    planFromAI = await runPlanning(
      sanitizedNodes,
      framework,
      designIR,
      model,
      provider,
      abortSignal,
      false,
      withProviderTiming(timing, collectText, {
        stage: 'planning',
        attempt: 1,
        provider,
        model,
      }),
    );
    addTiming(timing, 'planningMs', elapsedSince(started));
    onProgress({ step: 'planning', status: 'done', plan: planFromAI, designIR });
  } catch (err) {
    if (abortSignal?.aborted) throw err;
    // Retry 一次，提示更严格
    try {
      const started = nowMs();
      planFromAI = await runPlanning(
        sanitizedNodes,
        framework,
        designIR,
        model,
        provider,
        abortSignal,
        true,
        withProviderTiming(timing, collectText, {
          stage: 'planning',
          attempt: 2,
          provider,
          model,
        }),
      );
      addTiming(timing, 'planningMs', elapsedSince(started));
      onProgress({ step: 'planning', status: 'done', plan: planFromAI, designIR });
    } catch (retryErr) {
      const msg = retryErr instanceof Error ? retryErr.message : 'Planning failed';
      onProgress({ step: 'planning', status: 'failed', error: msg });
      onProgress({ step: 'error', message: msg });
      throw retryErr;
    }
  }

  // Hydrate 计划与实际节点数据
  const execPlan = hydratePlan(planFromAI, sanitizedNodes);
  if (execPlan.chunks.length === 0) {
    const msg = 'Planning produced no valid chunks';
    onProgress({ step: 'planning', status: 'failed', error: msg });
    onProgress({ step: 'error', message: msg });
    throw new Error(msg);
  }

  // Initialize 所有块均处于待处理状态
  for (const chunk of execPlan.chunks) {
    onProgress({ step: 'chunk', chunkId: chunk.id, name: chunk.name, status: 'pending' });
  }

  // ── Step 2: Parallel Chunk Generation ──
  const results = new Map<string, ChunkResult>();
  const statuses = new Map<string, ChunkStatus>();

  // Group 按执行顺序
  const maxOrder = Math.max(...execPlan.chunks.map((c) => c.order));

  for (let order = 0; order <= maxOrder; order++) {
    if (abortSignal?.aborted) throw new Error('Aborted');

    const batch = execPlan.chunks.filter((c) => c.order === order);
    const batchPromises = batch.map(async (chunk) => {
      // Check 如果依赖失败
      const depsFailed = chunk.dependencies.some((depId) => statuses.get(depId) === 'failed');
      if (depsFailed) {
        statuses.set(chunk.id, 'skipped');
        onProgress({ step: 'chunk', chunkId: chunk.id, name: chunk.name, status: 'skipped' });
        return;
      }

      // Collect 依赖合约
      const depContracts: ChunkContract[] = chunk.dependencies
        .map((depId) => results.get(depId)?.contract)
        .filter((c): c is ChunkContract => c !== undefined && c.componentName !== '');

      onProgress({ step: 'chunk', chunkId: chunk.id, name: chunk.name, status: 'running' });

      try {
        const assetHints = collectChunkAssetHints(chunk.nodes, assets);
        const result = await runChunkGeneration(
          chunk.nodes,
          framework,
          chunk.suggestedComponentName,
          depContracts,
          chunk.id,
          designIR,
          model,
          provider,
          abortSignal,
          assetHints,
          withProviderTiming(timing, collectText, {
            stage: 'chunk',
            attempt: 1,
            provider,
            model,
            chunkId: chunk.id,
          }),
        );

        // Ensure componentName 有效 PascalCase — AI 可能返回 kebab-case 或空
        if (
          !result.contract.componentName ||
          !/^[A-Z][a-zA-Z0-9]*$/.test(result.contract.componentName)
        ) {
          result.contract.componentName = sanitizeName(chunk.suggestedComponentName);
        }

        const validation = validateContract(result);
        if (validation.valid) {
          results.set(chunk.id, result);
          statuses.set(chunk.id, 'done');
          onProgress({
            step: 'chunk',
            chunkId: chunk.id,
            name: chunk.name,
            status: 'done',
            result,
          });
        } else {
          // Contract 无效 — 标记已降级
          results.set(chunk.id, result);
          statuses.set(chunk.id, 'degraded');
          onProgress({
            step: 'chunk',
            chunkId: chunk.id,
            name: chunk.name,
            status: 'degraded',
            result,
            error: validation.issues.join('; ') || 'Chunk contract validation failed',
          });
        }
      } catch {
        // Retry 一次
        try {
          const assetHints = collectChunkAssetHints(chunk.nodes, assets);
          const result = await runChunkGeneration(
            chunk.nodes,
            framework,
            chunk.suggestedComponentName,
            depContracts,
            chunk.id,
            designIR,
            model,
            provider,
            abortSignal,
            assetHints,
            withProviderTiming(timing, collectText, {
              stage: 'chunk',
              attempt: 2,
              provider,
              model,
              chunkId: chunk.id,
            }),
          );
          if (
            !result.contract.componentName ||
            !/^[A-Z][a-zA-Z0-9]*$/.test(result.contract.componentName)
          ) {
            result.contract.componentName = sanitizeName(chunk.suggestedComponentName);
          }
          results.set(chunk.id, result);
          const validation = validateContract(result);
          statuses.set(chunk.id, validation.valid ? 'done' : 'degraded');
          onProgress({
            step: 'chunk',
            chunkId: chunk.id,
            name: chunk.name,
            status: statuses.get(chunk.id)!,
            result,
            ...(validation.valid
              ? {}
              : { error: validation.issues.join('; ') || 'Chunk contract validation failed' }),
          });
        } catch (retryErr) {
          statuses.set(chunk.id, 'failed');
          const msg = retryErr instanceof Error ? retryErr.message : 'Chunk generation failed';
          onProgress({
            step: 'chunk',
            chunkId: chunk.id,
            name: chunk.name,
            status: 'failed',
            error: msg,
          });
        }
      }
    });

    const started = nowMs();
    await Promise.all(batchPromises);
    addTiming(timing, 'chunkMs', elapsedSince(started));
  }

  // ── Step 3: Assembly ──
  onProgress({ step: 'assembly', status: 'running' });

  const chunkInputs = execPlan.chunks.map((chunk) => {
    const status = statuses.get(chunk.id);
    const result = results.get(chunk.id);
    return {
      chunkId: chunk.id,
      name: chunk.name,
      code: result?.code ?? '',
      contract: result?.contract,
      status: (status === 'done' ? 'successful' : status === 'degraded' ? 'degraded' : 'failed') as
        | 'successful'
        | 'degraded'
        | 'failed',
    };
  });

  const hasAnyCode = chunkInputs.some((c) => c.code.length > 0);
  if (!hasAnyCode) {
    const msg = 'All chunks failed — no code to assemble';
    onProgress({ step: 'assembly', status: 'failed', error: msg });
    onProgress({ step: 'error', message: msg });
    throw new Error(msg);
  }

  let finalCode: string;
  let degraded = chunkInputs.some((c) => c.status !== 'successful');
  try {
    const started = nowMs();
    finalCode = await runAssembly(
      chunkInputs,
      planFromAI,
      framework,
      variables,
      model,
      provider,
      abortSignal,
      exportedAssetPaths,
      designIR,
      withProviderTiming(timing, collectText, {
        stage: 'assembly',
        attempt: 1,
        provider,
        model,
      }),
    );
    addTiming(timing, 'assemblyMs', elapsedSince(started));
    onProgress({
      step: 'assembly',
      status: 'done',
      code: finalCode,
      files: buildCodegenFiles({ framework, code: finalCode }),
    });
  } catch {
    // Retry 一次
    try {
      const started = nowMs();
      finalCode = await runAssembly(
        chunkInputs,
        planFromAI,
        framework,
        variables,
        model,
        provider,
        abortSignal,
        exportedAssetPaths,
        designIR,
        withProviderTiming(timing, collectText, {
          stage: 'assembly',
          attempt: 2,
          provider,
          model,
        }),
      );
      addTiming(timing, 'assemblyMs', elapsedSince(started));
      onProgress({ step: 'assembly', status: 'done' });
    } catch {
      // Best-effort 后备：连接块代码
      finalCode = chunkInputs
        .filter((c) => c.code)
        .map((c) => `// ── ${c.name} (${c.status}) ──\n\n${c.code}`)
        .join('\n\n');
      degraded = true;
      onProgress({
        step: 'assembly',
        status: 'failed',
        code: finalCode,
        files: buildCodegenFiles({ framework, code: finalCode }),
        error: 'Assembly failed — showing concatenated chunks',
      });
    }
  }

  let files = buildCodegenFiles({ framework, code: finalCode });
  const repairAttempts: CodegenRepairAttempt[] = [];
  let qualityReport: CodegenQualityReport;

  onProgress({ step: 'quality_check', status: 'running' });
  const qualityStart = nowMs();
  qualityReport = buildCodegenQualityReport({ framework, files, designIR });
  addTiming(timing, 'qualityCheckMs', elapsedSince(qualityStart));
  onProgress({
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

  const maxRepairAttempts = options.maxRepairAttempts ?? 2;
  for (let attempt = 1; hasCodegenQualityErrors(qualityReport) && attempt <= maxRepairAttempts; attempt++) {
    if (abortSignal?.aborted) throw new Error('Aborted');
    onProgress({ step: 'repair', status: 'running', attempt, report: qualityReport });
    const repairStart = nowMs();
    try {
      const repairedCode = await runRepair(
        {
          framework,
          code: finalCode,
          files,
          issues: qualityReport.issues.filter((issue) => issue.severity === 'error'),
          designIR,
          exportedAssetPaths,
        },
        model,
        provider,
        abortSignal,
        withProviderTiming(timing, collectText, {
          stage: 'repair',
          attempt,
          provider,
          model,
        }),
      );
      const durationMs = elapsedSince(repairStart);
      addTiming(timing, 'repairMs', durationMs);
      finalCode = repairedCode;
      files = buildCodegenFiles({ framework, code: finalCode });
      const repairedReport = buildCodegenQualityReport({
        framework,
        files,
        designIR,
        repaired: true,
      });
      repairAttempts.push({
        attempt,
        issues: qualityReport.issues,
        code: finalCode,
        report: repairedReport,
        durationMs,
      });
      qualityReport = repairedReport;
      onProgress({
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
    } catch (repairErr) {
      const durationMs = elapsedSince(repairStart);
      addTiming(timing, 'repairMs', durationMs);
      const msg = repairErr instanceof Error ? repairErr.message : 'Repair failed';
      repairAttempts.push({
        attempt,
        issues: qualityReport.issues,
        code: finalCode,
        report: qualityReport,
        durationMs,
      });
      onProgress({ step: 'repair', status: 'failed', attempt, report: qualityReport, error: msg });
      break;
    }
  }

  if (!hasCodegenQualityErrors(qualityReport)) {
    onProgress({ step: 'final_validation', status: 'running' });
    onProgress({ step: 'final_validation', status: 'done' });
  } else {
    degraded = true;
    onProgress({
      step: 'final_validation',
      status: 'failed',
      error: qualityReport.issues
        .filter((issue) => issue.severity === 'error')
        .map((issue) => issue.message)
        .join('; '),
    });
  }

  const filesValidation = validateCodegenFiles(framework, files);
  if (!filesValidation.valid) degraded = true;

  finalizeTiming(timing, elapsedSince(totalStart));

  onProgress({
    step: 'complete',
    finalCode,
    degraded,
    qualityReport,
    timing,
    repairAttempts,
    pipelineMode: 'full_pipeline',
  });
  return {
    code: finalCode,
    degraded,
    assets,
    files,
    qualityReport,
    timing,
    repairAttempts,
    designIR,
    pipelineMode: 'full_pipeline',
  };
}
