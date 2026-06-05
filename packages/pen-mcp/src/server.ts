#!/usr/bin/env node

import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { randomUUID } from 'node:crypto';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { MCP_DEFAULT_PORT } from './constants';
import { DEBUG_TOOL_DEFINITIONS } from './routes/debug-routes';

// Tool dispatch core (no side effects — safe to import from tests).
import {
  pkg,
  DEBUG_ENABLED,
  TOOL_DEFINITIONS,
  handleToolCall,
  handlePlainJsonRpc,
} from './tool-dispatch';

// Re-export so existing `import type { ToolContent } from '../server'` keep
// working (debug-routes.ts, debug-screenshot.ts). Type-only, so no cycle.
export type { ToolContent } from './tool-dispatch';

/** Register tool handlers on a Server instance. */
function registerTools(server: Server): void {
  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: TOOL_DEFINITIONS,
  }));

  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    try {
      const result = await handleToolCall(name, args);
      if (typeof result === 'string') {
        return { content: [{ type: 'text', text: result }] };
      }
      return { content: result };
    } catch (err) {
      return {
        content: [
          { type: 'text', text: `Error: ${err instanceof Error ? err.message : String(err)}` },
        ],
        isError: true,
      };
    }
  });
}

// --- HTTP server helper ---

/**
 * Read and JSON-parse a request body. On parse failure, write a JSON-RPC
 * -32700 (Parse error) response and return `undefined` — the caller must
 * `return` immediately. Shared by the plain-JSON-RPC and streamable-HTTP POST
 * paths so a malformed body never rejects the async request handler (which
 * would otherwise hang the client instead of replying). No valid JSON parses
 * to `undefined`, so it is a safe sentinel.
 */
async function readJsonBody(
  req: IncomingMessage,
  res: ServerResponse,
): Promise<unknown | undefined> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(chunk as Buffer);
  try {
    return JSON.parse(Buffer.concat(chunks).toString());
  } catch {
    res.writeHead(400, { 'Content-Type': 'application/json' });
    res.end(
      JSON.stringify({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } }),
    );
    return undefined;
  }
}

function startHttpServer(port: number): void {
  // Per-session transport map: each client gets its own Server + Transport
  const sessions = new Map<string, { transport: StreamableHTTPServerTransport; server: Server }>();

  const httpServer = createServer(async (req, res) => {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, DELETE, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, mcp-session-id');
    res.setHeader('Access-Control-Expose-Headers', 'mcp-session-id');

    if (req.method === 'OPTIONS') {
      res.writeHead(204);
      res.end();
      return;
    }

    if (req.url !== '/mcp') {
      res.writeHead(404, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Not found. Use /mcp endpoint.' }));
      return;
    }

    const sessionId = req.headers['mcp-session-id'] as string | undefined;

    // Plain JSON-RPC clients (e.g. the Rust `op` CLI) POST `{jsonrpc,...}`
    // with NO `mcp-session-id` and NO `text/event-stream` Accept — the SDK
    // streamable-HTTP path below requires SSE negotiation. Handle plain
    // JSON-RPC directly so a JSON-RPC client can drive this server
    // cross-stack, alongside the streamable-HTTP transport (no regression:
    // SDK clients always send `Accept: …text/event-stream`).
    const accept = (req.headers['accept'] as string | undefined) ?? '';
    if (req.method === 'POST' && !sessionId && !accept.includes('text/event-stream')) {
      const msg = await readJsonBody(req, res);
      if (msg === undefined) return;
      const reply = await handlePlainJsonRpc(msg);
      if (reply === null) {
        res.writeHead(202);
        res.end();
        return;
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(reply));
      return;
    }

    // Route to existing session
    if (sessionId && sessions.has(sessionId)) {
      const session = sessions.get(sessionId)!;
      if (req.method === 'POST') {
        const body = await readJsonBody(req, res);
        if (body === undefined) return;
        await session.transport.handleRequest(req, res, body);
      } else {
        await session.transport.handleRequest(req, res);
      }
      return;
    }

    // New session — only POST (initialize) is valid without session ID
    if (req.method === 'POST') {
      const mcpServer = new Server(
        { name: pkg.name, version: pkg.version },
        { capabilities: { tools: {} } },
      );
      registerTools(mcpServer);

      const transport = new StreamableHTTPServerTransport({
        sessionIdGenerator: () => randomUUID(),
        onsessioninitialized: (sid: string) => {
          sessions.set(sid, { transport, server: mcpServer });
        },
        onsessionclosed: (sid: string) => {
          sessions.delete(sid);
        },
      });

      transport.onclose = () => {
        if (transport.sessionId) sessions.delete(transport.sessionId);
      };

      await mcpServer.connect(transport);

      const body = await readJsonBody(req, res);
      if (body === undefined) return;
      await transport.handleRequest(req, res, body);
      return;
    }

    // Invalid: GET/DELETE without valid session ID
    res.writeHead(400, { 'Content-Type': 'application/json' });
    res.end(
      JSON.stringify({
        jsonrpc: '2.0',
        error: { code: -32000, message: 'Invalid or missing session ID' },
        id: null,
      }),
    );
  });

  httpServer.listen(port, '0.0.0.0', () => {
    console.error(`OpenPencil MCP server listening on http://0.0.0.0:${port}/mcp`);
  });
}

// --- Start ---

function parseArgs(): { stdio: boolean; http: boolean; port: number } {
  const args = process.argv.slice(2);
  const hasHttp = args.includes('--http');
  const hasStdio = args.includes('--stdio');
  const portIdx = args.indexOf('--port');
  const port = portIdx !== -1 ? parseInt(args[portIdx + 1], 10) : MCP_DEFAULT_PORT;

  if (hasHttp && hasStdio)
    return { stdio: true, http: true, port: isNaN(port) ? MCP_DEFAULT_PORT : port };
  if (hasHttp) return { stdio: false, http: true, port: isNaN(port) ? MCP_DEFAULT_PORT : port };
  return { stdio: true, http: false, port: MCP_DEFAULT_PORT };
}

async function main() {
  const { stdio, http, port } = parseArgs();

  if (DEBUG_ENABLED) {
    console.error(
      `[pen-mcp] DEBUG tools enabled — ${DEBUG_TOOL_DEFINITIONS.length} debug tools loaded`,
    );
  }

  if (stdio && http) {
    // Both: stdio server + HTTP server (per-session)
    const stdioServer = new Server(
      { name: pkg.name, version: pkg.version },
      { capabilities: { tools: {} } },
    );
    registerTools(stdioServer);
    await stdioServer.connect(new StdioServerTransport());

    startHttpServer(port);
  } else if (http) {
    startHttpServer(port);
  } else {
    const server = new Server(
      { name: pkg.name, version: pkg.version },
      { capabilities: { tools: {} } },
    );
    registerTools(server);
    await server.connect(new StdioServerTransport());
  }
}

// Prevent uncaught errors from crashing the MCP server process
process.on('uncaughtException', (err) => {
  console.error('MCP server uncaught exception:', err);
});
process.on('unhandledRejection', (err) => {
  console.error('MCP server unhandled rejection:', err);
});

main().catch((err) => {
  console.error('MCP server failed to start:', err);
  process.exit(1);
});
