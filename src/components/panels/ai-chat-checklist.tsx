import { useState, useMemo } from 'react'
import { Pencil, ChevronDown, Check, AlertTriangle } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { ChatMessage as ChatMessageType } from '@/services/ai/ai-types'
import {
  parseStepBlocks,
  countDesignJsonBlocks,
  buildPipelineProgress,
} from './chat-message'

/** Parse [done]/[pending]/[error] prefix from a detail line */
function parseDetailStatus(line: string): { status: 'done' | 'pending' | 'error' | null; text: string } {
  const match = line.match(/^\[(done|pending|error)\]\s*(.*)$/)
  if (match) return { status: match[1] as 'done' | 'pending' | 'error', text: match[2] }
  return { status: null, text: line }
}

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
    const isApplied = content.includes('\u2705') || content.includes('<!-- APPLIED -->') || content.includes('[done] Applied')
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
            <div key={`${item.label}-${index}`} className="flex flex-col gap-0.5">
              <div className="flex items-center gap-2 text-[11px] text-muted-foreground/90">
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
              {item.details && item.details.length > 0 && (
                <div className="ml-[22px] flex flex-col gap-px">
                  {item.details.map((line, di) => {
                    const { status, text } = parseDetailStatus(line)
                    return (
                      <span key={di} className="flex items-center gap-1.5 text-[10px] text-muted-foreground/70">
                        {status === 'done' && (
                          <span className="w-2.5 h-2.5 rounded-full border border-emerald-500/70 text-emerald-500/80 flex items-center justify-center shrink-0">
                            <Check size={7} strokeWidth={2.5} />
                          </span>
                        )}
                        {status === 'pending' && (
                          <span className="w-2.5 h-2.5 rounded-full border border-primary/70 flex items-center justify-center shrink-0">
                            <span className="w-1 h-1 rounded-full bg-primary animate-pulse" />
                          </span>
                        )}
                        {status === 'error' && (
                          <AlertTriangle size={10} className="text-amber-500/80 shrink-0" />
                        )}
                        <span>{text}</span>
                      </span>
                    )
                  })}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
