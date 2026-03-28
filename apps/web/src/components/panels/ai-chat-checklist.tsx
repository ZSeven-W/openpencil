import { useState, useMemo } from 'react'
import { Pencil, ChevronDown, Check, AlertTriangle, Loader2, Circle } from 'lucide-react'
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
  const progress = items.length > 0 ? (completed / items.length) * 100 : 0

  // Hide checklist when streaming stopped with nothing completed
  if (!isStreaming && completed === 0) return null

  return (
    <div className="shrink-0 border-t border-border bg-card/95 backdrop-blur-sm">
      {/* Progress bar */}
      <div className="h-[2px] bg-secondary/50">
        <div
          className="h-full bg-primary transition-all duration-500 ease-out"
          style={{ width: `${progress}%` }}
        />
      </div>

      {/* Header */}
      <button
        type="button"
        onClick={() => setCollapsed(!collapsed)}
        className="flex items-center justify-between w-full px-3 py-2 hover:bg-secondary/30 transition-colors"
      >
        <div className="flex items-center gap-2">
          <Pencil size={13} className="text-muted-foreground shrink-0" />
          <span className="text-xs font-medium text-foreground">Pencil it out</span>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-[10px] font-medium tabular-nums text-muted-foreground bg-secondary/60 rounded-full px-1.5 py-0.5">
            {completed}/{items.length}
          </span>
          <ChevronDown
            size={12}
            className={cn(
              'text-muted-foreground transition-transform duration-200',
              collapsed ? '' : 'rotate-180',
            )}
          />
        </div>
      </button>

      {/* Item list */}
      {!collapsed && (
        <div className="px-3 pb-2.5 flex max-h-48 flex-col gap-0.5 overflow-y-auto">
          {items.map((item, index) => (
            <div key={`${item.label}-${index}`} className="flex flex-col">
              <div
                className={cn(
                  'flex items-center gap-2.5 py-1 px-1.5 rounded-md text-xs transition-colors',
                  item.active ? 'bg-primary/[0.06]' : '',
                )}
              >
                {/* Status indicator */}
                {item.done ? (
                  <span className="w-4 h-4 rounded-full bg-emerald-500/15 flex items-center justify-center shrink-0">
                    <Check size={10} strokeWidth={2.5} className="text-emerald-500" />
                  </span>
                ) : item.active ? (
                  <Loader2 size={14} className="text-primary animate-spin shrink-0" />
                ) : (
                  <Circle size={14} className="text-muted-foreground/30 shrink-0" />
                )}

                {/* Label */}
                <span
                  className={cn(
                    'truncate',
                    item.done
                      ? 'text-muted-foreground'
                      : item.active
                        ? 'text-foreground font-medium'
                        : 'text-muted-foreground/60',
                  )}
                >
                  {item.label}
                </span>
              </div>

              {/* Detail lines */}
              {item.details && item.details.length > 0 && (
                <div className="ml-[30px] flex flex-col gap-px pb-0.5">
                  {item.details.map((line, di) => {
                    const { status, text } = parseDetailStatus(line)
                    return (
                      <span key={di} className="flex items-center gap-1.5 text-[10px] text-muted-foreground/60">
                        {status === 'done' && (
                          <span className="w-2.5 h-2.5 rounded-full bg-emerald-500/15 flex items-center justify-center shrink-0">
                            <Check size={7} strokeWidth={2.5} className="text-emerald-500" />
                          </span>
                        )}
                        {status === 'pending' && (
                          <Loader2 size={9} className="text-primary/70 animate-spin shrink-0" />
                        )}
                        {status === 'error' && (
                          <AlertTriangle size={9} className="text-amber-500/80 shrink-0" />
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
