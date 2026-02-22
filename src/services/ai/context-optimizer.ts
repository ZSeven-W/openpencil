/**
 * Context optimization utilities for AI chat.
 * Prevents unbounded growth of chat history and context size.
 */

const DEFAULT_MAX_MESSAGES = 10
const DEFAULT_MAX_CHARS = 32_000

/**
 * Sliding window for chat history.
 * Keeps the most recent messages while respecting character limits.
 * Always preserves the first user message for context continuity.
 */
export function trimChatHistory<T extends { role: string; content: string }>(
  messages: T[],
  maxMessages: number = DEFAULT_MAX_MESSAGES,
  maxChars: number = DEFAULT_MAX_CHARS,
): T[] {
  if (messages.length <= maxMessages) {
    const totalChars = messages.reduce((sum, m) => sum + m.content.length, 0)
    if (totalChars <= maxChars) return messages
  }

  // Always keep the first user message for context continuity
  const firstUser = messages.find((m) => m.role === 'user')
  const recentMessages = messages.slice(-maxMessages)

  const window: T[] = []
  let charCount = 0

  // Add first user message if it's not already in the recent window
  if (firstUser && !recentMessages.includes(firstUser)) {
    window.push(firstUser)
    charCount += firstUser.content.length
  }

  // Add recent messages, respecting char limit
  for (const msg of recentMessages) {
    const msgChars = msg.content.length
    if (charCount + msgChars > maxChars) {
      // Truncate this message to fit
      const remaining = maxChars - charCount
      if (remaining > 200) {
        window.push({
          ...msg,
          content: msg.content.slice(0, remaining) + '\n[...truncated...]',
        } as T)
      }
      break
    }
    window.push(msg)
    charCount += msgChars
  }

  return window
}
