import { streamText } from 'ai'
import type { ModelMessage } from 'ai'
import type { AgentProvider } from './providers/types'
import type { ToolRegistry } from './tools/tool-registry'
import type { ToolResult } from './tools/types'
import type { AgentEvent } from './streaming/types'
import type { ContextStrategy } from './context/types'
import { createSlidingWindowStrategy } from './context/sliding-window'

export interface AgentConfig {
  provider: AgentProvider
  tools: ToolRegistry
  systemPrompt: string
  maxTurns?: number
  maxOutputTokens?: number
  turnTimeout?: number
  contextStrategy?: ContextStrategy
  abortSignal?: AbortSignal
}

export interface Agent {
  run(messages: ModelMessage[]): AsyncGenerator<AgentEvent>
  resolveToolResult(toolCallId: string, result: ToolResult): void
}

/** Classify API errors into actionable categories for the user. */
function classifyAPIError(err: any): { userMessage: string; retryWithoutTools: boolean } {
  const msg = err?.message ?? String(err)
  const body = err?.responseBody ?? ''

  // Model doesn't support function calling
  if (
    /tool_calls.*required|function.*arguments.*required|does not support.*tool/i.test(msg + body)
  ) {
    return {
      userMessage: 'This model does not support function calling properly. The agent will try without tools.',
      retryWithoutTools: true,
    }
  }

  // OpenRouter privacy/guardrail restrictions
  if (/No endpoints available.*guardrail|data policy/i.test(msg + body)) {
    return {
      userMessage: 'This model is blocked by your OpenRouter privacy settings. Visit https://openrouter.ai/settings/privacy to configure, or switch to a different model.',
      retryWithoutTools: false,
    }
  }

  // Credit/billing issues
  if (/more credits|can only afford|insufficient.*funds|billing/i.test(msg + body)) {
    return {
      userMessage: 'Insufficient credits for this model. Try a smaller/free model or add credits at your provider.',
      retryWithoutTools: false,
    }
  }

  // Rate limiting
  if (/rate.limit|too many requests|429/i.test(msg + body)) {
    return {
      userMessage: 'Rate limited by the API provider. Please wait a moment and try again.',
      retryWithoutTools: false,
    }
  }

  // Generic 400 — often means the model can't handle our request format
  if (err?.statusCode === 400 && /provider returned error/i.test(msg + body)) {
    return {
      userMessage: 'The model returned an error. It may not support tool calling. Retrying without tools.',
      retryWithoutTools: true,
    }
  }

  // Fallback
  return { userMessage: msg, retryWithoutTools: false }
}

export function createAgent(config: AgentConfig): Agent {
  const {
    provider,
    tools,
    systemPrompt,
    maxTurns = 20,
    maxOutputTokens = 4096,
    turnTimeout = 60_000,
    contextStrategy = createSlidingWindowStrategy({ maxTurns: 50 }),
    abortSignal,
  } = config

  // Pending tool call resolution map — for tools without execute()
  const pending = new Map<string, {
    resolve: (result: ToolResult) => void
    reject: (error: Error) => void
  }>()

  function resolveToolResult(toolCallId: string, result: ToolResult): void {
    const entry = pending.get(toolCallId)
    if (!entry) throw new Error(`No pending tool call: ${toolCallId}`)
    pending.delete(toolCallId)
    entry.resolve(result)
  }

  function waitForToolResult(toolCallId: string): Promise<ToolResult> {
    return new Promise<ToolResult>((resolve, reject) => {
      const timeout = setTimeout(
        () => {
          pending.delete(toolCallId)
          reject(new Error(`Tool call ${toolCallId} timed out after ${turnTimeout}ms`))
        },
        turnTimeout,
      )
      pending.set(toolCallId, {
        resolve: (result) => { clearTimeout(timeout); resolve(result) },
        reject: (error) => { clearTimeout(timeout); reject(error) },
      })
    })
  }

  async function* run(messages: ModelMessage[]): AsyncGenerator<AgentEvent> {
    let turn = 0
    const history = [...messages]
    let toolsDisabled = false

    while (turn < maxTurns) {
      if (abortSignal?.aborted) {
        yield { type: 'abort' }
        return
      }

      yield { type: 'turn', turn, maxTurns }

      // Apply context strategy before each LLM call
      const trimmedMessages = contextStrategy.trim(
        history,
        provider.maxContextTokens,
      )

      let response: ReturnType<typeof streamText>
      try {
        response = streamText({
          model: provider.model,
          system: systemPrompt,
          messages: trimmedMessages as ModelMessage[],
          tools: toolsDisabled ? undefined : tools.toAISDKFormat(),
          maxOutputTokens,
          abortSignal,
        })
      } catch (err: any) {
        // Synchronous errors from streamText (rare — usually validation)
        const { userMessage, retryWithoutTools } = classifyAPIError(err)
        if (retryWithoutTools && !toolsDisabled) {
          toolsDisabled = true
          yield { type: 'error', message: userMessage, fatal: false }
          continue // retry this turn without tools
        }
        yield { type: 'error', message: userMessage, fatal: true }
        return
      }

      // Collect tool calls emitted during this turn
      const pendingToolCalls: Array<{
        toolCallId: string
        toolName: string
        input: unknown
      }> = []

      let accumulatedText = ''
      let streamError: any = null

      try {
        for await (const part of response.fullStream) {
          switch (part.type) {
            case 'text-delta':
              if (part.text) {
                accumulatedText += part.text
                yield { type: 'text', content: part.text }
              }
              break

            case 'tool-call':
              pendingToolCalls.push({
                toolCallId: part.toolCallId,
                toolName: part.toolName,
                input: part.input,
              })
              break

            case 'reasoning-delta':
              if (part.text) {
                yield { type: 'thinking', content: part.text }
              }
              break

            case 'error':
              streamError = (part as any).error
              break
          }
        }
      } catch (err: any) {
        // API errors during streaming (tool_calls format issues, provider errors, etc.)
        const { userMessage, retryWithoutTools } = classifyAPIError(err)
        if (retryWithoutTools && !toolsDisabled && turn === 0) {
          // Only auto-retry on the first turn — later turns have tool history
          toolsDisabled = true
          yield { type: 'error', message: userMessage, fatal: false }
          continue
        }
        yield { type: 'error', message: userMessage, fatal: true }
        return
      }

      // Handle stream-level errors
      if (streamError) {
        yield { type: 'error', message: String(streamError), fatal: false }
      }

      // No tool calls means the model is done
      if (!pendingToolCalls.length) {
        yield { type: 'done', totalTurns: turn + 1 }
        return
      }

      // Process each tool call: either execute directly or suspend for external resolution
      const toolResults: Array<{ id: string; name: string; result: ToolResult }> = []

      for (const toolCall of pendingToolCalls) {
        const level = tools.getLevel(toolCall.toolName) ?? 'read'

        yield {
          type: 'tool_call',
          id: toolCall.toolCallId,
          name: toolCall.toolName,
          args: toolCall.input,
          level,
        }

        let toolResult: ToolResult

        if (tools.hasExecute(toolCall.toolName)) {
          try {
            const tool = tools.get(toolCall.toolName)!
            const data = await tool.execute!(toolCall.input)
            toolResult = { success: true, data }
          } catch (err) {
            toolResult = { success: false, error: String(err) }
          }
        } else {
          toolResult = await waitForToolResult(toolCall.toolCallId)
        }

        toolResults.push({
          id: toolCall.toolCallId,
          name: toolCall.toolName,
          result: toolResult,
        })

        yield {
          type: 'tool_result',
          id: toolCall.toolCallId,
          name: toolCall.toolName,
          result: toolResult,
        }
      }

      // Manually construct ModelMessage[] for conversation history.
      // Include BOTH text and tool-call parts so the model retains context.
      const assistantParts: any[] = []
      if (accumulatedText) {
        assistantParts.push({ type: 'text' as const, text: accumulatedText })
      }
      for (const tc of pendingToolCalls) {
        assistantParts.push({
          type: 'tool-call' as const,
          toolCallId: tc.toolCallId,
          toolName: tc.toolName,
          args: tc.input,
        })
      }
      history.push({
        role: 'assistant' as const,
        content: assistantParts,
      } as unknown as ModelMessage)

      for (const tr of toolResults) {
        history.push({
          role: 'tool' as const,
          content: [
            {
              type: 'tool-result' as const,
              toolCallId: tr.id,
              toolName: tr.name,
              output: {
                type: 'text' as const,
                value: JSON.stringify(tr.result.data ?? tr.result.error ?? ''),
              },
            },
          ],
        } as unknown as ModelMessage)
      }

      turn++
    }

    yield { type: 'error', message: `Max turns (${maxTurns}) reached`, fatal: false }
  }

  return { run, resolveToolResult }
}
