import { handleExportNodes } from '../tools/export-nodes';

export const EXPORT_TOOL_DEFINITIONS = [
  {
    name: 'export_nodes',
    description:
      'Export raw PenNode data with design variables and themes. Pure data export — no AI, no analysis. Use with get_design_prompt(section="codegen-*") to let your LLM generate code.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file. If omitted, uses the currently opened document.',
        },
        nodeIds: {
          type: 'array',
          items: { type: 'string' },
          description:
            'Specific node IDs to export. If omitted, exports all nodes on the target page.',
        },
        pageId: {
          type: 'string',
          description: 'Target page ID. If omitted, uses the first/active page.',
        },
      },
      required: [],
    },
  },
];

export const EXPORT_TOOL_NAMES = new Set(['export_nodes']);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleExportToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  const a = args as any; // eslint-disable-line @typescript-eslint/no-explicit-any
  switch (name) {
    case 'export_nodes':
      return JSON.stringify(await handleExportNodes(a), null, 2);
    default:
      return '';
  }
}
