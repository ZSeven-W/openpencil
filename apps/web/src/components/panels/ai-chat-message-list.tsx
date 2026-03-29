import { useRef, useEffect, useCallback } from 'react';
import { useAIStore } from '@/stores/ai-store';
import { extractAndApplyDesign } from '@/services/ai/design-generator';
import type { ChatMessage as ChatMessageType } from '@/services/ai/ai-types';
import ChatMessage from './chat-message';
import { FixedChecklist } from './ai-chat-checklist';
import { ToolCallBlock } from './tool-call-block';
import { AIChatQuickActions } from './ai-chat-quick-actions';

interface AIChatMessageListProps {
  messages: ChatMessageType[];
  isStreaming: boolean;
  onSend: (prompt: string) => void;
  quickActionsDisabled: boolean;
}

/**
 * Scrollable message list with auto-scroll, quick actions (when empty),
 * tool call blocks, and the fixed checklist overlay.
 */
export function AIChatMessageList({
  messages,
  isStreaming,
  onSend,
  quickActionsDisabled,
}: AIChatMessageListProps) {
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const toolCallBlocks = useAIStore((s) => s.toolCallBlocks);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleApplyDesign = useCallback((jsonString: string) => {
    const count = extractAndApplyDesign('```json\n' + jsonString + '\n```');
    if (count > 0) {
      useAIStore.setState((s) => {
        const msgs = [...s.messages];
        for (let i = msgs.length - 1; i >= 0; i--) {
          if (msgs[i].role === 'assistant' && msgs[i].content.includes(jsonString.slice(0, 50))) {
            if (
              !msgs[i].content.includes('\u2705') &&
              !msgs[i].content.includes('<!-- APPLIED -->')
            ) {
              msgs[i] = {
                ...msgs[i],
                content: msgs[i].content + `\n\n<!-- APPLIED -->`,
              };
            }
            break;
          }
        }
        return { messages: msgs };
      });
    }
  }, []);

  return (
    <>
      <div className="min-h-0 flex-1 overflow-y-auto rounded-b-xl bg-background/80 px-3.5 py-3">
        {messages.length === 0 ? (
          <AIChatQuickActions onSend={onSend} disabled={quickActionsDisabled} />
        ) : (
          <>
            {messages.map((msg) => (
              <ChatMessage
                key={msg.id}
                role={msg.role}
                content={msg.content}
                isStreaming={msg.isStreaming && isStreaming}
                onApplyDesign={handleApplyDesign}
                attachments={msg.attachments}
              />
            ))}
            {/* Tool call blocks (built-in provider / agent pipeline) */}
            {toolCallBlocks.length > 0 && (
              <div className="mt-1">
                {toolCallBlocks.map((block) => (
                  <ToolCallBlock key={block.id} block={block} />
                ))}
              </div>
            )}
          </>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* --- Fixed Checklist --- */}
      <FixedChecklist messages={messages} isStreaming={isStreaming} />
    </>
  );
}
