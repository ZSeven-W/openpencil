import type { ModelMessage } from 'ai'
import type { AgentProvider } from './providers/types'
import type { ToolRegistry } from './tools/tool-registry'
import type { AgentEvent } from './streaming/types'
import type { ContextStrategy } from './context/types'
import type { ToolResult } from './tools/types'
import { createAgent } from './agent-loop'
import { createDelegateTool } from './tools/delegate'
import { createToolRegistry } from './tools/tool-registry'

export interface TeamMemberConfig {
  id: string
  provider: AgentProvider
  tools: ToolRegistry
  systemPrompt: string
  maxTurns?: number
  contextStrategy?: ContextStrategy
}

export interface TeamConfig {
  lead: {
    provider: AgentProvider
    tools: ToolRegistry
    systemPrompt: string
    maxTurns?: number
    contextStrategy?: ContextStrategy
  }
  members: TeamMemberConfig[]
}

export interface AgentTeam {
  run(messages: ModelMessage[]): AsyncGenerator<AgentEvent>
  abort(): void
  resolveToolResult(toolCallId: string, result: ToolResult): void
}

/** Build a team-awareness suffix for the lead's system prompt. */
function buildTeamSuffix(members: TeamMemberConfig[]): string {
  const lines = members.map(m => {
    const desc = m.systemPrompt.split('\n')[0] || 'Available for sub-tasks'
    return `- **${m.id}**: ${desc}`
  })
  return `\n\n## Team Members\nYou have team members available via the \`delegate\` tool:\n${lines.join('\n')}\n\nWhen the user's request involves specialized work, delegate to the appropriate member.\nHandle conversation, planning, and simple queries yourself.`
}

export function createTeam(config: TeamConfig): AgentTeam {
  const parentAbort = new AbortController()
  const memberIds = config.members.map(m => m.id)

  // Clone tools to avoid mutating the caller's registry
  const leadTools = createToolRegistry()
  for (const tool of config.lead.tools.list()) {
    leadTools.register(tool)
  }
  leadTools.register(createDelegateTool(memberIds))

  const leadAgent = createAgent({
    provider: config.lead.provider,
    tools: leadTools,
    systemPrompt: config.lead.systemPrompt + buildTeamSuffix(config.members),
    maxTurns: config.lead.maxTurns ?? 30,
    contextStrategy: config.lead.contextStrategy,
    abortSignal: parentAbort.signal,
  })

  const memberAgents = new Map(
    config.members.map(m => [
      m.id,
      {
        config: m,
        agent: createAgent({
          provider: m.provider,
          tools: m.tools,
          systemPrompt: m.systemPrompt,
          maxTurns: m.maxTurns ?? 20,
          contextStrategy: m.contextStrategy,
          abortSignal: parentAbort.signal,
        }),
      },
    ]),
  )

  async function* run(messages: ModelMessage[]): AsyncGenerator<AgentEvent> {
    for await (const event of leadAgent.run(messages)) {
      if (event.type === 'tool_call' && event.name === 'delegate') {
        const { member, task, context } = event.args as { member: string; task: string; context?: string }
        const memberEntry = memberAgents.get(member)

        if (!memberEntry) {
          leadAgent.resolveToolResult(event.id, {
            success: false,
            error: `Unknown team member: ${member}`,
          })
          continue
        }

        // Yield the delegate tool_call with lead source
        yield { ...event, source: 'lead' }

        // Signal member start
        yield { type: 'member_start', memberId: member, task }

        const memberMessages: ModelMessage[] = [
          { role: 'user', content: typeof context === 'string' ? `${task}\n\nContext: ${context}` : task },
        ]

        let memberResult = ''
        for await (const memberEvent of memberEntry.agent.run(memberMessages)) {
          // Tag all member events with source
          yield { ...memberEvent, source: member } as AgentEvent
          if (memberEvent.type === 'text') {
            memberResult += memberEvent.content
          }
        }

        // Signal member end
        yield { type: 'member_end', memberId: member, result: memberResult || 'Task completed.' }

        leadAgent.resolveToolResult(event.id, {
          success: true,
          data: memberResult || 'Task completed.',
        })
        continue
      }

      // Tag lead events with source
      yield { ...event, source: 'lead' } as AgentEvent
    }
  }

  return {
    run,
    abort: () => parentAbort.abort(),
    resolveToolResult: (id, result) => leadAgent.resolveToolResult(id, result),
  }
}
