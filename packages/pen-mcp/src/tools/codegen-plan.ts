// packages/pen-mcp/src/tools/codegen-plan.ts

import type { CodePlanFromAI } from '@zseven-w/pen-types';
import { getSyncUrl } from '../document-manager';

export interface CodegenPlanParams {
  plan: CodePlanFromAI;
  filePath?: string;
  pageId?: string;
}

export async function handleCodegenPlan(params: CodegenPlanParams): Promise<string> {
  const syncUrl = await getSyncUrl();
  if (!syncUrl) {
    throw new Error(
      'codegen_plan requires a running App (live canvas mode). Pipeline state is stored in App memory.',
    );
  }

  const res = await fetch(`${syncUrl}/api/mcp/codegen/plan`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      plan: params.plan,
      filePath: params.filePath,
      pageId: params.pageId,
    }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`codegen_plan failed: ${text}`);
  }
  return JSON.stringify(await res.json(), null, 2);
}
