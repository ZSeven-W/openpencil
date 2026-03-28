# Agent Team Web Integration

**Date:** 2026-03-28
**Status:** Approved
**Scope:** Phase 3 of builtin agent design — integrate Agent Team into the web app

## Goal

Enable model routing via Agent Team: a fast/cheap model handles conversation (lead), a capable model handles design generation (member). Users assign models to roles in settings; the system automatically constructs a team when both roles are configured.

## Non-Goals

- Custom team builder UI (arbitrary roles, system prompts, tool configs)
- More than two roles (lead + designer)
- Nested agent block UI (member events displayed inline)
- Agent Team in CLI or desktop-specific code

## Architecture

### SDK Layer (`packages/agent/`)

All generic team functionality lives in the SDK. No OpenPencil domain knowledge.

#### 1. AgentEvent Source Tracking

Add optional `source` field to all event types plus two new team-specific events.

**File: `streaming/types.ts`**

```typescript
// Add source to existing event variants (optional, undefined for single-agent mode)
| { type: 'text'; content: string; source?: string }
| { type: 'thinking'; content: string; source?: string }
| { type: 'tool_call'; id: string; name: string; args: unknown; level: AuthLevel; source?: string }
| { type: 'tool_result'; id: string; name: string; result: ToolResult; source?: string }
| { type: 'turn'; turn: number; maxTurns: number; source?: string }
| { type: 'error'; message: string; fatal: boolean; source?: string }

// New team events
| { type: 'member_start'; memberId: string; task: string }
| { type: 'member_end'; memberId: string; result: string }
```

Backward compatible: `source` is optional, absent in single-agent mode.

#### 2. Team Event Injection

**File: `agent-team.ts`**

Wrap lead events with `source: 'lead'`, member events with `source: memberId`. Yield `member_start` before member execution and `member_end` after.

```typescript
// Lead events
for await (const event of leadAgent.run(messages)) {
  if (event.type === 'tool_call' && event.name === 'delegate') {
    yield { type: 'member_start', memberId: member, task }
    for await (const memberEvent of memberEntry.agent.run(memberMessages)) {
      yield { ...memberEvent, source: member }
    }
    yield { type: 'member_end', memberId: member, result: memberResult }
  } else {
    yield { ...event, source: 'lead' }
  }
}
```

#### 3. Auto-inject Team Suffix into Lead System Prompt

**File: `agent-team.ts`**

Append member descriptions to the lead's system prompt so the LLM knows it has team members:

```typescript
const teamSuffix = `

## Team Members
You have team members available via the \`delegate\` tool:
${config.members.map(m => `- **${m.id}**: ${m.systemPrompt?.split('\n')[0] || 'Available for sub-tasks'}`).join('\n')}

When the user's request involves design generation, delegate to the appropriate member.
Handle conversation, planning, and simple queries yourself.`
```

This is domain-agnostic — it just lists members and their first-line descriptions.

#### 4. SSE Encoder/Decoder

**Files: `streaming/sse-encoder.ts`, `streaming/sse-decoder.ts`**

Extend to handle `source`, `member_start`, `member_end`. The encoder serializes them as JSON in the SSE data field. The decoder parses them back. No format changes needed — these are just new fields/types in the existing JSON payload.

### Server Layer (`apps/web/server/`)

#### 5. Extend Agent Endpoint

**File: `server/api/ai/agent.ts`**

Expand `AgentBody` with optional `members` array:

```typescript
interface AgentBody {
  // ... existing fields unchanged ...

  members?: Array<{
    id: string
    providerType: 'anthropic' | 'openai-compat'
    apiKey: string
    model: string
    baseURL?: string
    systemPrompt?: string
  }>
}
```

Routing logic:
- No `members` or empty → `createAgent()` (existing path, fully backward compatible)
- `members` present → `createTeam()` with lead from main body fields, each member gets its own provider

`resolveToolResult` routing: `AgentTeam` maintains a `toolCallOwners` map that tracks which agent (lead or member) issued each tool call. When the server receives a tool result, `team.resolveToolResult()` looks up the owner and routes to the correct agent. This is critical — member tool calls (e.g. `generate_design`) must be routed to the member agent, not the lead.

Member tools: Members receive the **same tool registry** as the lead (server creates a copy from `body.toolDefs` for each member). This ensures the designer member can call `generate_design` and other design tools.

SSE stream code unchanged — `encodeAgentEvent` handles new event types automatically.

Source and ChatMessage: The `source` field on `ChatMessage` represents the overall message origin. Within a single assistant message, lead and member text are visually separated via markdown dividers (`---`) inserted by `member_start`/`member_end` event handlers. Per-segment source tracking is not needed for the MVP.

### Client Layer (`apps/web/`)

#### 6. Settings Store

**File: `stores/agent-settings-store.ts`**

Add two persisted fields:

```typescript
teamEnabled: boolean           // default: false
teamDesignModel: string | null // format: 'builtin:{id}:{model}', default: null
```

Methods: `setTeamEnabled(enabled)`, `setTeamDesignModel(model)`.

#### 7. Team Configuration UI

**File: `components/shared/builtin-provider-settings.tsx`**

Add a "Team" section below the provider list:

- **Enable toggle**: switches team mode on/off
- **Design Model dropdown**: selects from configured builtin providers (same model selector format as chat)
- Info text explaining: "Chat model handles conversation. Design model handles generate_design."
- Only shown when at least 2 builtin providers are configured and enabled

All text uses i18n keys under `builtin.team*` namespace.

#### 8. Chat Handler — Team Request Construction

**File: `components/panels/ai-chat-handlers.ts`**

When `teamEnabled && teamDesignModel`:

```typescript
const designParts = teamDesignModel.split(':')
const designBp = builtinProviders.find(p => p.id === designParts[1])

body.members = [{
  id: 'designer',
  providerType: designBp.type === 'anthropic' ? 'anthropic' : 'openai-compat',
  apiKey: designBp.apiKey,
  model: designParts.slice(2).join(':'),
  baseURL: designBp.baseURL,
  systemPrompt: 'You are a design specialist. Use the generate_design tool to create designs based on the task description. Focus on high-quality visual output.',
}]
```

#### 9. Chat Handler — New Event Types

**File: `components/panels/ai-chat-handlers.ts`**

```typescript
case 'member_start':
  // Add inline status in chat: "🎨 Designer working on: {task}"
  break

case 'member_end':
  // Update status to complete
  break
```

For events with `source`, tag the message with source info. Only display source labels in team mode (when `source` is present).

#### 10. AI Store

**File: `stores/ai-store.ts`**

Add optional `source?: string` to `ChatMessage` interface. Used for display only — distinguishes lead vs member text in the chat.

## Data Flow

```
User → "设计一个电商首页"
  │
  ▼
[Client] teamEnabled=true → body.members=[{designer}]
  │
  ▼
[Server] members present → createTeam(lead=chatModel, members=[designerModel])
  │
  ▼
[SDK] lead → delegate({member:'designer', task:'...'})
      yield member_start
      designer → generate_design tool call
      yield tool_call (source:'designer')
      client executes tool, posts result
      designer → text output (source:'designer')
      yield member_end
      lead → summary (source:'lead')
      yield done
  │
  ▼
[Client] renders inline: "Designer working..." → design appears → lead summary
```

## Backward Compatibility

- `source` is optional on all events — single agent mode unchanged
- No `members` in body → existing `createAgent` path, zero impact
- `teamEnabled` defaults to false — new users see no change
- All new UI behind the team toggle

## File Change Summary

| Layer | File | Change |
|-------|------|--------|
| SDK | `streaming/types.ts` | `source` field + 2 new event types |
| SDK | `agent-team.ts` | Inject source, yield member_start/end, team suffix |
| SDK | `streaming/sse-encoder.ts` | Encode new fields |
| SDK | `streaming/sse-decoder.ts` | Decode new fields |
| SDK | Tests | Update existing team tests, add source/event tests |
| Server | `agent.ts` | Accept members, conditionally use createTeam |
| Client | `agent-settings-store.ts` | teamEnabled + teamDesignModel |
| Client | `builtin-provider-settings.tsx` | Team section UI |
| Client | `ai-chat-handlers.ts` | Construct members, handle new events |
| Client | `ai-store.ts` | source field on ChatMessage |
| i18n | All 15 locales | builtin.team* keys |
