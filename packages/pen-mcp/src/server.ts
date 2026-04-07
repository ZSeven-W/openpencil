#!/usr/bin/env node

import { createServer } from 'node:http';
import { randomUUID } from 'node:crypto';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { MCP_DEFAULT_PORT } from './constants';

// Route modules
import {
  DOCUMENT_TOOL_DEFINITIONS,
  DOCUMENT_TOOL_NAMES,
  handleDocumentToolCall,
} from './routes/document-routes';
import { NODE_TOOL_DEFINITIONS, NODE_TOOL_NAMES, handleNodeToolCall } from './routes/node-routes';
import {
  DESIGN_TOOL_DEFINITIONS,
  DESIGN_TOOL_NAMES,
  handleDesignToolCall,
} from './routes/design-routes';
import {
  VARIABLE_TOOL_DEFINITIONS,
  VARIABLE_TOOL_NAMES,
  handleVariableToolCall,
} from './routes/variable-routes';
import {
  CODEGEN_TOOL_DEFINITIONS,
  CODEGEN_TOOL_NAMES,
  handleCodegenToolCall,
} from './routes/codegen-routes';
import {
  STYLE_GUIDE_TOOL_DEFINITIONS,
  STYLE_GUIDE_TOOL_NAMES,
  handleStyleGuideToolCall,
} from './routes/style-guide-routes';
import {
  STYLE_OPS_TOOL_DEFINITIONS,
  STYLE_OPS_TOOL_NAMES,
  handleStyleOpsToolCall,
} from './routes/style-operations-routes';
import {
  DEBUG_TOOL_DEFINITIONS,
  DEBUG_TOOL_NAMES,
  handleDebugToolCall,
} from './routes/debug-routes';

const pkg = { name: '@zseven-w/pen-mcp', version: '0.6.0' };

const DEBUG_ENABLED =
  process.env.OPENPENCIL_DEBUG_TOOLS === '1' || process.argv.includes('--debug');

/**
 * MCP content block types supported by this server's tool handlers.
 * Phase 1 tools all return string (wrapped to a single text block); Phase 2
 * will add `debug_screenshot` which returns ToolContent[] directly.
 * See Decision 6 in docs/superpowers/specs/2026-04-06-mcp-debug-tools-design.md
 */
export type ToolContent =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mimeType: string };

// --- Tool definitions (shared across all Server instances) ---

const TOOL_DEFINITIONS = [
  ...DOCUMENT_TOOL_DEFINITIONS,
  ...NODE_TOOL_DEFINITIONS,
  ...DESIGN_TOOL_DEFINITIONS,
  ...VARIABLE_TOOL_DEFINITIONS,
  ...CODEGEN_TOOL_DEFINITIONS,
  ...STYLE_GUIDE_TOOL_DEFINITIONS,
  ...STYLE_OPS_TOOL_DEFINITIONS,
  ...(DEBUG_ENABLED ? DEBUG_TOOL_DEFINITIONS : []),
];

// --- Tool execution handler ---

// eslint-disable-next-line @typescript-eslint/no-explicit-any -- MCP args are validated at runtime by the protocol
async function handleToolCall(
  name: string,
  args: Record<string, unknown> | undefined,
): Promise<string | ToolContent[]> {
  const a = args ?? {};
  if (DOCUMENT_TOOL_NAMES.has(name)) return handleDocumentToolCall(name, a);
  if (NODE_TOOL_NAMES.has(name)) return handleNodeToolCall(name, a);
  if (DESIGN_TOOL_NAMES.has(name)) return handleDesignToolCall(name, a);
  if (VARIABLE_TOOL_NAMES.has(name)) return handleVariableToolCall(name, a);
  if (CODEGEN_TOOL_NAMES.has(name)) return handleCodegenToolCall(name, a);
  if (STYLE_GUIDE_TOOL_NAMES.has(name)) return handleStyleGuideToolCall(name, a);
  if (STYLE_OPS_TOOL_NAMES.has(name)) return handleStyleOpsToolCall(name, a);
  if (DEBUG_ENABLED && DEBUG_TOOL_NAMES.has(name)) return handleDebugToolCall(name, a);
  throw new Error(`Unknown tool: ${name}`);
}

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

    // Route to existing session
    if (sessionId && sessions.has(sessionId)) {
      const session = sessions.get(sessionId)!;
      if (req.method === 'POST') {
        const chunks: Buffer[] = [];
        for await (const chunk of req) chunks.push(chunk as Buffer);
        const body = JSON.parse(Buffer.concat(chunks).toString());
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

      const chunks: Buffer[] = [];
      for await (const chunk of req) chunks.push(chunk as Buffer);
      const body = JSON.parse(Buffer.concat(chunks).toString());
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
