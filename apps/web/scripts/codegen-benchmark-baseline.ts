import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { Framework } from '@zseven-w/pen-types';
import {
  createCodegenBenchmarkOutput,
  getCodegenBenchmarkCases,
  getCodegenBenchmarkFrameworks,
  runCodegenBenchmarkBaseline,
  runCodegenBenchmarkModel,
  type CodegenBenchmarkMode,
  type CodegenBenchmarkProvider,
} from '../src/services/ai/codegen-benchmark';
import type { CodegenBenchmarkCase, CodegenBenchmarkResult } from '../src/services/ai/codegen-benchmark';
import type { GenerateCodeOptions } from '../src/services/ai/code-generation-pipeline';

const projectRoot = resolve(import.meta.dirname, '../../..');
const outputDir = resolve(projectRoot, 'output/codegen-benchmark');

const PROVIDERS = new Set<CodegenBenchmarkProvider>(['anthropic', 'openai', 'gemini']);
const FRAMEWORKS = new Set<Framework>(getCodegenBenchmarkFrameworks());

function readArg(name: string): string | undefined {
  const prefix = `--${name}=`;
  const inline = process.argv.find((arg) => arg.startsWith(prefix));
  if (inline) return inline.slice(prefix.length);

  const index = process.argv.indexOf(`--${name}`);
  if (index !== -1) return process.argv[index + 1];

  return undefined;
}

function readList(value: string | undefined): string[] | undefined {
  if (!value?.trim()) return undefined;
  return value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
}

function parseMode(): CodegenBenchmarkMode {
  const value = readArg('mode') ?? process.env.CODEGEN_BENCHMARK_MODE ?? 'rules';
  if (value === 'rules' || value === 'model') return value;
  throw new Error(`Unsupported benchmark mode "${value}". Use rules or model.`);
}

function parseProvider(): CodegenBenchmarkProvider {
  const value = readArg('provider') ?? process.env.CODEGEN_BENCHMARK_PROVIDER;
  if (value && PROVIDERS.has(value as CodegenBenchmarkProvider)) {
    return value as CodegenBenchmarkProvider;
  }
  throw new Error('Model benchmark requires --provider=anthropic|openai|gemini.');
}

function parseFrameworks(): Framework[] {
  const requested = readList(readArg('frameworks') ?? process.env.CODEGEN_BENCHMARK_FRAMEWORKS);
  if (!requested) return getCodegenBenchmarkFrameworks();

  const frameworks = requested.filter((item): item is Framework => FRAMEWORKS.has(item as Framework));
  const rejected = requested.filter((item) => !FRAMEWORKS.has(item as Framework));
  if (rejected.length > 0) {
    throw new Error(`Unsupported benchmark frameworks: ${rejected.join(', ')}`);
  }
  return frameworks;
}

function parseConcurrency(): number | undefined {
  const raw = readArg('concurrency') ?? process.env.CODEGEN_BENCHMARK_CONCURRENCY;
  if (!raw?.trim()) return undefined;
  const value = Number(raw);
  if (!Number.isFinite(value) || value < 1) {
    throw new Error('Benchmark concurrency must be a positive number.');
  }
  return Math.floor(value);
}

function parseFastPath(): GenerateCodeOptions['fastPath'] {
  const raw = readArg('fast-path') ?? process.env.CODEGEN_BENCHMARK_FAST_PATH;
  if (!raw?.trim()) return undefined;
  if (raw === 'auto' || raw === 'never') return raw;
  throw new Error('Benchmark fast path must be auto or never.');
}

function filterCases(cases: CodegenBenchmarkCase[]): CodegenBenchmarkCase[] {
  const requested = readList(readArg('cases') ?? process.env.CODEGEN_BENCHMARK_CASES);
  if (!requested) return cases;

  const requestedSet = new Set(requested);
  const filtered = cases.filter((item) => requestedSet.has(item.id));
  const missing = requested.filter((id) => !cases.some((item) => item.id === id));
  if (missing.length > 0) {
    throw new Error(`Unknown benchmark cases: ${missing.join(', ')}`);
  }
  return filtered;
}

function printSummary(outputPath: string, results: CodegenBenchmarkResult[]) {
  console.log(`[codegen-benchmark] wrote ${outputPath}`);
  for (const result of results) {
    const summary = result.frameworkResults
      .map((item) =>
        [
          `${item.framework}:${item.status}/${item.qualityStatus}`,
          `pipeline=${item.pipelineMode ?? 'unknown'}`,
          `errors=${item.errorCount}`,
          `warnings=${item.warningCount}`,
          `missingText=${item.missingTextCount}`,
          `missingAssets=${item.missingAssetCount}`,
          `repairs=${item.repairCount ?? 0}`,
          `wallMs=${item.wallTimeMs}`,
        ].join('/'),
      )
      .join(' ');
    console.log(`[codegen-benchmark] ${result.caseId} ${summary}`);
    for (const item of result.frameworkResults) {
      for (const warning of item.warningIssues ?? []) {
        console.log(
          `[codegen-benchmark] ${result.caseId}/${item.framework} warning ${warning.code}: ${warning.message}`,
        );
      }
    }
  }
}

async function main() {
  const mode = parseMode();
  const generatedAt = new Date().toISOString();
  const cases = filterCases(getCodegenBenchmarkCases());
  const frameworks = parseFrameworks();
  const concurrency = parseConcurrency();
  const fastPath = parseFastPath();
  const startedAt = Date.now();
  mkdirSync(outputDir, { recursive: true });

  if (mode === 'model') {
    const provider = parseProvider();
    const model = readArg('model') ?? process.env.CODEGEN_BENCHMARK_MODEL;
    if (!model?.trim()) {
      throw new Error('Model benchmark requires --model=<model> or CODEGEN_BENCHMARK_MODEL.');
    }
    const maxRepairAttempts = Number(
      readArg('max-repair-attempts') ?? process.env.CODEGEN_BENCHMARK_MAX_REPAIR_ATTEMPTS ?? 2,
    );
    const { createChatCompletionText } = await import('../server/utils/server-codegen-provider');
    const results = await runCodegenBenchmarkModel({
      provider,
      model,
      cases,
      frameworks,
      maxRepairAttempts,
      concurrency,
      fastPath,
      collectText: (system, message, modelName, providerName, abortSignal) =>
        createChatCompletionText({
          system,
          message,
          model: modelName,
          provider: providerName,
          abortSignal,
        }),
    });
    const outputPath = resolve(
      outputDir,
      [
        'model-baseline',
        fastPath ? `fast-path-${fastPath}` : '',
        concurrency ? `concurrency-${concurrency}` : '',
      ]
        .filter(Boolean)
        .join('-') + '.json',
    );
    const output = createCodegenBenchmarkOutput({
      mode,
      generatedAt,
      frameworks,
      provider,
      model,
      concurrency,
      fastPath,
      totalDurationMs: Date.now() - startedAt,
      results,
    });
    writeFileSync(outputPath, `${JSON.stringify(output, null, 2)}\n`);
    printSummary(outputPath, results);
    return;
  }

  const results = runCodegenBenchmarkBaseline({ cases, frameworks });
  const outputPath = resolve(outputDir, 'baseline.json');
  const output = createCodegenBenchmarkOutput({
    mode,
    generatedAt,
    frameworks,
    totalDurationMs: Date.now() - startedAt,
    results,
  });
  writeFileSync(outputPath, `${JSON.stringify(output, null, 2)}\n`);
  printSummary(outputPath, results);
}

await main();
