import type { SessionNotification } from '@agentclientprotocol/sdk';

/** Convert ACP session/update 通知至 OpenPencil SSE 事件字符串。 */
export function acpUpdateToSSE(notification: SessionNotification): string | null {
  const update = notification.update;
  if (!update) return null;

  switch (update.sessionUpdate) {
    case 'agent_message_chunk': {
      const content = update.content;
      if (content && 'text' in content && content.type === 'text') {
        return formatSSE('text', { type: 'text', content: content.text });
      }
      return null;
    }

    case 'tool_call': {
      // ACP 工具调用仅显示 - 代理通过 MCP 执行它们。 level: 'orchestrate' 使
      // AgentToolExecutor 跳过执行。
      return formatSSE('tool_call', {
        type: 'tool_call',
        id: update.toolCallId,
        name: update.title ?? 'unknown',
        args: update.rawInput ?? {},
        level: 'orchestrate',
      });
    }

    case 'tool_call_update': {
      if (update.status === 'completed' || update.status === 'failed') {
        // On 失败，从内容块中提取错误详细信息（当工具执行失败时，ACP 将错误文本放置在 content[].content 中）。
        let errorMsg: string | undefined;
        if (update.status === 'failed') {
          const content = (update as { content?: Array<{ content?: unknown }> }).content;
          if (Array.isArray(content)) {
            for (const block of content) {
              if (block?.content && typeof block.content === 'object' && 'text' in block.content) {
                errorMsg = (block.content as { text?: string }).text ?? errorMsg;
              }
            }
          }
          // Log 到服务器控制台进行调试
          console.error(
            `[acp] tool ${update.toolCallId} failed:`,
            errorMsg ?? JSON.stringify(update.rawOutput),
          );
        }
        return formatSSE('tool_result', {
          type: 'tool_result',
          id: update.toolCallId,
          name: '',
          result: {
            success: update.status === 'completed',
            data: update.rawOutput,
            error: errorMsg,
          },
        });
      }
      return null;
    }

    default:
      return null;
  }
}

function formatSSE(event: string, data: unknown): string {
  return `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
}
