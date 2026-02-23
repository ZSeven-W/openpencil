import { useState, useRef, useEffect, useCallback } from 'react'
import { Send, Plus, ChevronDown, ChevronUp, Check, MessageSquare, Loader2 } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { useAIStore } from '@/stores/ai-store'
import type { PanelCorner } from '@/stores/ai-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import { fetchAvailableModels } from '@/services/ai/ai-service'
import {
  extractAndApplyDesign,
} from '@/services/ai/design-generator'
import type { AIProviderType } from '@/types/agent-settings'
import ClaudeLogo from '@/components/icons/claude-logo'
import OpenAILogo from '@/components/icons/openai-logo'
import OpenCodeLogo from '@/components/icons/opencode-logo'
import ChatMessage from './chat-message'
import { useChatHandlers } from './ai-chat-handlers'
import { FixedChecklist } from './ai-chat-checklist'

const PROVIDER_ICON: Record<AIProviderType, typeof ClaudeLogo> = {
  anthropic: ClaudeLogo,
  openai: OpenAILogo,
  opencode: OpenCodeLogo,
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
        opencode: 'OpenCode',
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
            if (!msgs[i].content.includes('\u2705') && !msgs[i].content.includes('<!-- APPLIED -->')) {
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
        'absolute z-50 flex w-[320px] flex-col overflow-hidden rounded-xl border border-border bg-card/95 shadow-2xl backdrop-blur-sm',
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
      <div className="min-h-0 flex-1 overflow-y-auto rounded-b-xl bg-background/80 px-3.5 py-3">
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

      {/* --- Fixed Checklist --- */}
      <FixedChecklist messages={messages} isStreaming={isStreaming} />

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

          <div className="flex items-center gap-1 w-full">
            <span
              className={cn(
                'ml-1 shrink-0 whitespace-nowrap text-[10px] select-none',
                selectedIds.length > 0 ? 'text-muted-foreground/80' : 'text-muted-foreground/40',
              )}
            >
              {selectedIds.length} selected
            </span>

            {/* Action icons */}
            <div className="ml-auto flex items-center gap-0.5">
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
