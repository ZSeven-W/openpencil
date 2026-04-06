import { handleReadNodes } from '../tools/read-nodes';
import { handleCodegenPlan } from '../tools/codegen-plan';
import { handleCodegenSubmit } from '../tools/codegen-submit';
import { handleCodegenAssemble } from '../tools/codegen-assemble';
import { handleCodegenClean } from '../tools/codegen-clean';

export const CODEGEN_TOOL_DEFINITIONS = [
  {
    name: 'read_nodes',
    description:
      'Read nodes from the document with depth control. Omit nodeIds to get all top-level children of the page. Use depth=0 for node properties only (children truncated to "..."), depth=1 for direct children, depth=-1 for full subtree. Use this instead of the deprecated export_nodes.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        nodeIds: {
          type: 'array',
          items: { type: 'string' },
          description: 'Node IDs to read. If omitted, returns all top-level page children.',
        },
        depth: {
          type: 'number',
          description:
            'Subtree depth. 0=node only (children="..."), 1=direct children, -1=full. Default: -1.',
        },
        pageId: { type: 'string', description: 'Target page ID. Default: active page.' },
        filePath: { type: 'string', description: 'Path to .op file. Default: live canvas.' },
        includeVariables: {
          type: 'boolean',
          description: 'Attach variables/themes. Default: false.',
        },
      },
      required: [],
    },
  },
  {
    name: 'codegen_plan',
    description:
      'Submit a code generation plan (CodePlanFromAI). The server validates the plan (rejects on duplicate chunkIds, empty nodeIds, unknown dependencies, circular deps, missing nodeIds) and returns a topologically sorted executionPlan with warnings for shared nodeIds. The planId returned is used by codegen_submit_chunk and codegen_assemble.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        plan: {
          type: 'object',
          description: 'CodePlanFromAI: { chunks, sharedStyles, rootLayout }',
        },
        filePath: { type: 'string', description: 'Path to .op file. Default: live canvas.' },
        pageId: { type: 'string', description: 'Target page ID. Default: active page.' },
      },
      required: ['plan'],
    },
  },
  {
    name: 'codegen_submit_chunk',
    description:
      'Submit generated code for one chunk. Returns validation result, progress of all chunks, and the next chunk ready for generation (if any). Resubmitting the same chunkId overrides the previous result.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        planId: { type: 'string', description: 'Plan ID from codegen_plan.' },
        result: {
          type: 'object',
          description: 'ChunkResult: { chunkId, code, contract }',
        },
        status: {
          type: 'string',
          enum: ['failed', 'skipped'],
          description:
            'Optional override to mark chunk as failed or skipped (dependency contract passed as null downstream).',
        },
      },
      required: ['planId', 'result'],
    },
  },
  {
    name: 'codegen_assemble',
    description:
      'Retrieve all chunk results in topological order for final assembly. Terminal operation — clears plan cache after returning. Returns degraded=true if any chunk is not done.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        planId: { type: 'string', description: 'Plan ID from codegen_plan.' },
        framework: {
          type: 'string',
          enum: ['react', 'vue', 'svelte', 'html', 'flutter', 'swiftui', 'compose', 'react-native'],
          description: 'Target framework.',
        },
      },
      required: ['planId', 'framework'],
    },
  },
  {
    name: 'codegen_clean',
    description: 'Manually clean up an abandoned codegen plan. Idempotent.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        planId: { type: 'string', description: 'Plan ID to clean up.' },
      },
      required: ['planId'],
    },
  },
];

export const CODEGEN_TOOL_NAMES = new Set([
  'read_nodes',
  'codegen_plan',
  'codegen_submit_chunk',
  'codegen_assemble',
  'codegen_clean',
]);

// eslint-disable-next-line @typescript-eslint/no-explicit-any -- MCP args are validated at runtime by the protocol
export async function handleCodegenToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  const a = args as any; // eslint-disable-line @typescript-eslint/no-explicit-any
  switch (name) {
    case 'read_nodes':
      return JSON.stringify(await handleReadNodes(a), null, 2);
    case 'codegen_plan':
      return handleCodegenPlan(a);
    case 'codegen_submit_chunk':
      return handleCodegenSubmit(a);
    case 'codegen_assemble':
      return handleCodegenAssemble(a);
    case 'codegen_clean':
      return handleCodegenClean(a);
    default:
      return '';
  }
}
