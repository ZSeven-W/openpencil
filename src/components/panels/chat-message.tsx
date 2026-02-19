import React, { useState, useMemo, type ReactNode } from 'react'
import { Copy, Check, Wand2, ChevronDown, ChevronRight, ListOrdered, FileJson, Palette, LayoutTemplate, ScanSearch } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

interface ChatMessageProps {
  role: 'user' | 'assistant'
  content: string
  isStreaming?: boolean
  onApplyDesign?: (json: string) => void
}

/** Strip raw tool-call / function-call XML that should never be shown to users */
/** Strip raw tool-call / function-call XML that should never be shown to users */
function stripToolCallXml(text: string): string {
  let cleaned = text

  // Remove <function_calls> blocks
  cleaned = cleaned.replace(/<function_calls>[\s\S]*?<\/function_calls>/g, '')
  
  // Remove <result> blocks (often tool outputs)
  cleaned = cleaned.replace(/<result>[\s\S]*?<\/result>/g, '')

  // Remove <inference_process> or similar internal blocks if they appear
  cleaned = cleaned.replace(/<inference_process>[\s\S]*?<\/inference_process>/g, '')

  // Remove <invoke> blocks (tool usage) - handle both closed and streaming/unclosed
  cleaned = cleaned.replace(/<invoke[\s\S]*?<\/invoke>/g, '')
  cleaned = cleaned.replace(/<invoke[\s\S]*?$/g, '') // Hide unclosed invoke at end of stream

  // Remove <parameter> blocks if they appear outside invoke for some reason
  cleaned = cleaned.replace(/<parameter[\s\S]*?<\/parameter>/g, '')

  // Remove stray tags
  cleaned = cleaned.replace(/<\/?invoke.*?>/g, '')
  cleaned = cleaned.replace(/<\/?parameter.*?>/g, '')
  cleaned = cleaned.replace(/<\/?function_calls>/g, '')
  cleaned = cleaned.replace(/<\/?search_quality_reflection>/g, '') // Sometimes this appears too
  cleaned = cleaned.replace(/<\/?thought_process>/g, '') // And this

  // Remove the hidden marker so it doesn't show up in UI even as whitespace
  cleaned = cleaned.replace(/<!-- APPLIED -->/g, '')
  
  // Collapse leftover blank lines into at most one
  cleaned = cleaned.replace(/\n{3,}/g, '\n\n')
  return cleaned.trim()
}

/** Check if a line is a step action */
function isActionStep(line: string): boolean {
  return /<step.*<\/step>/.test(line) || line.trim().startsWith('<step')
}

function parseStepTitle(step: string): string {
  const match = step.match(/title="([^"]+)"/)
  return match ? match[1] : 'Processing'
}

function parseStepContent(step: string): string {
  return step.replace(/<step[^>]*>/, '').replace(/<\/step>/, '').trim()
}

/** Component for rendering a list of action steps as accordions */
function ActionSteps({ steps }: { steps: string[] }) {
  if (steps.length === 0) return null
  
  return (
    <div className="flex flex-col gap-2 my-1 w-full">
      {steps.map((step, i) => {
        const title = parseStepTitle(step)
        const content = parseStepContent(step)
        const isDesign = title.toLowerCase() === 'design'
        
        // Icon mapping based on step title
        let Icon = ScanSearch
        if (title.toLowerCase().includes('guidelines')) Icon = FileJson
        if (title.toLowerCase().includes('state')) Icon = ListOrdered
        if (title.toLowerCase().includes('styleguide')) Icon = Palette
        if (isDesign) Icon = LayoutTemplate

        return (
          <ActionStepItem 
            key={i} 
            title={title} 
            content={content} 
            icon={Icon}
            defaultOpen={isDesign}
            isLast={i === steps.length - 1}
          />
        )
      })}
    </div>
  )
}

function ActionStepItem({ 
  title, 
  content, 
  icon: Icon, 
  defaultOpen = false,
  isLast 
}: { 
  title: string
  content: string
  icon: any
  defaultOpen?: boolean 
  isLast?: boolean
}) {
  const [isOpen, setIsOpen] = useState(defaultOpen)

  return (
    <div className="border-b border-border/40 last:border-0 bg-transparent">
      <button 
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-3 w-full px-2 py-2 text-left hover:bg-secondary/30 transition-colors group"
      >
        <div className={cn(
          "w-4 h-4 rounded-full flex items-center justify-center shrink-0 transition-colors",
          isLast 
            ? "bg-primary/10 text-primary" 
            : "text-muted-foreground/40 group-hover:text-muted-foreground/80"
        )}>
          {isLast ? (
             <div className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse shadow-[0_0_8px_rgba(34,197,94,0.5)]" />
          ) : (
             <Check size={10} />
          )}
        </div>
        
        <span title={title} className={cn(
          "text-[11px] font-medium flex-1 transition-colors truncate",
          isLast ? "text-foreground" : "text-muted-foreground"
        )}>
          {title}
        </span>
        
        <Icon size={12} className={cn(
          "transition-opacity",
          isOpen ? "text-foreground opacity-100" : "text-muted-foreground opacity-0 group-hover:opacity-50"
        )} />
      </button>

      {isOpen && (
        <div className="px-2 pb-2 pl-[34px] text-[10px] text-muted-foreground/80 leading-relaxed font-mono animate-in slide-in-from-top-0.5 duration-200 whitespace-pre-wrap break-words">
           {content}
        </div>
      )}
    </div>
  )
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

  // Pre-process: extract sequential steps at the start or throughout?
  // Our prompt puts them at the start. Let's process line by line.
  // If we encounter steps, we collect them. If we encounter non-step content, we flush steps.

  // Actually, simpler: Treat <step> lines as special blocks.
  
  let currentSteps: string[] = []
  
  const flushSteps = () => {
    if (currentSteps.length > 0) {
      parts.push(<ActionSteps key={`steps-${blockKey++}`} steps={[...currentSteps]} />)
      currentSteps = []
    }
  }

  for (const line of lines) {
    if (isActionStep(line)) {
      // Check if it's a complete step or partial (streaming)
      // For now assume complete lines or handle partials if needed
      // If valid step line, add to current buffer
      currentSteps.push(line)
      continue
    }

    // specific hack for streaming partially completed step
    if (line.trim().startsWith('<step') && !line.trim().includes('</step>')) {
       // It's a streaming step, possibly unfinished. 
       // We can show it as "Working..." or just text. 
       // Let's just treat it as a step for now?
       currentSteps.push(line + '</step>') // Auto-close for display
       continue
    }

    // Not a step -> flush any pending steps
    flushSteps()

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

  // Flush remaining steps at the end
  flushSteps()

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
    <div className="my-1.5 rounded border border-border/30 overflow-hidden bg-background/50 backdrop-blur-[1px]">
      {/* Header */}
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex items-center justify-between w-full px-2 py-1.5 hover:bg-secondary/30 transition-colors text-left group"
      >
        <div className="flex items-center gap-2">
          {expanded ? (
            <ChevronDown size={10} className="text-muted-foreground/50" />
          ) : (
            <ChevronRight size={10} className="text-muted-foreground/50" />
          )}
          <div className="w-4 h-4 rounded flex items-center justify-center bg-primary/5 text-primary">
             <Wand2 size={9} />
          </div>
          <span className={cn('text-[10px] font-medium tracking-tight', isStreaming ? 'text-muted-foreground animate-pulse' : 'text-muted-foreground/80 group-hover:text-foreground')}>
            {isStreaming
              ? 'Generating...'
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
          className="text-muted-foreground/30 hover:text-foreground transition-colors p-1 opacity-0 group-hover:opacity-100"
          title="Copy JSON"
        >
          {copied ? <Check size={9} /> : <Copy size={9} />}
        </span>
      </button>

      {/* Expandable JSON content */}
      {expanded && (
        <pre className="p-2 overflow-x-auto text-[9px] leading-relaxed max-h-32 overflow-y-auto border-t border-border/30 font-mono bg-card/30">
          <code className="text-muted-foreground">{code}</code>
        </pre>
      )}

      {/* Apply button - hidden if applied or streaming */}
      {onApply && !isApplied && !isStreaming && (
        <div className="px-2 py-1.5 border-t border-border/30 bg-secondary/10">
          <Button
            onClick={() => onApply(code)}
            variant="ghost"
            className="w-full h-6 text-[10px] font-medium text-muted-foreground hover:text-primary hover:bg-primary/5"
            size="sm"
          >
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
    () => role === 'assistant' && (content.includes('\u2705') || content.includes('<!-- APPLIED -->')),
    [role, content],
  )


  const isUser = role === 'user'
  // Strip raw tool-call XML that the model may emit (should never be visible)
  const displayContent = isUser ? content : stripToolCallXml(content)
  const isEmpty = !displayContent.trim()

  // Don't render an empty non-streaming assistant message
  // UNLESS we stripped something out (meaning the AI did something, but we hid it).
  // In that case, show a generic "Design generated" message or similar to avoid confusion?
  // Or better, if it's empty, it means we probably just suppressed a tool call.
  // Let's show a "Processing..." or "Action completed" placeholder if it's empty but had content.
  const hadContent = content.trim().length > 0
  if (!isUser && isEmpty && !isStreaming) {
     if (hadContent) {
       return (
         <div className="text-xs text-muted-foreground italic px-2 py-1">
           (Automated action completed)
         </div>
       )
     }
     return null
  }

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
            <div className="whitespace-pre-wrap mb-2">
              {parseMarkdown(displayContent, onApplyDesign, isApplied, isStreaming && !isEmpty)}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
