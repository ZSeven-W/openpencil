import { useState } from 'react';
import { Check, ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';
import { isDesignJson } from './chat-message-content';

export interface ParsedStep {
  title: string;
  content: string;
  /** Explicit 协调器步骤的状态（对于正常步骤未定义） */
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

/** Count 完成了 JSONL 内容中的部分（根框架的直接子级）。 */
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

    // JSONL 格式：将根的直接子节点计为节
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
  /** Optional 详细信息行（例如验证日志） */
  details?: string[];
}

export function buildPipelineProgress(
  steps: ParsedStep[],
  jsonBlockCount: number,
  isStreaming: boolean,
  isApplied: boolean,
  hasError: boolean,
): PipelineItem[] {
  // No 步骤 = 无清单
  if (steps.length === 0) return [];

  // Parse 步骤内容中的详细信息行（每个条目一行）
  function extractDetails(content: string): string[] | undefined {
    if (!content) return undefined;
    const lines = content
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean);
    return lines.length > 0 ? lines : undefined;
  }

  // If 步骤具有明确的状态（协调器模式），可以直接使用它。 Check 此 BEFORE
  // 终端结果逻辑，以便用户停止的生成保留实际的每步状态，而不是将所有操作标记为已完成。
  const hasExplicitStatus = steps.some((s) => s.status !== undefined);
  if (hasExplicitStatus) {
    return steps.map((s) => ({
      label: s.title,
      done: s.status === 'done',
      active: isStreaming && s.status === 'streaming',
      details: extractDetails(s.content),
    }));
  }

  // If 生成已完成并应用，标记所有步骤已完成
  const hasTerminalResult = !isStreaming && !hasError && (isApplied || jsonBlockCount > 0);
  if (hasTerminalResult) {
    return steps.map((s) => ({
      label: s.title,
      done: true,
      active: false,
      details: extractDetails(s.content),
    }));
  }

  // Fallback：Map 到 done/active/pending 的每一步都基于已完成的 JSON 块。当 jsonBlockCount > i
  // 时，Step[i] 完成。 jsonBlockCount 处的 The
  // 步骤处于活动状态（当前正在生成）。
  return steps.map((s, index) => {
    const done = index < jsonBlockCount;
    const active = isStreaming && !done && index === jsonBlockCount;
    return { label: s.title, done, active, details: extractDetails(s.content) };
  });
}

/** Component 用于将操作步骤列表呈现为手风琴。 Only 显示非空内容的步骤（例如思考、分析）。 Empty
 * 计划步骤显示在 PipelineChecklist 中。
 *  */
export function ActionSteps({
  steps,
  isStreaming,
}: {
  steps: ParsedStep[];
  isStreaming?: boolean;
}) {
  // Filter 仅显示具有实际内容的步骤（不是空的计划步骤）
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
