import type { AIStreamChunk } from './ai-types'
import type { AIModelInfo } from '@/stores/ai-store'

/**
 * Streams a chat response from the server-side AI endpoint.
 * The server uses ANTHROPIC_API_KEY or local Agent SDK (no client-side key needed).
 */
export async function* streamChat(
  systemPrompt: string,
  messages: Array<{ role: 'user' | 'assistant'; content: string }>,
  model?: string,
): AsyncGenerator<AIStreamChunk> {
  try {
    const response = await fetch('/api/ai/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ system: systemPrompt, messages, model }),
    })

    if (!response.ok) {
      const errBody = await response.text()
      yield { type: 'error', content: `Server error: ${response.status} ${errBody}` }
      return
    }

    const reader = response.body?.getReader()
    if (!reader) {
      yield { type: 'error', content: 'No response stream available' }
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
            yield chunk
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
          yield JSON.parse(data) as AIStreamChunk
        } catch {
          // Skip
        }
      }
    }
  } catch (error) {
    const message =
      error instanceof Error ? error.message : 'Unknown error occurred'
    yield { type: 'error', content: message }
  }
}

/**
 * Non-streaming completion for design/code generation.
 * Calls the server-side endpoint which reads ANTHROPIC_API_KEY from env.
 */
export async function generateCompletion(
  systemPrompt: string,
  userMessage: string,
  model?: string,
): Promise<string> {
  const response = await fetch('/api/ai/generate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ system: systemPrompt, message: userMessage, model }),
  })

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

