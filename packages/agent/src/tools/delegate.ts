import { jsonSchema } from 'ai';
import type { AgentTool } from './types';

export function createDelegateTool(memberIds: string[]): AgentTool {
  return {
    name: 'delegate',
    description: `Delegate a sub-task to a team member. Available members: ${memberIds.join(', ')}`,
    level: 'orchestrate',
    schema: jsonSchema(
      {
        type: 'object',
        properties: {
          member: { type: 'string', enum: memberIds, description: 'Team member to delegate to' },
          task: { type: 'string', description: 'Clear description of the sub-task' },
          context: { type: 'string', description: 'Additional context for the member' },
        },
        required: ['member', 'task'],
        additionalProperties: false,
      },
      {
        validate: (value) => {
          if (typeof value !== 'object' || value === null) {
            return { success: false, error: new Error('Expected an object') };
          }
          const v = value as Record<string, unknown>;
          if (typeof v.member !== 'string' || !memberIds.includes(v.member)) {
            return {
              success: false,
              error: new Error(`member must be one of: ${memberIds.join(', ')}`),
            };
          }
          if (typeof v.task !== 'string' || v.task.length === 0) {
            return { success: false, error: new Error('task must be a non-empty string') };
          }
          if (v.context !== undefined && typeof v.context !== 'string') {
            return { success: false, error: new Error('context must be a string if provided') };
          }
          return { success: true, value: v };
        },
      },
    ),
    // No execute — handled by agent-team.ts
  };
}
