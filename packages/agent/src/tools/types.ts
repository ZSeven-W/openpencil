import type { z } from 'zod'

export type AuthLevel = 'read' | 'create' | 'modify' | 'delete' | 'orchestrate'

export interface AgentTool<TArgs = any, TResult = any> {
  name: string
  description: string
  schema: z.ZodType<TArgs>
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
