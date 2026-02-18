import { defineEventHandler } from 'h3'

interface ModelInfo {
  value: string
  displayName: string
  description: string
}

let cachedModels: ModelInfo[] | null = null

/**
 * Returns the list of available AI models via Claude Agent SDK.
 * Used as a fallback when no providers are explicitly connected.
 */
export default defineEventHandler(async () => {
  if (cachedModels) {
    return { models: cachedModels }
  }

  try {
    const { query } = await import('@anthropic-ai/claude-agent-sdk')

    const env = { ...process.env } as Record<string, string | undefined>
    delete env.CLAUDECODE

    const q = query({
      prompt: '',
      options: {
        model: 'claude-sonnet-4-6',
        maxTurns: 1,
        tools: [],
        permissionMode: 'plan',
        persistSession: false,
        env,
      },
    })

    const models = await q.supportedModels()
    cachedModels = models
    q.close()

    return { models }
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error'
    return { models: [], error: message }
  }
})
