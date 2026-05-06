import type { ToolContent } from '../server';
import { handleValidationReport } from '../tools/debug-validation-report';
import { handleLogsTail } from '../tools/debug-logs-tail';
import { handleScreenshot, type DebugScreenshotParams } from '../tools/debug-screenshot';

/**
 * DEBUG MCP 工具
 * — 仅在使用 `--debug` 标志或 `OPENPENCIL_DEBUG_TOOLS=1` 环境变量启动服务器时公开。
 *
 * These 工具用于笔人工智能技能开发和诊断。 They MUST NOT 暴露给生产 MCP 客户端。
 *
 */
export const DEBUG_TOOL_DEFINITIONS = [
  {
    name: 'debug_validation_report',
    description:
      'Run pure-function design quality detectors on the current document ' +
      '(or a subtree) and return issues without applying any fixes. ' +
      'Intended for debugging pen-ai-skills output quality. ' +
      'Requires --debug mode.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas.',
        },
        pageId: { type: 'string', description: 'Target page ID.' },
        rootNodeId: {
          type: 'string',
          description: 'Limit detection to this subtree. Omit for whole page.',
        },
        categories: {
          type: 'array',
          items: {
            type: 'string',
            enum: [
              'invisible-container',
              'empty-path',
              'text-explicit-height',
              'sibling-inconsistency',
            ],
          },
          description: 'Filter to specific detector categories.',
        },
        maxIssues: {
          type: 'number',
          description: 'Cap on returned issues (default 200).',
        },
      },
      required: [],
    },
  },
  {
    name: 'debug_logs_tail',
    description:
      'Read the tail of the Nitro server log (~/.openpencil/logs/server-YYYY-MM-DD.log) ' +
      'with API keys and Authorization headers redacted. ' +
      'Requires --debug mode.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        tailLines: {
          type: 'number',
          description: 'Maximum lines to return (default 100, max 500).',
        },
        sinceMs: {
          type: 'number',
          description: 'Unix ms timestamp — only return lines newer than this.',
        },
        grep: {
          type: 'string',
          description: 'Regex to filter lines by content (applied after redaction).',
        },
      },
      required: [],
    },
  },
  {
    name: 'debug_screenshot',
    description:
      'Capture a PNG screenshot of the live canvas via the renderer. ' +
      'Requires an active Electron/dev session with the canvas loaded and focused. ' +
      'Returns an MCP image content block that vision-capable clients (Claude Code, Codex) can see. ' +
      'Only "render" preset is supported; no debug overlays in this phase. ' +
      'Requires --debug mode.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        target: {
          type: 'string',
          enum: ['node', 'root'],
          description: '"node" captures a specific node bbox; "root" captures the full document.',
        },
        nodeId: {
          type: 'string',
          description: 'Required when target="node".',
        },
        padding: {
          type: 'number',
          description: 'Extra pixels around the bounds (default 0).',
        },
        dpr: {
          type: 'number',
          description: 'Device pixel ratio override (default: renderer default).',
        },
        timeoutMs: {
          type: 'number',
          description: 'Max wait for renderer response (default 15000, max 60000).',
        },
      },
      required: ['target'],
    },
  },
] as const;

export const DEBUG_TOOL_NAMES: ReadonlySet<string> = new Set(
  DEBUG_TOOL_DEFINITIONS.map((t) => t.name),
);

/**
 * Future 安全签名
 * (Promise<string | ToolContent[]>)，因此 Phase 2 可以添加 `debug_screenshot`
 * 和图像内容块结果，而无需再次触及此文件的类型签名。
 */
export async function handleDebugToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string | ToolContent[]> {
  switch (name) {
    case 'debug_validation_report':
      return handleValidationReport(args);
    case 'debug_logs_tail':
      return handleLogsTail(args);
    case 'debug_screenshot':
      return handleScreenshot(args as unknown as DebugScreenshotParams);
    default:
      throw new Error(`Unknown debug tool: ${name}`);
  }
}
