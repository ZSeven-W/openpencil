// 跑出 TS planner-prompt builder 的 golden 输出,供 Rust parity.rs 比对。
// 用法:bun run crates/op-orchestrator/tools/dump-planner-golden.ts
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { buildCompactPlanningPrompt } from '../../../apps/web/src/services/ai/orchestrator-prompt-optimizer';

const dir = join(import.meta.dir, '../tests/planner-golden');
const cases = JSON.parse(readFileSync(join(dir, 'cases.json'), 'utf8')) as Array<{
  name: string;
  fn: string;
  prompt: string;
}>;

let written = 0;
for (const c of cases) {
  if (c.fn === 'compact') {
    const { systemPrompt, userPrompt, selectedStyleGuideName } = buildCompactPlanningPrompt(
      c.prompt,
      undefined,
      undefined,
    );
    writeFileSync(
      join(dir, `${c.name}.json`),
      JSON.stringify({ system: systemPrompt, userPrompt, selectedStyleGuideName }, null, 2) + '\n',
    );
    written += 1;
  }
}
console.log(`dumped ${written} golden files`);
