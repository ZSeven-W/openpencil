import { tool as aiTool } from 'ai'
import type { Tool } from 'ai'
import type { AgentTool, AuthLevel } from './types'

export interface ToolRegistry {
  register(tool: AgentTool): void
  get(name: string): AgentTool | undefined
  getLevel(name: string): AuthLevel | undefined
  list(): AgentTool[]
  toAISDKFormat(): Record<string, Tool>
  hasExecute(name: string): boolean
}

export function createToolRegistry(): ToolRegistry {
  const tools = new Map<string, AgentTool>()
  return {
    register(tool) {
      if (tools.has(tool.name)) throw new Error(`Tool "${tool.name}" is already registered`)
      tools.set(tool.name, tool)
    },
    get(name) { return tools.get(name) },
    getLevel(name) { return tools.get(name)?.level },
    list() { return Array.from(tools.values()) },
    toAISDKFormat() {
      const result: Record<string, Tool> = {}
      for (const [name, t] of tools) {
        result[name] = aiTool({
          description: t.description,
          inputSchema: t.schema,
          ...(t.execute ? { execute: t.execute } : {}),
        })
      }
      return result
    },
    hasExecute(name) { return typeof tools.get(name)?.execute === 'function' },
  }
}
