import { useMemo } from 'react';
import { cn } from '@/lib/utils';
import type { ChatAttachment } from '@/services/ai/ai-types';
import { parseMarkdown } from './chat-message-content';
import { parseStepBlocks, stripStepBlocks, ActionSteps } from './chat-message-tool-call';
import { ChatMessageAttachments } from './chat-message-attachment';

// Re-导出其他模块使用的类型和实用程序 (ai-chat-checklist.tsx)
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

/** Strip 原始工具调用/函数调用 XML 永远不应该向用户显示 */
function stripToolCallXml(text: string): string {
  let cleaned = text;

  // Remove <function_calls> 块
  cleaned = cleaned.replace(/<function_calls>[\s\S]*?<\/function_calls>/g, '');

  // Remove <结果> 块（通常是工具输出）
  cleaned = cleaned.replace(/<result>[\s\S]*?<\/result>/g, '');

  // Remove <inference_process> 或类似的内部块（如果出现）
  cleaned = cleaned.replace(/<inference_process>[\s\S]*?<\/inference_process>/g, '');

  // Remove <invoke> 块（工具使用） - 处理关闭和 streaming/unclosed
  cleaned = cleaned.replace(/<invoke[\s\S]*?<\/invoke>/g, '');
  cleaned = cleaned.replace(/<invoke[\s\S]*?$/g, ''); // Hide 在流末尾未关闭调用

  // Remove <parameter> 如果由于某种原因出现在调用之外，则会阻塞
  cleaned = cleaned.replace(/<parameter[\s\S]*?<\/parameter>/g, '');

  // Remove 流浪标签
  cleaned = cleaned.replace(/<\/?invoke.*?>/g, '');
  cleaned = cleaned.replace(/<\/?parameter.*?>/g, '');
  cleaned = cleaned.replace(/<\/?function_calls>/g, '');
  cleaned = cleaned.replace(/<\/?search_quality_reflection>/g, ''); // Sometimes 这也出现了
  cleaned = cleaned.replace(/<\/?thought_process>/g, ''); // And 这个

  // Remove 隐藏标记，因此它不会显示在 UI 中，即使是空白
  cleaned = cleaned.replace(/<!-- APPLIED -->/g, '');

  // Collapse 将剩余空白行最多插入一个
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
  // Strip 模型可能发出的原始工具调用 XML （永远不应该可见）
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

  // Don 不会呈现空的非流式助理消息
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
          {/* Streaming 还没有内容 -> 思考指示器 */}
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
