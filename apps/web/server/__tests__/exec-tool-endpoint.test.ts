import { describe, it, expect, beforeEach, vi } from 'vitest';
import type { PenDocument } from '../../src/types/pen';

// Mock h3 before importing the handler so `defineEventHandler` returns
// the inner function as-is (testable directly, no real h3 runtime).
vi.mock('h3', () => ({
  defineEventHandler: <T extends (...args: unknown[]) => unknown>(fn: T): T => fn,
  readBody: vi.fn(),
  setResponseStatus: vi.fn(),
}));

// Mock server-logger to a no-op so tests don't spam the console.
vi.mock('../utils/server-logger', () => ({
  serverLog: {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  },
}));

import { readBody, setResponseStatus } from 'h3';
import execTool from '../api/mcp/exec-tool.post';
import { clearSyncState, setSyncDocument, getSyncDocument } from '../utils/mcp-sync-state';

type AnyEvent = Record<string, unknown>;

/**
 * Minimal doc with one root frame so parent_id resolution has
 * something to find.
 */
function seedDoc(extraChildren: Array<{ id: string; [k: string]: unknown }> = []): PenDocument {
  const doc: PenDocument = {
    version: '1.0.0',
    children: [],
    pages: [
      {
        id: 'p1',
        name: 'Page 1',
        children: [
          {
            id: 'root-1',
            type: 'frame',
            name: 'Root',
            width: 375,
            height: 812,
            layout: 'vertical',
            children: extraChildren as never,
          },
        ],
      },
    ],
  };
  setSyncDocument(doc);
  return doc;
}

describe('POST /api/mcp/exec-tool', () => {
  const event: AnyEvent = {};

  beforeEach(() => {
    clearSyncState();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockReset();
    (setResponseStatus as unknown as ReturnType<typeof vi.fn>).mockReset();
  });

  it('rejects missing body with 400', async () => {
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(null);
    const res = await execTool(event as never);
    expect(res.ok).toBe(false);
    expect(setResponseStatus).toHaveBeenCalledWith(event, 400);
  });

  it('rejects unknown tool name with 404 + coverage list in error', async () => {
    seedDoc();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      name: 'add_fictional_v999',
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(false);
    if (!res.ok) {
      expect(res.error).toContain('add_fictional_v999');
      expect(res.error).toContain('add_card_row_v0');
    }
    expect(setResponseStatus).toHaveBeenCalledWith(event, 404);
  });

  it('rejects real filePath (element-tool route) with 501 — live canvas only', async () => {
    seedDoc();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      name: 'add_heading_v0',
      arguments: { content: 'Hi', filePath: '/tmp/saved.op' },
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.error).toContain('/tmp/saved.op');
    expect(setResponseStatus).toHaveBeenCalledWith(event, 501);
  });

  it('accepts filePath="live://canvas" as the default target', async () => {
    seedDoc();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      name: 'add_heading_v0',
      arguments: { content: 'Hi', filePath: 'live://canvas' },
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(true);
  });

  it('rejects invalid pageId with 404', async () => {
    seedDoc();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      name: 'add_heading_v0',
      arguments: { content: 'Hi', pageId: 'nonexistent-page' },
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.error).toContain('nonexistent-page');
    expect(setResponseStatus).toHaveBeenCalledWith(event, 404);
  });

  it('rejects invalid parent_id with 404', async () => {
    seedDoc();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      name: 'add_heading_v0',
      arguments: { content: 'Hi', parent_id: 'does-not-exist' },
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.error).toContain('does-not-exist');
    expect(setResponseStatus).toHaveBeenCalledWith(event, 404);
  });

  it('rejects invalid default_parent_id (when parent_id absent) with 404', async () => {
    seedDoc();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      name: 'add_heading_v0',
      arguments: { content: 'Hi' },
      default_parent_id: 'stale-id',
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.error).toContain('stale-id');
    expect(setResponseStatus).toHaveBeenCalledWith(event, 404);
  });

  it('happy path: valid element-tool call mutates sync-state + returns doc', async () => {
    seedDoc();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      name: 'add_heading_v0',
      arguments: { content: 'Welcome' },
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(true);
    if (res.ok) {
      expect(res.insertedNodeId).toBeTruthy();
      expect(res.insertedNodeIds).toEqual([res.insertedNodeId]);
      // sync-state now holds the updated doc
      const { doc } = getSyncDocument();
      expect(doc).not.toBeNull();
      // Element tool inserted at first page root when no parent_id;
      // should see at least one new child beyond the seeded root frame.
      const firstPage = (doc as PenDocument).pages?.[0];
      expect((firstPage?.children ?? []).length).toBeGreaterThanOrEqual(1);
    }
  });

  it('happy path: parent_id targets seeded node + respects hierarchy', async () => {
    // Seed with a named container to insert INTO.
    const doc: PenDocument = {
      version: '1.0.0',
      children: [],
      pages: [
        {
          id: 'p1',
          name: 'Page 1',
          children: [
            {
              id: 'container-1',
              type: 'frame',
              name: 'Container',
              width: 375,
              height: 400,
              layout: 'vertical',
              children: [],
            },
          ],
        },
      ],
    };
    setSyncDocument(doc);
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      name: 'add_heading_v0',
      arguments: { content: 'Nested', parent_id: 'container-1' },
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(true);
    if (res.ok) {
      const { doc: updated } = getSyncDocument();
      const page = (updated as PenDocument).pages?.[0];
      const container = page?.children?.[0] as { children?: Array<{ id: string }> };
      expect(container.children).toBeDefined();
      expect(container.children?.length).toBe(1);
      expect(container.children?.[0].id).toBe(res.insertedNodeId);
    }
  });

  it('DSL route: rejects missing doc with 409', async () => {
    clearSyncState();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      dsl: 'x=I(null, {"type":"frame","name":"X"})',
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(false);
    expect(setResponseStatus).toHaveBeenCalledWith(event, 409);
  });

  it('DSL route: happy path runs runBatchDesignDsl + returns doc', async () => {
    seedDoc();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      dsl: 'node=I(null, {"type":"frame","name":"DSL Frame","width":200,"height":100})',
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(true);
    if (res.ok) {
      expect(res.insertedNodeIds.length).toBeGreaterThanOrEqual(1);
    }
  });

  it('DSL route: collects errors → 400 with per-line preview', async () => {
    seedDoc();
    (readBody as unknown as ReturnType<typeof vi.fn>).mockResolvedValue({
      dsl: 'garbage nonsense that will not parse',
    });
    const res = await execTool(event as never);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.error.toLowerCase()).toContain('failing operation');
    expect(setResponseStatus).toHaveBeenCalledWith(event, 400);
  });
});
