import { useState, useCallback } from 'react'
import { nanoid } from 'nanoid'
import { useAIStore } from '@/stores/ai-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { getActivePageChildren } from '@/stores/document-tree-utils'
import { streamChat } from '@/services/ai/ai-service'
import { CHAT_SYSTEM_PROMPT } from '@/services/ai/ai-prompts'
import {
  generateDesign,
  generateDesignModification,
  animateNodesToCanvas,
  extractAndApplyDesignModification,
} from '@/services/ai/design-generator'
import { trimChatHistory } from '@/services/ai/context-optimizer'
import type { ChatMessage as ChatMessageType } from '@/services/ai/ai-types'
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

      const chatHistory = messages.map((m) => ({
        role: m.role,
        content: m.content,
        ...(m.attachments?.length ? { attachments: m.attachments } : {}),
      }))
      const currentProvider = useAIStore.getState().modelGroups.find((g) =>
        g.models.some((m) => m.value === model),
      )?.provider

      let accumulated = ''
      let appliedCount = 0
      let isDesign = false

      const abortController = new AbortController()
      useAIStore.getState().setAbortController(abortController)

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
            let chatThinking = ''
            for await (const chunk of streamChat(
              CHAT_SYSTEM_PROMPT,
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
