import type { AIStreamChunk } from './ai-types'
import type { AIModelInfo } from '@/stores/ai-store'

const DEFAULT_STREAM_HARD_TIMEOUT_MS = 180_000
const DEFAULT_STREAM_NO_TEXT_TIMEOUT_MS = 75_000

interface StreamChatOptions {
  hardTimeoutMs?: number
  noTextTimeoutMs?: number
}

/**
 * Streams a chat response from the server-side AI endpoint.
 * The server uses ANTHROPIC_API_KEY or local Agent SDK (no client-side key needed).
 */
export async function* streamChat(
  systemPrompt: string,
  messages: Array<{ role: 'user' | 'assistant'; content: string }>,
  model?: string,
  options?: StreamChatOptions,
  provider?: string,
): AsyncGenerator<AIStreamChunk> {
  const hardTimeoutMs = Math.max(10_000, options?.hardTimeoutMs ?? DEFAULT_STREAM_HARD_TIMEOUT_MS)
  const noTextTimeoutMs = Math.max(10_000, options?.noTextTimeoutMs ?? DEFAULT_STREAM_NO_TEXT_TIMEOUT_MS)

  const controller = new AbortController()
  let abortReason: 'hard_timeout' | 'no_text_timeout' | null = null
  let noTextTimeout: ReturnType<typeof setTimeout> | null = null

  const clearNoTextTimeout = () => {
    if (noTextTimeout) {
      clearTimeout(noTextTimeout)
      noTextTimeout = null
    }
  }

  const resetActivityTimeout = () => {
    clearNoTextTimeout()
    noTextTimeout = setTimeout(() => {
      abortReason = 'no_text_timeout'
      controller.abort()
    }, noTextTimeoutMs)
  }

  const hardTimeout = setTimeout(() => {
    abortReason = 'hard_timeout'
    controller.abort()
  }, hardTimeoutMs)

  resetActivityTimeout()

  try {
    const response = await fetch('/api/ai/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ system: systemPrompt, messages, model, provider }),
      signal: controller.signal,
    })

    if (!response.ok) {
      const errBody = await response.text()
      yield { type: 'error', content: `Server error: ${response.status} ${errBody}` }
      clearTimeout(hardTimeout)
      clearNoTextTimeout()
      return
    }

    const reader = response.body?.getReader()
    if (!reader) {
      yield { type: 'error', content: 'No response stream available' }
      clearTimeout(hardTimeout)
      clearNoTextTimeout()
      return
    }

    const decoder = new TextDecoder()
    let buffer = ''

    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      buffer += decoder.decode(value, { stream: true })

      // Parse SSE events from the buffer
      const lines = buffer.split('\n')
      buffer = lines.pop() ?? ''

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6).trim()
          if (!data) continue
          try {
            const chunk = JSON.parse(data) as AIStreamChunk
            if (chunk.type === 'done') {
              clearTimeout(hardTimeout)
              clearNoTextTimeout()
              try {
                await reader.cancel()
              } catch {
                // ignore cancellation errors
              }
              return
            }

            // Keep-alive pings from server — reset activity timeout but don't yield
            if (chunk.type === 'ping') {
              resetActivityTimeout()
              continue
            }

            if (chunk.type === 'thinking' && !chunk.content) {
              continue
            }

            // Any non-empty content (text or thinking) counts as activity
            if ((chunk.type === 'text' || chunk.type === 'thinking') && chunk.content.trim().length > 0) {
              resetActivityTimeout()
            }

            yield chunk
            if (chunk.type === 'error') {
              clearTimeout(hardTimeout)
              clearNoTextTimeout()
              try {
                await reader.cancel()
              } catch {
                // ignore cancellation errors
              }
              return
            }
          } catch {
            // Skip malformed lines
          }
        }
      }
    }

    // Process remaining buffer
    if (buffer.startsWith('data: ')) {
      const data = buffer.slice(6).trim()
      if (data) {
        try {
          const chunk = JSON.parse(data) as AIStreamChunk
          if (chunk.type === 'done') {
            clearTimeout(hardTimeout)
            clearNoTextTimeout()
            return
          }
          if (chunk.type === 'thinking' && !chunk.content) {
            clearTimeout(hardTimeout)
            clearNoTextTimeout()
            return
          }
          clearTimeout(hardTimeout)
          clearNoTextTimeout()
          yield chunk
          if (chunk.type === 'error') {
            return
          }
        } catch {
          // Skip
        }
      }
    }
  } catch (error) {
    if (controller.signal.aborted) {
      if (abortReason === 'no_text_timeout') {
        yield {
          type: 'error',
          content: 'AI has been thinking too long without output. Request stopped, please retry.',
        }
      } else if (abortReason === 'hard_timeout') {
        yield {
          type: 'error',
          content: 'AI request timed out. Please retry.',
        }
      } else {
        yield {
          type: 'error',
          content: 'AI request was aborted.',
        }
      }
      clearTimeout(hardTimeout)
      clearNoTextTimeout()
      return
    }

    const message =
      error instanceof Error ? error.message : 'Unknown error occurred'
    yield { type: 'error', content: message }
  } finally {
    clearTimeout(hardTimeout)
    clearNoTextTimeout()
  }
}

/**
 * Non-streaming completion for design/code generation.
 * Calls the server-side endpoint which reads ANTHROPIC_API_KEY from env.
 */
const DEFAULT_GENERATE_TIMEOUT_MS = 180_000

export async function generateCompletion(
  systemPrompt: string,
  userMessage: string,
  model?: string,
  provider?: string,
): Promise<string> {
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), DEFAULT_GENERATE_TIMEOUT_MS)

  let response: Response
  try {
    response = await fetch('/api/ai/generate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ system: systemPrompt, message: userMessage, model, provider }),
      signal: controller.signal,
    })
  } catch (error) {
    clearTimeout(timeout)
    if (controller.signal.aborted) {
      throw new Error('AI generation request timed out. Please retry.')
    }
    throw error
  } finally {
    clearTimeout(timeout)
  }

  if (!response.ok) {
    throw new Error(`Server error: ${response.status}`)
  }

  const data = await response.json()
  if (data.error) {
    throw new Error(data.error)
  }
  return data.text ?? ''
}

/**
 * Fetches available AI models from the server.
 * The server queries Claude Agent SDK for the supported model list.
 */
export async function fetchAvailableModels(): Promise<AIModelInfo[]> {
  try {
    const response = await fetch('/api/ai/models')
    if (!response.ok) return []
    const data = await response.json()
    return data.models ?? []
  } catch {
    return []
  }
}
