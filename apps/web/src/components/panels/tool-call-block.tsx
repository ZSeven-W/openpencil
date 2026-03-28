import { useState } from 'react'
import {
  ChevronRight,
  ChevronDown,
  Loader2,
  Check,
  X,
  Undo2,
  Wrench,
  Search,
  Plus,
  Pencil,
  Trash2,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import type { AuthLevel } from '@zseven-w/agent'

export interface ToolCallBlockData {
  id: string
  name: string
  args: unknown
  level: AuthLevel
  result?: { success: boolean; data?: unknown; error?: string }
  status: 'pending' | 'running' | 'done' | 'error'
  source?: string
}

interface ToolCallBlockProps {
  block: ToolCallBlockData
  onUndo?: (toolCallId: string) => void
}

const levelConfig: Record<
  AuthLevel,
  {
    icon: typeof Search
    defaultOpen: boolean
    className: string
    label: string
  }
> = {
  read: {
    icon: Search,
    defaultOpen: false,
    className: 'text-muted-foreground',
    label: 'Read',
  },
  create: {
    icon: Plus,
    defaultOpen: false,
    className: 'text-foreground',
    label: 'Create',
  },
  modify: {
    icon: Pencil,
    defaultOpen: true,
    className: 'text-foreground',
    label: 'Modify',
  },
  delete: {
    icon: Trash2,
    defaultOpen: true,
    className: 'text-destructive',
    label: 'Delete',
  },
  orchestrate: {
    icon: Wrench,
    defaultOpen: true,
    className: 'text-primary',
    label: 'Delegate',
  },
}

export function ToolCallBlock({ block, onUndo }: ToolCallBlockProps) {
  const config = levelConfig[block.level] ?? levelConfig.read
  const [isOpen, setIsOpen] = useState(config.defaultOpen)
  const Icon = config.icon
  const ChevronIcon = isOpen ? ChevronDown : ChevronRight

  return (
    <div
      className={cn(
        'my-1 rounded-md border border-border bg-card/50 text-sm',
        block.level === 'delete' && 'border-destructive/30 bg-destructive/5',
      )}
    >
      <button
        className={cn(
          'flex w-full items-center gap-1.5 px-2 py-1 text-left',
          config.className,
        )}
        onClick={() => setIsOpen(!isOpen)}
      >
        <ChevronIcon className="h-3 w-3 shrink-0 opacity-50" />
        <Icon className="h-3.5 w-3.5 shrink-0" />
        <span className="truncate font-medium">{block.name}</span>
        {block.source && block.source !== 'lead' && (
          <span className="shrink-0 rounded bg-primary/10 px-1 py-0.5 text-[10px] text-primary">{block.source}</span>
        )}
        <span className="ml-auto flex items-center gap-1">
          {block.status === 'running' && (
            <Loader2 className="h-3 w-3 animate-spin" />
          )}
          {block.status === 'done' && block.result?.success && (
            <Check className="h-3 w-3 text-green-500" />
          )}
          {(block.status === 'error' ||
            (block.status === 'done' && !block.result?.success)) && (
            <X className="h-3 w-3 text-destructive" />
          )}
          {block.level === 'delete' &&
            block.status === 'done' &&
            onUndo && (
              <button
                className="ml-1 rounded px-1.5 py-0.5 text-xs text-destructive hover:bg-destructive/10"
                onClick={(e) => {
                  e.stopPropagation()
                  onUndo(block.id)
                }}
              >
                <Undo2 className="inline h-3 w-3 mr-0.5" />
                Undo
              </button>
            )}
        </span>
      </button>
      {isOpen && (
        <div className="border-t border-border px-2 py-1.5 text-xs text-muted-foreground">
          <pre className="whitespace-pre-wrap break-all">
            {JSON.stringify(block.args, null, 2)}
          </pre>
          {block.result && (
            <div
              className={cn(
                'mt-1 pt-1 border-t border-border',
                !block.result.success && 'text-destructive',
              )}
            >
              {block.result.success ? (
                <pre className="whitespace-pre-wrap break-all">
                  {JSON.stringify(block.result.data, null, 2)}
                </pre>
              ) : (
                <span>{block.result.error}</span>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
