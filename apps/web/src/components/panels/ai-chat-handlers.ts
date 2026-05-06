import { useState, useCallback } from 'react';
import { nanoid } from 'nanoid';
import i18n from '@/i18n';
import type { AgentEvent } from '@/types/agent';
import type { AIProviderType } from '@/types/agent-settings';

function decodeAgentEvent(raw: string): AgentEvent | null {
  const eventMatch = raw.match(/^event:\s*(\S+)/);
  const dataMatch = raw.match(/^data:\s*(.+)$/m);
  if (!eventMatch || !dataMatch) return null;
  try {
    const type = eventMatch[1] as AgentEvent['type'];
    const payload = JSON.parse(dataMatch[1]);
    return { type, ...payload } as AgentEvent;
  } catch {
    return null;
  }
}
import { useAIStore } from '@/stores/ai-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { useDesignMdStore } from '@/stores/design-md-store';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import { getActivePageChildren } from '@/stores/document-tree-utils';
import { streamChat } from '@/services/ai/ai-service';
import { buildChatSystemPrompt } from '@/services/ai/ai-prompts';
import {
  generateDesign,
  generateDesignModification,
  animateNodesToCanvas,
  extractAndApplyDesignModification,
} from '@/services/ai/design-generator';
import { StreamingDesignRenderer } from '@/services/ai/streaming-design-renderer';
import { assignAgentIdentities } from '@/services/ai/agent-identity';
import type { AgentIdentity } from '@/services/ai/agent-identity';
import { applyPostStreamingTreeHeuristics } from '@/services/ai/design-canvas-ops';
import { trimChatHistory } from '@/services/ai/context-optimizer';
import { AgentToolExecutor } from '@/services/ai/agent-tool-executor';
import { getDesignToolDefs } from '@/services/ai/agent-tools';
import type { ChatMessage as ChatMessageType } from '@/services/ai/ai-types';
import type { ToolCallBlockData } from '@/components/panels/tool-call-block';
import { CHAT_STREAM_THINKING_CONFIG } from '@/services/ai/ai-runtime-config';
import { classifyIntent } from './ai-chat-intent-classifier';
import { buildContextString } from './ai-chat-context-builder';
import { detectAgentIntent, getCrudToolDefs } from '@/services/ai/agent-tools';

// Re-为任何外部消费者导出
export { buildContextString } from './ai-chat-context-builder';

// ---------------------------------------------------------------------------
// Agent 模式 SSE 流处理程序
// ---------------------------------------------------------------------------

/** Agent 提供者的 Agent 工具说明 — 通过 generate_design 委托给协调器。 */
const AGENT_TOOL_INSTRUCTIONS_CLI = `You are a design assistant. You MUST use tools to do your work.

RULE 1: To create or design ANYTHING, call generate_design. NEVER output JSON yourself.
RULE 2: After every tool call, write 1-2 sentences summarizing what happened.
RULE 3: When calling generate_design, write a detailed prompt with style direction (colors, shadows, spacing, visual hierarchy).

IMPORTANT: Always end your turn with a short natural-language reply for the user.
- After any tool call completes, reply with ONLY a 1-3 sentence summary of what was created.
- NEVER output JSON, code blocks, or PenNode data in your text reply. The tool already handled it.
- If work completed successfully, say what changed or what was created.
- If nothing changed, explain why in one sentence.
- Never end with tool calls only.

FORBIDDEN: Do not output JSON, code blocks, or node definitions directly. Always use generate_design instead.`;

/** 内置提供程序的 Agent 工具说明 — 与 CLI 相同的 generate_design 管道。 */
const AGENT_TOOL_INSTRUCTIONS_BUILTIN = `You are a design assistant. You MUST use tools to do your work.

RULE 1: To create or design ANYTHING, call generate_design. NEVER output JSON yourself.
RULE 2: After the tool call completes, write 1-2 sentences summarizing what happened.
RULE 3: Do NOT call generate_design more than once unless the user asks for a new design.

FORBIDDEN: Do not output JSON, code blocks, or node definitions directly. Always use generate_design instead.`;

/** Lightweight 提示 CRUD 操作 — 没有设计技巧，只有工具使用。 */
const AGENT_TOOL_INSTRUCTIONS_CRUD = `You are a design editor. Use tools to inspect, modify, insert, and delete elements on the canvas.

WORKFLOW:
1. Use snapshot_layout or batch_get FIRST to see the tree structure and find node IDs.
2. Use the appropriate tool: insert_node to add, update_node to modify, delete_node to remove, move_node to reparent.
3. When inserting, use "after" parameter with a sibling ID to place the new node in the correct position.
4. After each operation, write 1-2 sentences summarizing what changed.

DIAGNOSING OVERLAP / STACKING BUGS — read this before "fixing" any visual overlap:
- When snapshot_layout.overlaps is non-empty, two or more siblings share screen area. Do NOT blindly enlarge heights, shrink fonts, or tweak padding — those are surface patches.
- Inspect the overlapping nodes' shared PARENT via batch_get. Look at its \`layout\` field:
  • \`layout: "none"\` (or missing) → children positioned via absolute x/y. OpenPencil's renderer has a known bug where absolute-positioned children stack vertically instead of honoring x/y. This is almost always the true root cause.
  • \`layout: "vertical"\` with gap=0 and children using textGrowth:"fit_content" → text can visually touch; bump \`gap\` or add padding on the children.
- Preferred fix for \`layout: "none"\` parents that contain stacked content (badges, titles, rows):
  update_node(parent, { layout: "vertical", gap: 8, alignItems: "flex-start" })
  and strip the children's absolute x/y (the flex engine positions them).
- For a circle/ring with centered content: NEVER use \`layout: "none"\`. Use a frame with cornerRadius = width/2, layout:"horizontal", alignItems:"center", justifyContent:"center", children:[ the text/icon ].

INSERT_NODE GUIDE — always include complete node data with children:
- Button example: {"type":"frame","name":"My Button","width":"fill_container","height":50,"cornerRadius":8,"fill":[{"type":"solid","color":"#1877F2"}],"layout":"horizontal","gap":8,"alignItems":"center","justifyContent":"center","children":[{"type":"icon_font","name":"Icon","iconName":"facebook","width":20,"height":20,"fill":[{"type":"solid","color":"#FFFFFF"}]},{"type":"text","name":"Label","text":"Continue with Facebook","fontSize":15,"fontWeight":600,"fill":[{"type":"solid","color":"#FFFFFF"}]}]}
- Text example: {"type":"text","name":"Title","text":"Hello","fontSize":24,"fontWeight":700,"fill":[{"type":"solid","color":"#1A1A2E"}]}
- When adding next to a similar element, use batch_get to read that element's full data first, then create matching structure.

Focus on the specific operation the user requested.`;

/** Agent 针对协调团队的首席代理的说明。 */
const AGENT_TOOL_INSTRUCTIONS_TEAM = `You are a design lead coordinating a team.

Do not create the design directly in this mode. Analyze the request, delegate the work to team members, then summarize the outcome for the user.`;

/**
 * Build 代理系统根据
 * 提供商类型和检测到的意图进行提示。 CRUD 意图 (read/update/delete) 获得轻量级提示，无需设计技能。 Design
 * 意图获取完整的设计生成流程。
 */
function buildAgentSystemPrompt(
  userMessage: string,
  isBuiltin: boolean,
  teamMode: boolean,
): string {
  if (teamMode) return AGENT_TOOL_INSTRUCTIONS_TEAM;
  const intent = detectAgentIntent(userMessage);
  if (intent === 'crud') return AGENT_TOOL_INSTRUCTIONS_CRUD;
  return isBuiltin ? AGENT_TOOL_INSTRUCTIONS_BUILTIN : AGENT_TOOL_INSTRUCTIONS_CLI;
}

/**
 * Parse SSE 来自
 * ReadableStream 块并产生 AgentEvents。 Handles 可以在读取之间分割的部分块。
 */
async function* parseAgentSSE(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  signal: AbortSignal,
): AsyncGenerator<AgentEvent> {
  const decoder = new TextDecoder();
  let buffer = '';

  while (!signal.aborted) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const chunks = buffer.split('\n\n');
    buffer = chunks.pop() ?? '';

    for (const chunk of chunks) {
      const trimmed = chunk.trim();
      if (!trimmed) continue;
      const evt = decodeAgentEvent(trimmed);
      if (evt) yield evt;
    }
  }

  if (buffer.trim()) {
    const evt = decodeAgentEvent(buffer.trim());
    if (evt) yield evt;
  }
}

/** Provider 代理管道的配置 */
interface AgentProviderConfig {
  providerType: 'anthropic' | 'openai-compat' | 'acp';
  apiKey: string;
  model: string;
  baseURL?: string;
  maxOutputTokens?: number;
  maxContextTokens?: number;
}

/** Strip <think>...</think> 模型文本输出中的标签（闭合和未闭合）。 */
function stripThinkTags(text: string): string {
  return text.replace(/<think>[\s\S]*?<\/think>\s*/g, '').replace(/<think>[\s\S]*$/g, '');
}

function buildAgentEmptyOutputFallback({
  sawThinking,
  sawToolActivity,
  sawMemberActivity,
}: {
  sawThinking: boolean;
  sawToolActivity: boolean;
  sawMemberActivity: boolean;
}): string {
  if (sawToolActivity) {
    return '*Completed, but the agent returned no final summary. The requested action may still have been applied.*';
  }

  if (sawMemberActivity) {
    return '*Completed team execution, but no final summary was returned.*';
  }

  if (sawThinking) {
    return '*The agent finished without a final answer. Please retry if you need a written response.*';
  }

  return '*The agent finished without producing a visible response. Please retry.*';
}

/**
 * Send 通过代理管道的
 * 消息。 Opens 与 /api/ai/agent 的 SSE
 * 连接，调度工具调用客户端，并实时更新 AI 存储。
 */
async function runAgentStream(
  assistantMsgId: string,
  providerConfig: AgentProviderConfig,
  abortController: AbortController,
) {
  const store = useAIStore.getState();
  const { updateLastMessage } = store;

  const sessionId = nanoid();
  const executor = new AgentToolExecutor(sessionId);

  const isBuiltin =
    providerConfig.providerType === 'anthropic' || providerConfig.providerType === 'openai-compat';

  const messages = useAIStore
    .getState()
    .messages.filter((m) => m.id !== assistantMsgId)
    .map((m) => ({ role: m.role, content: m.content }));

  const context = buildContextString();
  const lastUserMsg = messages[messages.length - 1]?.content ?? '';

  // Read 团队成员技能加载的文档上下文
  const { useDesignMdStore } = await import('@/stores/design-md-store');
  const { useDocumentStore } = await import('@/stores/document-store');
  const { buildDesignMdStylePolicy } = await import('@/services/ai/ai-prompts');
  const designMd = useDesignMdStore.getState().designMd;
  const docVariables = useDocumentStore.getState().document.variables;
  const hasVariables = !!docVariables && Object.keys(docVariables).length > 0;
  const designMdContent = designMd ? buildDesignMdStylePolicy(designMd) : undefined;

  const { useAIStore: concurrencyStore } = await import('@/stores/ai-store');
  const concurrency = concurrencyStore.getState().concurrency;
  const teamMode = concurrency > 1;
  const intent = detectAgentIntent(lastUserMsg);
  const toolDefs = intent === 'crud' ? getCrudToolDefs() : getDesignToolDefs();
  const systemPrompt = buildAgentSystemPrompt(lastUserMsg, isBuiltin, teamMode) + context;

  const agentBody: Record<string, unknown> = {
    sessionId,
    messages,
    systemPrompt,
    providerType: providerConfig.providerType,
    apiKey: providerConfig.apiKey,
    model: providerConfig.model,
    ...(providerConfig.baseURL ? { baseURL: providerConfig.baseURL } : {}),
    ...(providerConfig.maxOutputTokens ? { maxOutputTokens: providerConfig.maxOutputTokens } : {}),
    ...(providerConfig.maxContextTokens
      ? { maxContextTokens: providerConfig.maxContextTokens }
      : {}),
    toolDefs,
    maxTurns: 20,
    ...(teamMode ? { teamMode: true, concurrency } : {}),
    ...(designMdContent ? { designMdContent } : {}),
    ...(hasVariables ? { hasVariables } : {}),
  };

  // ACP：将 agentId + config 添加到请求正文（如果由于开发服务器重新启动而导致内存中连接丢失，配置将启用服务器端自动重新连接）。
  if (providerConfig.providerType === 'acp') {
    const agentId = providerConfig.model.slice(4);
    const acpConfig = useAgentSettingsStore.getState().acpAgents.find((a) => a.id === agentId);
    (agentBody as any).acpAgentId = agentId;
    if (acpConfig) (agentBody as any).acpConfig = acpConfig;
  }

  const response = await fetch('/api/ai/agent', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(agentBody),
    signal: abortController.signal,
  });

  if (!response.ok || !response.body) {
    const errText = await response.text().catch(() => 'Unknown error');
    // h3 错误的形式为 JSON: { message, error, status, ... } — 仅提取消息。
    let errorMessage = errText;
    try {
      const parsed = JSON.parse(errText);
      if (parsed && typeof parsed === 'object' && typeof parsed.message === 'string') {
        errorMessage = parsed.message;
      }
    } catch {
      /* 不是 JSON — 使用原始文本 */
    }
    throw new Error(errorMessage);
  }

  const reader = response.body.getReader();
  let accumulated = '';
  let thinkingContent = '';
  let sawThinking = false;
  let sawToolActivity = false;
  let sawMemberActivity = false;
  const [defaultIdentity] = assignAgentIdentities(1);
  const renderer = new StreamingDesignRenderer({
    agentColor: defaultIdentity.color,
    agentName: defaultIdentity.name,
    animated: true,
  });

  let identityPool: AgentIdentity[] = [];
  let nextIdentityIdx = 0;
  const memberIdentities = new Map<string, AgentIdentity>();
  // Track 最近一次失败的工具调用，因此终端 `error_server` 事件（不包含来自 Zig 引擎的详细信息）可以显示实际发生的问题。
  const toolNames = new Map<string, string>();
  let lastToolError: { name: string; message: string } | null = null;

  try {
    for await (const evt of parseAgentSSE(reader, abortController.signal)) {
      switch (evt.type) {
        case 'thinking': {
          sawThinking = true;
          thinkingContent += evt.content;
          const thinkingStep = `<step title="Thinking">${thinkingContent}</step>`;
          updateLastMessage(thinkingStep + (accumulated ? '\n' + accumulated : ''));
          break;
        }

        case 'text': {
          accumulated += evt.content ?? '';
          const prefix = thinkingContent
            ? `<step title="Thinking">${thinkingContent}</step>\n`
            : '';
          updateLastMessage(prefix + stripThinkTags(accumulated));
          // Don't call renderer.feedText() here — agent text output should go
          // through generateDesign() if it contains design JSON (handled in 'done').
          // Calling feedText() would insert nodes that we'd have to delete later.
          break;
        }

        case 'tool_call': {
          sawToolActivity = true;
          const block: ToolCallBlockData = {
            id: evt.id,
            name: evt.name,
            args: evt.args,
            level: evt.level,
            status: evt.level === 'orchestrate' ? 'done' : 'running',
            source: evt.source,
          };
          useAIStore.getState().addToolCallBlock(block);

          // Skip internal team coordination tools — they are resolved by agent-team, not the client
          if (evt.level === 'orchestrate') break;

          toolNames.set(evt.id, evt.name);
          executor
            .execute(evt as Extract<AgentEvent, { type: 'tool_call' }>)
            .then((result) => {
              const block = useAIStore.getState().toolCallBlocks.find((b) => b.id === evt.id);
              if (block && block.status === 'running') {
                useAIStore.getState().updateToolCallBlock(evt.id, {
                  status: result?.success !== false ? 'done' : 'error',
                  result: result ?? undefined,
                });
              }
              if (result && result.success === false) {
                lastToolError = {
                  name: evt.name,
                  message: String(result.error ?? 'unknown error'),
                };
              }
            })
            .catch((err) => {
              useAIStore.getState().updateToolCallBlock(evt.id, {
                status: 'error',
                result: { success: false, error: String(err) },
              });
              lastToolError = { name: evt.name, message: String(err) };
            });
          break;
        }

        case 'tool_result': {
          sawToolActivity = true;
          useAIStore.getState().updateToolCallBlock(evt.id, {
            status: evt.result.success ? 'done' : 'error',
            result: evt.result,
          });
          if (!evt.result.success) {
            lastToolError = {
              name: toolNames.get(evt.id) ?? 'tool',
              message: String((evt.result as { error?: unknown }).error ?? 'unknown error'),
            };
          }
          break;
        }

        case 'turn':
          break;

        case 'done': {
          if (!accumulated.trim()) {
            accumulated = buildAgentEmptyOutputFallback({
              sawThinking,
              sawToolActivity,
              sawMemberActivity,
            });
            // Preserve thinking steps in the final message
            const prefix = thinkingContent
              ? `<step title="Thinking">${thinkingContent}</step>\n`
              : '';
            updateLastMessage(prefix + accumulated);
          }

          if (renderer.getAppliedIds().size === 0) {
            renderer.flushRemaining(accumulated);
          }

          // Force-insert any orphan nodes whose parents never arrived
          renderer.forceFlushPending();

          const rootId = renderer.getRootId();
          if (rootId) {
            applyPostStreamingTreeHeuristics(rootId);
          }

          renderer.finish();
          break;
        }

        case 'error': {
          // Terminal `Agent error: error_server` events from the Zig engine
          // carry no detail. Fall back to the last failed tool call so the
          // user sees the actual cause (e.g. an upstream 529 surfaced via
          // a tool error, or a runtime exception inside the design pipeline).
          let detail = '';
          if (lastToolError && /^Agent error:/i.test(evt.message)) {
            detail = `\n> Last tool failure (\`${lastToolError.name}\`): ${lastToolError.message}`;
          }
          accumulated += `\n\n**Error:** ${evt.message}${detail}`;
          updateLastMessage(accumulated);
          renderer.finish();
          if (evt.fatal) return stripThinkTags(accumulated);
          break;
        }

        case 'member_start': {
          sawMemberActivity = true;
          if (identityPool.length === 0) {
            identityPool = assignAgentIdentities(6);
          }
          const identity = identityPool[nextIdentityIdx % identityPool.length];
          nextIdentityIdx++;
          memberIdentities.set(evt.memberId, identity);
          renderer.setIdentity(identity.color, identity.name);
          accumulated += `\n\n> **[${identity.name}]** ${evt.task}\n`;
          updateLastMessage(accumulated);
          break;
        }

        case 'member_end': {
          sawMemberActivity = true;
          const id = memberIdentities.get(evt.memberId);
          accumulated += `\n> **[${id?.name ?? evt.memberId}]** done\n\n`;
          updateLastMessage(accumulated);
          renderer.setIdentity('#2563EB', 'Agent');
          break;
        }

        case 'abort':
          return;
      }
    }
  } catch (error) {
    if (abortController.signal.aborted) {
      return stripThinkTags(accumulated);
    }
    throw error;
  } finally {
    renderer.finish();
    reader.releaseLock();
  }

  return stripThinkTags(accumulated);
}

/** Shared chat logic hook — orchestrates intent classification, context building, and dispatching. */
export function useChatHandlers() {
  const [input, setInput] = useState('');
  const messages = useAIStore((s) => s.messages);
  const isStreaming = useAIStore((s) => s.isStreaming);
  const model = useAIStore((s) => s.model);
  const availableModels = useAIStore((s) => s.availableModels);
  const isLoadingModels = useAIStore((s) => s.isLoadingModels);
  const addMessage = useAIStore((s) => s.addMessage);
  const updateLastMessage = useAIStore((s) => s.updateLastMessage);
  const setStreaming = useAIStore((s) => s.setStreaming);

  const handleSend = useCallback(
    async (text?: string) => {
      const messageText = text ?? input.trim();
      const pendingAttachments = useAIStore.getState().pendingAttachments;
      const hasAttachments = pendingAttachments.length > 0;
      if (
        (!messageText && !hasAttachments) ||
        isStreaming ||
        isLoadingModels ||
        availableModels.length === 0
      )
        return;

      setInput('');
      useAIStore.getState().clearPendingAttachments();

      const selectedIds = useCanvasStore.getState().selection.selectedIds;
      const hasSelection = selectedIds.length > 0;

      const context = buildContextString();
      const fullUserMessage = messageText + context;

      const userMsg: ChatMessageType = {
        id: nanoid(),
        role: 'user',
        content: messageText || '',
        timestamp: Date.now(),
        ...(hasAttachments ? { attachments: pendingAttachments } : {}),
      };
      addMessage(userMsg);

      const assistantMsg: ChatMessageType = {
        id: nanoid(),
        role: 'assistant',
        content: '',
        timestamp: Date.now(),
        isStreaming: true,
      };
      addMessage(assistantMsg);
      setStreaming(true);

      // Set chat title if it's the first message
      if (messages.length === 0) {
        const cleanText = messageText.replace(/^(Design|Create|Generate|Make)\s+/i, '');
        const words = cleanText.split(' ').slice(0, 4).join(' ');
        const title = words.length > 30 ? words.slice(0, 30) + '...' : words;
        useAIStore.getState().setChatTitle(title || 'New Chat');
      }

      // For builtin models, force provider to 'builtin' — modelGroups may
      // report 'anthropic' based on the upstream API type, but streamChat/
      // orchestrator need 'builtin' to route through the correct server path.
      const currentProvider = useAIStore
        .getState()
        .modelGroups.find((g) => g.models.some((m) => m.value === model))?.provider;

      const abortController = new AbortController();
      useAIStore.getState().setAbortController(abortController);

      let accumulated = '';

      // -----------------------------------------------------------------------
      // BUILT-IN PROVIDER (Agent) MODE — uses Zig engine via runAgentStream()
      // Rendering is consistent with orchestrator path via shared
      // StreamingDesignRenderer (breathing glow, animation, cleanup).
      // -----------------------------------------------------------------------
      if (model.startsWith('builtin:')) {
        const parts = model.split(':');
        const builtinProviderId = parts[1];
        const modelName = parts.slice(2).join(':');

        const { builtinProviders } = useAgentSettingsStore.getState();
        const bp = builtinProviders.find((p) => p.id === builtinProviderId);
        if (!bp || !bp.apiKey) {
          accumulated = !bp
            ? `**Error:** ${i18n.t('builtin.errorProviderNotFound')}`
            : `**Error:** ${i18n.t('builtin.errorApiKeyEmpty')}`;
          updateLastMessage(accumulated);
          useAIStore.getState().setAbortController(null);
          setStreaming(false);
          useAIStore.setState((s) => {
            const msgs = [...s.messages];
            const last = msgs.find((m) => m.id === assistantMsg.id);
            if (last) {
              last.content = accumulated;
              last.isStreaming = false;
            }
            return { messages: msgs };
          });
          return;
        }

        useAIStore.getState().clearToolCallBlocks();
        try {
          const result = await runAgentStream(
            assistantMsg.id,
            {
              providerType: bp.type === 'anthropic' ? 'anthropic' : 'openai-compat',
              apiKey: bp.apiKey,
              model: modelName,
              baseURL: bp.baseURL,
              maxContextTokens: bp.maxContextTokens,
            },
            abortController,
          );
          if (result) accumulated = result;
        } catch (error) {
          if (!abortController.signal.aborted) {
            const errMsg = error instanceof Error ? error.message : 'Unknown error';
            accumulated += `\n\n**Error:** ${errMsg}`;
            updateLastMessage(accumulated);
          }
        } finally {
          useAIStore.getState().setAbortController(null);
          setStreaming(false);
        }

        useAIStore.setState((s) => {
          const msgs = [...s.messages];
          const last = msgs.find((m) => m.id === assistantMsg.id);
          if (last) {
            last.content = accumulated;
            last.isStreaming = false;
          }
          return { messages: msgs };
        });
        return;
      }

      // -----------------------------------------------------------------------
      // ACP AGENT MODE — routes to ACP agent via runAgentStream()
      // -----------------------------------------------------------------------
      if (model.startsWith('acp:')) {
        const agentId = model.slice(4);
        const { acpAgents } = useAgentSettingsStore.getState();
        const acpConfig = acpAgents.find((a: any) => a.id === agentId);
        if (!acpConfig) {
          useAIStore.setState((s) => {
            const msgs = [...s.messages];
            const last = msgs[msgs.length - 1];
            if (last) {
              last.content = 'ACP agent not found. Please check your settings.';
              last.isStreaming = false;
            }
            return { messages: msgs };
          });
          return;
        }

        useAIStore.getState().clearToolCallBlocks();
        try {
          await runAgentStream(
            assistantMsg.id,
            {
              providerType: 'acp',
              apiKey: 'acp',
              model: model,
            },
            abortController,
          );
        } catch (err) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          useAIStore.setState((s) => {
            const msgs = [...s.messages];
            const last = msgs[msgs.length - 1];
            if (last) {
              last.content = last.content
                ? `${last.content}\n\n**Error:** ${errorMsg}`
                : `**Error:** ${errorMsg}`;
              last.isStreaming = false;
            }
            return { messages: msgs };
          });
        } finally {
          // Always clear streaming state — both on success and on error.
          useAIStore.getState().setAbortController(null);
          setStreaming(false);
          useAIStore.setState((s) => {
            const msgs = [...s.messages];
            const last = msgs[msgs.length - 1];
            if (last?.isStreaming) last.isStreaming = false;
            return { messages: msgs };
          });
        }
        return;
      }

      // -----------------------------------------------------------------------
      // STANDARD MODE — design/chat pipeline (external CLI providers)
      // -----------------------------------------------------------------------
      const chatHistory = messages.map((m) => ({
        role: m.role,
        content: m.content,
        ...(m.attachments?.length ? { attachments: m.attachments } : {}),
      }));

      let appliedCount = 0;
      let isDesign = false;

      try {
        const classified = await classifyIntent(messageText, model, currentProvider);
        let intent = classified.intent;

        const { document: currentDoc } = useDocumentStore.getState();
        const activePageId = useCanvasStore.getState().activePageId;
        const pageChildren = getActivePageChildren(currentDoc, activePageId);
        if (intent === 'modify' && pageChildren.length === 0) {
          intent = 'new';
        }

        isDesign = intent === 'new' || intent === 'modify';
        const isModification = intent === 'modify' && (hasSelection || pageChildren.length > 0);

        if (isDesign) {
          if (isModification) {
            const { getNodeById, document: modDoc } = useDocumentStore.getState();
            let modTargets: any[];
            if (hasSelection) {
              modTargets = selectedIds.map((id) => getNodeById(id)).filter(Boolean);
            } else {
              const frames = pageChildren.filter((n) => n.type === 'frame');
              modTargets =
                frames.length > 0
                  ? [frames[frames.length - 1]]
                  : [pageChildren[pageChildren.length - 1]];
            }

            accumulated =
              '<step title="Checking guidelines">Analyzing modification request...</step>';
            updateLastMessage(accumulated);

            const { rawResponse, nodes } = await generateDesignModification(
              modTargets,
              messageText,
              {
                variables: modDoc.variables,
                themes: modDoc.themes,
                designMd: useDesignMdStore.getState().designMd,
                model,
                provider: currentProvider as AIProviderType | undefined,
              },
              abortController.signal,
            );
            accumulated = rawResponse;
            updateLastMessage(accumulated);

            const count = extractAndApplyDesignModification(JSON.stringify(nodes));
            appliedCount += count;
          } else {
            const doc = useDocumentStore.getState().document;
            const concurrency = useAIStore.getState().concurrency;
            const { rawResponse, nodes } = await generateDesign(
              {
                prompt: fullUserMessage,
                model,
                provider: currentProvider as AIProviderType | undefined,
                concurrency,
                context: {
                  canvasSize: { width: 1200, height: 800 },
                  documentSummary: `Current selection: ${hasSelection ? selectedIds.length + ' items' : 'Empty'}`,
                  variables: doc.variables,
                  themes: doc.themes,
                  designMd: useDesignMdStore.getState().designMd,
                },
              },
              {
                animated: true,
                onApplyPartial: (partialCount: number) => {
                  appliedCount += partialCount;
                },
                onTextUpdate: (text: string) => {
                  accumulated = text;
                  updateLastMessage(text);
                },
              },
              abortController.signal,
            );
            accumulated = rawResponse;
            if (appliedCount === 0 && nodes.length > 0) {
              animateNodesToCanvas(nodes);
              appliedCount += nodes.length;
            }
          }
        } else {
          // --- CHAT MODE ---
          chatHistory.push({
            role: 'user',
            content: fullUserMessage,
            ...(hasAttachments ? { attachments: pendingAttachments } : {}),
          });
          const trimmedHistory = trimChatHistory(chatHistory);
          const chatDoc = useDocumentStore.getState().document;
          const chatDesignMd = useDesignMdStore.getState().designMd;
          const chatSystemPrompt = buildChatSystemPrompt(fullUserMessage, {
            hasDesignMd: !!chatDesignMd,
            hasVariables: !!chatDoc.variables && Object.keys(chatDoc.variables).length > 0,
            designMd: chatDesignMd,
          });
          let chatThinking = '';
          for await (const chunk of streamChat(
            chatSystemPrompt,
            trimmedHistory,
            model,
            CHAT_STREAM_THINKING_CONFIG,
            currentProvider,
            abortController.signal,
          )) {
            if (chunk.type === 'thinking') {
              chatThinking += chunk.content;
              const thinkingStep = `<step title="Thinking">${chatThinking}</step>`;
              updateLastMessage(thinkingStep + (accumulated ? '\n' + accumulated : ''));
            } else if (chunk.type === 'text') {
              accumulated += chunk.content;
              const thinkingPrefix = chatThinking
                ? `<step title="Thinking">${chatThinking}</step>\n`
                : '';
              updateLastMessage(thinkingPrefix + accumulated);
            } else if (chunk.type === 'error') {
              accumulated += `\n\n**Error:** ${chunk.content}`;
              updateLastMessage(accumulated);
            }
          }
        }
      } catch (error) {
        if (abortController.signal.aborted) {
          // Keep partial content, don't show error
        } else {
          const errMsg = error instanceof Error ? error.message : 'Unknown error';
          accumulated += `\n\n**Error:** ${errMsg}`;
          updateLastMessage(accumulated);
        }
      } finally {
        useAIStore.getState().setAbortController(null);
        setStreaming(false);
      }

      if (isDesign && appliedCount > 0) {
        accumulated += `\n\n<!-- APPLIED -->`;
      }

      useAIStore.setState((s) => {
        const msgs = [...s.messages];
        const last = msgs.find((m) => m.id === assistantMsg.id);
        if (last) {
          last.content = accumulated;
          last.isStreaming = false;
        }
        return { messages: msgs };
      });
    },
    [
      input,
      isStreaming,
      isLoadingModels,
      model,
      availableModels,
      messages,
      addMessage,
      updateLastMessage,
      setStreaming,
    ],
  );

  return { input, setInput, handleSend, isStreaming };
}
