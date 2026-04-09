import {
  handleInsertNode,
  handleUpdateNode,
  handleDeleteNode,
  handleMoveNode,
  handleCopyNode,
  handleReplaceNode,
} from '../tools/node-crud';
import { handleImportSvg } from '../tools/import-svg';

export const NODE_TOOL_DEFINITIONS = [
  {
    name: 'insert_node',
    description:
      'Insert a new node into the document. Node types: frame, rectangle, ellipse, text, path, image, group, line, polygon, ref. ' +
      'Fill is always an array: [{ type: "solid", color: "#hex" }]. ' +
      'When inserting a frame at root level and an empty root frame exists, it is auto-replaced. ' +
      'Returns the final node state (after post-processing if enabled).',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        parent: {
          type: ['string', 'null'] as const,
          description: 'Parent node ID, or null for root level',
        },
        data: {
          type: 'object',
          description:
            'PenNode data. Required: type. Key props by type:\n' +
            '- frame: width, height, layout (none|vertical|horizontal), gap, padding, justifyContent, alignItems, clipContent, children[]\n' +
            '- text: content (required), fontSize, fontWeight, fontFamily, textGrowth (auto|fixed-width), lineHeight, fill\n' +
            '- rectangle/ellipse: width, height, fill, stroke, cornerRadius\n' +
            '- path: d (SVG path string) or name (icon name like "SearchIcon"), width, height\n' +
            '- image: src (URL), width, height\n' +
            'Common: name, role, x, y, opacity, fill (array), stroke, effects, cornerRadius',
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
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['parent', 'data'],
    },
  },
  {
    name: 'update_node',
    description:
      'Update properties of an existing node. Only provided properties are shallow-merged; unmentioned properties remain unchanged. Returns the updated node state.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
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
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['nodeId', 'data'],
    },
  },
  {
    name: 'delete_node',
    description: 'Delete a node (and all its children) from an .op file.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        nodeId: { type: 'string', description: 'ID of the node to delete' },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['nodeId'],
    },
  },
  {
    name: 'move_node',
    description:
      'Move a node to a new parent (or root level) in an .op file. Optionally specify insertion index.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        nodeId: { type: 'string', description: 'ID of the node to move' },
        parent: {
          type: ['string', 'null'] as const,
          description: 'New parent node ID, or null for root level',
        },
        index: {
          type: 'number',
          description: 'Insertion index within the parent (default: append at end)',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['nodeId', 'parent'],
    },
  },
  {
    name: 'copy_node',
    description:
      'Deep-copy an existing node (with new IDs) and insert the clone under a parent. Optionally apply property overrides.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        sourceId: { type: 'string', description: 'ID of the node to copy' },
        parent: {
          type: ['string', 'null'] as const,
          description: 'Parent node ID for the clone, or null for root level',
        },
        overrides: {
          type: 'object',
          description: 'Properties to override on the cloned node (name, x, y, etc.)',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['sourceId', 'parent'],
    },
  },
  {
    name: 'replace_node',
    description:
      'Replace a node with entirely new data. The old node is removed and a new node is inserted at the same position.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
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
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['nodeId', 'data'],
    },
  },
  {
    name: 'import_svg',
    description:
      'Import a local SVG file into an .op document as editable PenNodes. Supports path, rect, circle, ellipse, line, polygon, polyline, and nested groups. No network access required.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        svgPath: { type: 'string', description: 'Absolute path to a local .svg file' },
        parent: {
          type: ['string', 'null'] as const,
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
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['svgPath'],
    },
  },
];

export const NODE_TOOL_NAMES = new Set([
  'insert_node',
  'update_node',
  'delete_node',
  'move_node',
  'copy_node',
  'replace_node',
  'import_svg',
]);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleNodeToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  const a = args as any; // eslint-disable-line @typescript-eslint/no-explicit-any
  switch (name) {
    case 'insert_node':
      return JSON.stringify(await handleInsertNode(a), null, 2);
    case 'update_node':
      return JSON.stringify(await handleUpdateNode(a), null, 2);
    case 'delete_node':
      return JSON.stringify(await handleDeleteNode(a), null, 2);
    case 'move_node':
      return JSON.stringify(await handleMoveNode(a), null, 2);
    case 'copy_node':
      return JSON.stringify(await handleCopyNode(a), null, 2);
    case 'replace_node':
      return JSON.stringify(await handleReplaceNode(a), null, 2);
    case 'import_svg':
      return JSON.stringify(await handleImportSvg(a), null, 2);
    default:
      return '';
  }
}
