# Agent Team Web Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable model routing via Agent Team — fast model for chat (lead), capable model for design (member).

**Architecture:** SDK adds `source` field to AgentEvent + `member_start`/`member_end` events. Server conditionally uses `createTeam` when `members` array is present. Client adds team toggle + design model selector in settings.

**Tech Stack:** TypeScript, Vercel AI SDK, Zustand, React, i18next

**Spec:** `docs/superpowers/specs/2026-03-28-agent-team-web-integration.md`

---

### Task 1: Add `source` and team events to AgentEvent type

**Files:**
- Modify: `packages/agent/src/streaming/types.ts`

- [ ] **Step 1: Update AgentEvent type**

Add optional `source` to existing variants and two new team events:

```typescript
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
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit --project packages/agent/tsconfig.json`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add packages/agent/src/streaming/types.ts
git commit -m "feat(agent): add source field and team events to AgentEvent type"
```

---

### Task 2: Update SSE encoder/decoder tests and implementation

**Files:**
- Modify: `packages/agent/__tests__/streaming/sse-encoder.test.ts`
- Modify: `packages/agent/__tests__/streaming/sse-decoder.test.ts`
- Verify: `packages/agent/src/streaming/sse-encoder.ts` (no change needed — generic JSON serialization)
- Verify: `packages/agent/src/streaming/sse-decoder.ts` (no change needed — generic JSON parsing)

- [ ] **Step 1: Add encoder tests for new event types**

Append to `packages/agent/__tests__/streaming/sse-encoder.test.ts`:

```typescript
  it('encodes a text event with source', () => {
    const event: AgentEvent = { type: 'text', content: 'hello', source: 'lead' }
    const encoded = encodeAgentEvent(event)
    expect(encoded).toBe('event: text\ndata: {"content":"hello","source":"lead"}\n\n')
  })

  it('encodes a member_start event', () => {
    const event: AgentEvent = { type: 'member_start', memberId: 'designer', task: 'Create landing page' }
    const encoded = encodeAgentEvent(event)
    expect(encoded).toBe('event: member_start\ndata: {"memberId":"designer","task":"Create landing page"}\n\n')
  })

  it('encodes a member_end event', () => {
    const event: AgentEvent = { type: 'member_end', memberId: 'designer', result: 'Done' }
    const encoded = encodeAgentEvent(event)
    expect(encoded).toBe('event: member_end\ndata: {"memberId":"designer","result":"Done"}\n\n')
  })
```

- [ ] **Step 2: Add decoder tests for new event types**

Append to `packages/agent/__tests__/streaming/sse-decoder.test.ts`:

```typescript
  it('decodes a text event with source', () => {
    const raw = 'event: text\ndata: {"content":"hello","source":"designer"}'
    const event = decodeAgentEvent(raw)
    expect(event).toEqual({ type: 'text', content: 'hello', source: 'designer' })
  })

  it('decodes a member_start event', () => {
    const raw = 'event: member_start\ndata: {"memberId":"designer","task":"Build UI"}'
    const event = decodeAgentEvent(raw)
    expect(event).toEqual({ type: 'member_start', memberId: 'designer', task: 'Build UI' })
  })

  it('decodes a member_end event', () => {
    const raw = 'event: member_end\ndata: {"memberId":"designer","result":"Done"}'
    const event = decodeAgentEvent(raw)
    expect(event).toEqual({ type: 'member_end', memberId: 'designer', result: 'Done' })
  })
```

- [ ] **Step 3: Run tests**

Run: `cd packages/agent && npx vitest run __tests__/streaming/`
Expected: all tests PASS (encoder/decoder are generic JSON — no code changes needed)

- [ ] **Step 4: Commit**

```bash
git add packages/agent/__tests__/streaming/
git commit -m "test(agent): add SSE encoder/decoder tests for source and team events"
```

---

### Task 3: Inject source and team events in agent-team.ts

**Files:**
- Modify: `packages/agent/src/agent-team.ts`
- Modify: `packages/agent/__tests__/agent-team.test.ts`

- [ ] **Step 1: Write failing test for source injection and team events**

Append to `packages/agent/__tests__/agent-team.test.ts`:

```typescript
  it('does not mutate the caller tools registry', () => {
    const tools = createToolRegistry()
    const initialCount = tools.list().length
    createTeam({
      lead: { provider: mockProvider, tools, systemPrompt: 'Lead' },
      members: [{ id: 'worker', provider: mockProvider, tools: createToolRegistry(), systemPrompt: 'Worker' }],
    })
    expect(tools.list().length).toBe(initialCount)
  })
```

- [ ] **Step 2: Run test to verify it passes** (already fixed in prior P1 commit)

Run: `cd packages/agent && npx vitest run __tests__/agent-team.test.ts`
Expected: PASS

- [ ] **Step 3: Update agent-team.ts with source injection and team suffix**

Replace the `run` generator and team setup in `packages/agent/src/agent-team.ts`:

```typescript
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
```

- [ ] **Step 4: Run all agent tests**

Run: `cd packages/agent && npx vitest run`
Expected: all 29+ tests PASS

- [ ] **Step 5: Commit**

```bash
git add packages/agent/src/agent-team.ts packages/agent/__tests__/agent-team.test.ts
git commit -m "feat(agent): inject source field and team events in agent-team"
```

---

### Task 4: Extend server agent endpoint for team mode

**Files:**
- Modify: `apps/web/server/api/ai/agent.ts`

- [ ] **Step 1: Add member type and extend AgentBody**

Add `MemberDef` interface and `members` field to `AgentBody` in `apps/web/server/api/ai/agent.ts`:

```typescript
interface MemberDef {
  id: string
  providerType: 'anthropic' | 'openai-compat'
  apiKey: string
  model: string
  baseURL?: string
  systemPrompt?: string
}

interface AgentBody {
  // ... existing fields ...
  members?: MemberDef[]
}
```

- [ ] **Step 2: Add team creation logic**

Import `createTeam` and `createToolRegistry` at the top (createToolRegistry is already imported). In the "Start agent loop" section, after creating `tools` and `abortController`, add conditional team creation:

```typescript
  // After existing tools setup and abortController creation...

  let agentOrTeam: { run: (msgs: any) => AsyncGenerator<any>; resolveToolResult: (id: string, result: any) => void }

  if (body.members?.length) {
    // Team mode — create member agents with their own providers
    const members = body.members.map(m => {
      const memberProvider = m.providerType === 'anthropic'
        ? createAnthropicProvider({ apiKey: m.apiKey, model: m.model, baseURL: m.baseURL })
        : createOpenAICompatProvider({ apiKey: m.apiKey, model: m.model, baseURL: m.baseURL })

      // Members share the same tools as the lead
      const memberTools = createToolRegistry()
      for (const def of body.toolDefs ?? []) {
        const params = def.parameters ? { ...def.parameters } : { type: 'object' }
        delete (params as any).$schema
        memberTools.register({
          name: def.name,
          description: def.description,
          level: def.level,
          schema: jsonSchema(params as any),
        })
      }

      return {
        id: m.id,
        provider: memberProvider,
        tools: memberTools,
        systemPrompt: m.systemPrompt || `You are a ${m.id} specialist.`,
      }
    })

    const team = createTeam({
      lead: {
        provider,
        tools,
        systemPrompt: body.systemPrompt,
        maxTurns: body.maxTurns ?? 20,
        contextStrategy: undefined,
      },
      members,
    })

    agentOrTeam = {
      run: (msgs) => team.run(msgs),
      resolveToolResult: (id, result) => team.resolveToolResult(id, result),
    }
  } else {
    // Single agent mode (existing path)
    const agent = createAgent({
      provider,
      tools,
      systemPrompt: body.systemPrompt,
      maxTurns: body.maxTurns ?? 20,
      maxOutputTokens: body.maxOutputTokens,
      turnTimeout: 5 * 60_000,
      abortSignal: abortController.signal,
    })
    agentOrTeam = agent
  }
```

Update the session registration and SSE loop to use `agentOrTeam` instead of `agent`:

```typescript
  agentSessions.set(body.sessionId, { agent: agentOrTeam as any, abortController, createdAt: Date.now(), lastActivity: Date.now() })
```

And in the stream:
```typescript
  for await (const agentEvent of agentOrTeam.run(toModelMessages(body.messages))) {
```

- [ ] **Step 3: Add createTeam import**

Add to imports at top of file:
```typescript
import {
  createAgent,
  createTeam,
  createAnthropicProvider,
  createOpenAICompatProvider,
  createToolRegistry,
  encodeAgentEvent,
} from '@zseven-w/agent'
```

- [ ] **Step 4: Type check**

Run: `npx tsc --noEmit --project apps/web/tsconfig.json`
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add apps/web/server/api/ai/agent.ts
git commit -m "feat(ai): extend agent endpoint to support team mode with members"
```

---

### Task 5: Add team settings to store

**Files:**
- Modify: `apps/web/src/stores/agent-settings-store.ts`

- [ ] **Step 1: Add team fields to PersistedState and store**

In `PersistedState` interface, add:
```typescript
  teamEnabled: boolean
  teamDesignModel: string | null
```

In the default state of `useAgentSettingsStore`, add:
```typescript
  teamEnabled: false,
  teamDesignModel: null,
```

- [ ] **Step 2: Add setter methods to AgentSettingsState interface and implementation**

In the interface:
```typescript
  setTeamEnabled: (enabled: boolean) => void
  setTeamDesignModel: (model: string | null) => void
```

In the implementation:
```typescript
  setTeamEnabled: (teamEnabled) => set({ teamEnabled }),
  setTeamDesignModel: (teamDesignModel) => set({ teamDesignModel }),
```

- [ ] **Step 3: Update persist and hydrate**

In `persist()`, add `teamEnabled, teamDesignModel` to the destructured state and JSON.stringify.

In `hydrate()`, add:
```typescript
      if ((data as Record<string, unknown>).teamEnabled !== undefined) {
        set({ teamEnabled: (data as Record<string, unknown>).teamEnabled as boolean })
      }
      if ((data as Record<string, unknown>).teamDesignModel !== undefined) {
        set({ teamDesignModel: (data as Record<string, unknown>).teamDesignModel as string | null })
      }
```

- [ ] **Step 4: Type check**

Run: `npx tsc --noEmit --project apps/web/tsconfig.json`
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/stores/agent-settings-store.ts
git commit -m "feat(ai): add teamEnabled and teamDesignModel to agent settings store"
```

---

### Task 6: Add team section UI in provider settings

**Files:**
- Modify: `apps/web/src/components/shared/builtin-provider-settings.tsx`

- [ ] **Step 1: Create TeamSection component**

Add a new `TeamSection` component at the end of the file (before the final export or after `BuiltinProvidersSection`):

```typescript
/** Team configuration — assign a separate model for design work */
export function TeamSection() {
  const { t } = useTranslation()
  const builtinProviders = useAgentSettingsStore((s) => s.builtinProviders)
  const teamEnabled = useAgentSettingsStore((s) => s.teamEnabled)
  const teamDesignModel = useAgentSettingsStore((s) => s.teamDesignModel)
  const setTeamEnabled = useAgentSettingsStore((s) => s.setTeamEnabled)
  const setTeamDesignModel = useAgentSettingsStore((s) => s.setTeamDesignModel)
  const persist = useAgentSettingsStore((s) => s.persist)

  // Only show when at least 2 enabled providers exist
  const enabledProviders = builtinProviders.filter((p) => p.enabled && p.apiKey)
  if (enabledProviders.length < 2) return null

  // Build model options from enabled providers
  const modelOptions = enabledProviders.map((bp) => ({
    value: `builtin:${bp.id}:${bp.model}`,
    label: `${bp.model} (${bp.displayName})`,
  }))

  const handleToggle = (enabled: boolean) => {
    setTeamEnabled(enabled)
    persist()
  }

  const handleModelChange = (value: string) => {
    setTeamDesignModel(value)
    persist()
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">{t('builtin.teamTitle')}</h3>
        <Switch checked={teamEnabled} onCheckedChange={handleToggle} />
      </div>
      <p className="text-[11px] text-muted-foreground leading-relaxed">
        {t('builtin.teamDescription')}
      </p>
      {teamEnabled && (
        <div>
          <label className="text-[11px] text-muted-foreground mb-1 block">{t('builtin.teamDesignModel')}</label>
          <select
            value={teamDesignModel ?? ''}
            onChange={(e) => handleModelChange(e.target.value || null as any)}
            className="w-full h-8 px-2 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors"
          >
            <option value="">{t('builtin.teamSelectModel')}</option>
            {modelOptions.map((opt) => (
              <option key={opt.value} value={opt.value}>{opt.label}</option>
            ))}
          </select>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Add TeamSection to agent-settings-dialog.tsx**

In `apps/web/src/components/shared/agent-settings-dialog.tsx`, import and render `TeamSection` in the Agents tab, between the `BuiltinProvidersSection` and the agents list. Find where `<BuiltinProvidersSection />` is rendered and add `<TeamSection />` after it with a divider:

```typescript
import { BuiltinProvidersSection, TeamSection } from './builtin-provider-settings'
```

```tsx
<BuiltinProvidersSection />
<TeamSection />
```

- [ ] **Step 3: Type check**

Run: `npx tsc --noEmit --project apps/web/tsconfig.json`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/components/shared/builtin-provider-settings.tsx apps/web/src/components/shared/agent-settings-dialog.tsx
git commit -m "feat(ai): add Team section UI with design model selector"
```

---

### Task 7: Add i18n keys for team UI

**Files:**
- Modify: all 15 files in `apps/web/src/i18n/locales/`

- [ ] **Step 1: Add English keys**

In `apps/web/src/i18n/locales/en.ts`, append after the last `builtin.*` key (before `// ── Figma Import ──`):

```typescript
  'builtin.teamTitle': 'Team',
  'builtin.teamDescription': 'Use different models for chat and design. The chat model handles conversation; the design model handles generate_design.',
  'builtin.teamDesignModel': 'Design Model',
  'builtin.teamSelectModel': 'Select a model...',
```

- [ ] **Step 2: Add translations for all other 14 locales**

Add the same 4 keys to each locale file with appropriate translations. Keep "generate_design" untranslated (it's a tool name). Insert after the last `builtin.*` key in each file.

zh.ts:
```typescript
  'builtin.teamTitle': '团队',
  'builtin.teamDescription': '为对话和设计使用不同模型。对话模型处理聊天，设计模型处理 generate_design。',
  'builtin.teamDesignModel': '设计模型',
  'builtin.teamSelectModel': '选择模型...',
```

zh-tw.ts:
```typescript
  'builtin.teamTitle': '團隊',
  'builtin.teamDescription': '為對話和設計使用不同模型。對話模型處理聊天，設計模型處理 generate_design。',
  'builtin.teamDesignModel': '設計模型',
  'builtin.teamSelectModel': '選擇模型...',
```

ja.ts:
```typescript
  'builtin.teamTitle': 'チーム',
  'builtin.teamDescription': 'チャットとデザインに異なるモデルを使用します。チャットモデルは会話を、デザインモデルは generate_design を処理します。',
  'builtin.teamDesignModel': 'デザインモデル',
  'builtin.teamSelectModel': 'モデルを選択...',
```

ko.ts:
```typescript
  'builtin.teamTitle': '팀',
  'builtin.teamDescription': '대화와 디자인에 다른 모델을 사용합니다. 대화 모델은 채팅을, 디자인 모델은 generate_design을 처리합니다.',
  'builtin.teamDesignModel': '디자인 모델',
  'builtin.teamSelectModel': '모델 선택...',
```

fr.ts:
```typescript
  'builtin.teamTitle': 'Équipe',
  'builtin.teamDescription': 'Utilisez différents modèles pour le chat et le design. Le modèle de chat gère la conversation ; le modèle de design gère generate_design.',
  'builtin.teamDesignModel': 'Modèle de design',
  'builtin.teamSelectModel': 'Sélectionner un modèle...',
```

es.ts:
```typescript
  'builtin.teamTitle': 'Equipo',
  'builtin.teamDescription': 'Usa modelos diferentes para chat y diseño. El modelo de chat maneja la conversación; el modelo de diseño maneja generate_design.',
  'builtin.teamDesignModel': 'Modelo de diseño',
  'builtin.teamSelectModel': 'Seleccionar modelo...',
```

de.ts:
```typescript
  'builtin.teamTitle': 'Team',
  'builtin.teamDescription': 'Verwenden Sie verschiedene Modelle für Chat und Design. Das Chat-Modell übernimmt die Konversation, das Design-Modell übernimmt generate_design.',
  'builtin.teamDesignModel': 'Design-Modell',
  'builtin.teamSelectModel': 'Modell auswählen...',
```

pt.ts:
```typescript
  'builtin.teamTitle': 'Equipe',
  'builtin.teamDescription': 'Use modelos diferentes para chat e design. O modelo de chat gerencia a conversa; o modelo de design gerencia generate_design.',
  'builtin.teamDesignModel': 'Modelo de design',
  'builtin.teamSelectModel': 'Selecionar modelo...',
```

ru.ts:
```typescript
  'builtin.teamTitle': 'Команда',
  'builtin.teamDescription': 'Используйте разные модели для чата и дизайна. Модель чата обрабатывает беседу, модель дизайна обрабатывает generate_design.',
  'builtin.teamDesignModel': 'Модель дизайна',
  'builtin.teamSelectModel': 'Выберите модель...',
```

hi.ts:
```typescript
  'builtin.teamTitle': 'टीम',
  'builtin.teamDescription': 'चैट और डिज़ाइन के लिए अलग-अलग मॉडल का उपयोग करें। चैट मॉडल बातचीत संभालता है; डिज़ाइन मॉडल generate_design संभालता है।',
  'builtin.teamDesignModel': 'डिज़ाइन मॉडल',
  'builtin.teamSelectModel': 'मॉडल चुनें...',
```

tr.ts:
```typescript
  'builtin.teamTitle': 'Takım',
  'builtin.teamDescription': 'Sohbet ve tasarım için farklı modeller kullanın. Sohbet modeli konuşmayı, tasarım modeli generate_design\'ı yönetir.',
  'builtin.teamDesignModel': 'Tasarım Modeli',
  'builtin.teamSelectModel': 'Model seçin...',
```

th.ts:
```typescript
  'builtin.teamTitle': 'ทีม',
  'builtin.teamDescription': 'ใช้โมเดลที่แตกต่างกันสำหรับแชทและการออกแบบ โมเดลแชทจัดการการสนทนา โมเดลออกแบบจัดการ generate_design',
  'builtin.teamDesignModel': 'โมเดลออกแบบ',
  'builtin.teamSelectModel': 'เลือกโมเดล...',
```

vi.ts:
```typescript
  'builtin.teamTitle': 'Nhóm',
  'builtin.teamDescription': 'Sử dụng các mô hình khác nhau cho trò chuyện và thiết kế. Mô hình trò chuyện xử lý hội thoại; mô hình thiết kế xử lý generate_design.',
  'builtin.teamDesignModel': 'Mô hình thiết kế',
  'builtin.teamSelectModel': 'Chọn mô hình...',
```

id.ts:
```typescript
  'builtin.teamTitle': 'Tim',
  'builtin.teamDescription': 'Gunakan model berbeda untuk obrolan dan desain. Model obrolan menangani percakapan; model desain menangani generate_design.',
  'builtin.teamDesignModel': 'Model Desain',
  'builtin.teamSelectModel': 'Pilih model...',
```

- [ ] **Step 3: Commit**

```bash
git add apps/web/src/i18n/locales/
git commit -m "feat(ai): add i18n keys for team settings (15 locales)"
```

---

### Task 8: Wire team mode in chat handlers

**Files:**
- Modify: `apps/web/src/components/panels/ai-chat-handlers.ts`
- Modify: `apps/web/src/services/ai/ai-types.ts`

- [ ] **Step 1: Add source to ChatMessage**

In `apps/web/src/services/ai/ai-types.ts`, add `source` to `ChatMessage`:

```typescript
export interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: number
  isStreaming?: boolean
  attachments?: ChatAttachment[]
  source?: string
}
```

- [ ] **Step 2: Construct team members in agent request**

In `apps/web/src/components/panels/ai-chat-handlers.ts`, find where the agent body is constructed for the POST to `/api/ai/agent`. After the existing body construction, add team member injection:

```typescript
// After body is constructed, before fetch:
const { teamEnabled, teamDesignModel, builtinProviders: allBps } = useAgentSettingsStore.getState()
if (teamEnabled && teamDesignModel) {
  const designParts = teamDesignModel.split(':')
  const designBpId = designParts[1]
  const designModelName = designParts.slice(2).join(':')
  const designBp = allBps.find((p) => p.id === designBpId)
  if (designBp?.apiKey) {
    ;(body as any).members = [{
      id: 'designer',
      providerType: designBp.type === 'anthropic' ? 'anthropic' : 'openai-compat',
      apiKey: designBp.apiKey,
      model: designModelName,
      baseURL: designBp.baseURL,
      systemPrompt: 'You are a design specialist. Use the generate_design tool to create designs based on the task description. Focus on high-quality visual output.',
    }]
  }
}
```

- [ ] **Step 3: Handle member_start and member_end events**

In the SSE event switch statement (inside `runAgentStream`), add cases for new event types:

```typescript
        case 'member_start': {
          const status = `**${evt.memberId}** working: ${evt.task}`
          accumulated += `\n\n---\n${status}\n`
          updateLastMessage(accumulated)
          break
        }

        case 'member_end': {
          accumulated += `\n---\n`
          updateLastMessage(accumulated)
          break
        }
```

- [ ] **Step 4: Type check**

Run: `npx tsc --noEmit --project apps/web/tsconfig.json`
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/services/ai/ai-types.ts apps/web/src/components/panels/ai-chat-handlers.ts
git commit -m "feat(ai): wire team mode in chat handlers with member events"
```

---

### Task 9: End-to-end verification

- [ ] **Step 1: Run all agent SDK tests**

Run: `cd packages/agent && npx vitest run`
Expected: all tests pass

- [ ] **Step 2: Run type check**

Run: `npx tsc --noEmit --project apps/web/tsconfig.json`
Expected: no errors

- [ ] **Step 3: Manual verification**

1. Open editor at `http://localhost:3000/editor`
2. Agents & MCP → verify Team section appears when 2+ providers are enabled
3. Enable Team toggle → select a design model
4. Send a design request → verify member_start/member_end events appear in chat
5. Disable Team toggle → verify single agent mode works as before

- [ ] **Step 4: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix(ai): address team integration issues found during verification"
```
