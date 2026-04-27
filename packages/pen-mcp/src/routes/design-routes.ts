import { handleBatchDesign } from '../tools/batch-design';
import {
  buildDesignPrompt,
  listPromptSections,
  designMdSpecToPromptPolicy,
} from '../tools/design-prompt';
import { openDocument, resolveDocPath } from '../document-manager';
import { handleDesignSkeleton } from '../tools/design-skeleton';
import { handleDesignContent } from '../tools/design-content';
import { handleDesignRefine } from '../tools/design-refine';
import { LAYERED_DESIGN_TOOLS } from '../tools/layered-design-defs';
import { handleAddSectionV0 } from '../tools/add-section-v0';
import {
  ELEMENT_TOOL_DEFINITIONS,
  ELEMENT_TOOL_NAMES,
  handleElementToolCall,
} from './element-tool-defs';

// Core design tools that predate the N-tool element family. Kept stable so
// Plan 14's byte-level parity gate stays honorable (§C.5.1). Element tools
// live in element-tool-defs.ts to keep this file under the repo's 800-line
// cap (enforced by CLAUDE.md "Single files must not exceed 800 lines").

const CORE_DESIGN_TOOL_DEFINITIONS = [
  {
    name: 'get_design_prompt',
    description:
      'Get design knowledge prompt. Pass `section` to retrieve a focused subset instead of the ' +
      'full prompt; the available section names are published in `inputSchema.section.enum` ' +
      '(derived at server-start from the implementation registry, so this description never ' +
      'drifts as sections are added). Omit `section` for the full prompt. The "style" and ' +
      '"design-md" sections are derived from the active document\'s design.md so the response ' +
      "never leaks another file's design system.",
    inputSchema: {
      type: 'object' as const,
      properties: {
        section: {
          type: 'string',
          // Derived from SECTION_MAP at server-start — keeping the
          // schema in sync with the implementation so external clients
          // never get an "advertised section can't actually be fetched"
          // mismatch (or its inverse). Drift between the union, the
          // SECTION_MAP, and this enum was exactly the trap Codex
          // caught when elements-cookbook landed.
          enum: listPromptSections(),
          description:
            'Which section of design knowledge to retrieve. Default: all. Use "planning" for the layered generation workflow; "elements" for the N-tool element-tool decision tree; "elements-cookbook" for the per-tool arg-shape examples that pair with elements; "design-md" for the active document\'s design system; "codegen-*" for per-target codegen guides; otherwise see listPromptSections() for the full set.',
        },
        filePath: {
          type: 'string',
          description:
            "Path to .op file, or omit to use the live canvas. The prompt's 'style' / 'design-md' sections are derived from THIS document's design.md so the reply never leaks another file's design system.",
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
      '  D(nodeId)                           — Delete node (use batch_get to find IDs first)\n' +
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
];

export const DESIGN_TOOL_DEFINITIONS = [
  ...CORE_DESIGN_TOOL_DEFINITIONS,
  ...LAYERED_DESIGN_TOOLS,
  ...ELEMENT_TOOL_DEFINITIONS,
];

const CORE_DESIGN_TOOL_NAMES = new Set([
  'get_design_prompt',
  'batch_design',
  'design_skeleton',
  'design_content',
  'design_refine',
]);

export const DESIGN_TOOL_NAMES: ReadonlySet<string> = new Set([
  ...CORE_DESIGN_TOOL_NAMES,
  ...ELEMENT_TOOL_NAMES,
]);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleDesignToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  const a = args as any; // eslint-disable-line @typescript-eslint/no-explicit-any
  switch (name) {
    case 'get_design_prompt': {
      // Derive design.md policy from THIS document so the prompt cannot
      // leak another document's design system (per-document isolation).
      let designMdPolicy: string | null = null;
      try {
        const doc = await openDocument(resolveDocPath(a.filePath as string | undefined));
        designMdPolicy = designMdSpecToPromptPolicy(doc.designMd);
      } catch {
        // live canvas / file unavailable — prompt still works without policy
      }
      return JSON.stringify(
        {
          section: (a.section as string | undefined) ?? 'all',
          availableSections: listPromptSections(),
          designPrompt: buildDesignPrompt(a.section as string | undefined, designMdPolicy),
        },
        null,
        2,
      );
    }
    case 'batch_design':
      return JSON.stringify(await handleBatchDesign(a), null, 2);
    case 'design_skeleton':
      return JSON.stringify(await handleDesignSkeleton(a), null, 2);
    case 'design_content':
      return JSON.stringify(await handleDesignContent(a), null, 2);
    case 'design_refine':
      return JSON.stringify(await handleDesignRefine(a), null, 2);
    default:
      if (ELEMENT_TOOL_NAMES.has(name)) return handleElementToolCall(name, a);
      return '';
  }
}

// --- D0 parity spike tool (gated behind OPENPENCIL_D0_SPIKE=1) ---
//
// add_section_v0 是一次性 parity 验证工具，不进生产工具集。与 debug tools
// 同样走环境变量条件暴露模式（见 server.ts 里对 DEBUG_TOOL_DEFINITIONS 的处理）。
// 默认不出现在 ListTools 响应里，避免外部 MCP 客户端误用。
// Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §D0

export const D0_SPIKE_TOOL_DEFINITIONS = [
  {
    name: 'add_section_v0',
    description:
      'D0 parity spike tool (not for production, gated by OPENPENCIL_D0_SPIKE=1). ' +
      'Insert one section frame with minimal schema: title (becomes frame.name) + ' +
      'layout (horizontal|vertical). Validates N-tool additive-only hypothesis. ' +
      'See spec openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §D0. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description:
            'Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        title: { type: 'string', description: 'Section name (becomes frame.name)' },
        layout: {
          type: 'string',
          enum: ['horizontal', 'vertical'],
          description: 'Flex direction',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['title', 'layout'],
    },
  },
];

export const D0_SPIKE_TOOL_NAMES = new Set(['add_section_v0']);

export async function handleD0SpikeToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const a = args as any;
  switch (name) {
    case 'add_section_v0':
      return JSON.stringify(await handleAddSectionV0(a), null, 2);
    default:
      return '';
  }
}
