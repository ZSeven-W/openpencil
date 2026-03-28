import type { z } from 'zod'
import type { FlexibleSchema } from 'ai'

export type AuthLevel = 'read' | 'create' | 'modify' | 'delete' | 'orchestrate'

export interface AgentTool<TArgs = any, TResult = any> {
  name: string
  description: string
  /** Zod schema or AI SDK jsonSchema() — used as inputSchema for the LLM. */
  schema: z.ZodType<TArgs> | FlexibleSchema<TArgs>
  level: AuthLevel
  execute?: (args: TArgs) => Promise<TResult>
}

export interface ToolResult {
  success: boolean
  data?: unknown
  error?: string
}

export interface ToolCallInfo {
  id: string
  name: string
  args: unknown
  level: AuthLevel
}
