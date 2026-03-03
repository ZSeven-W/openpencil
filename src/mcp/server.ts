#!/usr/bin/env node

import { createServer } from 'node:http'
import { randomUUID } from 'node:crypto'
import { Server } from '@modelcontextprotocol/sdk/server/index.js'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js'
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from '@modelcontextprotocol/sdk/types.js'

import pkg from '../../package.json'
import { handleOpenDocument } from './tools/open-document'
import { handleBatchGet } from './tools/batch-get'
import { handleBatchDesign } from './tools/batch-design'
import { handleGetVariables, handleSetVariables } from './tools/variables'
import { handleSnapshotLayout } from './tools/snapshot-layout'
import { handleFindEmptySpace } from './tools/find-empty-space'

// --- Tool definitions (shared across all Server instances) ---

const TOOL_DEFINITIONS = [
  {
    name: 'open_document',
    description:
      'Open an existing .op file or connect to the live Electron canvas. Returns document metadata, context summary, and design prompt. Always call this first. Omit filePath to connect to the live canvas.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description:
            'Absolute path to the .op file to open or create. Omit to connect to the live Electron canvas, or pass "live://canvas" explicitly.',
        },
      },
      required: [],
    },
  },
  {
    name: 'batch_get',
    description:
      'Search and read nodes from an .op file. Search by patterns (type, name regex, reusable flag) or read specific node IDs. Control depth with readDepth and searchDepth.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        patterns: {
          type: 'array',
          description: 'Search patterns to match nodes',
          items: {
            type: 'object',
            properties: {
              type: { type: 'string', description: 'Node type (frame, text, rectangle, etc.)' },
              name: { type: 'string', description: 'Regex pattern to match node name' },
              reusable: { type: 'boolean', description: 'Match reusable components' },
            },
          },
        },
        nodeIds: { type: 'array', items: { type: 'string' }, description: 'Specific node IDs to read' },
        parentId: { type: 'string', description: 'Limit search to children of this parent node' },
        readDepth: { type: 'number', description: 'How deep to include children in results (default 1)' },
        searchDepth: { type: 'number', description: 'How deep to search for matching nodes (default unlimited)' },
      },
      required: ['filePath'],
    },
  },
  {
    name: 'batch_design',
    description: `Execute design operations on an .op file using a DSL. Each line is one operation:
- Insert: binding=I(parent, { type: "frame", ... })
- Copy: binding=C(sourceId, parent, { ...overrides })
- Update: U(nodeId, { fill: [...] })
- Replace: binding=R(nodeId, { type: "text", ... })
- Move: M(nodeId, newParent, index)
- Delete: D(nodeId)

Bindings can reference earlier results: U(myFrame+"/childId", { ... })

Set postProcess=true to automatically apply role defaults, icon resolution, and sanitization after operations complete.`,
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        operations: { type: 'string', description: 'Operations DSL (one operation per line)' },
        postProcess: {
          type: 'boolean',
          description: 'Apply post-processing (role defaults, icon resolution, sanitization) after operations. Always use when generating designs.',
        },
        canvasWidth: {
          type: 'number',
          description: 'Canvas width for post-processing layout calculations (default 1200, use 375 for mobile). Only used when postProcess=true.',
        },
      },
      required: ['filePath', 'operations'],
    },
  },
  {
    name: 'get_variables',
    description: 'Get all design variables and themes defined in an .op file.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
      },
      required: ['filePath'],
    },
  },
  {
    name: 'set_variables',
    description: 'Add or update design variables in an .op file. By default merges with existing variables.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        variables: { type: 'object', description: 'Variables to set (name → { type, value })' },
        replace: { type: 'boolean', description: 'Replace all variables instead of merging (default false)' },
      },
      required: ['filePath', 'variables'],
    },
  },
  {
    name: 'snapshot_layout',
    description: 'Get the hierarchical bounding box layout tree of an .op file. Useful for understanding spatial arrangement.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        parentId: { type: 'string', description: 'Only return layout under this parent node' },
        maxDepth: { type: 'number', description: 'Max depth to traverse (default 1)' },
      },
      required: ['filePath'],
    },
  },
  {
    name: 'find_empty_space',
    description: 'Find empty canvas space in a given direction for placing new content.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        width: { type: 'number', description: 'Required width of empty space' },
        height: { type: 'number', description: 'Required height of empty space' },
        padding: { type: 'number', description: 'Minimum padding from other elements (default 50)' },
        direction: { type: 'string', enum: ['top', 'right', 'bottom', 'left'], description: 'Direction to search for empty space' },
        nodeId: { type: 'string', description: 'Search relative to this node (default: entire canvas)' },
      },
      required: ['filePath', 'width', 'height', 'direction'],
    },
  },
]

// --- Tool execution handler ---

async function handleToolCall(name: string, args: any) {
  switch (name) {
    case 'open_document':
      return JSON.stringify(await handleOpenDocument(args), null, 2)
    case 'batch_get':
      return JSON.stringify(await handleBatchGet(args), null, 2)
    case 'batch_design':
      return JSON.stringify(await handleBatchDesign(args), null, 2)
    case 'get_variables':
      return JSON.stringify(await handleGetVariables(args), null, 2)
    case 'set_variables':
      return JSON.stringify(await handleSetVariables(args), null, 2)
    case 'snapshot_layout':
      return JSON.stringify(await handleSnapshotLayout(args), null, 2)
    case 'find_empty_space':
      return JSON.stringify(await handleFindEmptySpace(args), null, 2)
    default:
      throw new Error(`Unknown tool: ${name}`)
  }
}

/** Register tool handlers on a Server instance. */
function registerTools(server: Server): void {
  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: TOOL_DEFINITIONS,
  }))

  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params
    try {
      const text = await handleToolCall(name, args)
      return { content: [{ type: 'text', text }] }
    } catch (err) {
      return {
        content: [{ type: 'text', text: `Error: ${err instanceof Error ? err.message : String(err)}` }],
        isError: true,
      }
    }
  })
}

// --- HTTP server helper ---

function startHttpServer(server: Server, port: number): void {
  const transport = new StreamableHTTPServerTransport({
    sessionIdGenerator: () => randomUUID(),
  })

  server.connect(transport)

  const httpServer = createServer(async (req, res) => {
    res.setHeader('Access-Control-Allow-Origin', '*')
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, DELETE, OPTIONS')
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, mcp-session-id')
    res.setHeader('Access-Control-Expose-Headers', 'mcp-session-id')

    if (req.method === 'OPTIONS') {
      res.writeHead(204)
      res.end()
      return
    }

    if (req.url === '/mcp') {
      if (req.method === 'POST') {
        const chunks: Buffer[] = []
        for await (const chunk of req) chunks.push(chunk as Buffer)
        const body = JSON.parse(Buffer.concat(chunks).toString())
        await transport.handleRequest(req, res, body)
      } else {
        await transport.handleRequest(req, res)
      }
    } else {
      res.writeHead(404, { 'Content-Type': 'application/json' })
      res.end(JSON.stringify({ error: 'Not found. Use /mcp endpoint.' }))
    }
  })

  httpServer.listen(port, '127.0.0.1', () => {
    console.error(`OpenPencil MCP server listening on http://127.0.0.1:${port}/mcp`)
  })
}

// --- Start ---

function parseArgs(): { stdio: boolean; http: boolean; port: number } {
  const args = process.argv.slice(2)
  const hasHttp = args.includes('--http')
  const hasStdio = args.includes('--stdio')
  const portIdx = args.indexOf('--port')
  const port = portIdx !== -1 ? parseInt(args[portIdx + 1], 10) : 3100

  if (hasHttp && hasStdio) return { stdio: true, http: true, port: isNaN(port) ? 3100 : port }
  if (hasHttp) return { stdio: false, http: true, port: isNaN(port) ? 3100 : port }
  return { stdio: true, http: false, port: 3100 }
}

async function main() {
  const { stdio, http, port } = parseArgs()

  if (stdio && http) {
    // Both: two Server instances sharing the same tool handlers
    const stdioServer = new Server(
      { name: pkg.name, version: pkg.version },
      { capabilities: { tools: {} } },
    )
    registerTools(stdioServer)
    await stdioServer.connect(new StdioServerTransport())

    const httpServer = new Server(
      { name: pkg.name, version: pkg.version },
      { capabilities: { tools: {} } },
    )
    registerTools(httpServer)
    startHttpServer(httpServer, port)
  } else if (http) {
    const server = new Server(
      { name: pkg.name, version: pkg.version },
      { capabilities: { tools: {} } },
    )
    registerTools(server)
    startHttpServer(server, port)
  } else {
    const server = new Server(
      { name: pkg.name, version: pkg.version },
      { capabilities: { tools: {} } },
    )
    registerTools(server)
    await server.connect(new StdioServerTransport())
  }
}

main().catch((err) => {
  console.error('MCP server failed to start:', err)
  process.exit(1)
})
