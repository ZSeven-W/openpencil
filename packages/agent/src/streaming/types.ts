import type { AuthLevel } from '../tools/types'

export type AgentEvent =
  | { type: 'thinking'; content: string; source?: string }
  | { type: 'text'; content: string; source?: string }
  | { type: 'tool_call'; id: string; name: string; args: unknown; level: AuthLevel; source?: string }
  | { type: 'tool_result'; id: string; name: string; result: { success: boolean; data?: unknown; error?: string }; source?: string }
  | { type: 'turn'; turn: number; maxTurns: number; source?: string }
  | { type: 'done'; totalTurns: number; source?: string }
  | { type: 'error'; message: string; fatal: boolean; source?: string }
  | { type: 'abort' }
  | { type: 'member_start'; memberId: string; task: string }
  | { type: 'member_end'; memberId: string; result: string }

export interface AgentMessage {
  role: 'user' | 'assistant' | 'system'
  content: string | AgentMessagePart[]
}

export type AgentMessagePart =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mediaType: string }
  | { type: 'tool_call'; id: string; name: string; args: unknown }
  | { type: 'tool_result'; id: string; content: string }
