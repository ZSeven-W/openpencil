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
import {
  handleInsertNode,
  handleUpdateNode,
  handleDeleteNode,
  handleMoveNode,
  handleCopyNode,
  handleReplaceNode,
} from './tools/node-crud'
import { handleGetVariables, handleSetVariables, handleSetThemes } from './tools/variables'
import { handleImportSvg } from './tools/import-svg'
import { handleSnapshotLayout } from './tools/snapshot-layout'
import { handleFindEmptySpace } from './tools/find-empty-space'
import { handleSaveThemePreset, handleLoadThemePreset, handleListThemePresets } from './tools/theme-presets'

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
    name: 'insert_node',
    description:
      'Insert a new node into an .op file. The node data is standard JSON (type, name, width, height, fill, children, etc.). When inserting a frame at root level and an empty root frame exists, it will be auto-replaced.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        parent: {
          type: ['string', 'null'] as any,
          description: 'Parent node ID, or null for root level',
        },
        data: {
          type: 'object',
          description: 'PenNode data (type, name, width, height, fill, children, ...)',
        },
        postProcess: {
          type: 'boolean',
          description:
            'Apply post-processing (role defaults, icon resolution, sanitization). Always use when generating designs.',
        },
        canvasWidth: {
          type: 'number',
          description:
            'Canvas width for post-processing layout (default 1200, use 375 for mobile).',
        },
      },
      required: ['filePath', 'parent', 'data'],
    },
  },
  {
    name: 'update_node',
    description:
      'Update properties of an existing node in an .op file. Only the provided properties are merged; others remain unchanged.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        nodeId: { type: 'string', description: 'ID of the node to update' },
        data: {
          type: 'object',
          description: 'Properties to merge into the node (fill, width, name, etc.)',
        },
        postProcess: {
          type: 'boolean',
          description: 'Apply post-processing after update.',
        },
        canvasWidth: {
          type: 'number',
          description: 'Canvas width for post-processing layout (default 1200).',
        },
      },
      required: ['filePath', 'nodeId', 'data'],
    },
  },
  {
    name: 'delete_node',
    description: 'Delete a node (and all its children) from an .op file.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        nodeId: { type: 'string', description: 'ID of the node to delete' },
      },
      required: ['filePath', 'nodeId'],
    },
  },
  {
    name: 'move_node',
    description:
      'Move a node to a new parent (or root level) in an .op file. Optionally specify insertion index.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        nodeId: { type: 'string', description: 'ID of the node to move' },
        parent: {
          type: ['string', 'null'] as any,
          description: 'New parent node ID, or null for root level',
        },
        index: {
          type: 'number',
          description: 'Insertion index within the parent (default: append at end)',
        },
      },
      required: ['filePath', 'nodeId', 'parent'],
    },
  },
  {
    name: 'copy_node',
    description:
      'Deep-copy an existing node (with new IDs) and insert the clone under a parent. Optionally apply property overrides.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        sourceId: { type: 'string', description: 'ID of the node to copy' },
        parent: {
          type: ['string', 'null'] as any,
          description: 'Parent node ID for the clone, or null for root level',
        },
        overrides: {
          type: 'object',
          description: 'Properties to override on the cloned node (name, x, y, etc.)',
        },
      },
      required: ['filePath', 'sourceId', 'parent'],
    },
  },
  {
    name: 'replace_node',
    description:
      'Replace a node with entirely new data. The old node is removed and a new node is inserted at the same position.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        nodeId: { type: 'string', description: 'ID of the node to replace' },
        data: {
          type: 'object',
          description: 'Complete new PenNode data (type, name, width, height, fill, children, ...)',
        },
        postProcess: {
          type: 'boolean',
          description: 'Apply post-processing after replacement.',
        },
        canvasWidth: {
          type: 'number',
          description: 'Canvas width for post-processing layout (default 1200).',
        },
      },
      required: ['filePath', 'nodeId', 'data'],
    },
  },
  {
    name: 'import_svg',
    description:
      'Import a local SVG file into an .op document as editable PenNodes. Supports path, rect, circle, ellipse, line, polygon, polyline, and nested groups. No network access required.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        svgPath: { type: 'string', description: 'Absolute path to a local .svg file' },
        parent: {
          type: ['string', 'null'] as any,
          description: 'Parent node ID, or null/omit for root level',
        },
        maxDim: {
          type: 'number',
          description: 'Max dimension to scale SVG to (default 400)',
        },
        postProcess: {
          type: 'boolean',
          description: 'Apply post-processing (role defaults, icon resolution, sanitization).',
        },
        canvasWidth: {
          type: 'number',
          description: 'Canvas width for post-processing layout (default 1200).',
        },
      },
      required: ['filePath', 'svgPath'],
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
    name: 'set_themes',
    description:
      'Create or update theme axes and their variants in an .op file. Each theme axis (e.g. "Color Scheme") has an array of variant names (e.g. ["Light", "Dark"]). Multiple independent axes are supported. By default merges with existing themes.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file' },
        themes: {
          type: 'object',
          description:
            'Theme axes to set (axis name → variant names array). Example: { "Color": ["Light", "Dark"], "Density": ["Compact", "Comfortable"] }',
        },
        replace: {
          type: 'boolean',
          description: 'Replace all themes instead of merging (default false)',
        },
      },
      required: ['filePath', 'themes'],
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
  {
    name: 'save_theme_preset',
    description: 'Save the themes and variables from an .op document as a reusable .optheme preset file.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file to extract themes from' },
        presetPath: { type: 'string', description: 'Absolute path for the output .optheme file' },
        name: { type: 'string', description: 'Display name for the preset (defaults to file name)' },
      },
      required: ['filePath', 'presetPath'],
    },
  },
  {
    name: 'load_theme_preset',
    description: 'Load a .optheme preset file and merge its themes and variables into an .op document.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: { type: 'string', description: 'Absolute path to the .op file to update' },
        presetPath: { type: 'string', description: 'Absolute path to the .optheme file to load' },
      },
      required: ['filePath', 'presetPath'],
    },
  },
  {
    name: 'list_theme_presets',
    description: 'List all .optheme preset files in a directory.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        directory: { type: 'string', description: 'Absolute path to the directory to scan' },
      },
      required: ['directory'],
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
    case 'insert_node':
      return JSON.stringify(await handleInsertNode(args), null, 2)
    case 'update_node':
      return JSON.stringify(await handleUpdateNode(args), null, 2)
    case 'delete_node':
      return JSON.stringify(await handleDeleteNode(args), null, 2)
    case 'move_node':
      return JSON.stringify(await handleMoveNode(args), null, 2)
    case 'copy_node':
      return JSON.stringify(await handleCopyNode(args), null, 2)
    case 'replace_node':
      return JSON.stringify(await handleReplaceNode(args), null, 2)
    case 'import_svg':
      return JSON.stringify(await handleImportSvg(args), null, 2)
    case 'get_variables':
      return JSON.stringify(await handleGetVariables(args), null, 2)
    case 'set_variables':
      return JSON.stringify(await handleSetVariables(args), null, 2)
    case 'set_themes':
      return JSON.stringify(await handleSetThemes(args), null, 2)
    case 'snapshot_layout':
      return JSON.stringify(await handleSnapshotLayout(args), null, 2)
    case 'find_empty_space':
      return JSON.stringify(await handleFindEmptySpace(args), null, 2)
    case 'save_theme_preset':
      return JSON.stringify(await handleSaveThemePreset(args), null, 2)
    case 'load_theme_preset':
      return JSON.stringify(await handleLoadThemePreset(args), null, 2)
    case 'list_theme_presets':
      return JSON.stringify(await handleListThemePresets(args), null, 2)
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

  httpServer.listen(port, '0.0.0.0', () => {
    console.error(`OpenPencil MCP server listening on http://0.0.0.0:${port}/mcp`)
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
