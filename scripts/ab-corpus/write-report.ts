/**
 * Render an aggregated Report as both a markdown file (human review) and
 * a json file (machine-diffable + fed to downstream plotting). Only
 * writes; computes nothing — aggregation is the scorer's job.
 */

import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import type { Report } from '@zseven-w/pen-ai-skills';

export function writeReport(outDir: string, report: Report): { mdPath: string; jsonPath: string } {
  const jsonPath = join(outDir, 'report.json');
  writeFileSync(jsonPath, JSON.stringify(report, null, 2), 'utf-8');
  const mdPath = join(outDir, 'report.md');
  writeFileSync(mdPath, renderMarkdown(report), 'utf-8');
  return { mdPath, jsonPath };
}

function renderMarkdown(r: Report): string {
  const lines: string[] = [];
  lines.push(`# Element Tools A/B Report`, '');
  lines.push(`Generated: ${r.generatedAt}`);
  lines.push(`Total runs: ${r.totalRuns}`, '');

  lines.push(`## By model`, '');
  lines.push(
    '| Model | N (B) | N (T) | M1 B | M1 T | Δ M1 (pp) | M3 B | M3 T | Δ M3 (pp) | Right tool | Wrong tool | Fallback | Garbage |',
  );
  lines.push(
    '|-------|-------|-------|------|------|-----------|------|------|-----------|------------|------------|----------|---------|',
  );
  for (const m of r.byModel) {
    lines.push(
      `| ${m.model} | ${m.runCountBaseline} | ${m.runCountTreatment} | ${pct(m.m1_baseline)} | ${pct(m.m1_treatment)} | ${signed(m.m1_delta_pp)} | ${pct(m.m3_baseline)} | ${pct(m.m3_treatment)} | ${signed(m.m3_delta_pp)} | ${pct(m.m5_right_tool)} | ${pct(m.m5_wrong_tool)} | ${pct(m.m5_fallback)} | ${pct(m.m5_garbage)} |`,
    );
  }
  lines.push('');
  lines.push(
    '_Right / Wrong / Fallback / Garbage = routing breakdown on the `difficulty: obvious` treatment subset; all four rates sum to 100%. Right = tool named in `expected_tool_if_any`. Wrong = routed to a different `add_*_v0` (schema constraint worked, intent match failed). Fallback = emitted `batch_design` DSL. Garbage = output unparseable — counted here so right-tool never looks rosy when most outputs fail to parse._',
    '',
  );

  lines.push(`## By category`, '');
  lines.push('| Category | M1 B | M1 T | Δ M1 (pp) |');
  lines.push('|----------|------|------|-----------|');
  for (const c of r.byCategory) {
    lines.push(
      `| ${c.category} | ${pct(c.m1_baseline)} | ${pct(c.m1_treatment)} | ${signed(c.m1_delta_pp)} |`,
    );
  }
  lines.push('');

  lines.push(`## Tool usage (treatment arm only)`, '');
  if (r.byTool.length === 0) {
    lines.push(
      "_No element tools were invoked in the treatment arm. Check decision-tree prompt phrasing and the weak model's tool-use capabilities._",
    );
  } else {
    lines.push('| Tool | Invocations | Successful |');
    lines.push('|------|-------------|------------|');
    for (const t of r.byTool) {
      lines.push(`| \`${t.tool}\` | ${t.invocations} | ${t.successfulInvocations} |`);
    }
  }
  lines.push('');
  return lines.join('\n');
}

function pct(n: number): string {
  if (Number.isNaN(n)) return '—';
  return `${(n * 100).toFixed(1)}%`;
}
function signed(n: number): string {
  if (Number.isNaN(n)) return '—';
  const sign = n > 0 ? '+' : '';
  return `${sign}${n.toFixed(1)}`;
}
