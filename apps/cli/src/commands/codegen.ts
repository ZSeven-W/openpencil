// apps/cli/src/commands/codegen.ts

import { resolveDocPath, getSyncUrl } from '@zseven-w/pen-mcp';
import { outputError, resolveArg, parseJsonArg } from '../output';

interface GlobalFlags {
  file?: string;
  page?: string;
}

async function requireSyncUrl(): Promise<string | null> {
  const syncUrl = await getSyncUrl();
  if (!syncUrl) {
    outputError('No running OpenPencil instance found. Start the app first with: op start');
  }
  return syncUrl;
}

export async function cmdCodegenPlan(args: string[], flags: GlobalFlags): Promise<void> {
  const raw = await resolveArg(args[0]);
  if (!raw) {
    outputError('Usage: op codegen:plan <plan-json|@file|->');
  }
  const plan = await parseJsonArg(raw);

  const syncUrl = await requireSyncUrl();
  if (!syncUrl) return;

  try {
    const res = await fetch(`${syncUrl}/api/mcp/codegen/plan`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        plan,
        filePath: flags.file ? resolveDocPath(flags.file) : undefined,
        pageId: flags.page,
      }),
    });
    if (!res.ok) {
      outputError(await res.text());
    }
    process.stdout.write(JSON.stringify(await res.json(), null, 2));
  } catch (err) {
    outputError(err instanceof Error ? err.message : String(err));
  }
}

export async function cmdCodegenSubmit(args: string[], _flags: GlobalFlags): Promise<void> {
  const planId = args[0];
  const raw = await resolveArg(args[1]);
  if (!planId || !raw) {
    outputError('Usage: op codegen:submit <planId> <chunk-result|@file|->');
  }
  const result = await parseJsonArg(raw);

  const syncUrl = await requireSyncUrl();
  if (!syncUrl) return;

  try {
    const res = await fetch(`${syncUrl}/api/mcp/codegen/submit`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ planId, result }),
    });
    if (!res.ok) {
      outputError(await res.text());
    }
    process.stdout.write(JSON.stringify(await res.json(), null, 2));
  } catch (err) {
    outputError(err instanceof Error ? err.message : String(err));
  }
}

export async function cmdCodegenAssemble(
  args: string[],
  flags: GlobalFlags & { framework?: string },
): Promise<void> {
  const planId = args[0];
  if (!planId) {
    outputError('Usage: op codegen:assemble <planId> [--framework react]');
  }
  const framework = flags.framework || 'react';

  const syncUrl = await requireSyncUrl();
  if (!syncUrl) return;

  try {
    const res = await fetch(
      `${syncUrl}/api/mcp/codegen/assemble/${encodeURIComponent(planId)}?framework=${encodeURIComponent(framework)}`,
    );
    if (!res.ok) {
      outputError(await res.text());
    }
    process.stdout.write(JSON.stringify(await res.json(), null, 2));
  } catch (err) {
    outputError(err instanceof Error ? err.message : String(err));
  }
}

export async function cmdCodegenClean(args: string[], _flags: GlobalFlags): Promise<void> {
  const planId = args[0];
  if (!planId) {
    outputError('Usage: op codegen:clean <planId>');
  }

  const syncUrl = await requireSyncUrl();
  if (!syncUrl) return;

  try {
    const res = await fetch(`${syncUrl}/api/mcp/codegen/plan/${encodeURIComponent(planId)}`, {
      method: 'DELETE',
    });
    if (!res.ok) {
      outputError(await res.text());
    }
    process.stdout.write(JSON.stringify(await res.json(), null, 2));
  } catch (err) {
    outputError(err instanceof Error ? err.message : String(err));
  }
}
