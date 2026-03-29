import { useState } from 'react';
import { Check, ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';
import { isDesignJson } from './chat-message-content';

export interface ParsedStep {
  title: string;
  content: string;
  /** Explicit status from orchestrator steps (undefined for normal steps) */
  status?: 'pending' | 'streaming' | 'done' | 'error';
}

export function parseStepBlocks(text: string, isStreaming?: boolean): ParsedStep[] {
  const stepRegex = /<step([^>]*)>([\s\S]*?)<\/step>/gi;
  const parsed: ParsedStep[] = [];
  let match: RegExpExecArray | null;

  while ((match = stepRegex.exec(text)) !== null) {
    const attrs = match[1];
    const titleMatch = attrs.match(/title="([^"]+)"/);
    const statusMatch = attrs.match(/status="([^"]+)"/);
    parsed.push({
      title: (titleMatch?.[1] ?? 'Processing').trim() || 'Processing',
      status: (statusMatch?.[1] as ParsedStep['status']) ?? undefined,
      content: (match[2] ?? '').trim(),
    });
  }

  const lastOpen = text.lastIndexOf('<step');
  const lastClose = text.lastIndexOf('</step>');
  if (isStreaming && lastOpen > lastClose) {
    const partial = text.slice(lastOpen);
    const titleMatch = partial.match(/title="([^"]+)"/i);
    const statusMatch = partial.match(/status="([^"]+)"/i);
    const contentStart = partial.indexOf('>');
    parsed.push({
      title: (titleMatch?.[1] ?? 'Design').trim() || 'Design',
      status: (statusMatch?.[1] as ParsedStep['status']) ?? undefined,
      content:
        contentStart >= 0
          ? partial
              .slice(contentStart + 1)
              .replace(/<\/step>$/i, '')
              .trim()
          : '',
    });
  }

  return parsed;
}

export function stripStepBlocks(text: string): string {
  return text
    .replace(/<step(?:[^>]*title="[^"]*")?[^>]*>[\s\S]*?<\/step>/gi, '')
    .replace(/<step(?:[^>]*title="[^"]*")?[^>]*>[\s\S]*$/gi, '')
    .trim();
}

/** Count completed sections in JSONL content (direct children of root frame). */
function countJsonlSections(content: string): number {
  const lines = content.split('\n');
  let rootId: string | null = null;
  let sectionCount = 0;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('{')) continue;

    const parentMatch = trimmed.match(/"_parent"\s*:\s*(null|"([^"]*)")/);
    if (!parentMatch) continue;

    if (parentMatch[1] === 'null') {
      const idMatch = trimmed.match(/"id"\s*:\s*"([^"]*)"/);
      if (idMatch) rootId = idMatch[1];
    } else if (rootId && parentMatch[2] === rootId) {
      sectionCount++;
    }
  }

  return sectionCount;
}

export function countDesignJsonBlocks(text: string): number {
  const blockRegex = /```(?:json)?\s*\n?([\s\S]*?)(?:\n?```|$)/gi;
  let count = 0;
  let match: RegExpExecArray | null;
  while ((match = blockRegex.exec(text)) !== null) {
    const content = match[1].trim();
    if (!isDesignJson(content)) continue;

    // JSONL format: count direct children of root as sections
    if (/"_parent"\s*:/.test(content)) {
      count += countJsonlSections(content);
    } else {
      count += 1;
    }
  }
  return count;
}

export interface PipelineItem {
  label: string;
  done: boolean;
  active: boolean;
  /** Optional detail lines (e.g. validation log) */
  details?: string[];
}

export function buildPipelineProgress(
  steps: ParsedStep[],
  jsonBlockCount: number,
  isStreaming: boolean,
  isApplied: boolean,
  hasError: boolean,
): PipelineItem[] {
  // No steps = no checklist
  if (steps.length === 0) return [];

  // Parse detail lines from step content (one line per entry)
  function extractDetails(content: string): string[] | undefined {
    if (!content) return undefined;
    const lines = content
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean);
    return lines.length > 0 ? lines : undefined;
  }

  // If steps have explicit status (orchestrator mode), use that directly.
  // Check this BEFORE terminal result logic so that user-stopped generations
  // preserve the actual per-step status instead of marking everything done.
  const hasExplicitStatus = steps.some((s) => s.status !== undefined);
  if (hasExplicitStatus) {
    return steps.map((s) => ({
      label: s.title,
      done: s.status === 'done',
      active: isStreaming && s.status === 'streaming',
      details: extractDetails(s.content),
    }));
  }

  // If generation is complete and applied, mark all steps done
  const hasTerminalResult = !isStreaming && !hasError && (isApplied || jsonBlockCount > 0);
  if (hasTerminalResult) {
    return steps.map((s) => ({
      label: s.title,
      done: true,
      active: false,
      details: extractDetails(s.content),
    }));
  }

  // Fallback: Map each step to done/active/pending based on completed JSON blocks.
  // Step[i] is done when jsonBlockCount > i.
  // The step at jsonBlockCount is active (currently being generated).
  return steps.map((s, index) => {
    const done = index < jsonBlockCount;
    const active = isStreaming && !done && index === jsonBlockCount;
    return { label: s.title, done, active, details: extractDetails(s.content) };
  });
}

/** Component for rendering a list of action steps as accordions.
 *  Only shows steps with non-empty content (e.g. thinking, analysis).
 *  Empty plan steps are shown in PipelineChecklist instead. */
export function ActionSteps({
  steps,
  isStreaming,
}: {
  steps: ParsedStep[];
  isStreaming?: boolean;
}) {
  // Filter to only show steps with actual content (not empty plan steps)
  const stepsWithContent = steps.filter((s) => s.content.trim());
  if (stepsWithContent.length === 0) return null;

  return (
    <div className="flex flex-col gap-1 w-full">
      {stepsWithContent.map((step, i) => {
        const isDone = !isStreaming || i < stepsWithContent.length - 1;
        const isActive = !!isStreaming && i === stepsWithContent.length - 1;
        return (
          <ActionStepItem
            key={`${step.title}-${i}`}
            title={step.title}
            content={step.content}
            defaultOpen={isActive}
            isDone={isDone}
            isActive={isActive}
          />
        );
      })}
    </div>
  );
}

function ActionStepItem({
  title,
  content,
  defaultOpen = false,
  isDone,
  isActive,
}: {
  title: string;
  content: string;
  defaultOpen?: boolean;
  isDone: boolean;
  isActive: boolean;
}) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="group">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={cn(
          'flex items-center justify-between w-full px-3 py-2 text-left transition-all rounded-md border',
          isOpen
            ? 'bg-secondary/40 border-border/60'
            : 'bg-background/40 hover:bg-secondary/20 border-border/30 hover:border-border/50',
        )}
      >
        <div className="flex items-center gap-2.5 overflow-hidden">
          <div
            className={cn(
              'w-4 h-4 rounded-full flex items-center justify-center shrink-0 transition-colors',
              isDone
                ? 'text-emerald-500/80'
                : isActive
                  ? 'text-primary'
                  : 'text-muted-foreground/50',
            )}
          >
            {isDone ? (
              <Check size={12} strokeWidth={2.5} />
            ) : (
              <div
                className={cn(
                  'w-2 h-2 rounded-full',
                  isActive ? 'bg-primary animate-pulse' : 'bg-muted-foreground/60',
                )}
              />
            )}
          </div>

          <span
            title={title}
            className={cn(
              'text-[11px] font-medium transition-colors truncate select-none',
              isDone
                ? 'text-muted-foreground/90'
                : isActive
                  ? 'text-foreground'
                  : 'text-muted-foreground/70',
            )}
          >
            {title}
          </span>
        </div>

        <div className="flex items-center text-muted-foreground/30">
          <ChevronDown
            size={12}
            className={cn('transition-transform duration-200', isOpen ? 'rotate-180' : '')}
          />
        </div>
      </button>

      {isOpen && content && (
        <div className="px-3 py-2 mx-1 mt-0.5 border-l border-border/30 text-[10px] text-muted-foreground/80 leading-relaxed font-mono animate-in slide-in-from-top-0.5 duration-200 whitespace-pre-wrap break-words">
          {content}
        </div>
      )}
    </div>
  );
}
