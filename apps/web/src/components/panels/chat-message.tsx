import { useMemo } from 'react';
import { cn } from '@/lib/utils';
import type { ChatAttachment } from '@/services/ai/ai-types';
import { parseMarkdown } from './chat-message-content';
import { parseStepBlocks, stripStepBlocks, ActionSteps } from './chat-message-tool-call';
import { ChatMessageAttachments } from './chat-message-attachment';

// Re-export types and utilities used by other modules (ai-chat-checklist.tsx)
export type { ParsedStep, PipelineItem } from './chat-message-tool-call';
export {
  parseStepBlocks,
  countDesignJsonBlocks,
  buildPipelineProgress,
} from './chat-message-tool-call';

interface ChatMessageProps {
  role: 'user' | 'assistant';
  content: string;
  isStreaming?: boolean;
  onApplyDesign?: (json: string) => void;
  attachments?: ChatAttachment[];
}

/** Strip raw tool-call / function-call XML that should never be shown to users */
function stripToolCallXml(text: string): string {
  let cleaned = text;

  // Remove <function_calls> blocks
  cleaned = cleaned.replace(/<function_calls>[\s\S]*?<\/function_calls>/g, '');

  // Remove <result> blocks (often tool outputs)
  cleaned = cleaned.replace(/<result>[\s\S]*?<\/result>/g, '');

  // Remove <inference_process> or similar internal blocks if they appear
  cleaned = cleaned.replace(/<inference_process>[\s\S]*?<\/inference_process>/g, '');

  // Remove <invoke> blocks (tool usage) - handle both closed and streaming/unclosed
  cleaned = cleaned.replace(/<invoke[\s\S]*?<\/invoke>/g, '');
  cleaned = cleaned.replace(/<invoke[\s\S]*?$/g, ''); // Hide unclosed invoke at end of stream

  // Remove <parameter> blocks if they appear outside invoke for some reason
  cleaned = cleaned.replace(/<parameter[\s\S]*?<\/parameter>/g, '');

  // Remove stray tags
  cleaned = cleaned.replace(/<\/?invoke.*?>/g, '');
  cleaned = cleaned.replace(/<\/?parameter.*?>/g, '');
  cleaned = cleaned.replace(/<\/?function_calls>/g, '');
  cleaned = cleaned.replace(/<\/?search_quality_reflection>/g, ''); // Sometimes this appears too
  cleaned = cleaned.replace(/<\/?thought_process>/g, ''); // And this

  // Remove the hidden marker so it doesn't show up in UI even as whitespace
  cleaned = cleaned.replace(/<!-- APPLIED -->/g, '');

  // Collapse leftover blank lines into at most one
  cleaned = cleaned.replace(/\n{3,}/g, '\n\n');
  return cleaned.trim();
}

export default function ChatMessage({
  role,
  content,
  isStreaming,
  onApplyDesign,
  attachments,
}: ChatMessageProps) {
  const isApplied = useMemo(
    () =>
      role === 'assistant' && (content.includes('\u2705') || content.includes('<!-- APPLIED -->')),
    [role, content],
  );

  const isUser = role === 'user';
  // Strip raw tool-call XML that the model may emit (should never be visible)
  const displayContent = isUser ? content : stripToolCallXml(content);
  const steps = useMemo(
    () => (isUser ? [] : parseStepBlocks(displayContent, isStreaming)),
    [isUser, displayContent, isStreaming],
  );
  const hasFlow = !isUser && steps.length > 0;
  const contentWithoutSteps = useMemo(
    () => (isUser ? displayContent : stripStepBlocks(displayContent)),
    [isUser, displayContent],
  );
  const isEmpty = !contentWithoutSteps.trim() && !hasFlow;

  // Don't render an empty non-streaming assistant message
  const hadContent = content.trim().length > 0;
  if (!isUser && isEmpty && !isStreaming) {
    if (hadContent) {
      return (
        <div className="text-xs text-muted-foreground italic px-2 py-1">
          (Automated action completed)
        </div>
      );
    }
    return null;
  }

  return (
    <div className={cn('flex', isUser ? 'justify-end' : 'justify-start mt-2')}>
      {isUser ? (
        <div className="max-w-[85%] rounded-lg px-3 py-2 text-sm leading-relaxed whitespace-pre-wrap bg-primary text-primary-foreground rounded-br-sm">
          {attachments && attachments.length > 0 && (
            <ChatMessageAttachments attachments={attachments} />
          )}
          {content}
        </div>
      ) : (
        <div className="text-sm leading-relaxed text-foreground min-w-0 w-full overflow-hidden">
          {/* Streaming with no content yet -> thinking indicator */}
          {isEmpty && isStreaming ? (
            <div className="flex items-center gap-1.5 bg-secondary/50 rounded-full w-fit py-1 px-2.5 mt-2">
              <span className="text-xs text-muted-foreground">Thinking</span>
              <span className="flex gap-0.5">
                <span
                  className="w-1 h-1 rounded-full bg-muted-foreground/70 animate-bounce"
                  style={{ animationDelay: '0ms' }}
                />
                <span
                  className="w-1 h-1 rounded-full bg-muted-foreground/70 animate-bounce"
                  style={{ animationDelay: '150ms' }}
                />
                <span
                  className="w-1 h-1 rounded-full bg-muted-foreground/70 animate-bounce"
                  style={{ animationDelay: '300ms' }}
                />
              </span>
            </div>
          ) : (
            <>
              {hasFlow && (
                <div className="mb-2">
                  <ActionSteps steps={steps} isStreaming={isStreaming} />
                </div>
              )}
              {contentWithoutSteps.trim() ? (
                <div className="whitespace-pre-wrap">
                  {parseMarkdown(
                    contentWithoutSteps,
                    onApplyDesign,
                    isApplied,
                    isStreaming && !!contentWithoutSteps.trim(),
                  )}
                </div>
              ) : null}
            </>
          )}
        </div>
      )}
    </div>
  );
}
