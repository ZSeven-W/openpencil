import { useState, useRef, useEffect, useCallback } from 'react'
import { Send, Plus, ChevronDown, ChevronUp, Check, MessageSquare, Loader2 } from 'lucide-react'
import { nanoid } from 'nanoid'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { useAIStore } from '@/stores/ai-store'
import type { PanelCorner } from '@/stores/ai-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import { streamChat, fetchAvailableModels } from '@/services/ai/ai-service'
import {
  CHAT_SYSTEM_PROMPT,
} from '@/services/ai/ai-prompts'
import {
  generateDesign,
  generateDesignModification,
  animateNodesToCanvas,
  extractAndApplyDesign,
  extractAndApplyDesignModification
} from '@/services/ai/design-generator'
import type { ChatMessage as ChatMessageType } from '@/services/ai/ai-types'
import type { AIProviderType } from '@/types/agent-settings'
import ClaudeLogo from '@/components/icons/claude-logo'
import OpenAILogo from '@/components/icons/openai-logo'
import ChatMessage from './chat-message'

const PROVIDER_ICON: Record<AIProviderType, typeof ClaudeLogo> = {
  anthropic: ClaudeLogo,
  openai: OpenAILogo,
}

const QUICK_ACTIONS = [
  {
    label: 'Design a mobile login screen',
    prompt: 'Design a modern mobile login screen with email input, password input, login button, and social login options',
  },
  {
    label: 'Create a product card component',
    prompt: 'Create a product card with an image placeholder, title, price, and buy button',
  },
  {
    label: 'Design a bottom navigation bar',
    prompt: 'Design a mobile app bottom navigation bar with 5 tabs: Home, Search, Add, Messages, Profile',
  },
  {
    label: 'Suggest a color palette for my app',
    prompt: 'Suggest a modern color palette for a pet care app',
  },
]

const CORNER_CLASSES: Record<PanelCorner, string> = {
  'top-left': 'top-3 left-3',
  'top-right': 'top-3 right-3',
  'bottom-left': 'bottom-3 left-3',
  'bottom-right': 'bottom-3 right-3',
}

/** Detect if a message is a design generation request */
function isDesignRequest(text: string): boolean {
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

function buildContextString(): string {
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
function useChatHandlers() {
  const [input, setInput] = useState('')
  const messages = useAIStore((s) => s.messages)
  const isStreaming = useAIStore((s) => s.isStreaming)
  const model = useAIStore((s) => s.model)
  const addMessage = useAIStore((s) => s.addMessage)
  const updateLastMessage = useAIStore((s) => s.updateLastMessage)
  const setStreaming = useAIStore((s) => s.setStreaming)

  const handleSend = useCallback(
    async (text?: string) => {
      const messageText = text ?? input.trim()
      if (!messageText || isStreaming) return

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
            let chatThinking = false
            for await (const chunk of streamChat(CHAT_SYSTEM_PROMPT, chatHistory, model)) {
               if (chunk.type === 'thinking') {
                 // Show a brief thinking indicator so the UI isn't stuck on empty
                 if (!chatThinking && !accumulated) {
                   chatThinking = true
                   updateLastMessage('*Thinking...*')
                 }
               } else if (chunk.type === 'text') {
                 chatThinking = false
                 accumulated += chunk.content
                 updateLastMessage(accumulated)
               } else if (chunk.type === 'error') {
                 accumulated += `\n\n**Error:** ${chunk.content}`
                 updateLastMessage(accumulated)
               }
            }
        }
      } catch (error) {
         const errMsg = error instanceof Error ? error.message : 'Unknown error'
         accumulated += `\n\n**Error:** ${errMsg}`
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
    [input, isStreaming, model, messages, addMessage, updateLastMessage, setStreaming],
  )

  return { input, setInput, handleSend, isStreaming }
}

/**
 * Minimized AI bar — a compact clickable pill.
 * Parent is responsible for placing it in the layout.
 */
export function AIChatMinimizedBar() {
  const isMinimized = useAIStore((s) => s.isMinimized)
  const toggleMinimize = useAIStore((s) => s.toggleMinimize)

  if (!isMinimized) return null

  return (
    <button
      type="button"
      onClick={toggleMinimize}
      className="h-8 bg-card border border-border rounded-lg flex items-center gap-1.5 px-3 shadow-lg hover:bg-accent transition-colors"
    >
      <MessageSquare size={13} className="text-muted-foreground" />
      <span className="text-xs text-muted-foreground max-w-[120px] truncate">
        {useAIStore.getState().chatTitle}
      </span>
      <ChevronUp size={12} className="text-muted-foreground" />
    </button>
  )
}

/**
 * Expanded AI chat panel — floating, draggable.
 * Only renders when NOT minimized.
 */
export default function AIChatPanel() {
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const panelRef = useRef<HTMLDivElement>(null)
  const dragRef = useRef<{ offsetX: number; offsetY: number } | null>(null)
  const resizeRef = useRef<{ startY: number; startHeight: number; startTop: number } | null>(null)
  const [dragStyle, setDragStyle] = useState<React.CSSProperties | null>(null)
  const [panelHeight, setPanelHeight] = useState(400) // Default height

  const messages = useAIStore((s) => s.messages)
  const isStreaming = useAIStore((s) => s.isStreaming)
  const clearMessages = useAIStore((s) => s.clearMessages)
  const panelCorner = useAIStore((s) => s.panelCorner)
  const isMinimized = useAIStore((s) => s.isMinimized)
  const setPanelCorner = useAIStore((s) => s.setPanelCorner)
  const chatTitle = useAIStore((s) => s.chatTitle)
  const selectedIds = useCanvasStore((s) => s.selection.selectedIds)
  const toggleMinimize = useAIStore((s) => s.toggleMinimize)
  const model = useAIStore((s) => s.model)
  const setModel = useAIStore((s) => s.setModel)
  const availableModels = useAIStore((s) => s.availableModels)
  const setAvailableModels = useAIStore((s) => s.setAvailableModels)
  const modelGroups = useAIStore((s) => s.modelGroups)
  const setModelGroups = useAIStore((s) => s.setModelGroups)
  const isLoadingModels = useAIStore((s) => s.isLoadingModels)
  const setLoadingModels = useAIStore((s) => s.setLoadingModels)
  const providers = useAgentSettingsStore((s) => s.providers)
  const [modelDropdownOpen, setModelDropdownOpen] = useState(false)
  const { input, setInput, handleSend } = useChatHandlers()

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  // Build model list from connected providers in agent-settings-store.
  // Falls back to Agent SDK legacy fetch when no providers are connected.
  useEffect(() => {
    const connectedProviders = (Object.keys(providers) as AIProviderType[]).filter(
      (p) => providers[p].isConnected && (providers[p].models?.length ?? 0) > 0,
    )

    if (connectedProviders.length > 0) {
      // Build groups + flat list from stored models
      const providerNames: Record<AIProviderType, string> = {
        anthropic: 'Anthropic',
        openai: 'OpenAI',
      }
      const groups = connectedProviders.map((p) => ({
        provider: p,
        providerName: providerNames[p],
        models: providers[p].models,
      }))
      const flat = groups.flatMap((g) =>
        g.models.map((m) => ({
          value: m.value,
          displayName: m.displayName,
          description: m.description,
        })),
      )
      setModelGroups(groups)
      setAvailableModels(flat)
      // If current model not in list, select first
      if (!flat.some((m) => m.value === model)) {
        setModel(flat[0].value)
      }
      setLoadingModels(false)
    } else {
      // No providers connected — fall back to Agent SDK legacy model list
      setModelGroups([])
      setLoadingModels(true)
      fetchAvailableModels().then((models) => {
        if (models.length > 0) {
          setAvailableModels(models)
          if (!models.some((m) => m.value === model)) {
            setModel(models[0].value)
          }
        }
        setLoadingModels(false)
      })
    }
  }, [providers]) // eslint-disable-line react-hooks/exhaustive-deps

  // Close model dropdown when clicking outside
  useEffect(() => {
    if (!modelDropdownOpen) return
    const handler = (e: MouseEvent) => {
      const panel = panelRef.current
      if (panel && !panel.contains(e.target as Node)) {
        setModelDropdownOpen(false)
      }
    }
    document.addEventListener('pointerdown', handler)
    return () => document.removeEventListener('pointerdown', handler)
  }, [modelDropdownOpen])

  // Auto-expand when streaming starts while minimized
  useEffect(() => {
    if (isStreaming && isMinimized) {
      toggleMinimize()
    }
  }, [isStreaming, isMinimized, toggleMinimize])

  /* --- Drag-to-snap handlers --- */

  const handleDragStart = useCallback((e: React.PointerEvent) => {
    if ((e.target as HTMLElement).closest('button, input, textarea, select')) return

    const panel = panelRef.current
    if (!panel) return

    const panelRect = panel.getBoundingClientRect()
    dragRef.current = {
      offsetX: e.clientX - panelRect.left,
      offsetY: e.clientY - panelRect.top,
    }

    e.currentTarget.setPointerCapture(e.pointerId)

    const container = panel.parentElement!
    const containerRect = container.getBoundingClientRect()
    setDragStyle({
      left: panelRect.left - containerRect.left,
      top: panelRect.top - containerRect.top,
      right: 'auto',
      bottom: 'auto',
    })
  }, [])

  const handleDragMove = useCallback((e: React.PointerEvent) => {
    if (!dragRef.current) return

    const panel = panelRef.current
    if (!panel) return

    const container = panel.parentElement!
    const containerRect = container.getBoundingClientRect()
    setDragStyle({
      left: e.clientX - containerRect.left - dragRef.current.offsetX,
      top: e.clientY - containerRect.top - dragRef.current.offsetY,
      right: 'auto',
      bottom: 'auto',
    })
  }, [])

  const handleDragEnd = useCallback(() => {
    if (!dragRef.current) return

    const panel = panelRef.current
    if (!panel) return

    const container = panel.parentElement!
    const containerRect = container.getBoundingClientRect()
    const panelRect = panel.getBoundingClientRect()

    const centerX = panelRect.left + panelRect.width / 2 - containerRect.left
    const centerY = panelRect.top + panelRect.height / 2 - containerRect.top

    const isLeft = centerX < containerRect.width / 2
    const isTop = centerY < containerRect.height / 2

    const corner: PanelCorner = isLeft
      ? isTop ? 'top-left' : 'bottom-left'
      : isTop ? 'top-right' : 'bottom-right'

    setPanelCorner(corner)
    dragRef.current = null
    setDragStyle(null)
  }, [setPanelCorner])


  /* --- Resize handlers --- */
  const handleResizeStart = useCallback((e: React.PointerEvent) => {
    e.preventDefault()
    e.stopPropagation()
    const panel = panelRef.current
    if (!panel) return

    const rect = panel.getBoundingClientRect()
    const container = panel.parentElement!.getBoundingClientRect()

    // If we're not already in absolute positioning mode, snap to it now
    // so resizing works smoothly from the current visual position
    if (!dragStyle) {
      setDragStyle({
        left: rect.left - container.left,
        top: rect.top - container.top,
        width: 320,
        height: rect.height,
      })
    }

    resizeRef.current = {
      startY: e.clientY,
      startHeight: rect.height,
      startTop: rect.top - container.top,
    }
    
    e.currentTarget.setPointerCapture(e.pointerId)
  }, [dragStyle])

  const handleResizeMove = useCallback((e: React.PointerEvent) => {
    if (!resizeRef.current) return
    e.preventDefault()
    e.stopPropagation()

    const deltaY = e.clientY - resizeRef.current.startY
    // Dragging top handle up (negative delta) -> increase height, decrease top
    // Dragging top handle down (positive delta) -> decrease height, increase top
    
    let newHeight = resizeRef.current.startHeight - deltaY
    let newTop = resizeRef.current.startTop + deltaY

    // Constrain height
    if (newHeight < 200) {
      const diff = 200 - newHeight
      newHeight = 200
      newTop -= diff // correct top if we hit min height
    }
    if (newHeight > 1200) {
      const diff = newHeight - 1200
      newHeight = 1200
      newTop += diff // correct top if we hit max height
    }

    setPanelHeight(newHeight)
    setDragStyle(prev => ({
      ...prev,
      top: newTop,
      height: newHeight,
    }))
  }, [])

  const handleResizeEnd = useCallback((e: React.PointerEvent) => {
    if (!resizeRef.current) return
    e.preventDefault()
    e.stopPropagation()
    resizeRef.current = null
    e.currentTarget.releasePointerCapture(e.pointerId)
  }, [])

  const handleApplyDesign = useCallback((jsonString: string) => {
    // For manual apply, we always use the "add/create" logic for now, 
    // unless we want to try to infer if it's a modification.
    // But usually applying a block from history is "add this snippet".
    // If the snippet has IDs that exist, addNode might duplicate or error?
    // addNode usually generates new ID if we don't handle it, 
    // but our validateNodes checks for IDs.
    // `applyNodesToCanvas` (called by extractAndApplyDesign) calls `addNode`.
    // `addNode` in document-store generates new IDs?
    // Let's check `addNode` implementation.
    // It pushes to children. If ID exists, it might duplicate ID in tree (bad).
    
    // For safety, `extractAndApplyDesign` creates new nodes with same IDs?
    // No, it passes nodes as is.
    // We should probably regenerate IDs when applying from history to avoid ID conflicts.
    // But for this task, let's stick to existing behavior or use extractAndApplyDesign.
    const count = extractAndApplyDesign('```json\n' + jsonString + '\n```')
    if (count > 0) {
      useAIStore.setState((s) => {
        const msgs = [...s.messages]
        for (let i = msgs.length - 1; i >= 0; i--) {
          if (msgs[i].role === 'assistant' && msgs[i].content.includes(jsonString.slice(0, 50))) {
            if (!msgs[i].content.includes('✅') && !msgs[i].content.includes('<!-- APPLIED -->')) {
              msgs[i] = {
                ...msgs[i],
                content: msgs[i].content + `\n\n<!-- APPLIED -->`,
              }
            }
            break
          }
        }
        return { messages: msgs }
      })
    }
  }, [])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  // Don't render when minimized — the minimized bar is rendered by parent
  if (isMinimized) return null

  return (
    <div
      ref={panelRef}
      className={cn(
        'absolute z-50 w-[320px] rounded-xl shadow-2xl border border-border bg-card/95 backdrop-blur-sm flex flex-col',
        !dragStyle && CORNER_CLASSES[panelCorner],
      )}
        style={{ ...dragStyle, height: panelHeight }}
    >
      {/* --- Resize Handle (Top Edge) --- */}
      <div
        className="absolute -top-1.5 left-0 right-0 h-3 cursor-ns-resize z-50 hover:bg-primary/20 transition-colors group flex items-center justify-center"
        onPointerDown={handleResizeStart}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      >
         {/* Visual grip pill */}
         <div className="w-8 h-1 rounded-full bg-border group-hover:bg-primary/50 transition-colors" />
      </div>

      {/* --- Header (draggable) --- */}
      <div
        className="flex items-center justify-between px-1 py-1 border-b border-border cursor-grab active:cursor-grabbing select-none"
        onPointerDown={handleDragStart}
        onPointerMove={handleDragMove}
        onPointerUp={handleDragEnd}
      >
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={toggleMinimize}
            title="Collapse"
          >
            <ChevronDown size={14} />
          </Button>
          <span className="text-sm font-medium text-foreground max-w-[100px] truncate overflow-hidden text-ellipsis" title={chatTitle}>
            {chatTitle}
          </span>
          {isStreaming && <Loader2 size={13} className="animate-spin text-muted-foreground ml-2" />}
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={clearMessages}
          title="New chat"
        >
          <Plus size={14} />
        </Button>
      </div>

      {/* --- Messages --- */}
      <div className="flex-1 overflow-y-auto px-3.5 py-3 bg-background/80 rounded-b-xl">
        {messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-4">
            <p className="text-xs text-muted-foreground mb-4">
              Try an example to design...
            </p>
            <div className="flex flex-col gap-2 w-full px-2">
              {QUICK_ACTIONS.map((action) => (
                <button
                  key={action.label}
                  type="button"
                  onClick={() => handleSend(action.prompt)}
                  className="text-xs text-left px-3.5 py-1 rounded-full bg-secondary/50 border border-border text-muted-foreground hover:bg-secondary hover:text-foreground transition-colors"
                >
                  {action.label}
                </button>
              ))}
            </div>
            <p className="text-[10px] text-muted-foreground/50 mt-5">
              Tip: Select elements on canvas before chatting for context.
            </p>
          </div>
        ) : (
          messages.map((msg) => (
            <ChatMessage
              key={msg.id}
              role={msg.role}
              content={msg.content}
              isStreaming={msg.isStreaming && isStreaming}
              onApplyDesign={handleApplyDesign}
            />
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* --- Input area --- */}
      <div className="relative border-t border-border bg-card rounded-b-xl">
        <textarea
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={isStreaming ? 'Generating...' : 'Design with Claude or Codex...'}
          disabled={isStreaming}
          rows={2}
          className="w-full bg-transparent text-sm text-foreground placeholder-muted-foreground px-3.5 pt-3 pb-2 resize-none outline-none max-h-28 min-h-[52px]"
        />

        {/* --- Bottom bar: model selector + actions --- */}
        <div className="flex items-center justify-between px-2 pb-2">
          {/* Model selector */}
          <button
            type="button"
            onClick={() => setModelDropdownOpen((v) => !v)}
            disabled={isLoadingModels || availableModels.length === 0}
            className="flex items-center gap-1.5 text-[11px] text-muted-foreground hover:text-foreground transition-colors px-1.5 py-1 rounded-md hover:bg-secondary"
          >
            {(() => {
              const currentProvider = modelGroups.find((g) =>
                g.models.some((m) => m.value === model),
              )?.provider
              if (currentProvider) {
                const ProvIcon = PROVIDER_ICON[currentProvider]
                return <ProvIcon className="w-3.5 h-3.5 shrink-0" />
              }
              return null
            })()}
            <span className="truncate max-w-[160px]">
              {isLoadingModels
                ? 'Loading models...'
                : availableModels.find((m) => m.value === model)?.displayName ?? model}
            </span>
            <ChevronUp size={10} className="shrink-0" />
          </button>

          <div className="flex items-center gap-1 justify-between w-full">
            {selectedIds.length > 0 && (
              <span className="flex text-[10px] text-muted-foreground ml-2 select-none overflow-hidden text-ellipsis ">
                {selectedIds.length} object{selectedIds.length > 1 ? 's' : ''}
              </span>
            )}

            {/* Action icons */}
            <div className="flex items-center gap-0.5 w-full justify-end">
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => handleSend()}
                disabled={isStreaming || !input.trim()}
                title="Send message"
                className={cn(
                  'shrink-0 rounded-lg h-7 w-7',
                  input.trim() && !isStreaming
                    ? 'bg-foreground text-background hover:bg-foreground/90'
                    : '',
                )}
              >
                <Send size={13} />
              </Button>
            </div>
          </div>
        </div>

        {/* Upward model dropdown */}
        {modelDropdownOpen && availableModels.length > 0 && (
          <div className="absolute bottom-full left-2 right-2 mb-1 z-[60] rounded-lg border border-border bg-card shadow-xl py-1 max-h-72 overflow-y-auto">
            {modelGroups.length > 0
              ? modelGroups.map((group) => {
                  const GIcon = PROVIDER_ICON[group.provider]
                  return (
                    <div key={group.provider}>
                      <div className="flex items-center gap-1.5 px-3 pt-2.5 pb-1">
                        <GIcon className="w-3 h-3 text-muted-foreground" />
                        <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                          {group.providerName}
                        </span>
                      </div>
                      {group.models.map((m, idx) => {
                        const isSelected = m.value === model
                        return (
                          <button
                            key={m.value}
                            type="button"
                            onClick={() => {
                              setModel(m.value)
                              setModelDropdownOpen(false)
                            }}
                            className={cn(
                              'w-full flex items-center gap-2 px-3 py-1.5 text-xs transition-colors',
                              isSelected
                                ? 'bg-secondary text-foreground'
                                : 'text-muted-foreground hover:bg-accent hover:text-foreground',
                            )}
                          >
                            <span className="w-3.5 shrink-0">
                              {isSelected && <Check size={12} />}
                            </span>
                            <span className="font-medium">{m.displayName}</span>
                            {idx === 0 && (
                              <span className="text-[9px] text-muted-foreground bg-secondary px-1 py-0.5 rounded ml-auto">
                                Best
                              </span>
                            )}
                          </button>
                        )
                      })}
                    </div>
                  )
                })
              : availableModels.map((m) => {
                  const isSelected = m.value === model
                  return (
                    <button
                      key={m.value}
                      type="button"
                      onClick={() => {
                        setModel(m.value)
                        setModelDropdownOpen(false)
                      }}
                      className={cn(
                        'w-full flex items-center gap-2 px-3 py-1.5 text-xs transition-colors',
                        isSelected
                          ? 'bg-secondary text-foreground'
                          : 'text-muted-foreground hover:bg-accent hover:text-foreground',
                      )}
                    >
                      <span className="w-3.5 shrink-0">
                        {isSelected && <Check size={12} />}
                      </span>
                      <span className="font-medium">{m.displayName}</span>
                    </button>
                  )
                })}
          </div>
        )}
      </div>
    </div>
  )
}
