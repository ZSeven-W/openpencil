import { defineEventHandler, readBody, setResponseHeaders, getQuery, createError } from 'h3';
import {
  createAnthropicProvider,
  createOpenAICompatProvider,
  createToolRegistry,
  registerToolSchema,
  createQueryEngine,
  seedMessages,
  submitMessage,
  nextEvent,
  resolveToolResult,
  createTeam,
  runTeam,
  addTeamMember,
  resolveTeamToolResult,
  teamRegisterDelegate,
  runTeamMember,
  destroyIterator,
  resolveMemberToolResult,
  seedTeamMessages,
} from '@zseven-w/agent-native';
import { resolveSkills } from '@zseven-w/pen-ai-skills';
import type { Phase } from '@zseven-w/pen-ai-skills';
import type { AuthLevel } from '../../../src/types/agent';
import {
  agentSessions,
  cleanup,
  abortSession,
  createSession,
  touchSession,
  type AgentSession,
} from '../../utils/agent-sessions';
import {
  shouldShortCircuitPlanLayout,
  updateLayoutSessionState,
} from '../../utils/agent-tool-guard';
import { getAllToolDefs } from '../../../src/services/ai/agent-tools';
import {
  normalizeOptionalBaseURL,
  normalizeMemberBaseURL,
  requireOpenAICompatBaseURL,
} from './provider-url';
import { startSSEKeepAlive } from '../../utils/sse-keepalive';

const TOOL_LEVEL_MAP: Record<string, AuthLevel> = {
  batch_get: 'read',
  snapshot_layout: 'read',
  find_empty_space: 'read',
  generate_design: 'create',
  insert_node: 'create',
  update_node: 'modify',
  delete_node: 'delete',
};

const ROLE_TOOL_PRESETS: Record<string, string[]> = {
  designer: [
    'batch_get',
    'snapshot_layout',
    'find_empty_space',
    'generate_design',
    'insert_node',
    'plan_layout',
    'batch_insert',
  ],
  reviewer: ['batch_get', 'snapshot_layout', 'get_selection'],
  editor: [
    'batch_get',
    'snapshot_layout',
    'find_empty_space',
    'update_node',
    'delete_node',
    'insert_node',
  ],
  researcher: ['batch_get', 'snapshot_layout', 'find_empty_space', 'get_selection'],
};

const ROLE_SKILL_PHASE: Record<string, Phase> = {
  designer: 'generation',
  reviewer: 'validation',
  editor: 'maintenance',
  researcher: 'planning',
};

const ROLE_TOOL_INSTRUCTIONS: Record<string, string> = {
  designer: `You are a design team member. When asked to create designs, you MUST call the generate_design tool with a descriptive prompt. You can also use insert_node for manual node creation, batch_get and snapshot_layout to inspect the canvas, and find_empty_space to find placement locations. Always end with a short natural-language summary of what you created or changed. Never stop at tool calls only.`,
  reviewer: `You are a design reviewer. Use batch_get and snapshot_layout to inspect the current canvas state. Use get_selection to see what the user has selected. Provide detailed feedback on layout, spacing, typography, and visual hierarchy. Always end with a short natural-language summary for the lead agent.`,
  editor: `You are a design editor. Use batch_get and snapshot_layout to understand the current canvas. Use update_node to modify node properties, delete_node to remove elements, and insert_node to add new elements. Use find_empty_space to find placement locations. Always end with a short natural-language summary of what changed. Never stop at tool calls only.`,
  researcher: `You are a design researcher. Use batch_get and snapshot_layout to analyze the current canvas state. Use find_empty_space to identify available space. Use get_selection to see what the user has selected. Provide analysis and recommendations. Always end with a short natural-language summary for the lead agent.`,
};

function buildTeamCapabilitiesPrompt(concurrency: number): string {
  return `\n\n## Team Mode — MANDATORY parallel design

You MUST use your team of ${concurrency} designers. Do NOT call generate_design yourself.

**Workflow:**
1. Analyze the user's request and break it into ${concurrency} distinct sections/screens
2. Spawn ${concurrency} designer members: spawn_member({id: "designer-1", role: "designer"}), spawn_member({id: "designer-2", role: "designer"}), etc.
3. Delegate one section to each: delegate({member_id: "designer-1", task: "Design the [section] with [details]..."})
4. After all delegations complete, summarize what was created.

**Available roles:** designer, reviewer, editor, researcher

**Example for a food app with ${concurrency} designers:**
${Array.from({ length: concurrency }, (_, i) => `- designer-${i + 1}: a different screen or section`).join('\n')}

IMPORTANT: Always spawn exactly ${concurrency} designers and delegate to all of them. Each delegation should include a detailed description of that section. Never call generate_design directly — always delegate to spawned designers.
After all delegations, end with a short summary for the user.`;
}

function buildMemberSystemPrompt(
  role: string,
  designMdContent?: string,
  hasVariables?: boolean,
): string {
  const phase = ROLE_SKILL_PHASE[role] ?? 'generation';
  const toolInstructions = ROLE_TOOL_INSTRUCTIONS[role] ?? '';

  const skillCtx = resolveSkills(phase, '', {
    flags: {
      hasDesignMd: !!designMdContent,
      hasVariables: !!hasVariables,
    },
    dynamicContent: designMdContent ? { designMdContent } : undefined,
  });
  const knowledge = skillCtx.skills.map((s) => s.content).join('\n\n');

  return `${toolInstructions}\n\n${knowledge}`;
}

const SPAWN_MEMBER_SCHEMA = JSON.stringify({
  type: 'object',
  properties: {
    id: { type: 'string', description: 'Unique member ID, e.g. "designer-1"' },
    role: {
      type: 'string',
      enum: ['designer', 'reviewer', 'editor', 'researcher'],
      description: 'Member role — determines available tools and knowledge',
    },
    model: {
      type: 'string',
      description: 'Optional model override for this member. Defaults to lead model.',
    },
  },
  required: ['id', 'role'],
});

interface ToolDef {
  name: string;
  description: string;
  level: AuthLevel;
  parameters?: Record<string, unknown>;
}

interface MemberDef {
  id: string;
  providerType: 'anthropic' | 'openai-compat';
  apiKey: string;
  model: string;
  baseURL?: string;
  systemPrompt?: string;
}

interface AgentBody {
  sessionId: string;
  messages: Array<{ role: string; content: string }>;
  systemPrompt: string;
  providerType: 'anthropic' | 'openai-compat';
  apiKey: string;
  model: string;
  baseURL?: string;
  toolDefs: ToolDef[];
  maxTurns?: number;
  maxOutputTokens?: number;
  maxContextTokens?: number;
  members?: MemberDef[];
  teamMode?: boolean;
  concurrency?: number;
  designMdContent?: string;
  hasVariables?: boolean;
}

/** Map Zig event JSON to client SSE format.
 *  Zig events are tagged unions: {"result":{...}} or {"stream_event":{...}}.
 *  Extract the tag and inner data, then map to the flat client format.
 */
function zigEventToSSE(raw: string): string {
  const evt = JSON.parse(raw);

  // Zig tagged union: the single key is the event type, value is the data.
  // For stream_event, the inner object has its own "type" field (text_delta, etc.)
  let tag: string;
  let data: Record<string, unknown>;
  if (evt.tool_use) {
    // Complete tool call from Zig engine (after input_json_delta accumulation).
    // This is the authoritative tool_call event — content_block_start only has metadata.
    tag = 'tool_use';
    data = evt.tool_use;
  } else if (evt.stream_event) {
    tag = evt.stream_event.type ?? 'unknown';
    data = evt.stream_event;
  } else if (evt.result) {
    tag = 'result';
    data = evt.result;
  } else if (evt.tool_progress) {
    tag = 'tool_progress';
    data = evt.tool_progress;
  } else {
    tag = evt.type ?? 'unknown';
    data = evt;
  }

  let mapped: Record<string, unknown>;
  switch (tag) {
    case 'text_delta':
      mapped = { type: 'text', content: data.text };
      break;
    case 'thinking_delta':
      mapped = { type: 'thinking', content: data.text };
      break;
    case 'tool_use':
      // Complete tool call with full args — emitted by Zig engine after input_json_delta accumulation
      mapped = {
        type: 'tool_call',
        id: data.id,
        name: data.name,
        args:
          typeof data.input === 'string' ? JSON.parse(data.input as string) : (data.input ?? {}),
        level: TOOL_LEVEL_MAP[data.name as string] ?? 'read',
      };
      break;
    case 'content_block_start':
      // Skip tool_use content_block_start — args aren't available yet.
      // The complete tool_call is emitted later as a tool_use event.
      if (data.tool_name) {
        return ''; // suppress — will come as tool_use event with full args
      }
      mapped = { type: tag, ...data };
      break;
    case 'result':
      if (data.is_error) {
        mapped = {
          type: 'error',
          message: `Agent error: ${data.subtype ?? 'unknown'}${data.result ? ' — ' + data.result : ''}`,
          fatal: true,
        };
      } else {
        mapped = { type: 'done', totalTurns: data.num_turns ?? 0 };
      }
      break;
    case 'member_start':
      mapped = {
        type: 'member_start',
        memberId: data.member_id,
        task: data.task ?? '',
      };
      break;
    case 'member_end':
      mapped = {
        type: 'member_end',
        memberId: data.member_id,
        result: data.result ?? '',
      };
      break;
    default:
      mapped = { type: tag, ...data };
  }
  return `event: ${mapped.type}\ndata: ${JSON.stringify(mapped)}\n\n`;
}

/** Run a delegated member asynchronously — does NOT block the caller. */
async function runDelegateMember(
  session: AgentSession,
  body: AgentBody,
  controller: ReadableStreamDefaultController,
  encoder: TextEncoder,
  toolUseId: string,
  memberId: string,
  task: string,
) {
  // Resolve task-specific skills based on member role
  const memberRole = session.memberRoles.get(memberId);
  let enrichedTask = task;
  if (memberRole) {
    const phase = ROLE_SKILL_PHASE[memberRole] ?? 'generation';
    const taskSkills = resolveSkills(phase, task, {
      flags: {
        hasDesignMd: !!body.designMdContent,
        hasVariables: !!body.hasVariables,
      },
    });
    const skillPrefix = taskSkills.skills.map((s) => s.content).join('\n\n');
    if (skillPrefix) enrichedTask = skillPrefix + '\n\n' + task;
  }

  controller.enqueue(
    encoder.encode(
      `event: member_start\ndata: ${JSON.stringify({ type: 'member_start', memberId, task })}\n\n`,
    ),
  );

  let memberResult = '';
  const memberIter = await runTeamMember(session.team!, memberId, enrichedTask);
  try {
    let memberRaw: string | null;
    while ((memberRaw = await nextEvent(memberIter)) !== null) {
      session.lastActivity = Date.now();
      try {
        const mEvt = JSON.parse(memberRaw);

        // Member tool_use → record owner, forward with source
        if (mEvt.tool_use) {
          const mToolId = mEvt.tool_use.id;
          session.toolOwners.set(mToolId, memberId);
          const level = TOOL_LEVEL_MAP[mEvt.tool_use.name as string] ?? 'read';
          const toolCallEvt = {
            type: 'tool_call',
            id: mToolId,
            name: mEvt.tool_use.name,
            args:
              typeof mEvt.tool_use.input === 'string'
                ? JSON.parse(mEvt.tool_use.input as string)
                : (mEvt.tool_use.input ?? {}),
            level,
            source: memberId,
          };
          controller.enqueue(
            encoder.encode(`event: tool_call\ndata: ${JSON.stringify(toolCallEvt)}\n\n`),
          );
          continue;
        }

        // Collect text
        if (mEvt.stream_event?.text && mEvt.stream_event.type === 'text_delta') {
          memberResult += mEvt.stream_event.text;
        }
      } catch {
        /* ignore parse errors */
      }
      const memberSse = zigEventToSSE(memberRaw);
      if (memberSse) controller.enqueue(encoder.encode(memberSse));
    }
  } finally {
    destroyIterator(memberIter);
    for (const [tid, mid] of session.toolOwners) {
      if (mid === memberId) session.toolOwners.delete(tid);
    }
  }

  controller.enqueue(
    encoder.encode(
      `event: member_end\ndata: ${JSON.stringify({ type: 'member_end', memberId, result: '' })}\n\n`,
    ),
  );

  resolveTeamToolResult(
    session.team!,
    toolUseId,
    JSON.stringify({ result: memberResult || 'Member completed task.' }),
  );
}

function createProviderHandle(
  providerType: 'anthropic' | 'openai-compat',
  apiKey: string,
  model: string,
  baseURL?: string,
  maxContextTokens?: number,
) {
  return providerType === 'anthropic'
    ? createAnthropicProvider(apiKey, model, baseURL, maxContextTokens)
    : createOpenAICompatProvider(
        apiKey,
        requireOpenAICompatBaseURL(baseURL),
        model,
        maxContextTokens,
      );
}

/**
 * Unified agent endpoint. Routes by `?action=` query param:
 *   POST /api/ai/agent              — Start agent loop (SSE stream)
 *   POST /api/ai/agent?action=result — Resolve a pending tool call
 *   POST /api/ai/agent?action=abort  — Abort an agent session
 */
export default defineEventHandler(async (event) => {
  const { action } = getQuery(event) as { action?: string };

  // ── Tool result callback ────────────────────────────────────
  if (action === 'result') {
    const body = await readBody<{ sessionId: string; toolCallId: string; result: any }>(event);
    if (!body?.sessionId || !body.toolCallId || !body.result) {
      throw createError({ statusCode: 400, message: 'Missing: sessionId, toolCallId, result' });
    }
    const session = agentSessions.get(body.sessionId);
    if (!session) {
      throw createError({ statusCode: 404, message: 'Session not found' });
    }
    try {
      const toolName = session.toolNames.get(body.toolCallId);
      updateLayoutSessionState(session, toolName, body.result);

      const resultJson = JSON.stringify(body.result);
      // Per-toolCallId routing: check if this tool belongs to a member
      const memberId = session.toolOwners?.get(body.toolCallId);
      if (memberId && session.team) {
        resolveMemberToolResult(session.team, memberId, body.toolCallId, resultJson);
        session.toolOwners.delete(body.toolCallId);
      } else if (session.team) {
        resolveTeamToolResult(session.team, body.toolCallId, resultJson);
      } else if (session.engine) {
        resolveToolResult(session.engine, body.toolCallId, resultJson);
      }
      session.toolNames.delete(body.toolCallId);
    } catch {
      return { ok: true, ignored: true };
    }
    session.lastActivity = Date.now();
    return { ok: true };
  }

  // ── Abort ───────────────────────────────────────────────────
  if (action === 'abort') {
    const body = await readBody<{ sessionId?: string }>(event);
    const sid = body?.sessionId;
    if (sid) {
      const session = agentSessions.get(sid);
      if (session) {
        abortSession(session);
        cleanup(session);
        agentSessions.delete(sid);
      }
    }
    return { ok: true };
  }

  // ── Start agent loop (SSE stream) ──────────────────────────
  const body = await readBody<AgentBody>(event);
  if (
    !body?.sessionId ||
    !body.messages ||
    !body.systemPrompt ||
    !body.providerType ||
    !body.apiKey ||
    !body.model
  ) {
    throw createError({
      statusCode: 400,
      message:
        'Missing required fields: sessionId, messages, systemPrompt, providerType, apiKey, model',
    });
  }

  const normalizedBaseURL = normalizeOptionalBaseURL(body.baseURL);
  if (body.providerType === 'openai-compat' && !normalizedBaseURL) {
    throw createError({
      statusCode: 400,
      message: 'OpenAI-compatible provider requires baseURL',
    });
  }

  // Diagnostic logging for the cross-provider empty-response bug.
  // When a provider returns a 200 OK + message_start + immediate
  // stream close (0 content blocks), the failure is silent at the
  // provider edge. This log captures the ACTUAL upstream shape —
  // NOT the raw body fields — because the server applies several
  // transformations between reading the body and issuing the
  // upstream request:
  //
  //   1. teamMode && concurrency >= 2 appends
  //      `buildTeamCapabilitiesPrompt(concurrency)` to systemPrompt
  //   2. teamMode auto-registers the `spawn_member` tool on top of
  //      whatever the client sent in `toolDefs`
  //   3. Prior messages are filtered to `role in {user, assistant}
  //      && typeof content === 'string'` before being seeded; the
  //      LAST message becomes the new-turn prompt
  //   4. `registerToolSchema` only sends `parameters` (with $schema
  //      stripped), not the full `ToolDef`, so tool-schema size is
  //      computed from `parameters` alone
  //
  // This block mirrors all four transformations so the logged
  // numbers match what the native agent runtime actually sends to
  // the provider edge.
  //
  // Gated by a hard-coded constant so flipping it off is one line.
  const OUTER_AGENT_LOG_ENABLED = true;
  if (OUTER_AGENT_LOG_ENABLED) {
    const concurrency = body.concurrency ?? 1;

    // (1) Effective system prompt — mirrors teamSystemPrompt logic below.
    const effectiveSystemPrompt =
      body.teamMode && concurrency >= 2
        ? (body.systemPrompt ?? '') + buildTeamCapabilitiesPrompt(concurrency)
        : (body.systemPrompt ?? '');

    // (3) Seeded prior messages — same filter as seedMessages /
    // seedTeamMessages below.
    const allMessages = body.messages ?? [];
    const newPromptRaw = allMessages[allMessages.length - 1]?.content;
    const newPromptChars = typeof newPromptRaw === 'string' ? newPromptRaw.length : 0;
    const priorMessages = allMessages
      .slice(0, -1)
      .filter(
        (m) => (m.role === 'user' || m.role === 'assistant') && typeof m.content === 'string',
      );
    const priorMessageChars = priorMessages.reduce(
      (sum, m) => sum + (m.content as string).length,
      0,
    );

    // (2, 4) Effective tool count + on-wire schema bytes.
    //
    // On-wire tool list in team mode includes up to THREE classes
    // of additions on top of the client-supplied body.toolDefs:
    //
    //   a) `spawn_member` — registered ONLY when body.teamMode===true
    //      via `registerToolSchema(tools, 'spawn_member', SPAWN_MEMBER_SCHEMA)`.
    //
    //   b) `delegate` — registered by `teamRegisterDelegate(team)`,
    //      which runs whenever `body.teamMode || normalizedMembers.length`
    //      (i.e. any team-mode branch). This is a NATIVE runtime-side
    //      registration inside `team.registerDelegateTool()` in
    //      packages/agent-native/src/team.zig. The schema it registers
    //      has a fixed shape: `{type:"object", properties:{member_id,
    //      task}, required:[member_id,task]}` — 159 bytes as the
    //      `input_schema` parameters blob. I was missing this
    //      entirely in the previous log.
    //
    //   c) Member-specific tools registered via addTeamMember() when
    //      normalizedMembers.length > 0. Each member has its OWN
    //      tool registry and those tools are NOT on the leader's
    //      on-wire payload, so they don't count toward the leader
    //      request shape we log here.
    //
    // Tool schemas are serialized as JSON.stringify(parameters) with
    // `$schema` stripped — see registerToolSchema call below. We
    // mirror that transform here so the log matches the bytes the
    // native runtime actually pushes over the wire.
    const toolDefsChars = (body.toolDefs ?? []).reduce((sum, t) => {
      const params = t.parameters ? { ...(t.parameters as Record<string, unknown>) } : {};
      delete (params as Record<string, unknown>).$schema;
      return sum + JSON.stringify(params).length;
    }, 0);

    // Delegate schema text, verbatim from team.zig::registerDelegateTool.
    // Hard-coded here rather than imported because it lives inside a
    // Zig function body and isn't exported. Keep in sync if the Zig
    // side is ever edited (the unit test coverage in team.zig catches
    // drift on that side; this side is a diagnostic log only).
    const DELEGATE_INPUT_SCHEMA =
      '{"type":"object","properties":{"member_id":{"type":"string","description":"ID of the team member to delegate to"},"task":{"type":"string","description":"Task description for the member"}},"required":["member_id","task"]}';
    // The team-mode branch below is gated on `body.teamMode ||
    // normalizedMembers.length`. `normalizedMembers` is derived from
    // `body.members` 1:1 (same length, just adds a normalized
    // baseURL field), so the raw count matches. normalizedMembers
    // itself is computed AFTER this log block, so we use body.members
    // directly to predict whether the team branch will be taken.
    const teamModeBranch = !!(body.teamMode || (body.members ?? []).length);

    let effectiveToolCount = (body.toolDefs ?? []).length;
    let effectiveToolChars = toolDefsChars;
    if (body.teamMode) {
      effectiveToolCount += 1;
      effectiveToolChars += SPAWN_MEMBER_SCHEMA.length;
    }
    if (teamModeBranch) {
      effectiveToolCount += 1;
      effectiveToolChars += DELEGATE_INPUT_SCHEMA.length;
    }

    console.log(
      `[agent-request] provider=${body.providerType} model=${body.model} teamMode=${!!body.teamMode} concurrency=${concurrency} ` +
        `effectiveSystemPrompt=${effectiveSystemPrompt.length} ` +
        `newPromptChars=${newPromptChars} ` +
        `seededPriorMessages=${priorMessages.length}(totalChars=${priorMessageChars}) ` +
        `effectiveTools=${effectiveToolCount}(onWireSchemaChars=${effectiveToolChars}) ` +
        `members=${(body.members ?? []).length} maxOutputTokens=${body.maxOutputTokens ?? 'default'}`,
    );
  }

  // Validate all member baseURLs upfront before allocating any native handles
  const normalizedMembers = (body.members ?? []).map((m) => {
    try {
      return { ...m, normalizedBaseURL: normalizeMemberBaseURL(m.id, m.providerType, m.baseURL) };
    } catch (err: any) {
      throw createError({ statusCode: 400, message: err.message });
    }
  });

  const provider = createProviderHandle(
    body.providerType,
    body.apiKey,
    body.model,
    normalizedBaseURL,
    body.maxContextTokens,
  );
  const tools = createToolRegistry();
  for (const def of body.toolDefs ?? []) {
    const params = def.parameters ? { ...def.parameters } : { type: 'object' };
    delete (params as any).$schema;
    registerToolSchema(tools, def.name, JSON.stringify(params));
  }

  const prompt = body.messages[body.messages.length - 1]?.content ?? '';

  let session: AgentSession;

  if (body.teamMode || normalizedMembers.length) {
    const concurrency = body.concurrency ?? 1;
    console.info(`[agent] creating team (teamMode=${!!body.teamMode}, concurrency=${concurrency})`);

    // Append team capabilities to system prompt when teamMode
    const teamSystemPrompt =
      body.teamMode && concurrency >= 2
        ? body.systemPrompt + buildTeamCapabilitiesPrompt(concurrency)
        : body.systemPrompt;

    const team = createTeam(
      provider,
      tools,
      teamSystemPrompt,
      body.maxTurns ?? 20,
      body.maxOutputTokens,
    );

    const memberHandles: Array<{
      provider: ReturnType<typeof createProviderHandle>;
      tools: ReturnType<typeof createToolRegistry>;
    }> = [];

    // Legacy path: pre-configured members from client
    if (normalizedMembers.length) {
      for (const m of normalizedMembers) {
        const memberProvider = createProviderHandle(
          m.providerType,
          m.apiKey,
          m.model,
          m.normalizedBaseURL,
        );
        const memberTools = createToolRegistry();
        addTeamMember(team, m.id, memberProvider, memberTools, m.systemPrompt ?? '', 20);
        memberHandles.push({ provider: memberProvider, tools: memberTools });
      }
    }

    // Register spawn_member + delegate tools when teamMode
    if (body.teamMode) {
      registerToolSchema(tools, 'spawn_member', SPAWN_MEMBER_SCHEMA);
    }
    teamRegisterDelegate(team);

    // Seed prior conversation history onto the lead engine
    const priorMessages = body.messages
      .slice(0, -1)
      .filter(
        (m) => (m.role === 'user' || m.role === 'assistant') && typeof m.content === 'string',
      );
    if (priorMessages.length > 0) {
      seedTeamMessages(team, JSON.stringify(priorMessages));
    }

    session = createSession({
      team,
      provider,
      tools,
      memberHandles,
      createdAt: Date.now(),
      lastActivity: Date.now(),
    });
  } else {
    // Single engine mode
    const engine = createQueryEngine({
      provider,
      tools,
      systemPrompt: body.systemPrompt,
      maxTurns: body.maxTurns ?? 20,
      maxOutputTokens: body.maxOutputTokens,
      cwd: process.cwd(),
    });

    // Seed conversation history
    const priorMessages = body.messages
      .slice(0, -1)
      .filter(
        (m) => (m.role === 'user' || m.role === 'assistant') && typeof m.content === 'string',
      );
    if (priorMessages.length > 0) {
      seedMessages(engine, JSON.stringify(priorMessages));
    }

    session = createSession({
      engine,
      provider,
      tools,
      createdAt: Date.now(),
      lastActivity: Date.now(),
    });
  }

  // Register session for tool result callbacks and abort
  agentSessions.set(body.sessionId, session);

  setResponseHeaders(event, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    Connection: 'keep-alive',
  });

  const encoder = new TextEncoder();
  const stream = new ReadableStream({
    async start(controller) {
      // Keep pings active for the full orchestration. Delegated tool calls can
      // legitimately stall visible output for >10s while the model waits.
      const pingTimer = startSSEKeepAlive(() => {
        controller.enqueue(encoder.encode(': ping\n\n'));
        touchSession(session);
      }, 5_000);

      let iter;
      try {
        iter = session.team
          ? await runTeam(session.team, prompt)
          : await submitMessage(session.engine!, prompt);
        session.iter = iter;

        let raw: string | null;
        let eventCount = 0;
        while ((raw = await nextEvent(iter)) !== null) {
          eventCount++;
          session.lastActivity = Date.now();
          // Log first 5 events and any tool_use/result events for diagnostics
          if (eventCount <= 5 || raw.includes('tool_use') || raw.includes('"result"')) {
            const preview = raw.length > 200 ? raw.substring(0, 200) + '...' : raw;
            console.info(`[agent] event #${eventCount}: ${preview}`);
          }

          if (session.team) {
            try {
              const evt = JSON.parse(raw);

              // ── spawn_member intercept ──
              if (evt.tool_use && evt.tool_use.name === 'spawn_member') {
                const toolUseId = evt.tool_use.id;
                const inputData =
                  typeof evt.tool_use.input === 'string'
                    ? JSON.parse(evt.tool_use.input)
                    : evt.tool_use.input;
                const memberId: string = inputData?.id;
                const role: string = inputData?.role;
                const memberModel: string | undefined = inputData?.model;

                if (!memberId || !role || !ROLE_TOOL_PRESETS[role]) {
                  resolveTeamToolResult(
                    session.team,
                    toolUseId,
                    JSON.stringify({
                      success: false,
                      error: `Invalid spawn_member args: id=${memberId}, role=${role}`,
                    }),
                  );
                  continue;
                }

                // Check duplicate
                if (session.memberRoles.has(memberId)) {
                  resolveTeamToolResult(
                    session.team,
                    toolUseId,
                    JSON.stringify({
                      success: false,
                      error: `Member "${memberId}" already exists`,
                    }),
                  );
                  continue;
                }

                // Create provider (use member model or lead's)
                const mProvider = createProviderHandle(
                  body.providerType,
                  body.apiKey,
                  memberModel ?? body.model,
                  normalizedBaseURL,
                  body.maxContextTokens,
                );

                // Create tool registry with role preset
                const mTools = createToolRegistry();
                const allDefs = getAllToolDefs();
                const presetNames = ROLE_TOOL_PRESETS[role];
                for (const name of presetNames) {
                  const def = allDefs.find((d) => d.name === name);
                  if (def) {
                    const params = def.parameters ? { ...def.parameters } : { type: 'object' };
                    delete (params as any).$schema;
                    registerToolSchema(mTools, name, JSON.stringify(params));
                  }
                }

                // Build member system prompt with role skills
                const memberPrompt = buildMemberSystemPrompt(
                  role,
                  body.designMdContent,
                  body.hasVariables,
                );

                addTeamMember(session.team, memberId, mProvider, mTools, memberPrompt, 20);
                if (!session.memberHandles) session.memberHandles = [];
                session.memberHandles.push({ provider: mProvider, tools: mTools });
                session.memberRoles.set(memberId, role);

                resolveTeamToolResult(
                  session.team,
                  toolUseId,
                  JSON.stringify({
                    success: true,
                    member_id: memberId,
                    role,
                    tools: presetNames,
                  }),
                );
                continue;
              }

              // ── delegate intercept (enhanced with member tool routing) ──
              if (evt.tool_use && evt.tool_use.name === 'delegate') {
                const toolUseId = evt.tool_use.id;
                let memberIdRaw: string | undefined;
                let taskRaw: string | undefined;

                const inputData = evt.tool_use.input;
                if (typeof inputData === 'string') {
                  try {
                    const parsed = JSON.parse(inputData);
                    memberIdRaw = parsed.member_id;
                    taskRaw = parsed.task;
                  } catch {
                    /* fallback below */
                  }
                } else if (inputData && typeof inputData === 'object') {
                  memberIdRaw = inputData.member_id;
                  taskRaw = inputData.task;
                }

                if (memberIdRaw && taskRaw) {
                  // Fire-and-forget: run member in parallel. The Zig engine blocks in
                  // waiting_for_external_tools until ALL delegate results are resolved.
                  // By not awaiting, multiple delegates run concurrently.
                  runDelegateMember(
                    session,
                    body,
                    controller,
                    encoder,
                    toolUseId,
                    memberIdRaw,
                    taskRaw,
                  ).catch((err) => {
                    console.error(`[agent] delegate ${memberIdRaw} failed:`, err);
                    try {
                      resolveTeamToolResult(
                        session.team!,
                        toolUseId,
                        JSON.stringify({ result: `Error: ${err?.message ?? String(err)}` }),
                      );
                    } catch {
                      /* ignore */
                    }
                  });
                  continue;
                }
              }
            } catch {
              /* not JSON or not intercepted — fall through to normal forwarding */
            }
          }

          if (!session.team) {
            try {
              const evt = JSON.parse(raw);
              if (evt.tool_use?.id && evt.tool_use?.name) {
                const toolUseId = evt.tool_use.id as string;
                const toolName = evt.tool_use.name as string;
                session.toolNames.set(toolUseId, toolName);

                const syntheticResult = shouldShortCircuitPlanLayout(
                  session,
                  toolName,
                  evt.tool_use.input,
                );
                if (syntheticResult && session.engine) {
                  resolveToolResult(session.engine, toolUseId, JSON.stringify(syntheticResult));
                  session.toolNames.delete(toolUseId);
                  controller.enqueue(
                    encoder.encode(
                      `event: tool_result\ndata: ${JSON.stringify({
                        type: 'tool_result',
                        id: toolUseId,
                        name: toolName,
                        result: syntheticResult,
                      })}\n\n`,
                    ),
                  );
                  continue;
                }
              }
            } catch {
              /* ignore parse errors and forward raw event */
            }
          }

          const sse = zigEventToSSE(raw);
          if (sse) controller.enqueue(encoder.encode(sse));
        }
        console.info(`[agent] stream ended after ${eventCount} events`);
      } catch (err: any) {
        console.error(`[agent] stream error:`, err?.message ?? String(err));
        try {
          controller.enqueue(
            encoder.encode(
              `event: error\ndata: ${JSON.stringify({ type: 'error', message: err?.message ?? String(err), fatal: true })}\n\n`,
            ),
          );
        } catch {
          /* ignore */
        }
      } finally {
        clearInterval(pingTimer);
        agentSessions.delete(body.sessionId);
        cleanup(session);
        try {
          controller.close();
        } catch {
          /* ignore */
        }
      }
    },
  });

  return new Response(stream);
});
