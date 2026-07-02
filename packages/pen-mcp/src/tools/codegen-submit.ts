// packages/pen-mcp/src/tools/codegen-submit.ts

import type { ChunkResult } from '@zseven-w/pen-types';
import { getSyncUrl } from '../document-manager';

export interface CodegenSubmitParams {
  planId: string;
  result: ChunkResult;
  status?: 'failed' | 'skipped';
}

export async function handleCodegenSubmit(params: CodegenSubmitParams): Promise<string> {
  const syncUrl = await getSyncUrl();
  if (!syncUrl) {
    throw new Error('codegen_submit_chunk requires a running App (live canvas mode)');
  }

  const res = await fetch(`${syncUrl}/api/mcp/codegen/submit`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      planId: params.planId,
      result: params.result,
      status: params.status,
    }),
  });

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`codegen_submit_chunk failed: ${text}`);
  }
  return JSON.stringify(await res.json(), null, 2);
}
