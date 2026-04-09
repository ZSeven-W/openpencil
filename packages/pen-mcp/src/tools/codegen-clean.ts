// packages/pen-mcp/src/tools/codegen-clean.ts

import { getSyncUrl } from '../document-manager';

export interface CodegenCleanParams {
  planId: string;
}

export async function handleCodegenClean(params: CodegenCleanParams): Promise<string> {
  const syncUrl = await getSyncUrl();
  if (!syncUrl) {
    return JSON.stringify({ ok: true, deleted: false });
  }

  const res = await fetch(`${syncUrl}/api/mcp/codegen/plan/${encodeURIComponent(params.planId)}`, {
    method: 'DELETE',
  });

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`codegen_clean failed: ${text}`);
  }
  return JSON.stringify(await res.json(), null, 2);
}
