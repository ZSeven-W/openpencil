import React, { useState, useMemo, type ReactNode } from 'react';
import { Copy, Check, Wand2, ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';

/** Check if a JSON string looks like PenNode data */
export function isDesignJson(code: string): boolean {
  return /^\s*[[{]/.test(code) && /"type"\s*:/.test(code) && /"id"\s*:/.test(code);
}

export function parseMarkdown(
  text: string,
  onApplyDesign?: (json: string) => void,
  isApplied?: boolean,
  isStreaming?: boolean,
): ReactNode[] {
  const parts: ReactNode[] = [];
  const lines = text.split('\n');
  let inCodeBlock = false;
  let codeContent = '';
  let codeLang = '';
  let blockKey = 0;

  for (const line of lines) {
    if (line.startsWith('```') && !inCodeBlock) {
      inCodeBlock = true;
      codeLang = line.slice(3).trim();
      codeContent = '';
      continue;
    }

    if (line.startsWith('```') && inCodeBlock) {
      inCodeBlock = false;
      const code = codeContent.trimEnd();
      // For JSON blocks that look like design data, use the collapsed view
      if (codeLang === 'json' && isDesignJson(code)) {
        parts.push(
          <DesignJsonBlock
            key={`design-${blockKey++}`}
            code={code}
            onApply={onApplyDesign}
            isApplied={isApplied}
          />,
        );
      } else {
        parts.push(<CodeBlock key={`code-${blockKey++}`} code={code} language={codeLang} />);
      }
      continue;
    }

    if (inCodeBlock) {
      codeContent += (codeContent ? '\n' : '') + line;
      continue;
    }

    // Empty lines
    if (!line) {
      parts.push('\n');
      continue;
    }

    parts.push(
      <span key={`line-${blockKey++}`}>
        {parseInlineMarkdown(line)}
        {'\n'}
      </span>,
    );
  }

  // Handle unclosed code block (streaming)
  if (inCodeBlock && codeContent) {
    const code = codeContent.trimEnd();
    if (codeLang === 'json' && isDesignJson(code)) {
      parts.push(<DesignJsonBlock key={`design-${blockKey++}`} code={code} isStreaming />);
    } else {
      parts.push(<CodeBlock key={`code-${blockKey++}`} code={code} language={codeLang} />);
    }
  }

  // Strip bare '\n' entries adjacent to block-level components (DesignJsonBlock / CodeBlock)
  const isBlock = (n: ReactNode) =>
    typeof n === 'object' &&
    n !== null &&
    'type' in n &&
    ((n as React.ReactElement).type === DesignJsonBlock ||
      (n as React.ReactElement).type === CodeBlock);

  const cleaned: ReactNode[] = [];
  for (let i = 0; i < parts.length; i++) {
    if (parts[i] === '\n' && (isBlock(parts[i + 1]) || isBlock(parts[i - 1]))) continue;
    cleaned.push(parts[i]);
  }

  // Append inline streaming cursor — skip if last part is a block component
  if (isStreaming && cleaned.length > 0) {
    if (!isBlock(cleaned[cleaned.length - 1])) {
      cleaned.push(
        <span
          key="streaming-cursor"
          className="inline-block w-1.5 h-3.5 bg-muted-foreground/70 animate-pulse rounded-sm ml-0.5 align-text-bottom"
        />,
      );
    }
  }

  return cleaned;
}

function parseInlineMarkdown(text: string): ReactNode[] | string {
  // Fast path: no markdown syntax at all -> return plain string (no wrapper spans)
  if (!/[*`]/.test(text)) return text;

  const parts: ReactNode[] = [];
  let remaining = text;
  let key = 0;

  while (remaining.length > 0) {
    // Bold
    const boldMatch = remaining.match(/\*\*(.+?)\*\*/);
    // Inline code
    const codeMatch = remaining.match(/`([^`]+)`/);
    // Italic
    const italicMatch = remaining.match(/\*(.+?)\*/);

    const matches = [
      boldMatch && { match: boldMatch, type: 'bold' as const },
      codeMatch && { match: codeMatch, type: 'code' as const },
      italicMatch && { match: italicMatch, type: 'italic' as const },
    ]
      .filter(Boolean)
      .sort((a, b) => a!.match.index! - b!.match.index!);

    if (matches.length === 0) {
      parts.push(remaining);
      break;
    }

    const first = matches[0]!;
    const idx = first.match.index!;

    if (idx > 0) {
      parts.push(remaining.slice(0, idx));
    }

    if (first.type === 'bold') {
      parts.push(
        <strong key={key++} className="font-semibold">
          {first.match[1]}
        </strong>,
      );
    } else if (first.type === 'code') {
      parts.push(
        <code
          key={key++}
          className="bg-secondary text-foreground/80 px-1 py-0.5 rounded text-[0.85em]"
        >
          {first.match[1]}
        </code>,
      );
    } else {
      parts.push(
        <em key={key++} className="italic">
          {first.match[1]}
        </em>,
      );
    }

    remaining = remaining.slice(idx + first.match[0].length);
  }

  return parts;
}

function CodeBlock({ code, language }: { code: string; language: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

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
  );
}

/** Collapsed design JSON block — shows element count + expand toggle */
function DesignJsonBlock({
  code,
  onApply,
  isApplied,
  isStreaming,
}: {
  code: string;
  onApply?: (json: string) => void;
  isApplied?: boolean;
  isStreaming?: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  const elementCount = useMemo(() => {
    try {
      const parsed = JSON.parse(code);
      if (Array.isArray(parsed)) return parsed.length;
      return 1;
    } catch {
      // JSONL format: count lines that look like JSON objects
      if (/"_parent"\s*:/.test(code)) {
        return code.split('\n').filter((line) => line.trim().startsWith('{')).length;
      }
      return 0;
    }
  }, [code]);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="group mt-0.5 w-full">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className={cn(
          'flex items-center justify-between w-full px-3 py-2 text-left transition-all rounded-md border',
          expanded
            ? 'bg-secondary/40 border-border/60'
            : 'bg-background/40 hover:bg-secondary/20 border-border/30 hover:border-border/50',
        )}
      >
        <div className="flex items-center gap-2.5">
          <div className="w-4 h-4 rounded-full flex items-center justify-center bg-primary/10 text-primary shrink-0">
            <Wand2 size={10} />
          </div>
          <span
            className={cn(
              'text-[11px] font-medium tracking-tight',
              isStreaming
                ? 'text-muted-foreground animate-pulse'
                : 'text-foreground/90 group-hover:text-foreground',
            )}
          >
            {isStreaming
              ? 'Generating design...'
              : `${elementCount} design element${elementCount !== 1 ? 's' : ''}`}
          </span>
        </div>

        <div className="flex items-center gap-1">
          <span
            role="button"
            tabIndex={0}
            onClick={(e) => {
              e.stopPropagation();
              handleCopy();
            }}
            className="text-muted-foreground/30 hover:text-foreground transition-colors p-1 opacity-0 group-hover:opacity-100 mr-1"
            title="Copy JSON"
          >
            {copied ? <Check size={10} /> : <Copy size={10} />}
          </span>
          <ChevronDown
            size={12}
            className={cn(
              'text-muted-foreground/30 transition-transform duration-200',
              expanded ? 'rotate-180' : '',
            )}
          />
        </div>
      </button>

      {/* Expandable JSON content */}
      {expanded && (
        <div className="mt-1 rounded-md border border-border/30 overflow-hidden bg-card/50">
          <pre className="p-3 overflow-x-auto text-[9px] leading-relaxed max-h-48 overflow-y-auto font-mono text-muted-foreground/80">
            <code>{code}</code>
          </pre>

          {/* Apply button - hidden if applied or streaming */}
          {onApply && !isApplied && !isStreaming && (
            <div className="px-2 py-1.5 border-t border-border/30 bg-secondary/10">
              <Button
                onClick={() => onApply(code)}
                variant="ghost"
                className="w-full h-7 text-[10px] font-medium text-muted-foreground hover:text-primary hover:bg-primary/5"
                size="sm"
              >
                Apply to Canvas
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
