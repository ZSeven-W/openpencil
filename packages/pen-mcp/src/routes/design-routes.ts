import { handleBatchDesign } from '../tools/batch-design';
import { buildDesignPrompt, listPromptSections } from '../tools/design-prompt';
import { handleDesignSkeleton } from '../tools/design-skeleton';
import { handleDesignContent } from '../tools/design-content';
import { handleDesignRefine } from '../tools/design-refine';
import { LAYERED_DESIGN_TOOLS } from '../tools/layered-design-defs';

export const DESIGN_TOOL_DEFINITIONS = [
  {
    name: 'get_design_prompt',
    description:
      'Get design knowledge prompt. Use "section" to retrieve a focused subset instead of the full prompt. ' +
      'Sections: schema (PenNode types), layout (flexbox rules), roles (semantic roles), text (typography/CJK/copywriting), ' +
      'style (visual style policy), icons (icon names), examples (design examples), guidelines (design tips), planning (layered workflow guide). ' +
      'Omit section for the full prompt.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        section: {
          type: 'string',
          enum: [
            'all',
            'schema',
            'layout',
            'roles',
            'text',
            'style',
            'icons',
            'examples',
            'guidelines',
            'planning',
          ],
          description:
            'Which section of design knowledge to retrieve. Default: all. Use "planning" for layered generation workflow.',
        },
      },
      required: [],
    },
  },
  {
    name: 'batch_design',
    description:
      'Execute batch design operations in a compact DSL. Each line is one operation:\n' +
      '  binding=I(parent, { ...nodeData })  — Insert node (binding captures new ID)\n' +
      '  U(path, { ...updates })             — Update node properties\n' +
      '  binding=C(sourceId, parent, { overrides })  — Copy node\n' +
      '  binding=R(path, { ...newNodeData }) — Replace node\n' +
      '  M(nodeId, parent, index?)           — Move node\n' +
      '  D(nodeId)                           — Delete node\n' +
      'Use null for root-level parent. Reference previous bindings by name. ' +
      'Path expressions support binding+"/ childId" for nested access. ' +
      'Always set postProcess=true when generating designs for best visual quality.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        operations: {
          type: 'string',
          description:
            'DSL operations, one per line. Example:\nroot=I(null, { "type": "frame", "name": "Page", "width": 1200, "height": 0, "layout": "vertical", "children": [...] })',
        },
        postProcess: {
          type: 'boolean',
          description:
            'Apply post-processing (role defaults, icon resolution, layout sanitization). Always true for design generation.',
        },
        canvasWidth: {
          type: 'number',
          description: 'Canvas width for post-processing (default 1200, use 375 for mobile).',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['operations'],
    },
  },
  ...LAYERED_DESIGN_TOOLS,
];

export const DESIGN_TOOL_NAMES = new Set([
  'get_design_prompt',
  'batch_design',
  'design_skeleton',
  'design_content',
  'design_refine',
]);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleDesignToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  const a = args as any; // eslint-disable-line @typescript-eslint/no-explicit-any
  switch (name) {
    case 'get_design_prompt':
      return JSON.stringify(
        {
          section: (a.section as string | undefined) ?? 'all',
          availableSections: listPromptSections(),
          designPrompt: buildDesignPrompt(a.section as string | undefined),
        },
        null,
        2,
      );
    case 'batch_design':
      return JSON.stringify(await handleBatchDesign(a), null, 2);
    case 'design_skeleton':
      return JSON.stringify(await handleDesignSkeleton(a), null, 2);
    case 'design_content':
      return JSON.stringify(await handleDesignContent(a), null, 2);
    case 'design_refine':
      return JSON.stringify(await handleDesignRefine(a), null, 2);
    default:
      return '';
  }
}
