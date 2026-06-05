// Tool dispatch core — shared by the executable server entrypoint (`server.ts`)
// and by tests/other importers. This module has NO top-level side effects (it
// does not start a server), so importing it is safe: it only wires the route
// modules into a single `TOOL_DEFINITIONS` list plus the `handleToolCall` and
// `handlePlainJsonRpc` dispatchers. `server.ts` adds the stdio/HTTP transports
// and the `main()` startup on top of these.

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
  D0_SPIKE_TOOL_DEFINITIONS,
  D0_SPIKE_TOOL_NAMES,
  handleD0SpikeToolCall,
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
// Source name/version from package.json (kept in sync by `bun run bump`) so the
// `initialize` / serverInfo metadata never drifts. resolveJsonModule is on, and
// both esbuild (mcp:compile) and bun (mcp:dev) inline JSON imports.
import pkgJson from '../package.json';

export const pkg = { name: pkgJson.name, version: pkgJson.version };

export const DEBUG_ENABLED =
  process.env.OPENPENCIL_DEBUG_TOOLS === '1' || process.argv.includes('--debug');

export const D0_SPIKE_ENABLED = process.env.OPENPENCIL_D0_SPIKE === '1';

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

export const TOOL_DEFINITIONS = [
  ...DOCUMENT_TOOL_DEFINITIONS,
  ...NODE_TOOL_DEFINITIONS,
  ...DESIGN_TOOL_DEFINITIONS,
  ...VARIABLE_TOOL_DEFINITIONS,
  ...CODEGEN_TOOL_DEFINITIONS,
  ...STYLE_GUIDE_TOOL_DEFINITIONS,
  ...STYLE_OPS_TOOL_DEFINITIONS,
  ...(DEBUG_ENABLED ? DEBUG_TOOL_DEFINITIONS : []),
  ...(D0_SPIKE_ENABLED ? D0_SPIKE_TOOL_DEFINITIONS : []),
];

// --- Tool execution handler ---

export async function handleToolCall(
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
  if (D0_SPIKE_ENABLED && D0_SPIKE_TOOL_NAMES.has(name)) return handleD0SpikeToolCall(name, a);
  throw new Error(`Unknown tool: ${name}`);
}

/**
 * Handle a PLAIN JSON-RPC message (no streamable-HTTP session) for clients
 * that POST `{jsonrpc,id,method,params}` directly without negotiating SSE —
 * e.g. the Rust `op` CLI, which speaks plain JSON-RPC over `POST /mcp`. This
 * is the cross-stack seam: a JSON-RPC client (Rust or any) can drive this TS
 * server the same way it drives the Rust editor. Returns the JSON-RPC reply,
 * or `null` for a notification (caller answers HTTP 202). Reuses the same
 * `handleToolCall` dispatch + `{content[]}/isError` envelope as the SDK path.
 */
export async function handlePlainJsonRpc(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- raw JSON-RPC envelope, validated below
  msg: any,
): Promise<Record<string, unknown> | null> {
  // Validate the JSON-RPC 2.0 envelope (mirrors the strictness of the
  // streamable-HTTP SDK path). Batch arrays are unsupported; a non-object or
  // wrong-version/method-less request is -32600 Invalid Request.
  if (typeof msg !== 'object' || msg === null || Array.isArray(msg)) {
    return { jsonrpc: '2.0', id: null, error: { code: -32600, message: 'Invalid Request' } };
  }
  const id = msg.id ?? null;
  const method = msg.method;
  if (msg.jsonrpc !== '2.0' || typeof method !== 'string') {
    return { jsonrpc: '2.0', id, error: { code: -32600, message: 'Invalid Request' } };
  }
  // Never reply to a notification (the `notifications/*` namespace, e.g.
  // `notifications/initialized`); the caller answers HTTP 202.
  if (method.startsWith('notifications/')) return null;
  switch (method) {
    case 'initialize':
      return {
        jsonrpc: '2.0',
        id,
        result: {
          protocolVersion: '2024-11-05',
          capabilities: { tools: { listChanged: false } },
          serverInfo: { name: 'openpencil-mcp', version: pkg.version },
        },
      };
    case 'tools/list':
      return { jsonrpc: '2.0', id, result: { tools: TOOL_DEFINITIONS } };
    case 'ping':
      // Identity marker the `op` CLI verifies (`result.server`) before routing.
      return { jsonrpc: '2.0', id, result: { server: 'openpencil-mcp' } };
    case 'initialized':
      return null;
    case 'tools/call': {
      const name = msg?.params?.name;
      const args = msg?.params?.arguments;
      try {
        const result = await handleToolCall(name, args);
        const content = typeof result === 'string' ? [{ type: 'text', text: result }] : result;
        return { jsonrpc: '2.0', id, result: { content } };
      } catch (err) {
        return {
          jsonrpc: '2.0',
          id,
          result: {
            content: [
              { type: 'text', text: `Error: ${err instanceof Error ? err.message : String(err)}` },
            ],
            isError: true,
          },
        };
      }
    }
    default:
      return {
        jsonrpc: '2.0',
        id,
        error: { code: -32601, message: `Method not found: ${method}` },
      };
  }
}
