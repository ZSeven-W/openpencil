import type { AuthLevel } from '../tools/types'

export type AgentEvent =
  | { type: 'thinking'; content: string }
  | { type: 'text'; content: string }
  | { type: 'tool_call'; id: string; name: string; args: unknown; level: AuthLevel }
  | { type: 'tool_result'; id: string; name: string; result: { success: boolean; data?: unknown; error?: string } }
  | { type: 'turn'; turn: number; maxTurns: number }
  | { type: 'done'; totalTurns: number }
  | { type: 'error'; message: string; fatal: boolean }
  | { type: 'abort' }

export interface AgentMessage {
  role: 'user' | 'assistant' | 'system'
  content: string | AgentMessagePart[]
}

export type AgentMessagePart =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mediaType: string }
  | { type: 'tool_call'; id: string; name: string; args: unknown }
  | { type: 'tool_result'; id: string; content: string }
