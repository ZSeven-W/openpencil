// apps/cli/src/commands/read-nodes.ts

import { resolveDocPath, getSyncUrl } from '@zseven-w/pen-mcp';
import { output, outputError } from '../output';

interface Flags {
  file?: string;
  page?: string;
  depth?: string;
  vars?: boolean;
}

export async function cmdReadNodes(args: string[], flags: Flags): Promise<void> {
  const nodeIds = args[0]
    ? args[0]
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean)
    : undefined;
  const depth = flags.depth !== undefined ? parseInt(flags.depth, 10) : undefined;

  const syncUrl = await getSyncUrl();
  if (!syncUrl) {
    outputError('No running OpenPencil instance found. Start the app first with: op start');
  }

  try {
    const res = await fetch(`${syncUrl}/api/mcp/read-nodes`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        nodeIds,
        depth,
        pageId: flags.page,
        filePath: flags.file ? resolveDocPath(flags.file) : undefined,
        includeVariables: flags.vars,
      }),
    });
    if (!res.ok) {
      outputError(await res.text());
    }
    output(await res.json());
  } catch (err) {
    outputError(err instanceof Error ? err.message : String(err));
  }
}
