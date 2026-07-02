import { describe, it, expect } from 'vitest';
// Import from the side-effect-free dispatch module, NOT the executable
// `server.ts` entrypoint (which calls `main()` at module load).
import { handlePlainJsonRpc } from '../tool-dispatch';

// Cross-stack seam: the standalone pen-mcp HTTP server now accepts plain
// JSON-RPC (no streamable-HTTP session), so a JSON-RPC client — e.g. the Rust
// `op` CLI — can drive it the same way it drives the Rust editor.
describe('handlePlainJsonRpc', () => {
  it('initialize returns protocolVersion + capabilities + serverInfo', async () => {
    const r = await handlePlainJsonRpc({ jsonrpc: '2.0', id: 1, method: 'initialize' });
    expect(r?.jsonrpc).toBe('2.0');
    expect(r?.id).toBe(1);
    const result = r?.result as Record<string, unknown>;
    expect((result.serverInfo as { name: string }).name).toBe('openpencil-mcp');
    expect(result.capabilities).toHaveProperty('tools');
    expect(result.protocolVersion).toBe('2024-11-05');
  });

  it('tools/list returns the tool definitions', async () => {
    const r = await handlePlainJsonRpc({ jsonrpc: '2.0', id: 2, method: 'tools/list' });
    const tools = (r?.result as { tools: unknown[] }).tools;
    expect(Array.isArray(tools)).toBe(true);
    expect(tools.length).toBeGreaterThan(0);
  });

  it('ping returns the openpencil-mcp identity marker the CLI verifies', async () => {
    const r = await handlePlainJsonRpc({ jsonrpc: '2.0', id: 3, method: 'ping' });
    expect((r?.result as { server: string }).server).toBe('openpencil-mcp');
  });

  it('notifications/initialized is swallowed (null → HTTP 202)', async () => {
    const r = await handlePlainJsonRpc({ jsonrpc: '2.0', method: 'notifications/initialized' });
    expect(r).toBeNull();
  });

  it('unknown method → JSON-RPC -32601', async () => {
    const r = await handlePlainJsonRpc({ jsonrpc: '2.0', id: 4, method: 'bogus' });
    expect((r?.error as { code: number }).code).toBe(-32601);
  });

  it('tools/call with an unknown tool → isError result (not a JSON-RPC error)', async () => {
    const r = await handlePlainJsonRpc({
      jsonrpc: '2.0',
      id: 5,
      method: 'tools/call',
      params: { name: 'no_such_tool_xyz', arguments: {} },
    });
    const result = r?.result as { isError?: boolean; content: { text: string }[] };
    expect(result.isError).toBe(true);
    expect(r?.error).toBeUndefined();
    expect(result.content[0].text).toContain('Unknown tool');
  });

  it('any notifications/* method is swallowed (null → HTTP 202)', async () => {
    const r = await handlePlainJsonRpc({ jsonrpc: '2.0', method: 'notifications/cancelled' });
    expect(r).toBeNull();
  });

  it('batch arrays are unsupported → -32600 Invalid Request', async () => {
    const r = await handlePlainJsonRpc([{ jsonrpc: '2.0', id: 1, method: 'ping' }]);
    expect((r?.error as { code: number }).code).toBe(-32600);
    expect(r?.id).toBeNull();
  });

  it('a non-object envelope → -32600 Invalid Request', async () => {
    const r = await handlePlainJsonRpc('not-an-object');
    expect((r?.error as { code: number }).code).toBe(-32600);
  });

  it('missing jsonrpc version → -32600 Invalid Request (preserves id)', async () => {
    const r = await handlePlainJsonRpc({ id: 7, method: 'ping' });
    expect((r?.error as { code: number }).code).toBe(-32600);
    expect(r?.id).toBe(7);
  });

  it('a non-string method → -32600 Invalid Request', async () => {
    const r = await handlePlainJsonRpc({ jsonrpc: '2.0', id: 8, method: 42 });
    expect((r?.error as { code: number }).code).toBe(-32600);
  });
});
