import { streamText } from 'ai'
import type { ModelMessage } from 'ai'
import type { AgentProvider } from './providers/types'
import type { ToolRegistry } from './tools/tool-registry'
import type { ToolResult } from './tools/types'
import type { AgentEvent } from './streaming/types'
import type { ContextStrategy } from './context/types'
import { createSlidingWindowStrategy } from './context/sliding-window'

/**
 * Appended to system prompt when tools are disabled (model doesn't support function calling).
 * Tells the model to output design JSON in a code block — same approach as the CLI pipeline.
 * The client parses the JSON and inserts nodes.
 */
const TEXT_MODE_SUFFIX = `

## IMPORTANT: Text-Only Mode
Function calling is not available. Instead, output your design as a JSON code block.
Wrap the root node JSON in a \`\`\`json code fence. Example:

\`\`\`json
{
  "type": "frame", "name": "Login Screen", "x": 0, "y": 0, "width": 390, "height": 844,
  "fills": [{"type": "solid", "color": "#FFFFFF"}], "cornerRadius": 40,
  "layout": "vertical", "padding": [60, 24, 40, 24], "gap": 16, "alignItems": "stretch",
  "children": [
    {"type": "text", "text": "Welcome", "fontSize": 28, "fontWeight": 700, "fills": [{"type": "solid", "color": "#1a1a2e"}]},
    {"type": "frame", "name": "Button", "height": 48, "fills": [{"type": "solid", "color": "#4F46E5"}], "cornerRadius": 12, "justifyContent": "center", "alignItems": "center", "children": [
      {"type": "text", "text": "Sign In", "fontSize": 16, "fontWeight": 600, "fills": [{"type": "solid", "color": "#FFFFFF"}]}
    ]}
  ]
}
\`\`\`

Output ONE JSON code block with the complete design tree. Do NOT use function calls.`

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
  const status = err?.statusCode ?? err?.status

  // Match on HTTP status codes first (more reliable than regex)
  if (status === 429) {
    return {
      userMessage: 'Rate limited by the API provider. Please wait a moment and try again.',
      retryWithoutTools: false,
    }
  }
  if (status === 402 || status === 403) {
    if (/credit|funds|billing|afford/i.test(msg + body)) {
      return {
        userMessage: 'Insufficient credits for this model. Try a smaller/free model or add credits at your provider.',
        retryWithoutTools: false,
      }
    }
  }

  // Model doesn't support function calling properly
  if (
    /tool_calls.*required|function.*arguments.*required|invalid.*function.*arguments|does not support.*tool/i.test(msg + body)
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
    maxOutputTokens,
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

    try {
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

      // When tools are disabled (model doesn't support function calling),
      // switch to text-based JSON output — same approach as the CLI pipeline.
      const effectiveSystem = toolsDisabled
        ? systemPrompt + TEXT_MODE_SUFFIX
        : systemPrompt

      let response: ReturnType<typeof streamText>
      try {
        response = streamText({
          model: provider.model,
          system: effectiveSystem,
          messages: trimmedMessages as ModelMessage[],
          tools: toolsDisabled ? undefined : tools.toAISDKFormat(),
          maxOutputTokens,
          abortSignal,
        })
      } catch (err: any) {
        // Synchronous errors from streamText (rare — usually validation)
        const { userMessage } = classifyAPIError(err)
        if (!toolsDisabled && turn === 0) {
          toolsDisabled = true
          yield { type: 'error', message: `Tool calling failed: ${userMessage}. Retrying without tools.`, fatal: false }
          continue
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
                input: part.input ?? {},
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
        const { userMessage } = classifyAPIError(err)
        if (!toolsDisabled) {
          // Retry without tools — strip tool history and restart from user messages only
          toolsDisabled = true
          // Reset history to original user messages (remove tool call/result entries)
          history.length = 0
          history.push(...messages)
          turn = 0
          yield { type: 'error', message: `Tool calling failed: ${userMessage}. Retrying without tools.`, fatal: false }
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
            toolResult = { success: false, error: err instanceof Error ? err.message : JSON.stringify(err) }
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

      // Use the SDK's own response messages for conversation history.
      // This ensures correct format conversion (arguments stringification, etc.)
      // instead of manually constructing ModelMessage[] which loses format details.
      const resolved = await response.response
      // response.messages contains assistant + auto-executed tool results in a format
      // compatible with streamText(). Cast needed: ResponseMessage ≠ ModelMessage at type level.
      const responseMessages = resolved.messages as unknown as ModelMessage[]

      // The SDK includes assistant message + tool result messages (for tools with execute()).
      // For tools WITHOUT execute (client-side execution), we only get the assistant message.
      // We need to add tool result messages for those.
      history.push(...responseMessages)

      // Manually add tool results for client-executed tools (not auto-included by SDK).
      // Structure matches the AI SDK's expected tool result format.
      for (const tr of toolResults) {
        if (!tools.hasExecute(tr.name)) {
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
      }

      turn++
    }

    // Reached max turns — treat as a normal completion, not an error.
    // The model may have done useful work even if it didn't finish cleanly.
    yield { type: 'done', totalTurns: maxTurns }
    } finally {
      // Clean up pending tool calls to prevent timeout leaks
      for (const [, entry] of pending) {
        entry.reject(new Error('Agent loop ended'))
      }
      pending.clear()
    }
  }

  return { run, resolveToolResult }
}
