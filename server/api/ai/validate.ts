import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { resolveClaudeCli } from '../../utils/resolve-claude-cli'
import {
  buildClaudeAgentEnv,
  getClaudeAgentDebugFilePath,
} from '../../utils/resolve-claude-agent-env'
import { writeFile, unlink, mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

interface ValidateBody {
  system: string
  message: string
  imageBase64: string
  model?: string
  provider?: string
}

function shouldRetryClaudeWithoutModel(raw: string): boolean {
  return /process exited with code 1|invalid model|unknown model|model.*not/i.test(raw)
}

/**
 * Vision-based validation endpoint.
 * Accepts a base64 PNG screenshot and a text prompt, sends multimodal
 * content blocks for analysis.
 *
 * - Anthropic API key: uses SDK multimodal content blocks directly.
 * - Agent SDK fallback: saves screenshot to temp file, asks Claude Code
 *   to read it via its built-in Read tool.
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<ValidateBody>(event)

  if (!body?.system || !body?.message || !body?.imageBase64) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' })
    return { error: 'Missing required fields: system, message, imageBase64' }
  }

  // Try Anthropic SDK first (direct multimodal support)
  const apiKey = process.env.ANTHROPIC_API_KEY
  if (apiKey) {
    try {
      return await validateViaAnthropicSDK(apiKey, body, body.model)
    } catch {
      // Fall through to Agent SDK
    }
  }

  // Fallback: Agent SDK — save screenshot to temp file, let Claude read it
  try {
    return await validateViaAgentSDK(body, body.model)
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error'
    return { error: message }
  }
})

async function validateViaAnthropicSDK(
  apiKey: string,
  body: ValidateBody,
  model?: string,
): Promise<{ text: string; skipped?: boolean }> {
  const { default: Anthropic } = await import('@anthropic-ai/sdk')
  const client = new Anthropic({ apiKey })

  // Strip data URL prefix if present
  let base64Data = body.imageBase64
  const dataUrlPrefix = 'data:image/png;base64,'
  if (base64Data.startsWith(dataUrlPrefix)) {
    base64Data = base64Data.slice(dataUrlPrefix.length)
  }

  const response = await client.messages.create({
    model: model || 'claude-sonnet-4-5-20250929',
    max_tokens: 4096,
    system: body.system,
    messages: [
      {
        role: 'user',
        content: [
          {
            type: 'image',
            source: { type: 'base64', media_type: 'image/png', data: base64Data },
          },
          {
            type: 'text',
            text: body.message,
          },
        ],
      },
    ],
  })

  const textBlock = response.content.find((b: { type: string }) => b.type === 'text')
  return { text: textBlock && 'text' in textBlock ? textBlock.text : '' }
}

/**
 * Agent SDK fallback: save screenshot to a temp PNG file, then ask Claude
 * Code to read it (Claude Code's Read tool supports images natively).
 */
async function validateViaAgentSDK(
  body: ValidateBody,
  model?: string,
): Promise<{ text: string; skipped?: boolean; error?: string }> {
  // Save base64 image to temp file
  let base64Data = body.imageBase64
  const dataUrlPrefix = 'data:image/png;base64,'
  if (base64Data.startsWith(dataUrlPrefix)) {
    base64Data = base64Data.slice(dataUrlPrefix.length)
  }

  const tempDir = await mkdtemp(join(tmpdir(), 'openpencil-validate-'))
  const tempPath = join(tempDir, 'screenshot.png')

  try {
    await writeFile(tempPath, Buffer.from(base64Data, 'base64'))

    const { query } = await import('@anthropic-ai/claude-agent-sdk')

    const env = buildClaudeAgentEnv()
    const debugFile = getClaudeAgentDebugFilePath()

    const claudePath = resolveClaudeCli()

    // Prompt Claude Code to read the temp image and analyze it
    const prompt = `Read the image file at "${tempPath}" and analyze it as a UI design screenshot.

${body.message}

${body.system}

Output ONLY the JSON object, no markdown fences, no explanation.`

    const runQuery = async (modelOverride?: string): Promise<{ text: string; skipped?: boolean; error?: string }> => {
      const q = query({
        prompt,
        options: {
          ...(modelOverride ? { model: modelOverride } : {}),
          maxTurns: 2,
          tools: [],
          plugins: [],
          permissionMode: 'plan',
          persistSession: false,
          env,
          ...(debugFile ? { debugFile } : {}),
          ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
        },
      })

      try {
        for await (const message of q) {
          if (message.type === 'result') {
            const isErrorResult = 'is_error' in message && Boolean((message as { is_error?: boolean }).is_error)
            if (message.subtype === 'success' && !isErrorResult) {
              return { text: message.result }
            }
            const errors = 'errors' in message ? (message.errors as string[]) : []
            const resultText = 'result' in message ? String(message.result ?? '') : ''
            return { error: errors.join('; ') || resultText || `Query ended with: ${message.subtype}`, text: '' }
          }
        }
      } finally {
        q.close()
      }

      return { text: '', skipped: true }
    }

    try {
      const first = await runQuery(model)
      if (model && first.error && shouldRetryClaudeWithoutModel(first.error)) {
        return await runQuery(undefined)
      }
      return first
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      if (model && shouldRetryClaudeWithoutModel(message)) {
        try {
          return await runQuery(undefined)
        } catch (retryError) {
          const retryMessage = retryError instanceof Error ? retryError.message : String(retryError)
          return { error: retryMessage, text: '' }
        }
      }
      return { error: message, text: '' }
    }
  } finally {
    // Clean up temp file
    try {
      await unlink(tempPath)
    } catch { /* ignore */ }
  }
}
