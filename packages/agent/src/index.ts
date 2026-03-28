// @zseven-w/agent — Domain-agnostic agent SDK
// Public API will be exported here as modules are implemented

// Tools
export type { AgentTool, AuthLevel, ToolResult, ToolCallInfo } from './tools/types'
export { createToolRegistry } from './tools/tool-registry'
export type { ToolRegistry } from './tools/tool-registry'

// Streaming
export type { AgentEvent, AgentMessage, AgentMessagePart } from './streaming/types'
export { encodeAgentEvent } from './streaming/sse-encoder'
export { decodeAgentEvent } from './streaming/sse-decoder'

// Providers
export type { ProviderConfig, AgentProvider } from './providers/types'
export { createAnthropicProvider } from './providers/anthropic'
export { createOpenAICompatProvider } from './providers/openai-compat'

// Context
export type { ContextStrategy } from './context/types'
export { createSlidingWindowStrategy } from './context/sliding-window'

// Delegate
export { createDelegateTool } from './tools/delegate'

// Agent
export { createAgent } from './agent-loop'
export type { AgentConfig, Agent } from './agent-loop'

// Re-export AI SDK utilities needed by consumers
export { jsonSchema, streamText } from 'ai'

// Team
export { createTeam } from './agent-team'
export type { TeamConfig, TeamMemberConfig, AgentTeam } from './agent-team'
