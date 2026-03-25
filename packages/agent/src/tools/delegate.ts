import { z } from 'zod'
import type { AgentTool } from './types'

export function createDelegateTool(memberIds: string[]): AgentTool {
  return {
    name: 'delegate',
    description: `Delegate a sub-task to a team member. Available members: ${memberIds.join(', ')}`,
    level: 'orchestrate',
    schema: z.object({
      member: z.enum(memberIds as [string, ...string[]]),
      task: z.string().describe('Clear description of the sub-task'),
      context: z.string().optional().describe('Additional context for the member'),
    }),
    // No execute — handled by agent-team.ts
  }
}
