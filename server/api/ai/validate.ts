import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { resolveClaudeCli } from '../../utils/resolve-claude-cli'
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

    const env = { ...process.env } as Record<string, string | undefined>
    delete env.CLAUDECODE

    const claudePath = resolveClaudeCli()

    // Prompt Claude Code to read the temp image and analyze it
    const prompt = `Read the image file at "${tempPath}" and analyze it as a UI design screenshot.

${body.message}

${body.system}

Output ONLY the JSON object, no markdown fences, no explanation.`

    const q = query({
      prompt,
      options: {
        model: model || 'claude-sonnet-4-6',
        maxTurns: 2,
        tools: [],
        plugins: [],
        permissionMode: 'plan',
        persistSession: false,
        env,
        ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
      },
    })

    for await (const message of q) {
      if (message.type === 'result') {
        if (message.subtype === 'success') {
          return { text: message.result }
        }
        const errors = 'errors' in message ? (message.errors as string[]) : []
        return { error: errors.join('; ') || `Query ended with: ${message.subtype}`, text: '' }
      }
    }

    return { text: '', skipped: true }
  } finally {
    // Clean up temp file
    try {
      await unlink(tempPath)
    } catch { /* ignore */ }
  }
}
