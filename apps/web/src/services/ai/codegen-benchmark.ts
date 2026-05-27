import type {
  CodegenQualityReport,
  CodegenQualityIssue,
  CodegenQualityStatus,
  CodegenPipelineMode,
  CodegenRepairAttempt,
  CodegenTimingBreakdown,
  Framework,
} from '@zseven-w/pen-types';
import type { CodegenTextCollector, GenerateCodeOptions, GenerateCodeResult } from './code-generation-pipeline';
import { generateCode } from './code-generation-pipeline';
import {
  BENCHMARK_FRAMEWORKS,
  getCodegenBenchmarkCases,
  getCodegenBenchmarkFrameworks,
  makeBenchmarkBaselineCode,
  type CodegenBenchmarkCase,
  type CodegenBenchmarkCaseKind,
} from './codegen-benchmark-cases';
import { buildCodegenDesignIR } from './codegen-design-ir';
import { buildCodegenFiles } from './codegen-files';
import { buildCodegenQualityReport } from './codegen-quality';

export type { CodegenBenchmarkCase, CodegenBenchmarkCaseKind };
export { getCodegenBenchmarkCases, getCodegenBenchmarkFrameworks };

export interface CodegenBenchmarkFrameworkResult {
  mode: CodegenBenchmarkMode;
  framework: Framework;
  generatedAt: string;
  durationMs: number;
  wallTimeMs: number;
  status: CodegenBenchmarkRunStatus;
  provider?: CodegenBenchmarkProvider;
  model?: string;
  fileCount: number;
  textCount: number;
  imageCount: number;
  qualityStatus: CodegenQualityStatus | 'unknown';
  pipelineMode?: CodegenPipelineMode;
  errorCount: number;
  warningCount: number;
  warningIssues?: CodegenQualityIssue[];
  missingTextCount: number;
  missingAssetCount: number;
  finalCodeLength?: number;
  repairCount?: number;
  qualityReport?: CodegenQualityReport;
  timing?: CodegenTimingBreakdown;
  repairAttempts?: CodegenRepairAttempt[];
  error?: string;
}

export interface CodegenBenchmarkResult {
  caseId: string;
  caseName: string;
  kind: CodegenBenchmarkCaseKind;
  target: ReturnType<typeof buildCodegenDesignIR>['target'];
  frameworkResults: CodegenBenchmarkFrameworkResult[];
}

export interface CodegenBenchmarkOutput {
  version: 1;
  mode: CodegenBenchmarkMode;
  generatedAt: string;
  caseCount: number;
  frameworks: Framework[];
  provider?: CodegenBenchmarkProvider;
  model?: string;
  concurrency?: number;
  fastPath?: GenerateCodeOptions['fastPath'];
  totalDurationMs: number;
  summary: {
    totalRuns: number;
    succeeded: number;
    failed: number;
    skipped: number;
    quality: Record<string, number>;
    errorCount: number;
    warningCount: number;
    missingTextCount: number;
    missingAssetCount: number;
    repairCount: number;
  };
  results: CodegenBenchmarkResult[];
}

export type CodegenBenchmarkMode = 'rules' | 'model';
export type CodegenBenchmarkRunStatus = 'succeeded' | 'failed' | 'skipped';
export type CodegenBenchmarkProvider = 'anthropic' | 'openai' | 'gemini';

export interface CodegenBenchmarkModelOptions {
  provider: CodegenBenchmarkProvider;
  model: string;
  collectText: CodegenTextCollector;
  cases?: CodegenBenchmarkCase[];
  frameworks?: Framework[];
  maxRepairAttempts?: number;
  concurrency?: number;
  fastPath?: GenerateCodeOptions['fastPath'];
  generateCodeImpl?: typeof generateCode;
  now?: () => number;
}

function safeDurationMs(startedAt: number, finishedAt: number) {
  return Math.max(0, Math.round(finishedAt - startedAt));
}

function summarizeBenchmarkResults(results: CodegenBenchmarkResult[]): CodegenBenchmarkOutput['summary'] {
  const summary: CodegenBenchmarkOutput['summary'] = {
    totalRuns: 0,
    succeeded: 0,
    failed: 0,
    skipped: 0,
    quality: {},
    errorCount: 0,
    warningCount: 0,
    missingTextCount: 0,
    missingAssetCount: 0,
    repairCount: 0,
  };

  for (const result of results) {
    for (const frameworkResult of result.frameworkResults) {
      summary.totalRuns += 1;
      summary[frameworkResult.status] += 1;
      summary.quality[frameworkResult.qualityStatus] =
        (summary.quality[frameworkResult.qualityStatus] ?? 0) + 1;
      summary.errorCount += frameworkResult.errorCount;
      summary.warningCount += frameworkResult.warningCount;
      summary.missingTextCount += frameworkResult.missingTextCount;
      summary.missingAssetCount += frameworkResult.missingAssetCount;
      summary.repairCount += frameworkResult.repairCount ?? 0;
    }
  }

  return summary;
}

function getWarningIssues(report: CodegenQualityReport): CodegenQualityIssue[] {
  return report.issues.filter((issue) => issue.severity === 'warning');
}

export function createCodegenBenchmarkOutput(input: {
  mode: CodegenBenchmarkMode;
  generatedAt?: string;
  frameworks?: Framework[];
  provider?: CodegenBenchmarkProvider;
  model?: string;
  concurrency?: number;
  fastPath?: GenerateCodeOptions['fastPath'];
  totalDurationMs?: number;
  results: CodegenBenchmarkResult[];
}): CodegenBenchmarkOutput {
  const frameworks =
    input.frameworks ??
    Array.from(
      new Set(input.results.flatMap((result) => result.frameworkResults.map((item) => item.framework))),
    );

  return {
    version: 1,
    mode: input.mode,
    generatedAt: input.generatedAt ?? new Date().toISOString(),
    caseCount: input.results.length,
    frameworks,
    provider: input.provider,
    model: input.model,
    concurrency: input.concurrency,
    fastPath: input.fastPath,
    totalDurationMs: input.totalDurationMs ?? 0,
    summary: summarizeBenchmarkResults(input.results),
    results: input.results,
  };
}

export function runCodegenBenchmarkBaseline(input: {
  cases?: CodegenBenchmarkCase[];
  frameworks?: Framework[];
  now?: () => number;
} = {}): CodegenBenchmarkResult[] {
  const cases = input.cases ?? getCodegenBenchmarkCases();
  const frameworks = input.frameworks ?? BENCHMARK_FRAMEWORKS;
  const now = input.now ?? (() => Date.now());

  return cases.map((benchmark) => {
    const ir = buildCodegenDesignIR(benchmark.nodes);
    return {
      caseId: benchmark.id,
      caseName: benchmark.name,
      kind: benchmark.kind,
      target: ir.target,
      frameworkResults: frameworks.map((framework) => {
        const startedAt = now();
        const code = makeBenchmarkBaselineCode(framework, benchmark);
        const files = buildCodegenFiles({ framework, code });
        const report = buildCodegenQualityReport({ framework, files, designIR: ir });
        const finishedAt = now();

        return {
          mode: 'rules',
          framework,
          generatedAt: new Date().toISOString(),
          durationMs: safeDurationMs(startedAt, finishedAt),
          wallTimeMs: safeDurationMs(startedAt, finishedAt),
          status: 'succeeded',
          fileCount: files.length,
          textCount: ir.summary.textCount,
          imageCount: ir.summary.imageCount,
          qualityStatus: report.status,
          pipelineMode: 'unknown',
          errorCount: report.summary.errorCount,
          warningCount: report.summary.warningCount,
          warningIssues: getWarningIssues(report),
          missingTextCount: report.summary.missingTextCount,
          missingAssetCount: report.summary.missingAssetCount,
          finalCodeLength: code.length,
          qualityReport: report,
        };
      }),
    };
  });
}

export async function runWithConcurrencyLimit<T, R>(
  items: T[],
  concurrency: number | undefined,
  worker: (item: T, index: number) => Promise<R>,
): Promise<R[]> {
  const limit = Math.max(1, Math.floor(concurrency ?? (items.length || 1)));
  const results = new Array<R>(items.length);
  let nextIndex = 0;

  async function runNext() {
    while (nextIndex < items.length) {
      const index = nextIndex;
      nextIndex += 1;
      results[index] = await worker(items[index], index);
    }
  }

  const workers = Array.from({ length: Math.min(limit, items.length) }, () => runNext());
  await Promise.all(workers);
  return results;
}

export async function runCodegenBenchmarkModel(
  input: CodegenBenchmarkModelOptions,
): Promise<CodegenBenchmarkResult[]> {
  const cases = input.cases ?? getCodegenBenchmarkCases();
  const frameworks = input.frameworks ?? BENCHMARK_FRAMEWORKS;
  const generateCodeImpl = input.generateCodeImpl ?? generateCode;
  const now = input.now ?? (() => Date.now());

  const progress = () => undefined;
  const jobs = cases.flatMap((benchmark) =>
    frameworks.map((framework) => ({
      benchmark,
      framework,
      ir: buildCodegenDesignIR(benchmark.nodes),
    })),
  );

  const flatResults = await runWithConcurrencyLimit(
    jobs,
    input.concurrency,
    async ({ benchmark, framework, ir }) => {
      const startedAt = now();
      try {
        const result: GenerateCodeResult = await generateCodeImpl(
          benchmark.nodes,
          framework,
          undefined,
          progress,
          input.model,
          input.provider,
          undefined,
          {
            collectText: input.collectText,
            maxRepairAttempts: input.maxRepairAttempts ?? 2,
            fastPath: input.fastPath,
          },
        );
        const finishedAt = now();
        const files = result.files ?? buildCodegenFiles({ framework, code: result.code });
        const report = result.qualityReport ?? buildCodegenQualityReport({
          framework,
          files,
          designIR: result.designIR ?? ir,
        });

        return {
          mode: 'model' as const,
          framework,
          provider: input.provider,
          model: input.model,
          generatedAt: new Date().toISOString(),
          durationMs: result.timing?.totalMs ?? safeDurationMs(startedAt, finishedAt),
          wallTimeMs: safeDurationMs(startedAt, finishedAt),
          status: 'succeeded' as const,
          fileCount: files.length,
          textCount: (result.designIR ?? ir).summary.textCount,
          imageCount: (result.designIR ?? ir).summary.imageCount,
          qualityStatus: report.status,
          pipelineMode: result.pipelineMode ?? 'unknown',
          errorCount: report.summary.errorCount,
          warningCount: report.summary.warningCount,
          warningIssues: getWarningIssues(report),
          missingTextCount: report.summary.missingTextCount,
          missingAssetCount: report.summary.missingAssetCount,
          finalCodeLength: result.code.length,
          repairCount: result.repairAttempts?.length ?? 0,
          qualityReport: report,
          timing: result.timing,
          repairAttempts: result.repairAttempts,
        };
      } catch (error) {
        const finishedAt = now();
        const message = error instanceof Error ? error.message : 'Benchmark run failed';
        return {
          mode: 'model' as const,
          framework,
          provider: input.provider,
          model: input.model,
          generatedAt: new Date().toISOString(),
          durationMs: safeDurationMs(startedAt, finishedAt),
          wallTimeMs: safeDurationMs(startedAt, finishedAt),
          status: 'failed' as const,
          fileCount: 0,
          textCount: ir.summary.textCount,
          imageCount: ir.summary.imageCount,
          qualityStatus: 'unknown' as const,
          pipelineMode: 'unknown' as const,
          errorCount: 0,
          warningCount: 0,
          missingTextCount: 0,
          missingAssetCount: 0,
          error: message,
        };
      }
    },
  );

  return cases.map((benchmark) => {
    const ir = buildCodegenDesignIR(benchmark.nodes);
    return {
      caseId: benchmark.id,
      caseName: benchmark.name,
      kind: benchmark.kind,
      target: ir.target,
      frameworkResults: frameworks.map((framework) => {
        const index = jobs.findIndex(
          (job) => job.benchmark.id === benchmark.id && job.framework === framework,
        );
        return flatResults[index];
      }),
    };
  });
}
