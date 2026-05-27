import { describe, expect, it, vi } from 'vitest';
import type { Framework } from '@zseven-w/pen-types';
import type { GenerateCodeResult } from '../code-generation-pipeline';
import { generateCode } from '../code-generation-pipeline';
import { buildCodegenDesignIR } from '../codegen-design-ir';
import { buildCodegenFiles } from '../codegen-files';
import { buildCodegenQualityReport } from '../codegen-quality';
import {
  createCodegenBenchmarkOutput,
  getCodegenBenchmarkCases,
  runCodegenBenchmarkBaseline,
  runCodegenBenchmarkModel,
  runWithConcurrencyLimit,
} from '../codegen-benchmark';

describe('codegen benchmark baseline', () => {
  it('defines five fixed cases for production codegen evaluation', () => {
    const cases = getCodegenBenchmarkCases();

    expect(cases.map((item) => item.kind)).toEqual([
      'landing_page',
      'dashboard',
      'form_detail',
      'mobile_screen',
      'card_list',
    ]);
    expect(cases.every((item) => item.nodes.length > 0)).toBe(true);
  });

  it('records HTML, Vue, and UniApp quality metrics for every case', () => {
    const results = runCodegenBenchmarkBaseline({ now: () => 100 });

    expect(results).toHaveLength(5);
    for (const result of results) {
      expect(result.frameworkResults.map((item) => item.framework)).toEqual([
        'html',
        'vue',
        'uniapp',
      ]);
      for (const frameworkResult of result.frameworkResults) {
        expect(frameworkResult.fileCount).toBeGreaterThan(0);
        expect(frameworkResult.textCount).toBeGreaterThan(0);
        expect(frameworkResult.durationMs).toBe(0);
      }
    }
  });

  it('records provider, timing, repair, and quality metadata for model runs', async () => {
    const benchmarkCase = getCodegenBenchmarkCases()[0];
    const nowValues = [100, 260];
    const now = () => nowValues.shift() ?? 260;
    const generateCodeImpl = vi.fn(
      async (nodes, framework): Promise<GenerateCodeResult> => {
        const designIR = buildCodegenDesignIR(nodes);
        const htmlText = designIR.summary.textContent
          .map((text) => `<p>${text}</p>`)
          .join('');
        const code =
          framework === 'html'
            ? `<!doctype html><html><head><style>body{}</style></head><body><main>${htmlText}</main></body></html>`
            : `<template><main>${designIR.summary.textContent.join(' ')}</main></template><style scoped>main{}</style>`;
        const files = buildCodegenFiles({ framework: framework as Framework, code });
        return {
          code,
          degraded: false,
          assets: [],
          files,
          qualityReport: buildCodegenQualityReport({
            framework: framework as Framework,
            files,
            designIR,
            repaired: true,
          }),
          timing: {
            planningMs: 10,
            chunkMs: 20,
            assemblyMs: 30,
            qualityCheckMs: 5,
            repairMs: 15,
            providerMs: 75,
            providerCallCount: 4,
            providerCalls: [
              { stage: 'planning', durationMs: 10, attempt: 1, provider: 'anthropic', model: 'claude-test' },
              { stage: 'chunk', durationMs: 20, attempt: 1, provider: 'anthropic', model: 'claude-test' },
              { stage: 'assembly', durationMs: 30, attempt: 1, provider: 'anthropic', model: 'claude-test' },
              { stage: 'repair', durationMs: 15, attempt: 1, provider: 'anthropic', model: 'claude-test' },
            ],
            totalMs: 90,
          },
          repairAttempts: [
            {
              attempt: 1,
              issues: [],
              code,
              report: buildCodegenQualityReport({
                framework: framework as Framework,
                files,
                designIR,
                repaired: true,
              }),
              durationMs: 15,
            },
          ],
          designIR,
        };
      },
    ) as unknown as typeof generateCode;

    const results = await runCodegenBenchmarkModel({
      provider: 'anthropic',
      model: 'claude-test',
      cases: [benchmarkCase],
      frameworks: ['html'],
      fastPath: 'never',
      collectText: async () => '',
      generateCodeImpl,
      now,
    });
    const output = createCodegenBenchmarkOutput({
      mode: 'model',
      provider: 'anthropic',
      model: 'claude-test',
      frameworks: ['html'],
      concurrency: 1,
      fastPath: 'never',
      totalDurationMs: 160,
      results,
    });

    expect(results[0].frameworkResults[0]).toMatchObject({
      mode: 'model',
      provider: 'anthropic',
      model: 'claude-test',
      framework: 'html',
      status: 'succeeded',
      durationMs: 90,
      wallTimeMs: 160,
      qualityStatus: 'repaired',
      repairCount: 1,
      timing: expect.objectContaining({
        providerCallCount: 4,
        providerCalls: expect.arrayContaining([
          expect.objectContaining({ stage: 'repair', durationMs: 15 }),
        ]),
      }),
    });
    expect(output.summary).toMatchObject({
      totalRuns: 1,
      succeeded: 1,
      failed: 0,
      skipped: 0,
      repairCount: 1,
    });
    expect(output.concurrency).toBe(1);
    expect(output.fastPath).toBe('never');
    expect(generateCodeImpl).toHaveBeenCalledWith(
      expect.any(Array),
      'html',
      undefined,
      expect.any(Function),
      'claude-test',
      'anthropic',
      undefined,
      expect.objectContaining({ fastPath: 'never' }),
    );
    expect(output.summary.quality.repaired).toBe(1);
  });

  it('keeps warning details in benchmark results when quality passed', async () => {
    const benchmarkCase = getCodegenBenchmarkCases()[0];
    const generateCodeImpl = vi.fn(
      async (nodes, framework): Promise<GenerateCodeResult> => {
        const designIR = buildCodegenDesignIR(nodes);
        const code =
          '<!doctype html><html><head><style>body{}</style></head><body><main>Ship faster</main></body></html>';
        const files = buildCodegenFiles({ framework: framework as Framework, code });
        const report = buildCodegenQualityReport({
          framework: framework as Framework,
          files,
          designIR,
        });
        const warning = {
          code: 'placeholder_text',
          severity: 'warning' as const,
          message: 'Generated output contains placeholder text.',
          filePath: 'index.html',
        };
        return {
          code,
          degraded: false,
          assets: [],
          files,
          qualityReport: {
            ...report,
            issues: [warning],
            summary: {
              ...report.summary,
              warningCount: 1,
            },
          },
          timing: {
            assemblyMs: 10,
            providerMs: 10,
            providerCallCount: 1,
            providerCalls: [
              { stage: 'direct_generation', durationMs: 10, attempt: 1 },
            ],
            totalMs: 12,
          },
          pipelineMode: 'direct_generation',
          designIR,
        };
      },
    ) as unknown as typeof generateCode;

    const results = await runCodegenBenchmarkModel({
      provider: 'anthropic',
      model: 'claude-test',
      cases: [benchmarkCase],
      frameworks: ['html'],
      fastPath: 'auto',
      collectText: async () => '',
      generateCodeImpl,
      now: () => 100,
    });

    expect(results[0].frameworkResults[0]).toMatchObject({
      qualityStatus: 'passed',
      warningCount: 1,
      pipelineMode: 'direct_generation',
      warningIssues: [
        expect.objectContaining({
          code: 'placeholder_text',
          message: 'Generated output contains placeholder text.',
        }),
      ],
    });
  });

  it('limits model benchmark concurrency when requested', async () => {
    const started: string[] = [];
    const finished: string[] = [];
    let active = 0;
    let maxActive = 0;

    const results = await runWithConcurrencyLimit(['a', 'b', 'c'], 1, async (item) => {
      started.push(item);
      active += 1;
      maxActive = Math.max(maxActive, active);
      await Promise.resolve();
      active -= 1;
      finished.push(item);
      return item.toUpperCase();
    });

    expect(results).toEqual(['A', 'B', 'C']);
    expect(started).toEqual(['a', 'b', 'c']);
    expect(finished).toEqual(['a', 'b', 'c']);
    expect(maxActive).toBe(1);
  });

  it('records benchmark concurrency in output metadata', () => {
    const output = createCodegenBenchmarkOutput({
      mode: 'model',
      provider: 'anthropic',
      model: 'claude-test',
      frameworks: ['html'],
      concurrency: 1,
      results: [],
    });

    expect(output.concurrency).toBe(1);
  });
});
