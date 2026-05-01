#!/usr/bin/env bun
/**
 * Quick inline diagnostic — prints character + 4:1-token-estimate sizes
 * for every (variant, difficulty) combination of `buildSystemPrompt`.
 * Run when tuning the elements-cookbook diet to confirm the gate is
 * actually shaving the bytes the test floor predicts:
 *
 *   bun scripts/ab-corpus/measure-prompt-sizes.ts
 *
 * Not wired into the harness — purely for human inspection. The
 * authoritative regression guard is build-prompt.test.ts (>10kb floor).
 */

import { buildSystemPrompt } from './build-prompt';

const cases: Array<{ label: string; variant: 'B' | 'T'; difficulty?: 'obvious' | 'optional' | 'composite' }> = [
  { label: 'B (any difficulty)                ', variant: 'B' },
  { label: 'T + difficulty="obvious"          ', variant: 'T', difficulty: 'obvious' },
  { label: 'T + difficulty="optional"         ', variant: 'T', difficulty: 'optional' },
  { label: 'T + difficulty="composite"        ', variant: 'T', difficulty: 'composite' },
  { label: 'T + (no difficulty, safe default) ', variant: 'T' },
];

const tObvious = buildSystemPrompt('T', { difficulty: 'obvious' }).system.length;
const tComposite = buildSystemPrompt('T', { difficulty: 'composite' }).system.length;
const savings = tComposite - tObvious;

process.stdout.write(`Variant / Difficulty | Chars   | ~Tokens (chars/4)\n`);
process.stdout.write(`---------------------+---------+------------------\n`);
for (const c of cases) {
  const built = buildSystemPrompt(c.variant, c.difficulty ? { difficulty: c.difficulty } : {});
  const len = built.system.length;
  process.stdout.write(`${c.label} | ${String(len).padStart(7)} | ${String(Math.round(len / 4)).padStart(6)}\n`);
}
process.stdout.write(`\nObvious-vs-composite savings: ${savings} chars (~${Math.round(savings / 4)} tokens)\n`);
