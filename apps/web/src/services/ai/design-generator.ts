import type { PenNode } from '@/types/pen';
import type { VariableDefinition, ThemedValue } from '@/types/variables';
import type { AIProviderType } from '@/types/agent-settings';
import type { DesignMdSpec } from '@/types/design-md';
import type { AIDesignRequest } from './ai-types';
import { streamChat } from './ai-service';
import { resolveSkills } from '@zseven-w/pen-ai-skills';
import { buildDesignMdStylePolicy } from './ai-prompts';
import { executeOrchestration } from './orchestrator';
import { DESIGN_STREAM_TIMEOUTS } from './ai-runtime-config';
import { extractJsonFromResponse } from './design-parser';
import { resolveModelProfile, applyProfileToTimeouts } from './model-profiles';

// ---------------------------------------------------------------------------
// 向后兼容层：保留旧导出，避免现有调用方修改导入路径
// ---------------------------------------------------------------------------
// 从设计解析器重新导出
export { extractJsonFromResponse, extractStreamingNodes } from './design-parser';
export type { StreamingNodeResult } from './design-parser';

// 从画布操作模块重新导出
export {
  resetGenerationRemapping,
  setGenerationContextHint,
  setGenerationCanvasWidth,
  getGenerationRootFrameId,
  getGenerationRemappedIds,
  insertStreamingNode,
  applyNodesToCanvas,
  upsertNodesToCanvas,
  animateNodesToCanvas,
  extractAndApplyDesign,
  extractAndApplyDesignModification,
  adjustRootFrameHeightToContent,
  expandRootFrameHeight,
  applyPostStreamingTreeHeuristics,
  applyGenerationHeuristics,
} from './design-canvas-ops';

/** 为 AI 上下文构造一段精简的文档变量摘要。 */
export function buildVariableContext(
  variables?: Record<string, VariableDefinition>,
  themes?: Record<string, string[]>,
): string | null {
  if (!variables || Object.keys(variables).length === 0) return null;

  const lines: string[] = [
    'DOCUMENT VARIABLES (use "$name" to reference, e.g. fill color "$color-1"):',
  ];

  for (const [name, def] of Object.entries(variables)) {
    const val = def.value;
    if (Array.isArray(val)) {
      // 主题变量：这里只展示默认值，方便模型快速理解
      const defaultVal = (val as ThemedValue[])[0]?.value ?? '?';
      lines.push(`  - ${name} (${def.type}): ${defaultVal} [themed]`);
    } else {
      lines.push(`  - ${name} (${def.type}): ${val}`);
    }
  }

  if (themes && Object.keys(themes).length > 0) {
    const themeSummary = Object.entries(themes)
      .map(([axis, values]) => `${axis}: [${values.join(', ')}]`)
      .join('; ');
    lines.push(`Themes: ${themeSummary}`);
  }

  return lines.join('\n');
}

// ---------------------------------------------------------------------------
// 编排式设计生成
// ---------------------------------------------------------------------------

export async function generateDesign(
  request: AIDesignRequest,
  callbacks?: {
    onApplyPartial?: (count: number) => void;
    onTextUpdate?: (text: string) => void;
    /** 为 `true` 时，节点会以交错淡入动画插入画布。 */
    animated?: boolean;
  },
  abortSignal?: AbortSignal,
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  return executeOrchestration(request, callbacks, abortSignal);
}

// ---------------------------------------------------------------------------
// 设计修改（选中节点 + 修改指令）
// ---------------------------------------------------------------------------

export async function generateDesignModification(
  nodesToModify: PenNode[],
  instruction: string,
  options?: {
    variables?: Record<string, VariableDefinition>;
    themes?: Record<string, string[]>;
    designMd?: DesignMdSpec;
    model?: string;
    provider?: AIProviderType;
  },
  abortSignal?: AbortSignal,
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  // 从选中的节点构造上下文
  const contextJson = JSON.stringify(nodesToModify, (_key, value) => {
    // 这里保持原样返回，保留完整树结构给模型使用
    return value;
  });

  // 用普通字符串拼接，避免工具调用场景里的反引号转义问题
  let userMessage = 'CONTEXT NODES:\n' + contextJson + '\n\nINSTRUCTION:\n' + instruction;

  // 追加变量上下文，让模型知道可以直接引用 `$variable`
  const varContext = buildVariableContext(options?.variables, options?.themes);
  if (varContext) {
    userMessage += '\n\n' + varContext;
  }
  let fullResponse = '';
  let streamError: string | null = null;

  const profile = resolveModelProfile(options?.model);
  const timeouts = applyProfileToTimeouts({ ...DESIGN_STREAM_TIMEOUTS }, profile);

  // 解析 maintenance 阶段的技能，用于“修改已有设计”这类提示
  const maintenanceCtx = resolveSkills('maintenance', instruction, {
    flags: {
      hasVariables: !!options?.variables && Object.keys(options.variables).length > 0,
      hasDesignMd: !!options?.designMd,
    },
  });
  let modifierPrompt = maintenanceCtx.skills.map((s) => s.content).join('\n\n');
  // 如果有 design.md，再额外把它补进来
  // `design-md` 技能本身只在 generation 阶段自动生效，所以这里手动追加
  if (options?.designMd) {
    modifierPrompt += '\n\n' + buildDesignMdStylePolicy(options.designMd);
  }

  for await (const chunk of streamChat(
    modifierPrompt,
    [{ role: 'user', content: userMessage }],
    options?.model,
    timeouts,
    options?.provider,
    abortSignal,
  )) {
    if (chunk.type === 'thinking') {
      // 修改模式下忽略 thinking 块，调用方已经有自己的进度展示
    } else if (chunk.type === 'text') {
      fullResponse += chunk.content;
    } else if (chunk.type === 'error') {
      streamError = chunk.content;
      break;
    }
  }

  const streamedNodes = extractJsonFromResponse(fullResponse);
  if (streamedNodes && streamedNodes.length > 0) {
    return { nodes: streamedNodes, rawResponse: fullResponse };
  }

  if (streamError) {
    throw new Error(streamError);
  }

  const preview = fullResponse.trim().slice(0, 150);
  const hint =
    fullResponse.trim().length === 0
      ? 'The model returned an empty response.'
      : `Model output: "${preview}${fullResponse.length > 150 ? '…' : ''}"`;
  throw new Error(`Could not parse design nodes from model response. ${hint}`);
}
