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
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseModelOutput, scoreRun, aggregate, type ScoreRow } from '@zseven-w/pen-ai-skills';
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
  };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a === '--dry-run') args.dryRun = true;
    else if (a === '--models') args.models = (argv[++i] ?? '').split(',').filter(Boolean);
    else if (a === '--only') args.only = argv[++i];
    else if (a === '--out') args.outDir = argv[++i] ?? args.outDir;
    else if (a === '--help' || a === '-h') {
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
    `Usage: bun scripts/ab-corpus/run.ts [--dry-run] [--models ID,ID,...] [--only prompt-id] [--out DIR]\n`,
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
    'ab-v0',
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
        try {
          raw = args.dryRun ? await stubModelCall(call) : await realModelCall(call);
        } catch (err) {
          // Network / subprocess failure → treat as garbage so the
          // run still scores (M1=false, routing='garbage' for obvious
          // treatment). Beats aborting a 96-run sweep over one
          // transient failure.
          raw = `__HARNESS_ERROR__: ${err instanceof Error ? err.message : String(err)}`;
        }
        const parsed = parseModelOutput(raw);
        const row = await scoreRun({
          prompt,
          parsed,
          apply: applyToFreshDoc,
          model,
          variant,
        });
        rows.push(row);
      }
    }
    process.stderr.write(`  · ${prompt.id}\n`);
  }

  writeJsonl(join(args.outDir, 'scores.jsonl'), rows);
  const report = aggregate(rows);
  const { mdPath, jsonPath } = writeReport(args.outDir, report);
  process.stderr.write(`\nReport: ${mdPath}\n`);
  process.stderr.write(`JSON:   ${jsonPath}\n`);
  process.stderr.write(`Scores: ${join(args.outDir, 'scores.jsonl')}\n`);
}

function writeJsonl(path: string, rows: ScoreRow[]): void {
  const body = rows.map((r) => JSON.stringify(r)).join('\n') + '\n';
  mkdirSync(dirname(path), { recursive: true });
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  require('node:fs').writeFileSync(path, body, 'utf-8');
}

main().catch((err) => {
  process.stderr.write(`\nFATAL: ${err instanceof Error ? err.stack : String(err)}\n`);
  process.exit(1);
});
