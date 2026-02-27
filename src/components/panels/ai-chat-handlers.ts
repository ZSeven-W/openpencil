import { useState, useCallback } from 'react'
import { nanoid } from 'nanoid'
import { useAIStore } from '@/stores/ai-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
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

/** Detect if a message is a design generation request */
export function isDesignRequest(text: string): boolean {
  const lower = text.toLowerCase()
  const designKeywords = [
    '生成', '设计', '创建', '画', '做一个', '来一个', '弄一个',
    'generate', 'create', 'design', 'make', 'build', 'draw',
    'add a', 'add an', 'place a', 'insert',
    '界面', '页面', 'screen', 'page', 'layout', 'component',
    '按钮', '卡片', '导航', '表单', '输入框', '列表',
    'button', 'card', 'nav', 'form', 'input', 'list',
    'header', 'footer', 'sidebar', 'modal', 'dialog',
    'login', 'signup', 'dashboard', 'profile',
  ]
  return designKeywords.some((kw) => lower.includes(kw))
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
      .map((n) => `${n!.type}:${n!.name ?? n!.id}`)
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
      if (!messageText || isStreaming || isLoadingModels || availableModels.length === 0) return

      setInput('')

      // Determine context and mode
      const selectedIds = useCanvasStore.getState().selection.selectedIds
      const hasSelection = selectedIds.length > 0
      const isDesign = isDesignRequest(messageText)
      const isModification = isDesign && hasSelection

      const context = buildContextString()
      const fullUserMessage = messageText + context

      const userMsg: ChatMessageType = {
        id: nanoid(),
        role: 'user',
        content: messageText,
        timestamp: Date.now(),
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
      }))

      let accumulated = ''
      let appliedCount = 0

      try {
        if (isDesign) {
             if (isModification) {
               // --- MODIFICATION MODE ---
               const { getNodeById, document: modDoc } = useDocumentStore.getState()
               const selectedNodes = selectedIds.map(id => getNodeById(id)).filter(Boolean) as any[]

               // We update the UI to show we are working
               accumulated = '<step title="Checking guidelines">Analyzing modification request...</step>'
               updateLastMessage(accumulated)

               const { rawResponse, nodes } = await generateDesignModification(selectedNodes, messageText, {
                 variables: modDoc.variables,
                 themes: modDoc.themes,
               })
               accumulated = rawResponse
               updateLastMessage(accumulated)

               // Apply all changes
               const count = extractAndApplyDesignModification(JSON.stringify(nodes))
               appliedCount += count
             } else {
               // --- GENERATION MODE (animated) ---
               const doc = useDocumentStore.getState().document
               const { rawResponse, nodes } = await generateDesign({
                 prompt: fullUserMessage,
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
               })
               // Ensure final text is captured
               accumulated = rawResponse
               if (appliedCount === 0 && nodes.length > 0) {
                 animateNodesToCanvas(nodes)
                 appliedCount += nodes.length
               }
             }
        } else {
            // --- CHAT MODE ---
            chatHistory.push({ role: 'user', content: fullUserMessage })
            // Trim history to prevent unbounded context growth
            const trimmedHistory = trimChatHistory(chatHistory)
            // Resolve which provider the currently selected model belongs to
            const currentProvider = useAIStore.getState().modelGroups.find((g) =>
              g.models.some((m) => m.value === model),
            )?.provider
            let chatThinking = ''
            for await (const chunk of streamChat(
              CHAT_SYSTEM_PROMPT,
              trimmedHistory,
              model,
              CHAT_STREAM_THINKING_CONFIG,
              currentProvider,
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
         const errMsg = error instanceof Error ? error.message : 'Unknown error'
         accumulated += `\n\n**Error:** ${errMsg}`
         updateLastMessage(accumulated)
      } finally {
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
