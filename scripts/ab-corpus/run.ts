#!/usr/bin/env bun
/**
 * Element-tools A/B corpus eval — harness entry point.
 *
 * Usage:
 *   bun scripts/ab-corpus/run.ts --dry-run                   # stub model, exercises pipeline
 *   bun scripts/ab-corpus/run.ts --dry-run --out ./tmp-out   # custom output dir
 *   bun scripts/ab-corpus/run.ts --models minimax-m2,glm-5   # real run (needs API keys)
 *   bun scripts/ab-corpus/run.ts --only mobile-filter-chips  # single prompt
 *
 * Spec: ~/workspace/openpencil-docs/superpowers/plans/2026-04-20-element-tools-ab-corpus.md
 */

import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  parseModelOutput,
  scoreRun,
  aggregate,
  type ScoreRow,
  type TokenUsage,
} from '@zseven-w/pen-ai-skills';
// Node-only: pulls in `node:fs`, so it's NOT re-exported from the
// package barrel (which must stay browser-safe for the embedded
// orchestrator's design-parser). Package.json `exports` only declares
// the main entry, so sub-path imports via the package name fail at
// runtime — use a relative path to the source file instead. Harness
// runs under Bun from the repo root so this path is stable.
import { loadCorpus } from '../../packages/pen-ai-skills/src/corpus/corpus-loader';
import { applyToFreshDoc } from './apply';
import { stubModelCall, type ModelCall } from './stub-model';
import { realModelCall } from './real-model';
import { writeReport } from './write-report';

interface CliArgs {
  dryRun: boolean;
  models: string[];
  only?: string;
  outDir: string;
  corpus: 'ab-v0' | 'ab-v1' | 'ab-v3';
}

function parseArgs(argv: string[]): CliArgs {
  const args: CliArgs = {
    dryRun: false,
    // Default matches plan §2 after the 2026-04-20 update: user supplied
    // MiniMax M2.7 as the weak-model candidate and Codex CLI (GPT-5.4)
    // as the reference ceiling. Claude / GLM / KIMI are not in the
    // default set until keys / adapters land.
    models: ['gpt-5.4', 'minimax-m2.7'],
    outDir: defaultOutDir(),
    corpus: 'ab-v0',
  };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a === '--dry-run') args.dryRun = true;
    else if (a === '--models') args.models = (argv[++i] ?? '').split(',').filter(Boolean);
    else if (a === '--only') args.only = argv[++i];
    else if (a === '--out') args.outDir = argv[++i] ?? args.outDir;
    else if (a === '--corpus') {
      const v = argv[++i];
      if (v !== 'ab-v0' && v !== 'ab-v1' && v !== 'ab-v3') {
        process.stderr.write(`--corpus must be 'ab-v0', 'ab-v1', or 'ab-v3', got: ${v}\n`);
        process.exit(1);
      }
      args.corpus = v;
    } else if (a === '--help' || a === '-h') {
      printUsage();
      process.exit(0);
    }
  }
  return args;
}

function defaultOutDir(): string {
  const ts = new Date().toISOString().replace(/[:.]/g, '-');
  return join(fileURLToPath(new URL('.', import.meta.url)), 'runs', ts);
}

function printUsage(): void {
  process.stderr.write(
    `Usage: bun scripts/ab-corpus/run.ts [--dry-run] [--models ID,ID,...] [--only prompt-id] [--out DIR] [--corpus ab-v0|ab-v1|ab-v3]\n`,
  );
}

async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2));
  const corpusDir = join(
    fileURLToPath(new URL('.', import.meta.url)),
    '..',
    '..',
    'packages',
    'pen-ai-skills',
    'corpus',
    args.corpus,
  );
  const prompts = loadCorpus(corpusDir).filter((p) => (args.only ? p.id === args.only : true));
  if (prompts.length === 0) {
    process.stderr.write(
      `No prompts matched (--only=${args.only ?? 'none'}). Corpus dir: ${corpusDir}\n`,
    );
    process.exit(1);
  }

  mkdirSync(args.outDir, { recursive: true });
  process.stderr.write(
    `Running ${prompts.length} prompts × ${args.models.length} models × 2 variants = ${prompts.length * args.models.length * 2} runs\n`,
  );
  process.stderr.write(`Output: ${args.outDir}\n`);
  process.stderr.write(`Mode: ${args.dryRun ? 'DRY-RUN (stub model)' : 'LIVE'}\n\n`);

  // Stream scores to scores.jsonl as each run completes so a
  // kill-at-minute-30 (hung API call, accidental ^C) doesn't lose
  // all results. Final report.md still requires a full sweep for
  // aggregate counts — but scores.jsonl alone is useful for any
  // partial-run analysis.
  const scoresPath = join(args.outDir, 'scores.jsonl');
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const fs = require('node:fs') as typeof import('node:fs');
  // Truncate on start so a re-run into the same dir overwrites.
  fs.writeFileSync(scoresPath, '', 'utf-8');

  const rows: ScoreRow[] = [];
  for (const prompt of prompts) {
    for (const model of args.models) {
      for (const variant of ['B', 'T'] as const) {
        const call: ModelCall = {
          model,
          prompt,
          variant,
          systemPrompt: '<resolved in dispatcher>',
          userPrompt: prompt.prompt,
        };
        let raw: string;
        let usage: TokenUsage = { promptTokens: 0, completionTokens: 0 };
        try {
          const res = args.dryRun ? await stubModelCall(call) : await realModelCall(call);
          raw = res.content;
          usage = res.usage;
        } catch (err) {
          // Network / subprocess failure → treat as garbage so the
          // run still scores (M1=false, routing='garbage' for obvious
          // treatment). Beats aborting a 96-run sweep over one
          // transient failure. usage stays 0/0 — aggregate skips zero
          // rows when computing token averages so a flaky cell
          // doesn't drag the model's average down to ~0.
          raw = `__HARNESS_ERROR__: ${err instanceof Error ? err.message : String(err)}`;
        }
        const parsed = parseModelOutput(raw);
        const row = await scoreRun({
          prompt,
          parsed,
          apply: applyToFreshDoc,
          model,
          variant,
          usage,
        });
        rows.push(row);
        // Append this row to scores.jsonl immediately — durable
        // partial state for crash recovery.
        fs.appendFileSync(scoresPath, JSON.stringify(row) + '\n', 'utf-8');
      }
    }
    process.stderr.write(`  · ${prompt.id}\n`);
  }

  const report = aggregate(rows);
  const { mdPath, jsonPath } = writeReport(args.outDir, report);
  process.stderr.write(`\nReport: ${mdPath}\n`);
  process.stderr.write(`JSON:   ${jsonPath}\n`);
  process.stderr.write(`Scores: ${join(args.outDir, 'scores.jsonl')}\n`);
}

main().catch((err) => {
  process.stderr.write(`\nFATAL: ${err instanceof Error ? err.stack : String(err)}\n`);
  process.exit(1);
});
