/**
 * Golden parity dump — S1 `op-design-lint` parity oracle.
 *
 * Reads every hand-authored `PenDocument` fixture under
 * `crates/op-design-lint/tests/fixtures/docs/`, runs the TS `detectAllIssues`
 * (the parity oracle), and writes the resulting `Issue[]` as pretty JSON to
 * `crates/op-design-lint/tests/fixtures/golden/<same-name>.json`.
 *
 * The Rust `tests/parity.rs` then asserts `op_design_lint::detect_all`
 * structurally matches each committed golden file. A CI drift-guard job
 * re-runs this script and fails if the regenerated golden differs from the
 * committed copy (spec §8, Risk R4).
 *
 * Run:  bun run tools/dump-diagnostics-golden.ts
 *
 * Each fixture is a single-page `PenDocument`; its lone root node
 * (`doc.children[0]`) is the subtree `detectAllIssues` walks — matching how
 * the Rust parity test feeds `detect_all`.
 */
import { readdirSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { detectAllIssues } from '@zseven-w/pen-ai-skills';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';

const here = dirname(fileURLToPath(import.meta.url));
const fixturesRoot = join(here, '..', 'crates', 'op-design-lint', 'tests', 'fixtures');
const docsDir = join(fixturesRoot, 'docs');
const goldenDir = join(fixturesRoot, 'golden');

mkdirSync(goldenDir, { recursive: true });

const fixtureFiles = readdirSync(docsDir)
  .filter((name) => name.endsWith('.json'))
  .sort();

if (fixtureFiles.length === 0) {
  throw new Error(`no fixture docs found under ${docsDir}`);
}

let total = 0;
for (const file of fixtureFiles) {
  const docPath = join(docsDir, file);
  const doc = JSON.parse(readFileSync(docPath, 'utf8')) as PenDocument;

  const root = doc.children?.[0] as PenNode | undefined;
  if (!root) {
    throw new Error(`fixture ${file} has no root node in doc.children[0]`);
  }

  const issues = detectAllIssues(root, doc);
  const goldenPath = join(goldenDir, file);
  // Trailing newline so the file is POSIX-clean and `git diff` stays quiet.
  writeFileSync(goldenPath, JSON.stringify(issues, null, 2) + '\n', 'utf8');
  total += issues.length;
  console.log(`${file}: ${issues.length} issue(s)`);
}

console.log(`\nwrote ${fixtureFiles.length} golden file(s), ${total} issue(s) total`);
