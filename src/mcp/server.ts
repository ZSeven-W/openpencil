#!/usr/bin/env node

import { Server } from '@modelcontextprotocol/sdk/server/index.js'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from '@modelcontextprotocol/sdk/types.js'

import { handleOpenDocument } from './tools/open-document'
import { handleBatchGet } from './tools/batch-get'
import { handleBatchDesign } from './tools/batch-design'
import { handleGetVariables, handleSetVariables } from './tools/variables'
import { handleSnapshotLayout } from './tools/snapshot-layout'
import { handleFindEmptySpace } from './tools/find-empty-space'

const server = new Server(
  { name: 'openpencil', version: '0.1.0' },
  { capabilities: { tools: {} } },
)

// --- Tool definitions ---

server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: [
    {
      name: 'open_document',
      description:
        'Open an existing .op file or create a new empty document. Returns document metadata.',
      inputSchema: {
        type: 'object' as const,
        properties: {
          filePath: {
            type: 'string',
            description: 'Absolute path to the .op file to open or create',
          },
        },
        required: ['filePath'],
      },
    },
    {
      name: 'batch_get',
      description:
        'Search and read nodes from an .op file. Search by patterns (type, name regex, reusable flag) or read specific node IDs. Control depth with readDepth and searchDepth.',
      inputSchema: {
        type: 'object' as const,
        properties: {
          filePath: {
            type: 'string',
            description: 'Absolute path to the .op file',
          },
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
          nodeIds: {
            type: 'array',
            items: { type: 'string' },
            description: 'Specific node IDs to read',
          },
          parentId: {
            type: 'string',
            description: 'Limit search to children of this parent node',
          },
          readDepth: {
            type: 'number',
            description: 'How deep to include children in results (default 1)',
          },
          searchDepth: {
            type: 'number',
            description: 'How deep to search for matching nodes (default unlimited)',
          },
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

Bindings can reference earlier results: U(myFrame+"/childId", { ... })`,
      inputSchema: {
        type: 'object' as const,
        properties: {
          filePath: {
            type: 'string',
            description: 'Absolute path to the .op file',
          },
          operations: {
            type: 'string',
            description: 'Operations DSL (one operation per line)',
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
          filePath: {
            type: 'string',
            description: 'Absolute path to the .op file',
          },
        },
        required: ['filePath'],
      },
    },
    {
      name: 'set_variables',
      description:
        'Add or update design variables in an .op file. By default merges with existing variables.',
      inputSchema: {
        type: 'object' as const,
        properties: {
          filePath: {
            type: 'string',
            description: 'Absolute path to the .op file',
          },
          variables: {
            type: 'object',
            description: 'Variables to set (name → { type, value })',
          },
          replace: {
            type: 'boolean',
            description: 'Replace all variables instead of merging (default false)',
          },
        },
        required: ['filePath', 'variables'],
      },
    },
    {
      name: 'snapshot_layout',
      description:
        'Get the hierarchical bounding box layout tree of an .op file. Useful for understanding spatial arrangement.',
      inputSchema: {
        type: 'object' as const,
        properties: {
          filePath: {
            type: 'string',
            description: 'Absolute path to the .op file',
          },
          parentId: {
            type: 'string',
            description: 'Only return layout under this parent node',
          },
          maxDepth: {
            type: 'number',
            description: 'Max depth to traverse (default 1)',
          },
        },
        required: ['filePath'],
      },
    },
    {
      name: 'find_empty_space',
      description:
        'Find empty canvas space in a given direction for placing new content.',
      inputSchema: {
        type: 'object' as const,
        properties: {
          filePath: {
            type: 'string',
            description: 'Absolute path to the .op file',
          },
          width: {
            type: 'number',
            description: 'Required width of empty space',
          },
          height: {
            type: 'number',
            description: 'Required height of empty space',
          },
          padding: {
            type: 'number',
            description: 'Minimum padding from other elements (default 50)',
          },
          direction: {
            type: 'string',
            enum: ['top', 'right', 'bottom', 'left'],
            description: 'Direction to search for empty space',
          },
          nodeId: {
            type: 'string',
            description: 'Search relative to this node (default: entire canvas)',
          },
        },
        required: ['filePath', 'width', 'height', 'direction'],
      },
    },
  ],
}))

// --- Tool execution ---

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params

  try {
    switch (name) {
      case 'open_document': {
        const result = await handleOpenDocument(args as any)
        return { content: [{ type: 'text', text: JSON.stringify(result, null, 2) }] }
      }
      case 'batch_get': {
        const result = await handleBatchGet(args as any)
        return { content: [{ type: 'text', text: JSON.stringify(result, null, 2) }] }
      }
      case 'batch_design': {
        const result = await handleBatchDesign(args as any)
        return { content: [{ type: 'text', text: JSON.stringify(result, null, 2) }] }
      }
      case 'get_variables': {
        const result = await handleGetVariables(args as any)
        return { content: [{ type: 'text', text: JSON.stringify(result, null, 2) }] }
      }
      case 'set_variables': {
        const result = await handleSetVariables(args as any)
        return { content: [{ type: 'text', text: JSON.stringify(result, null, 2) }] }
      }
      case 'snapshot_layout': {
        const result = await handleSnapshotLayout(args as any)
        return { content: [{ type: 'text', text: JSON.stringify(result, null, 2) }] }
      }
      case 'find_empty_space': {
        const result = await handleFindEmptySpace(args as any)
        return { content: [{ type: 'text', text: JSON.stringify(result, null, 2) }] }
      }
      default:
        return {
          content: [{ type: 'text', text: `Unknown tool: ${name}` }],
          isError: true,
        }
    }
  } catch (err) {
    return {
      content: [
        {
          type: 'text',
          text: `Error: ${err instanceof Error ? err.message : String(err)}`,
        },
      ],
      isError: true,
    }
  }
})

// --- Start ---

async function main() {
  const transport = new StdioServerTransport()
  await server.connect(transport)
}

main().catch((err) => {
  console.error('MCP server failed to start:', err)
  process.exit(1)
})
