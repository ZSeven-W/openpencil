import { handleOpenDocument } from '../tools/open-document';
import { handleBatchGet } from '../tools/batch-get';
import { handleGetSelection } from '../tools/get-selection';
import { handleSnapshotLayout } from '../tools/snapshot-layout';
import { handleFindEmptySpace } from '../tools/find-empty-space';
import {
  handleAddPage,
  handleRemovePage,
  handleRenamePage,
  handleReorderPage,
  handleDuplicatePage,
} from '../tools/pages';

export const DOCUMENT_TOOL_DEFINITIONS = [
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
      'Search and read nodes. With no patterns/nodeIds, returns top-level children. Search by type/name regex, or read specific IDs. ' +
      'readDepth controls how deep children are included in results (default 1, use higher to see nested structure). ' +
      'Returns nodes with children truncated to "..." beyond readDepth.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
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
        parentId: { type: 'string', description: 'Limit search to children of this parent node' },
        readDepth: {
          type: 'number',
          description: 'How deep to include children in results (default 1)',
        },
        searchDepth: {
          type: 'number',
          description: 'How deep to search for matching nodes (default unlimited)',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
        resolve_refs: {
          type: 'boolean',
          description:
            'When true, recursively resolve $variable references (via resolveNodeForCanvas) so fill/stroke/effects/children/text return concrete values. Default false keeps current raw output. Useful for debugging visualization of what Skia actually receives.',
        },
      },
      required: [],
    },
  },
  {
    name: 'get_selection',
    description:
      'Get the currently selected nodes on the live canvas. Returns the full node data for each selected element. ' +
      'Use this to inspect what the user has selected without needing to search.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        readDepth: {
          type: 'number',
          description: 'How deep to include children in results (default 2)',
        },
      },
      required: [],
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
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        parentId: { type: 'string', description: 'Only return layout under this parent node' },
        maxDepth: { type: 'number', description: 'Max depth to traverse (default 1)' },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: [],
    },
  },
  {
    name: 'find_empty_space',
    description: 'Find empty canvas space in a given direction for placing new content.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        width: { type: 'number', description: 'Required width of empty space' },
        height: { type: 'number', description: 'Required height of empty space' },
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
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['width', 'height', 'direction'],
    },
  },
  {
    name: 'add_page',
    description:
      'Add a new page to an .op file. If the document has no pages yet, the existing children are migrated to the first page automatically.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        name: { type: 'string', description: 'Page name (default: "Page N")' },
        children: {
          type: 'array',
          description:
            'Initial child nodes for the page. Defaults to a single empty 1200×800 white frame.',
          items: { type: 'object' },
        },
      },
      required: [],
    },
  },
  {
    name: 'remove_page',
    description: 'Remove a page from an .op file. Cannot remove the last remaining page.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        pageId: { type: 'string', description: 'ID of the page to remove' },
      },
      required: ['pageId'],
    },
  },
  {
    name: 'rename_page',
    description: 'Rename a page in an .op file.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        pageId: { type: 'string', description: 'ID of the page to rename' },
        name: { type: 'string', description: 'New page name' },
      },
      required: ['pageId', 'name'],
    },
  },
  {
    name: 'reorder_page',
    description: 'Move a page to a new position (index) in an .op file.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        pageId: { type: 'string', description: 'ID of the page to move' },
        index: { type: 'number', description: 'New zero-based index for the page' },
      },
      required: ['pageId', 'index'],
    },
  },
  {
    name: 'duplicate_page',
    description:
      'Duplicate a page (deep-clone with new IDs) and insert the copy right after the original.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        pageId: { type: 'string', description: 'ID of the page to duplicate' },
        name: {
          type: 'string',
          description: 'Name for the duplicated page (default: "original copy")',
        },
      },
      required: ['pageId'],
    },
  },
];

export const DOCUMENT_TOOL_NAMES = new Set([
  'open_document',
  'batch_get',
  'get_selection',
  'snapshot_layout',
  'find_empty_space',
  'add_page',
  'remove_page',
  'rename_page',
  'reorder_page',
  'duplicate_page',
]);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleDocumentToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  const a = args as any; // eslint-disable-line @typescript-eslint/no-explicit-any
  switch (name) {
    case 'open_document':
      return JSON.stringify(await handleOpenDocument(a), null, 2);
    case 'batch_get':
      return JSON.stringify(
        await handleBatchGet({
          ...a,
          resolveRefs: a.resolve_refs as boolean | undefined,
        }),
        null,
        2,
      );
    case 'get_selection':
      return JSON.stringify(await handleGetSelection(a), null, 2);
    case 'snapshot_layout':
      return JSON.stringify(await handleSnapshotLayout(a), null, 2);
    case 'find_empty_space':
      return JSON.stringify(await handleFindEmptySpace(a), null, 2);
    case 'add_page':
      return JSON.stringify(await handleAddPage(a), null, 2);
    case 'remove_page':
      return JSON.stringify(await handleRemovePage(a), null, 2);
    case 'rename_page':
      return JSON.stringify(await handleRenamePage(a), null, 2);
    case 'reorder_page':
      return JSON.stringify(await handleReorderPage(a), null, 2);
    case 'duplicate_page':
      return JSON.stringify(await handleDuplicatePage(a), null, 2);
    default:
      return '';
  }
}
