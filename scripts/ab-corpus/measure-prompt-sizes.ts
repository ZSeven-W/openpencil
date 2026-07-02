#!/usr/bin/env bun
/**
 * Quick inline diagnostic — prints character + 4:1-token-estimate sizes
 * for every (variant, difficulty, category) combination of
 * `buildSystemPrompt`. Run when tuning the elements diet to confirm
 * the gates are actually shaving the bytes the test floor predicts:
 *
 *   bun scripts/ab-corpus/measure-prompt-sizes.ts
 *
 * Not wired into the harness — purely for human inspection. The
 * authoritative regression guards are build-prompt.test.ts.
 */

import { buildSystemPrompt } from './build-prompt';

type Difficulty = 'obvious' | 'optional' | 'composite';
type Category = 'mobile' | 'dashboard' | 'landing';

const cases: Array<{
  label: string;
  variant: 'B' | 'T';
  difficulty?: Difficulty;
  category?: Category;
}> = [
  { label: 'B (any)                                  ', variant: 'B' },
  { label: 'T + difficulty=obvious                   ', variant: 'T', difficulty: 'obvious' },
  { label: 'T + difficulty=composite                 ', variant: 'T', difficulty: 'composite' },
  {
    label: 'T + difficulty=obvious + category=mobile ',
    variant: 'T',
    difficulty: 'obvious',
    category: 'mobile',
  },
  {
    label: 'T + difficulty=obvious + category=dash   ',
    variant: 'T',
    difficulty: 'obvious',
    category: 'dashboard',
  },
  {
    label: 'T + difficulty=obvious + category=landing',
    variant: 'T',
    difficulty: 'obvious',
    category: 'landing',
  },
  {
    label: 'T + difficulty=composite + category=mobile',
    variant: 'T',
    difficulty: 'composite',
    category: 'mobile',
  },
  {
    label: 'T + difficulty=composite + category=dash  ',
    variant: 'T',
    difficulty: 'composite',
    category: 'dashboard',
  },
  {
    label: 'T + difficulty=composite + category=land  ',
    variant: 'T',
    difficulty: 'composite',
    category: 'landing',
  },
];

process.stdout.write(`Variant / Difficulty / Category          | Chars   | ~Tokens\n`);
process.stdout.write(`-----------------------------------------+---------+---------\n`);
for (const c of cases) {
  const opts: { difficulty?: Difficulty; category?: Category } = {};
  if (c.difficulty) opts.difficulty = c.difficulty;
  if (c.category) opts.category = c.category;
  const built = buildSystemPrompt(c.variant, opts);
  const len = built.system.length;
  process.stdout.write(
    `${c.label} | ${String(len).padStart(7)} | ${String(Math.round(len / 4)).padStart(6)}\n`,
  );
}

const tCompositeFull = buildSystemPrompt('T', { difficulty: 'composite' }).system.length;
const tCompositeMobile = buildSystemPrompt('T', { difficulty: 'composite', category: 'mobile' })
  .system.length;
const tCompositeDash = buildSystemPrompt('T', { difficulty: 'composite', category: 'dashboard' })
  .system.length;
const tCompositeLanding = buildSystemPrompt('T', { difficulty: 'composite', category: 'landing' })
  .system.length;

process.stdout.write(
  `\nCategory savings on composite (the cookbook-loaded path):\n` +
    `  mobile:    ${tCompositeFull - tCompositeMobile} chars (~${Math.round((tCompositeFull - tCompositeMobile) / 4)} tokens)\n` +
    `  dashboard: ${tCompositeFull - tCompositeDash} chars (~${Math.round((tCompositeFull - tCompositeDash) / 4)} tokens)\n` +
    `  landing:   ${tCompositeFull - tCompositeLanding} chars (~${Math.round((tCompositeFull - tCompositeLanding) / 4)} tokens)\n`,
);
