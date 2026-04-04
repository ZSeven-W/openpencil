import { useState, useCallback } from 'react';
import { nanoid } from 'nanoid';
import i18n from '@/i18n';
import type { AgentEvent } from '@/types/agent';

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

// Re-export for any external consumers
export { buildContextString } from './ai-chat-context-builder';

// ---------------------------------------------------------------------------
// Agent mode SSE stream handler
// ---------------------------------------------------------------------------

/** Agent-specific tool usage instructions — prepended to the dynamic skill-based prompt. */
const AGENT_TOOL_INSTRUCTIONS = `IMPORTANT: When the user asks you to create or design anything, you MUST call the generate_design tool with a descriptive prompt. Do NOT output JSON or code directly.

## Available Tools
- generate_design: Create complete designs. Pass a natural language description.
- snapshot_layout: View current canvas state.
- batch_get: Read specific nodes by ID.
- update_node: Modify existing node properties.
- delete_node: Remove nodes.`;

/**
 * Build the agent system prompt dynamically using pen-ai-skills.
 * Combines agent tool instructions with the same design knowledge the CLI pipeline uses.
 */
function buildAgentSystemPrompt(userMessage: string): string {
  const designKnowledge = buildChatSystemPrompt(userMessage);
  return `${AGENT_TOOL_INSTRUCTIONS}\n\n${designKnowledge}`;
}

/**
 * Parse SSE chunks from a ReadableStream and yield AgentEvents.
 * Handles partial chunks that may be split across reads.
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

/** Provider config for the agent pipeline */
interface AgentProviderConfig {
  providerType: 'anthropic' | 'openai-compat';
  apiKey: string;
  model: string;
  baseURL?: string;
  maxOutputTokens?: number;
}

/** Strip <think>...</think> tags (closed and unclosed) from model text output. */
function stripThinkTags(text: string): string {
  return text.replace(/<think>[\s\S]*?<\/think>\s*/g, '').replace(/<think>[\s\S]*$/g, '');
}

/**
 * Send a message through the agent pipeline.
 * Opens an SSE connection to /api/ai/agent, dispatches tool calls
 * client-side, and updates the AI store in real time.
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

  const toolDefs = getDesignToolDefs();

  const messages = useAIStore
    .getState()
    .messages.filter((m) => m.id !== assistantMsgId)
    .map((m) => ({ role: m.role, content: m.content }));

  const context = buildContextString();
  const lastUserMsg = messages[messages.length - 1]?.content ?? '';
  const systemPrompt = buildAgentSystemPrompt(lastUserMsg) + context;

  const agentBody: Record<string, unknown> = {
    sessionId,
    messages,
    systemPrompt,
    providerType: providerConfig.providerType,
    apiKey: providerConfig.apiKey,
    model: providerConfig.model,
    ...(providerConfig.baseURL ? { baseURL: providerConfig.baseURL } : {}),
    ...(providerConfig.maxOutputTokens ? { maxOutputTokens: providerConfig.maxOutputTokens } : {}),
    toolDefs,
    maxTurns: 20,
  };

  // Concurrency (⚡2x, 3x, etc.) is handled by the orchestrator in standard
  // mode via useAIStore.getState().concurrency — no Zig team members needed.

  const response = await fetch('/api/ai/agent', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(agentBody),
    signal: abortController.signal,
  });

  if (!response.ok || !response.body) {
    const errText = await response.text().catch(() => 'Unknown error');
    throw new Error(`Agent request failed: ${errText}`);
  }

  const reader = response.body.getReader();
  let accumulated = '';
  let thinkingContent = '';
  const [defaultIdentity] = assignAgentIdentities(1);
  const renderer = new StreamingDesignRenderer({
    agentColor: defaultIdentity.color,
    agentName: defaultIdentity.name,
    animated: true,
  });

  let identityPool: AgentIdentity[] = [];
  let nextIdentityIdx = 0;
  const memberIdentities = new Map<string, AgentIdentity>();

  try {
    for await (const evt of parseAgentSSE(reader, abortController.signal)) {
      switch (evt.type) {
        case 'thinking': {
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

          renderer.feedText(accumulated);
          break;
        }

        case 'tool_call': {
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

          executor
            .execute(evt as Extract<AgentEvent, { type: 'tool_call' }>)
            .then(() => {
              const block = useAIStore.getState().toolCallBlocks.find((b) => b.id === evt.id);
              if (block && block.status === 'running') {
                useAIStore
                  .getState()
                  .updateToolCallBlock(evt.id, { status: 'done', result: { success: true } });
              }
            })
            .catch((err) => {
              useAIStore.getState().updateToolCallBlock(evt.id, {
                status: 'error',
                result: { success: false, error: String(err) },
              });
            });
          break;
        }

        case 'tool_result': {
          useAIStore.getState().updateToolCallBlock(evt.id, {
            status: evt.result.success ? 'done' : 'error',
            result: evt.result,
          });
          break;
        }

        case 'turn':
          break;

        case 'done': {
          if (!accumulated.trim()) {
            const hasSuccessfulToolCalls = useAIStore
              .getState()
              .toolCallBlocks.some((b) => b.status === 'done');
            accumulated = hasSuccessfulToolCalls
              ? '*Design generated successfully.*'
              : '*Agent completed with no text output.*';
            updateLastMessage(accumulated);
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
          accumulated += `\n\n**Error:** ${evt.message}`;
          updateLastMessage(accumulated);
          renderer.finish();
          if (evt.fatal) return stripThinkTags(accumulated);
          break;
        }

        case 'member_start': {
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
              maxOutputTokens: bp.maxContextTokens
                ? Math.min(bp.maxContextTokens, 8192)
                : undefined,
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
                provider: currentProvider,
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
                provider: currentProvider,
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
