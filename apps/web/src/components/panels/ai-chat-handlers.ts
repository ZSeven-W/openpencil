import { useState, useCallback } from 'react'
import { nanoid } from 'nanoid'
import i18n from '@/i18n'
import { decodeAgentEvent } from '@zseven-w/agent'
import type { AgentEvent } from '@zseven-w/agent'
import { useAIStore } from '@/stores/ai-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useDesignMdStore } from '@/stores/design-md-store'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import { getActivePageChildren } from '@/stores/document-tree-utils'
import { streamChat } from '@/services/ai/ai-service'
import { buildChatSystemPrompt } from '@/services/ai/ai-prompts'
import {
  generateDesign,
  generateDesignModification,
  animateNodesToCanvas,
  extractAndApplyDesignModification,
} from '@/services/ai/design-generator'
import { trimChatHistory } from '@/services/ai/context-optimizer'
import { AgentToolExecutor } from '@/services/ai/agent-tool-executor'
import { createDesignToolRegistry } from '@/services/ai/agent-tools'
import type { ChatMessage as ChatMessageType } from '@/services/ai/ai-types'
import type { ToolCallBlockData } from '@/components/panels/tool-call-block'
import { CHAT_STREAM_THINKING_CONFIG } from '@/services/ai/ai-runtime-config'

/** Intent classification prompt — lightweight LLM call to determine message routing */
const CLASSIFY_PROMPT = `You are a UI design tool assistant. Classify the user's message intent.
Reply with EXACTLY one of these tags, nothing else:
- DESIGN_NEW — user wants to create or generate a NEW design, screen, page, or component from scratch
- DESIGN_MODIFY — user wants to modify, adjust, refine, or iterate on an EXISTING design (e.g. change colors, resize, restyle, add/remove elements)
- CHAT — user is asking a question, seeking help, or having a conversation`

type DesignIntent = 'new' | 'modify' | 'chat'

/** Classify user intent via a lightweight LLM call instead of hardcoded keyword matching */
async function classifyIntent(
  text: string,
  model: string,
  provider?: string,
): Promise<{ intent: DesignIntent }> {
  try {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 8_000)

    const response = await fetch('/api/ai/generate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        system: CLASSIFY_PROMPT,
        message: text,
        model,
        provider,
      }),
      signal: controller.signal,
    })
    clearTimeout(timeout)

    if (!response.ok) throw new Error('classify failed')
    const data = await response.json()
    const upper = (data.text ?? '').trim().toUpperCase()

    if (upper.includes('DESIGN_MODIFY')) return { intent: 'modify' }
    if (upper.includes('DESIGN_NEW') || upper.includes('DESIGN')) return { intent: 'new' }
    if (upper.includes('CHAT')) return { intent: 'chat' }

    // Fallback: in a design tool, default to new design mode
    return { intent: 'new' }
  } catch {
    // Fallback: in a design tool, default to new design mode
    return { intent: 'new' }
  }
}

export function buildContextString(): string {
  const selectedIds = useCanvasStore.getState().selection.selectedIds
  const { getFlatNodes, document: doc } = useDocumentStore.getState()
  const flatNodes = getFlatNodes()

  const parts: string[] = []

  if (flatNodes.length > 0) {
    const summary = flatNodes
      .slice(0, 20)
      .map((n) => `${n.type}:${n.name ?? n.id}`)
      .join(', ')
    parts.push(`Document has ${flatNodes.length} nodes: ${summary}`)
  }

  if (selectedIds.length > 0) {
    const selectedNodes = selectedIds
      .map((id) => useDocumentStore.getState().getNodeById(id))
      .filter(Boolean)
    const selectedSummary = selectedNodes
      .map((n) => {
        const dims = 'width' in n! && 'height' in n!
          ? ` (${n!.width}x${n!.height})`
          : ''
        return `${n!.type}:${n!.name ?? n!.id}${dims}`
      })
      .join(', ')
    parts.push(`Selected: ${selectedSummary}`)
  }

  // Include variable summary so chat mode also knows about design tokens
  if (doc.variables && Object.keys(doc.variables).length > 0) {
    const varNames = Object.entries(doc.variables)
      .map(([n, d]) => `$${n}(${d.type})`)
      .join(', ')
    parts.push(`Variables: ${varNames}`)
  }

  return parts.length > 0 ? `\n\n[Canvas context: ${parts.join('. ')}]` : ''
}

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
- delete_node: Remove nodes.`

/**
 * Build the agent system prompt dynamically using pen-ai-skills.
 * Combines agent tool instructions with the same design knowledge the CLI pipeline uses.
 */
function buildAgentSystemPrompt(userMessage: string): string {
  // Loads design skills dynamically based on user message keywords —
  // same knowledge base the CLI pipeline uses (via pen-ai-skills)
  const designKnowledge = buildChatSystemPrompt(userMessage)
  return `${AGENT_TOOL_INSTRUCTIONS}\n\n${designKnowledge}`
}

/**
 * Parse SSE chunks from a ReadableStream and yield AgentEvents.
 * Handles partial chunks that may be split across reads.
 */
async function* parseAgentSSE(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  signal: AbortSignal,
): AsyncGenerator<AgentEvent> {
  const decoder = new TextDecoder()
  let buffer = ''

  while (!signal.aborted) {
    const { done, value } = await reader.read()
    if (done) break

    buffer += decoder.decode(value, { stream: true })
    const chunks = buffer.split('\n\n')
    // Last item may be incomplete -- keep it in the buffer
    buffer = chunks.pop() ?? ''

    for (const chunk of chunks) {
      const trimmed = chunk.trim()
      if (!trimmed) continue
      const evt = decodeAgentEvent(trimmed)
      if (evt) yield evt
    }
  }

  // Process any remaining data in buffer
  if (buffer.trim()) {
    const evt = decodeAgentEvent(buffer.trim())
    if (evt) yield evt
  }
}

/** Provider config for the agent pipeline */
interface AgentProviderConfig {
  providerType: 'anthropic' | 'openai-compat'
  apiKey: string
  model: string
  baseURL?: string
  maxOutputTokens?: number
}

/** Strip <think>...</think> tags (closed and unclosed) from model text output. */
function stripThinkTags(text: string): string {
  return text.replace(/<think>[\s\S]*?<\/think>\s*/g, '').replace(/<think>[\s\S]*$/g, '')
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
  const store = useAIStore.getState()
  const { updateLastMessage } = store

  const sessionId = nanoid()
  const executor = new AgentToolExecutor(sessionId)

  // Build tool definitions from the registry.
  // The server uses these to register tools with the AI SDK.
  const registry = createDesignToolRegistry()
  const sdkTools = registry.toAISDKFormat()
  const toolDefs = registry.list().map((t) => {
    // Extract the JSON Schema from the AI SDK tool object
    const sdkTool = sdkTools[t.name] as any
    const parameters = sdkTool?.parameters?.jsonSchema ?? sdkTool?.inputSchema?.jsonSchema
    return {
      name: t.name,
      description: t.description,
      level: t.level,
      parameters,
    }
  })

  // Build conversation messages from chat history
  const messages = useAIStore.getState().messages
    .filter((m) => m.id !== assistantMsgId)
    .map((m) => ({ role: m.role, content: m.content }))

  const context = buildContextString()
  // Build the last user message for skill resolution
  const lastUserMsg = messages[messages.length - 1]?.content ?? ''
  const systemPrompt = buildAgentSystemPrompt(lastUserMsg) + context

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
  }

  // Auto team mode: always create a designer member using the same provider as chat.
  // The designer has a specialized prompt + scoped tools (generate_design only).
  // Lead handles conversation; designer handles design generation.
  if (providerConfig.apiKey) {
    ;(agentBody as any).members = [{
      id: 'designer',
      providerType: providerConfig.providerType,
      apiKey: providerConfig.apiKey,
      model: providerConfig.model,
      baseURL: providerConfig.baseURL,
      systemPrompt: 'You are a design specialist. Use the generate_design tool to create designs based on the task description. Focus on high-quality visual output.',
    }]
  }

  const response = await fetch('/api/ai/agent', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(agentBody),
    signal: abortController.signal,
  })

  if (!response.ok || !response.body) {
    const errText = await response.text().catch(() => 'Unknown error')
    throw new Error(`Agent request failed: ${errText}`)
  }

  const reader = response.body.getReader()
  let accumulated = ''
  let thinkingContent = ''

  try {
    for await (const evt of parseAgentSSE(reader, abortController.signal)) {
      switch (evt.type) {
        case 'thinking': {
          thinkingContent += evt.content
          const thinkingStep = `<step title="Thinking">${thinkingContent}</step>`
          updateLastMessage(thinkingStep + (accumulated ? '\n' + accumulated : ''))
          break
        }

        case 'text': {
          accumulated += evt.content
          const prefix = thinkingContent
            ? `<step title="Thinking">${thinkingContent}</step>\n`
            : ''
          updateLastMessage(prefix + stripThinkTags(accumulated))
          break
        }

        case 'tool_call': {
          const block: ToolCallBlockData = {
            id: evt.id,
            name: evt.name,
            args: evt.args,
            level: evt.level,
            status: 'running',
            source: evt.source,
          }
          useAIStore.getState().addToolCallBlock(block)

          // Execute tool client-side and post result back to server
          executor.execute(evt as Extract<AgentEvent, { type: 'tool_call' }>).then(() => {
            // Also update block status locally in case SSE tool_result event is lost
            const block = useAIStore.getState().toolCallBlocks.find((b) => b.id === evt.id)
            if (block && block.status === 'running') {
              useAIStore.getState().updateToolCallBlock(evt.id, { status: 'done', result: { success: true } })
            }
          }).catch((err) => {
            useAIStore.getState().updateToolCallBlock(evt.id, {
              status: 'error',
              result: { success: false, error: String(err) },
            })
          })
          break
        }

        case 'tool_result': {
          useAIStore.getState().updateToolCallBlock(evt.id, {
            status: evt.result.success ? 'done' : 'error',
            result: evt.result,
          })
          break
        }

        case 'turn': {
          // Optionally show turn progress
          break
        }

        case 'done': {
          // If no text was accumulated (model returned empty), add a note
          if (!accumulated.trim()) {
            accumulated = '*Agent completed with no text output.*'
            updateLastMessage(accumulated)
          }
          break
        }

        case 'error': {
          accumulated += `\n\n**Error:** ${evt.message}`
          updateLastMessage(accumulated)
          if (evt.fatal) return
          break
        }

        case 'member_start': {
          accumulated += `\n\n> **[${evt.memberId}]** ${evt.task}\n`
          updateLastMessage(accumulated)
          break
        }

        case 'member_end': {
          accumulated += `\n> **[${evt.memberId}]** done\n\n`
          updateLastMessage(accumulated)
          break
        }

        case 'abort': {
          return
        }
      }
    }
  } finally {
    reader.releaseLock()
  }

  return stripThinkTags(accumulated)
}

/** Shared chat logic hook */
export function useChatHandlers() {
  const [input, setInput] = useState('')
  const messages = useAIStore((s) => s.messages)
  const isStreaming = useAIStore((s) => s.isStreaming)
  const model = useAIStore((s) => s.model)
  const availableModels = useAIStore((s) => s.availableModels)
  const isLoadingModels = useAIStore((s) => s.isLoadingModels)
  const addMessage = useAIStore((s) => s.addMessage)
  const updateLastMessage = useAIStore((s) => s.updateLastMessage)
  const setStreaming = useAIStore((s) => s.setStreaming)

  const handleSend = useCallback(
    async (text?: string) => {
      const messageText = text ?? input.trim()
      const pendingAttachments = useAIStore.getState().pendingAttachments
      const hasAttachments = pendingAttachments.length > 0
      if ((!messageText && !hasAttachments) || isStreaming || isLoadingModels || availableModels.length === 0) return

      setInput('')
      useAIStore.getState().clearPendingAttachments()

      // Determine context and mode
      const selectedIds = useCanvasStore.getState().selection.selectedIds
      const hasSelection = selectedIds.length > 0

      const context = buildContextString()
      const fullUserMessage = messageText + context

      const userMsg: ChatMessageType = {
        id: nanoid(),
        role: 'user',
        content: messageText || '',
        timestamp: Date.now(),
        ...(hasAttachments ? { attachments: pendingAttachments } : {}),
      }
      addMessage(userMsg)

      const assistantMsg: ChatMessageType = {
        id: nanoid(),
        role: 'assistant',
        content: '',
        timestamp: Date.now(),
        isStreaming: true,
      }
      addMessage(assistantMsg)
      setStreaming(true)

      // Set chat title if it's the first message
      if (messages.length === 0) {
        // Simple heuristic: Take first ~4 words or up to 25 chars
        const cleanText = messageText.replace(/^(Design|Create|Generate|Make)\s+/i, '')
        const words = cleanText.split(' ').slice(0, 4).join(' ')
        const title = words.length > 30 ? words.slice(0, 30) + '...' : words
        useAIStore.getState().setChatTitle(title || 'New Chat')
      }

      const currentProvider = useAIStore.getState().modelGroups.find((g) =>
        g.models.some((m) => m.value === model),
      )?.provider

      const abortController = new AbortController()
      useAIStore.getState().setAbortController(abortController)

      let accumulated = ''

      // -----------------------------------------------------------------------
      // BUILT-IN PROVIDER (Agent) MODE — route through /api/ai/agent with tool execution
      // Triggered when the selected model has a `builtin:` prefix
      // -----------------------------------------------------------------------
      if (model.startsWith('builtin:')) {
        const parts = model.split(':')
        const builtinProviderId = parts[1]
        const modelName = parts.slice(2).join(':')

        const { builtinProviders } = useAgentSettingsStore.getState()
        const bp = builtinProviders.find((p) => p.id === builtinProviderId)
        if (!bp || !bp.apiKey) {
          accumulated = !bp
            ? `**Error:** ${i18n.t('builtin.errorProviderNotFound')}`
            : `**Error:** ${i18n.t('builtin.errorApiKeyEmpty')}`
          updateLastMessage(accumulated)
          useAIStore.getState().setAbortController(null)
          setStreaming(false)
          useAIStore.setState((s) => {
            const msgs = [...s.messages]
            const last = msgs.find((m) => m.id === assistantMsg.id)
            if (last) { last.content = accumulated; last.isStreaming = false }
            return { messages: msgs }
          })
          return
        }

        useAIStore.getState().clearToolCallBlocks()
        try {
          const result = await runAgentStream(
            assistantMsg.id,
            {
              providerType: bp.type === 'anthropic' ? 'anthropic' : 'openai-compat',
              apiKey: bp.apiKey,
              model: modelName,
              baseURL: bp.baseURL,
              maxOutputTokens: bp.maxContextTokens ? Math.min(bp.maxContextTokens, 8192) : undefined,
            },
            abortController,
          )
          accumulated = result ?? ''
        } catch (error) {
          if (!abortController.signal.aborted) {
            const errMsg = error instanceof Error ? error.message : 'Unknown error'
            accumulated += `\n\n**Error:** ${errMsg}`
            updateLastMessage(accumulated)
          }
        } finally {
          useAIStore.getState().setAbortController(null)
          setStreaming(false)
        }

        // Force update final message state
        useAIStore.setState((s) => {
          const msgs = [...s.messages]
          const last = msgs.find((m) => m.id === assistantMsg.id)
          if (last) {
            last.content = accumulated
            last.isStreaming = false
          }
          return { messages: msgs }
        })
        return
      }

      // -----------------------------------------------------------------------
      // STANDARD MODE — existing design/chat pipeline
      // -----------------------------------------------------------------------
      const chatHistory = messages.map((m) => ({
        role: m.role,
        content: m.content,
        ...(m.attachments?.length ? { attachments: m.attachments } : {}),
      }))

      let appliedCount = 0
      let isDesign = false

      try {
        // Classify intent via lightweight LLM call (three-way: new / modify / chat)
        const classified = await classifyIntent(
          messageText, model, currentProvider,
        )
        let intent = classified.intent

        // When LLM says "modify" but canvas is empty, degrade to new design
        const { document: currentDoc } = useDocumentStore.getState()
        const activePageId = useCanvasStore.getState().activePageId
        const pageChildren = getActivePageChildren(currentDoc, activePageId)
        if (intent === 'modify' && pageChildren.length === 0) {
          intent = 'new'
        }

        isDesign = intent === 'new' || intent === 'modify'

        // Determine modification target: explicit selection or auto-selected frame
        const isModification = intent === 'modify' && (hasSelection || pageChildren.length > 0)

        if (isDesign) {
             if (isModification) {
               // --- MODIFICATION MODE ---
               const { getNodeById, document: modDoc } = useDocumentStore.getState()
               let modTargets: any[]
               if (hasSelection) {
                 // User explicitly selected nodes
                 modTargets = selectedIds.map(id => getNodeById(id)).filter(Boolean)
               } else {
                 // Auto-select: last top-level frame on the active page
                 const frames = pageChildren.filter(n => n.type === 'frame')
                 modTargets = frames.length > 0 ? [frames[frames.length - 1]] : [pageChildren[pageChildren.length - 1]]
               }

               // We update the UI to show we are working
               accumulated = '<step title="Checking guidelines">Analyzing modification request...</step>'
               updateLastMessage(accumulated)

               const { rawResponse, nodes } = await generateDesignModification(modTargets, messageText, {
                 variables: modDoc.variables,
                 themes: modDoc.themes,
                 designMd: useDesignMdStore.getState().designMd,
                 model,
                 provider: currentProvider,
               }, abortController.signal)
               accumulated = rawResponse
               updateLastMessage(accumulated)

               // Apply all changes
               const count = extractAndApplyDesignModification(JSON.stringify(nodes))
               appliedCount += count
             } else {
               // --- GENERATION MODE (animated) ---
               const doc = useDocumentStore.getState().document
               const concurrency = useAIStore.getState().concurrency
               const { rawResponse, nodes } = await generateDesign({
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
               }, {
                 animated: true,
                 onApplyPartial: (partialCount: number) => {
                   appliedCount += partialCount
                 },
                 onTextUpdate: (text: string) => {
                    accumulated = text
                    updateLastMessage(text)
                 },
               }, abortController.signal)
               // Ensure final text is captured
               accumulated = rawResponse
               if (appliedCount === 0 && nodes.length > 0) {
                 animateNodesToCanvas(nodes)
                 appliedCount += nodes.length
               }
             }
        } else {
            // --- CHAT MODE ---
            chatHistory.push({
              role: 'user',
              content: fullUserMessage,
              ...(hasAttachments ? { attachments: pendingAttachments } : {}),
            })
            // Trim history to prevent unbounded context growth
            const trimmedHistory = trimChatHistory(chatHistory)
            // Progressive skill loading: resolve needed skills from user message
            const chatDoc = useDocumentStore.getState().document
            const chatDesignMd = useDesignMdStore.getState().designMd
            const chatSystemPrompt = buildChatSystemPrompt(fullUserMessage, {
              hasDesignMd: !!chatDesignMd,
              hasVariables: !!chatDoc.variables && Object.keys(chatDoc.variables).length > 0,
              designMd: chatDesignMd,
            })
            let chatThinking = ''
            for await (const chunk of streamChat(
              chatSystemPrompt,
              trimmedHistory,
              model,
              CHAT_STREAM_THINKING_CONFIG,
              currentProvider,
              abortController.signal,
            )) {
               if (chunk.type === 'thinking') {
                 chatThinking += chunk.content
                 // Show thinking content as a collapsible step in the panel
                 const thinkingStep = `<step title="Thinking">${chatThinking}</step>`
                 updateLastMessage(thinkingStep + (accumulated ? '\n' + accumulated : ''))
               } else if (chunk.type === 'text') {
                 accumulated += chunk.content
                 // Keep thinking step visible above text content
                 const thinkingPrefix = chatThinking
                   ? `<step title="Thinking">${chatThinking}</step>\n`
                   : ''
                 updateLastMessage(thinkingPrefix + accumulated)
               } else if (chunk.type === 'error') {
                 accumulated += `\n\n**Error:** ${chunk.content}`
                 updateLastMessage(accumulated)
               }
            }
        }
      } catch (error) {
         // Silently handle user-initiated stop
         if (abortController.signal.aborted) {
           // Keep partial content, don't show error
         } else {
           const errMsg = error instanceof Error ? error.message : 'Unknown error'
           accumulated += `\n\n**Error:** ${errMsg}`
           updateLastMessage(accumulated)
         }
      } finally {
         useAIStore.getState().setAbortController(null)
         setStreaming(false)
      }

      // Final update - mark as applied (hidden) so the "Apply" button doesn't show up
      if (isDesign && appliedCount > 0) {
        accumulated += `\n\n<!-- APPLIED -->`
      }

      // Force update the last message state to ensure sync
      useAIStore.setState((s) => {
        const msgs = [...s.messages]
        const last = msgs.find(m => m.id === assistantMsg.id)
        if (last) {
           last.content = accumulated
           last.isStreaming = false
        }
        return { messages: msgs }
      })
    },
    [input, isStreaming, isLoadingModels, model, availableModels, messages, addMessage, updateLastMessage, setStreaming],
  )

  return { input, setInput, handleSend, isStreaming }
}
