import { defineEventHandler } from 'h3'
import {
  buildClaudeAgentEnv,
  getClaudeAgentDebugFilePath,
} from '../../utils/resolve-claude-agent-env'

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

    const env = buildClaudeAgentEnv()
    const debugFile = getClaudeAgentDebugFilePath()

    const q = query({
      prompt: '',
      options: {
        maxTurns: 1,
        tools: [],
        permissionMode: 'plan',
        persistSession: false,
        env,
        ...(debugFile ? { debugFile } : {}),
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
