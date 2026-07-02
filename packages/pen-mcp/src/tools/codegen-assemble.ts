// packages/pen-mcp/src/tools/codegen-assemble.ts

import type { Framework } from '@zseven-w/pen-types';
import { getSyncUrl } from '../document-manager';

export interface CodegenAssembleParams {
  planId: string;
  framework: Framework;
}

export async function handleCodegenAssemble(params: CodegenAssembleParams): Promise<string> {
  const syncUrl = await getSyncUrl();
  if (!syncUrl) {
    throw new Error('codegen_assemble requires a running App (live canvas mode)');
  }

  const res = await fetch(
    `${syncUrl}/api/mcp/codegen/assemble/${encodeURIComponent(params.planId)}?framework=${params.framework}`,
  );

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`codegen_assemble failed: ${text}`);
  }
  return JSON.stringify(await res.json(), null, 2);
}
