import React, { useState, useMemo, type ReactNode } from 'react'
import { Copy, Check, Wand2, ChevronDown, ChevronRight } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

interface ChatMessageProps {
  role: 'user' | 'assistant'
  content: string
  isStreaming?: boolean
  onApplyDesign?: (json: string) => void
}

/** Check if a JSON string looks like PenNode data */
function isDesignJson(code: string): boolean {
  return /^\s*[\[{]/.test(code) && /"type"\s*:/.test(code) && /"id"\s*:/.test(code)
}

function parseMarkdown(
  text: string,
  onApplyDesign?: (json: string) => void,
  isApplied?: boolean,
  isStreaming?: boolean,
): ReactNode[] {
  const parts: ReactNode[] = []
  const lines = text.split('\n')
  let inCodeBlock = false
  let codeContent = ''
  let codeLang = ''
  let blockKey = 0

  for (const line of lines) {
    if (line.startsWith('```') && !inCodeBlock) {
      inCodeBlock = true
      codeLang = line.slice(3).trim()
      codeContent = ''
      continue
    }

    if (line.startsWith('```') && inCodeBlock) {
      inCodeBlock = false
      const code = codeContent.trimEnd()
      // For JSON blocks that look like design data, use the collapsed view
      if (codeLang === 'json' && isDesignJson(code)) {
        parts.push(
          <DesignJsonBlock
            key={`design-${blockKey++}`}
            code={code}
            onApply={onApplyDesign}
            isApplied={isApplied}
          />,
        )
      } else {
        parts.push(
          <CodeBlock
            key={`code-${blockKey++}`}
            code={code}
            language={codeLang}
          />,
        )
      }
      continue
    }

    if (inCodeBlock) {
      codeContent += (codeContent ? '\n' : '') + line
      continue
    }

    // Empty lines → plain newline (parent has whitespace-pre-wrap)
    if (!line) {
      parts.push('\n')
      continue
    }

    parts.push(
      <span key={`line-${blockKey++}`}>
        {parseInlineMarkdown(line)}
        {'\n'}
      </span>,
    )
  }

  // Handle unclosed code block (streaming)
  if (inCodeBlock && codeContent) {
    const code = codeContent.trimEnd()
    if (codeLang === 'json' && isDesignJson(code)) {
      parts.push(
        <DesignJsonBlock
          key={`design-${blockKey++}`}
          code={code}
          isStreaming
        />,
      )
    } else {
      parts.push(
        <CodeBlock
          key={`code-${blockKey++}`}
          code={code}
          language={codeLang}
        />,
      )
    }
  }

  // Strip bare '\n' entries adjacent to block-level components (DesignJsonBlock / CodeBlock)
  const isBlock = (n: ReactNode) =>
    typeof n === 'object' && n !== null && 'type' in n &&
    ((n as React.ReactElement).type === DesignJsonBlock || (n as React.ReactElement).type === CodeBlock)

  const cleaned: ReactNode[] = []
  for (let i = 0; i < parts.length; i++) {
    if (parts[i] === '\n' && (isBlock(parts[i + 1]) || isBlock(parts[i - 1]))) continue
    cleaned.push(parts[i])
  }

  // Append inline streaming cursor — skip if last part is a block component
  if (isStreaming && cleaned.length > 0) {
    if (!isBlock(cleaned[cleaned.length - 1])) {
      cleaned.push(
        <span
          key="streaming-cursor"
          className="inline-block w-1.5 h-3.5 bg-muted-foreground/70 animate-pulse rounded-sm ml-0.5 align-text-bottom"
        />,
      )
    }
  }

  return cleaned
}

function parseInlineMarkdown(text: string): ReactNode[] | string {
  // Fast path: no markdown syntax at all → return plain string (no wrapper spans)
  if (!/[*`]/.test(text)) return text

  const parts: ReactNode[] = []
  let remaining = text
  let key = 0

  while (remaining.length > 0) {
    // Bold
    const boldMatch = remaining.match(/\*\*(.+?)\*\*/)
    // Inline code
    const codeMatch = remaining.match(/`([^`]+)`/)
    // Italic
    const italicMatch = remaining.match(/\*(.+?)\*/)

    const matches = [
      boldMatch && { match: boldMatch, type: 'bold' as const },
      codeMatch && { match: codeMatch, type: 'code' as const },
      italicMatch && { match: italicMatch, type: 'italic' as const },
    ]
      .filter(Boolean)
      .sort((a, b) => a!.match.index! - b!.match.index!)

    if (matches.length === 0) {
      parts.push(remaining)
      break
    }

    const first = matches[0]!
    const idx = first.match.index!

    if (idx > 0) {
      parts.push(remaining.slice(0, idx))
    }

    if (first.type === 'bold') {
      parts.push(
        <strong key={key++} className="font-semibold">
          {first.match[1]}
        </strong>,
      )
    } else if (first.type === 'code') {
      parts.push(
        <code
          key={key++}
          className="bg-secondary text-foreground/80 px-1 py-0.5 rounded text-[0.85em]"
        >
          {first.match[1]}
        </code>,
      )
    } else {
      parts.push(
        <em key={key++} className="italic">
          {first.match[1]}
        </em>,
      )
    }

    remaining = remaining.slice(idx + first.match[0].length)
  }

  return parts
}

function CodeBlock({ code, language }: { code: string; language: string }) {
  const [copied, setCopied] = useState(false)

  const handleCopy = () => {
    navigator.clipboard.writeText(code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="my-2 rounded-md overflow-hidden bg-background border border-border">
      <div className="flex items-center justify-between px-3 py-1 bg-card border-b border-border">
        <span className="text-[10px] text-muted-foreground uppercase">{language || 'code'}</span>
        <button
          type="button"
          onClick={handleCopy}
          className="text-muted-foreground hover:text-foreground transition-colors p-0.5"
          title="Copy code"
        >
          {copied ? <Check size={12} /> : <Copy size={12} />}
        </button>
      </div>
      <pre className="p-3 overflow-x-auto text-xs leading-relaxed">
        <code className="text-foreground/80">{code}</code>
      </pre>
    </div>
  )
}

/** Collapsed design JSON block — shows element count + expand toggle */
function DesignJsonBlock({
  code,
  onApply,
  isApplied,
  isStreaming,
}: {
  code: string
  onApply?: (json: string) => void
  isApplied?: boolean
  isStreaming?: boolean
}) {
  const [expanded, setExpanded] = useState(false)
  const [copied, setCopied] = useState(false)

  const elementCount = useMemo(() => {
    try {
      const parsed = JSON.parse(code)
      if (Array.isArray(parsed)) return parsed.length
      return 1
    } catch {
      return 0
    }
  }, [code])

  const handleCopy = () => {
    navigator.clipboard.writeText(code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="my-2 rounded-lg border border-border/60 overflow-hidden">
      {/* Header */}
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex items-center justify-between w-full px-3 py-1.5 bg-card hover:bg-accent transition-colors text-left"
      >
        <div className="flex items-center gap-1.5">
          {expanded ? (
            <ChevronDown size={12} className="text-muted-foreground" />
          ) : (
            <ChevronRight size={12} className="text-muted-foreground" />
          )}
          <Wand2 size={12} className="text-primary" />
          <span className={cn('text-xs', isStreaming ? 'text-muted-foreground' : 'text-foreground')}>
            {isStreaming
              ? 'Generating design...'
              : `${elementCount} design element${elementCount !== 1 ? 's' : ''}`}
          </span>
        </div>
        <span
          role="button"
          tabIndex={0}
          onClick={(e) => {
            e.stopPropagation()
            handleCopy()
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.stopPropagation()
              handleCopy()
            }
          }}
          className="text-muted-foreground hover:text-foreground transition-colors p-0.5 cursor-pointer"
          title="Copy JSON"
        >
          {copied ? <Check size={10} /> : <Copy size={10} />}
        </span>
      </button>

      {/* Expandable JSON content */}
      {expanded && (
        <pre className="p-3 overflow-x-auto text-xs leading-relaxed max-h-40 overflow-y-auto border-t border-border/60">
          <code className="text-muted-foreground/80">{code}</code>
        </pre>
      )}

      {/* Apply button (only if not already applied and handler exists) */}
      {onApply && !isApplied && !isStreaming && (
        <div className="px-2.5 py-2 border-t border-border/60">
          <Button
            onClick={() => onApply(code)}
            variant="outline"
            className="w-full"
            size="sm"
          >
            <Wand2 size={12} />
            Apply to Canvas
          </Button>
        </div>
      )}
    </div>
  )
}

export default function ChatMessage({
  role,
  content,
  isStreaming,
  onApplyDesign,
}: ChatMessageProps) {
  const isApplied = useMemo(
    () => role === 'assistant' && content.includes('\u2705'),
    [role, content],
  )

  const isUser = role === 'user'
  const isEmpty = !content.trim()

  // Don't render an empty non-streaming assistant message
  if (!isUser && isEmpty && !isStreaming) return null

  return (
    <div className={cn('flex', isUser ? 'justify-end' : 'justify-start')}>
      {isUser ? (
        <div className="max-w-[85%] rounded-lg px-3 py-2 text-sm leading-relaxed whitespace-pre-wrap bg-primary text-primary-foreground rounded-br-sm">
          {content}
        </div>
      ) : (
        <div className="text-sm leading-relaxed text-foreground">
          {/* Streaming with no content yet → thinking indicator */}
          {isEmpty && isStreaming ? (
            <div className="flex items-center gap-1.5 bg-secondary/50 rounded-full w-fit py-1 px-2.5 mt-2">
              <span className="text-xs text-muted-foreground">Thinking</span>
              <span className="flex gap-0.5">
                <span className="w-1 h-1 rounded-full bg-muted-foreground/70 animate-bounce" style={{ animationDelay: '0ms' }} />
                <span className="w-1 h-1 rounded-full bg-muted-foreground/70 animate-bounce" style={{ animationDelay: '150ms' }} />
                <span className="w-1 h-1 rounded-full bg-muted-foreground/70 animate-bounce" style={{ animationDelay: '300ms' }} />
              </span>
            </div>
          ) : (
            <div className="whitespace-pre-wrap">
              {parseMarkdown(content, onApplyDesign, isApplied, isStreaming && !isEmpty)}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
