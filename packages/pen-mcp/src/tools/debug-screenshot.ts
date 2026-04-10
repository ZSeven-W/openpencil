import type { ToolContent } from '../server';
import { getSyncUrl } from '../document-manager';

export interface DebugScreenshotParams {
  target: 'node' | 'root';
  nodeId?: string;
  padding?: number;
  dpr?: number;
  timeoutMs?: number;
}

export async function handleScreenshot(args: DebugScreenshotParams): Promise<ToolContent[]> {
  if (args.target === 'node' && !args.nodeId) {
    throw new Error('target="node" requires nodeId');
  }

  const syncUrl = await getSyncUrl();
  if (!syncUrl) {
    throw new Error(
      'No live canvas available. Ensure an Electron window or /editor tab is running.',
    );
  }

  const body = {
    bounds: args.target === 'root' ? 'root' : undefined,
    nodeId: args.target === 'node' ? args.nodeId : undefined,
    opts: {
      ...(args.padding !== undefined ? { padding: args.padding } : {}),
      ...(args.dpr !== undefined ? { dpr: args.dpr } : {}),
    },
    timeoutMs: Math.min(args.timeoutMs ?? 15000, 60000),
  };

  const res = await fetch(`${syncUrl}/api/mcp/screenshot`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new Error(
      `Screenshot request failed: HTTP ${res.status} ${res.statusText}${text ? ` — ${text}` : ''}`,
    );
  }

  const data = (await res.json()) as {
    success: boolean;
    pngBase64?: string;
    actualBounds?: { x: number; y: number; w: number; h: number };
    error?: string;
  };

  if (!data.success || !data.pngBase64) {
    throw new Error(`Renderer reported failure: ${data.error ?? 'unknown error'}`);
  }

  return [
    {
      type: 'image',
      data: data.pngBase64,
      mimeType: 'image/png',
    },
    {
      type: 'text',
      text: JSON.stringify(
        {
          target: args.target,
          nodeId: args.nodeId,
          actualBounds: data.actualBounds,
          dpr: args.dpr,
        },
        null,
        2,
      ),
    },
  ];
}
