/**
 * Context 用于
 * AI 聊天的优化实用程序。 Prevents 聊天历史和上下文大小无限增长。
 */

export const DEFAULT_MAX_MESSAGES = 10;
export const DEFAULT_MAX_CHARS = 32_000;
/**
 * Leave 低于上游
 * 1,048,576 个字符限制的安全裕度。 This 是一个本地护栏，而不是精确的上游 token/character 计算器。
 */
export const MAX_CHAT_REQUEST_CHARS = 900_000;

type MessageWithOptionalAttachments = {
  role: string;
  content: string;
  attachments?: unknown[];
};

function stripHistoricalAttachments<T extends MessageWithOptionalAttachments>(
  message: T,
  keepAttachments: boolean,
): T {
  if (keepAttachments || !Array.isArray(message.attachments) || message.attachments.length === 0) {
    return message;
  }

  const cloned = { ...message } as T & { attachments?: unknown[] };
  delete cloned.attachments;
  return cloned;
}

/**
 * Estimate
 *
 * 聊天请求负载的字符数。 Notes：- This 是本地预检检查的近似值，而不是上游计费或真实模型上下文长度。
 * - The 的目标是尽早
 * 拒绝明显过大的请求并出现
 * 可操作的错误。
 */
export function estimateChatPayloadChars(payload: unknown): number {
  try {
    return JSON.stringify(payload).length;
  } catch {
    return Number.MAX_SAFE_INTEGER;
  }
}

export function formatChatPayloadTooLargeError(
  payloadChars: number,
  limit: number = MAX_CHAT_REQUEST_CHARS,
): string {
  return [
    `AI input is too large (${payloadChars.toLocaleString()} chars > ${limit.toLocaleString()} safe local limit).`,
    'Please remove older image attachments or large pasted content, start a new chat, or simplify the current selection and retry.',
  ].join(' ');
}

/**
 * Sliding window for chat history.
 * Keeps the most recent messages while respecting character limits.
 * Always preserves the first user message for context continuity.
 */
export function trimChatHistory<T extends MessageWithOptionalAttachments>(
  messages: T[],
  maxMessages: number = DEFAULT_MAX_MESSAGES,
  maxChars: number = DEFAULT_MAX_CHARS,
): T[] {
  if (messages.length <= maxMessages) {
    const totalChars = messages.reduce((sum, m) => sum + m.content.length, 0);
    if (totalChars <= maxChars) {
      const latestAttachmentMessage = [...messages]
        .reverse()
        .find((m) => Array.isArray(m.attachments) && m.attachments.length > 0);
      return messages.map((m) => stripHistoricalAttachments(m, m === latestAttachmentMessage));
    }
  }

  // Always keep the first user message for context continuity
  const firstUser = messages.find((m) => m.role === 'user');
  const recentMessages = messages.slice(-maxMessages);
  const latestAttachmentMessage = [...messages]
    .reverse()
    .find((m) => Array.isArray(m.attachments) && m.attachments.length > 0);

  const window: T[] = [];
  let charCount = 0;

  // Add first user message if it's not already in the recent window
  if (firstUser && !recentMessages.includes(firstUser)) {
    const sanitizedFirstUser = stripHistoricalAttachments(
      firstUser,
      firstUser === latestAttachmentMessage,
    );
    window.push(sanitizedFirstUser);
    charCount += sanitizedFirstUser.content.length;
  }

  // Add recent messages, respecting char limit
  for (const msg of recentMessages) {
    const sanitizedMessage = stripHistoricalAttachments(msg, msg === latestAttachmentMessage);
    const msgChars = sanitizedMessage.content.length;
    if (charCount + msgChars > maxChars) {
      // Truncate this message to fit
      const remaining = maxChars - charCount;
      if (remaining > 200) {
        window.push({
          ...sanitizedMessage,
          content: sanitizedMessage.content.slice(0, remaining) + '\n[...truncated...]',
        } as T);
      }
      break;
    }
    window.push(sanitizedMessage);
    charCount += msgChars;
  }

  return window;
}
