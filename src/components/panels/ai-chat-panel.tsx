import { useState, useRef, useEffect, useCallback } from 'react'
import { Send, Trash2, Minus, Maximize2, Sparkles, ChevronDown } from 'lucide-react'
import { nanoid } from 'nanoid'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { useAIStore } from '@/stores/ai-store'
import type { PanelCorner } from '@/stores/ai-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { streamChat, fetchAvailableModels } from '@/services/ai/ai-service'
import {
  CHAT_SYSTEM_PROMPT,
  DESIGN_GENERATOR_PROMPT,
} from '@/services/ai/ai-prompts'
import { extractAndApplyDesign } from '@/services/ai/design-generator'
import type { ChatMessage as ChatMessageType } from '@/services/ai/ai-types'
import ChatMessage from './chat-message'

const QUICK_ACTIONS = [
  {
    label: '生成登录页',
    prompt: 'Design a modern mobile login screen with email input, password input, login button, and social login options',
  },
  {
    label: '生成卡片',
    prompt: 'Create a product card with an image placeholder, title, price, and buy button',
  },
  {
    label: '生成导航栏',
    prompt: 'Design a mobile app bottom navigation bar with 5 tabs: Home, Search, Add, Messages, Profile',
  },
  {
    label: '配色建议',
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
  const flatNodes = useDocumentStore.getState().getFlatNodes()

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
      const context = buildContextString()
      const fullUserMessage = messageText + context
      const isDesign = isDesignRequest(messageText)

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

      const chatHistory = messages.map((m) => ({
        role: m.role,
        content: m.content,
      }))

      const effectiveUserMessage = isDesign
        ? `${fullUserMessage}\n\n[INSTRUCTION: Output a \`\`\`json code block containing a PenNode JSON array. The JSON must come FIRST in your response. After the JSON block, add a 1-2 sentence summary. Do NOT describe the design before the JSON — output the JSON immediately.]`
        : fullUserMessage
      chatHistory.push({ role: 'user' as const, content: effectiveUserMessage })

      const effectivePrompt = isDesign
        ? DESIGN_GENERATOR_PROMPT
        : CHAT_SYSTEM_PROMPT

      let accumulated = ''
      try {
        for await (const chunk of streamChat(effectivePrompt, chatHistory, model)) {
          if (chunk.type === 'text') {
            accumulated += chunk.content
            updateLastMessage(accumulated)
          } else if (chunk.type === 'error') {
            accumulated += `\n\n**Error:** ${chunk.content}`
            updateLastMessage(accumulated)
          }
        }
      } catch (error) {
        const errMsg = error instanceof Error ? error.message : 'Unknown error'
        accumulated += `\n\n**Error:** ${errMsg}`
        updateLastMessage(accumulated)
      }

      let appliedCount = extractAndApplyDesign(accumulated)

      if (isDesign && appliedCount === 0 && accumulated.length > 50) {
        const retryHistory = [
          ...chatHistory,
          { role: 'assistant' as const, content: accumulated },
          { role: 'user' as const, content: 'You forgot to output the PenNode JSON. Output ONLY a ```json code block with the PenNode JSON array now. No other text.' },
        ]
        accumulated += '\n\n*Generating design JSON...*\n'
        updateLastMessage(accumulated)

        try {
          for await (const chunk of streamChat(effectivePrompt, retryHistory, model)) {
            if (chunk.type === 'text') {
              accumulated += chunk.content
              updateLastMessage(accumulated)
            }
          }
          appliedCount = extractAndApplyDesign(accumulated)
        } catch {
          // Retry failed
        }
      }

      setStreaming(false)

      if (appliedCount > 0) {
        accumulated += `\n\n✅ **${appliedCount} element${appliedCount > 1 ? 's' : ''} added to canvas**`
      }

      useAIStore.setState((s) => {
        const msgs = [...s.messages]
        const last = msgs[msgs.length - 1]
        if (last) {
          msgs[msgs.length - 1] = { ...last, content: accumulated, isStreaming: false }
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
      className="h-7 bg-card border border-border rounded-lg flex items-center gap-1.5 px-3 shadow-lg hover:bg-accent transition-colors"
    >
      <Sparkles size={12} className="text-purple-400" />
      <span className="text-xs text-muted-foreground">AI Assistant</span>
      <Maximize2 size={12} className="text-muted-foreground" />
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
  const [dragStyle, setDragStyle] = useState<React.CSSProperties | null>(null)

  const messages = useAIStore((s) => s.messages)
  const isStreaming = useAIStore((s) => s.isStreaming)
  const clearMessages = useAIStore((s) => s.clearMessages)
  const panelCorner = useAIStore((s) => s.panelCorner)
  const isMinimized = useAIStore((s) => s.isMinimized)
  const setPanelCorner = useAIStore((s) => s.setPanelCorner)
  const toggleMinimize = useAIStore((s) => s.toggleMinimize)
  const model = useAIStore((s) => s.model)
  const setModel = useAIStore((s) => s.setModel)
  const availableModels = useAIStore((s) => s.availableModels)
  const setAvailableModels = useAIStore((s) => s.setAvailableModels)
  const isLoadingModels = useAIStore((s) => s.isLoadingModels)
  const setLoadingModels = useAIStore((s) => s.setLoadingModels)
  const [modelDropdownOpen, setModelDropdownOpen] = useState(false)
  const { input, setInput, handleSend } = useChatHandlers()

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  // Fetch available models on mount
  useEffect(() => {
    if (availableModels.length > 0) return
    setLoadingModels(true)
    fetchAvailableModels().then((models) => {
      if (models.length > 0) {
        setAvailableModels(models)
        // Set default model if current model is not in the list
        if (!models.some((m) => m.value === model)) {
          setModel(models[0].value)
        }
      }
      setLoadingModels(false)
    })
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

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

  const handleApplyDesign = useCallback((jsonString: string) => {
    const count = extractAndApplyDesign('```json\n' + jsonString + '\n```')
    if (count > 0) {
      useAIStore.setState((s) => {
        const msgs = [...s.messages]
        for (let i = msgs.length - 1; i >= 0; i--) {
          if (msgs[i].role === 'assistant' && msgs[i].content.includes(jsonString.slice(0, 50))) {
            if (!msgs[i].content.includes('✅')) {
              msgs[i] = {
                ...msgs[i],
                content: msgs[i].content + `\n\n✅ **${count} element${count > 1 ? 's' : ''} added to canvas**`,
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
        'absolute z-50 w-[360px] rounded-xl shadow-2xl border border-border bg-card/95 backdrop-blur-sm flex flex-col overflow-hidden',
        !dragStyle && CORNER_CLASSES[panelCorner],
      )}
      style={dragStyle ?? undefined}
    >
      {/* --- Header (draggable) --- */}
      <div
        className="flex items-center justify-between px-3 py-2 border-b border-border cursor-grab active:cursor-grabbing select-none"
        onPointerDown={handleDragStart}
        onPointerMove={handleDragMove}
        onPointerUp={handleDragEnd}
      >
        <div className="flex items-center gap-2">
          <Sparkles size={14} className="text-purple-400" />
          {/* Model selector */}
          <div className="relative">
            <button
              type="button"
              onClick={() => setModelDropdownOpen((v) => !v)}
              disabled={isLoadingModels || availableModels.length === 0}
              className="flex items-center gap-1 text-xs font-medium text-foreground hover:text-accent-foreground transition-colors"
            >
              <span className="max-w-[120px] truncate">
                {availableModels.find((m) => m.value === model)?.displayName ?? model}
              </span>
              <ChevronDown size={10} className="text-muted-foreground shrink-0" />
            </button>
            {modelDropdownOpen && availableModels.length > 0 && (
              <div className="absolute top-full left-0 mt-1 z-[60] w-56 rounded-lg border border-border bg-card shadow-xl py-1 max-h-60 overflow-y-auto">
                {availableModels.map((m) => (
                  <button
                    key={m.value}
                    type="button"
                    onClick={() => {
                      setModel(m.value)
                      setModelDropdownOpen(false)
                    }}
                    className={cn(
                      'w-full text-left px-3 py-1.5 text-xs transition-colors',
                      m.value === model
                        ? 'bg-secondary text-foreground'
                        : 'text-muted-foreground hover:bg-accent hover:text-foreground',
                    )}
                  >
                    <span className="font-medium">{m.displayName}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
        <div className="flex items-center gap-1">
          {messages.length > 0 && (
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={clearMessages}
              title="Clear chat"
            >
              <Trash2 size={12} />
            </Button>
          )}
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={toggleMinimize}
            title="Minimize"
          >
            <Minus size={12} />
          </Button>
        </div>
      </div>

      {/* --- Messages --- */}
      <div className="overflow-y-auto p-3 max-h-[350px]">
        {messages.length === 0 ? (
          <div className="text-center mt-4">
            <div className="text-2xl mb-2">🎨</div>
            <p className="text-sm text-foreground mb-1 font-medium">
              Design with AI
            </p>
            <p className="text-xs text-muted-foreground mb-4">
              Describe any UI and it appears on canvas
            </p>
            <div className="flex flex-wrap gap-2 justify-center">
              {QUICK_ACTIONS.map((action) => (
                <button
                  key={action.label}
                  type="button"
                  onClick={() => handleSend(action.prompt)}
                  className="text-xs px-2.5 py-1.5 rounded-full bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
                >
                  {action.label}
                </button>
              ))}
            </div>
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

      {/* --- Input --- */}
      <div className="p-2 border-t border-border">
        <div className="flex items-end gap-1.5 bg-background/50 rounded-lg border border-input focus-within:border-ring transition-colors">
          <textarea
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={isStreaming ? 'Generating...' : 'Ask AI anything...'}
            disabled={isStreaming}
            rows={1}
            className="flex-1 bg-transparent text-sm text-foreground placeholder-muted-foreground px-3 py-2 resize-none outline-none max-h-24 min-h-[36px]"
          />
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => handleSend()}
            disabled={isStreaming || !input.trim()}
            title="Send message"
            className="shrink-0 m-1"
          >
            <Send size={14} />
          </Button>
        </div>
      </div>
    </div>
  )
}
