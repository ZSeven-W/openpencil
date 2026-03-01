import { useState, useMemo } from 'react'
import { Pencil, ChevronDown, Check } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { ChatMessage as ChatMessageType } from '@/services/ai/ai-types'
import {
  parseStepBlocks,
  countDesignJsonBlocks,
  buildPipelineProgress,
} from './chat-message'

/** Fixed collapsible checklist pinned between messages and input */
export function FixedChecklist({ messages, isStreaming }: { messages: ChatMessageType[]; isStreaming: boolean }) {
  const [collapsed, setCollapsed] = useState(false)

  // Find the last assistant message to extract checklist data
  const lastAssistant = useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === 'assistant') return messages[i]
    }
    return null
  }, [messages])

  const items = useMemo(() => {
    if (!lastAssistant) return []
    const content = lastAssistant.content
    const steps = parseStepBlocks(content, isStreaming)
    const planSteps = steps.filter((s) => s.title !== 'Thinking')
    if (planSteps.length === 0) return []
    const jsonCount = countDesignJsonBlocks(content)
    const isApplied = content.includes('\u2705') || content.includes('<!-- APPLIED -->')
    const hasError = /\*\*Error:\*\*/i.test(content)
    return buildPipelineProgress(planSteps, jsonCount, isStreaming, isApplied, hasError)
  }, [lastAssistant, isStreaming])

  if (items.length === 0) return null

  const completed = items.filter((item) => item.done).length

  // Hide checklist when streaming stopped with nothing completed
  if (!isStreaming && completed === 0) return null

  return (
    <div className="shrink-0 border-t border-border bg-card/95">
      <button
        type="button"
        onClick={() => setCollapsed(!collapsed)}
        className="flex items-center justify-between w-full px-3 py-2 hover:bg-secondary/30 transition-colors"
      >
        <div className="flex items-center gap-2">
          <Pencil size={13} className="text-muted-foreground shrink-0" />
          <span className="text-xs font-medium text-foreground">Pencil it out</span>
        </div>
        <div className="flex items-center gap-1.5">
          <span className="text-xs text-muted-foreground">{completed}/{items.length}</span>
          <ChevronDown
            size={12}
            className={cn(
              'text-muted-foreground transition-transform duration-200',
              collapsed ? '' : 'rotate-180',
            )}
          />
        </div>
      </button>
      {!collapsed && (
        <div className="px-3 pb-2.5 flex max-h-44 flex-col gap-1 overflow-y-auto">
          {items.map((item, index) => (
            <div key={`${item.label}-${index}`} className="flex items-center gap-2 text-[11px] text-muted-foreground/90">
              <span
                className={cn(
                  'w-3.5 h-3.5 rounded-full border flex items-center justify-center shrink-0',
                  item.done
                    ? 'border-emerald-500/70 text-emerald-500/80'
                    : item.active
                      ? 'border-primary/70 text-primary'
                      : 'border-border/70 text-muted-foreground/50',
                )}
              >
                {item.done ? (
                  <Check size={9} strokeWidth={2.5} />
                ) : (
                  <span className={cn(
                    'w-1.5 h-1.5 rounded-full',
                    item.active ? 'bg-primary animate-pulse' : 'bg-muted-foreground/60',
                  )} />
                )}
              </span>
              <span className={cn(item.active ? 'text-foreground' : '')}>{item.label}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
