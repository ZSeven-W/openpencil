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
import type { AuthLevel } from '../../../src/types/agent';
import { agentSessions, cleanup, abortSession, createSession, type AgentSession } from '../../utils/agent-sessions';
import { resolveSkills } from '@zseven-w/pen-ai-skills';
import type { Phase } from '@zseven-w/pen-ai-skills';
import { getAllToolDefs } from '../../../src/services/ai/agent-tools';

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
  designer: ['batch_get', 'snapshot_layout', 'find_empty_space', 'generate_design', 'insert_node'],
  reviewer: ['batch_get', 'snapshot_layout', 'get_selection'],
  editor: ['batch_get', 'snapshot_layout', 'find_empty_space', 'update_node', 'delete_node', 'insert_node'],
  researcher: ['batch_get', 'snapshot_layout', 'find_empty_space', 'get_selection'],
};

const ROLE_SKILL_PHASE: Record<string, Phase> = {
  designer: 'generation',
  reviewer: 'validation',
  editor: 'maintenance',
  researcher: 'planning',
};

const ROLE_TOOL_INSTRUCTIONS: Record<string, string> = {
  designer: `You are a design team member. When asked to create designs, you MUST call the generate_design tool with a descriptive prompt. You can also use insert_node for manual node creation, batch_get and snapshot_layout to inspect the canvas, and find_empty_space to find placement locations.`,
  reviewer: `You are a design reviewer. Use batch_get and snapshot_layout to inspect the current canvas state. Use get_selection to see what the user has selected. Provide detailed feedback on layout, spacing, typography, and visual hierarchy.`,
  editor: `You are a design editor. Use batch_get and snapshot_layout to understand the current canvas. Use update_node to modify node properties, delete_node to remove elements, and insert_node to add new elements. Use find_empty_space to find placement locations.`,
  researcher: `You are a design researcher. Use batch_get and snapshot_layout to analyze the current canvas state. Use find_empty_space to identify available space. Use get_selection to see what the user has selected. Provide analysis and recommendations.`,
};

function buildTeamCapabilitiesPrompt(concurrency: number): string {
  return `\n\n## Team Capabilities
You can spawn up to ${concurrency} team members and delegate tasks to them.

Available roles:
- designer: Creates designs (tools: generate_design, insert_node, batch_get, snapshot_layout, find_empty_space)
- reviewer: Validates designs (tools: batch_get, snapshot_layout, get_selection)
- editor: Modifies existing designs (tools: update_node, delete_node, insert_node, batch_get, snapshot_layout, find_empty_space)
- researcher: Reads canvas and plans (tools: batch_get, snapshot_layout, find_empty_space, get_selection)

Use spawn_member({id, role}) to create a member, then delegate({member_id, task}) to assign work.
For simple tasks, handle them yourself without spawning members.
For complex multi-section designs, spawn designers and delegate sections to them.
Each member has its own design knowledge and tools — describe the task clearly.`;
}

function buildMemberSystemPrompt(role: string, designMdContent?: string, hasVariables?: boolean): string {
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
  if (evt.stream_event) {
    tag = evt.stream_event.type ?? 'unknown';
    data = evt.stream_event;
  } else if (evt.result) {
    tag = 'result';
    data = evt.result;
  } else if (evt.tool_progress) {
    tag = 'tool_progress';
    data = evt.tool_progress;
  } else {
    // Flat format fallback (shouldn't happen with current Zig serialization)
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
    case 'content_block_start':
      if (data.tool_name) {
        mapped = {
          type: 'tool_call',
          id: data.tool_use_id ?? data.id,
          name: data.tool_name,
          args: typeof data.tool_input === 'string' ? JSON.parse(data.tool_input as string) : (data.tool_input ?? {}),
          level: TOOL_LEVEL_MAP[data.tool_name as string] ?? 'read',
        };
      } else {
        mapped = { type: tag, ...data };
      }
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

function createProviderHandle(
  providerType: 'anthropic' | 'openai-compat',
  apiKey: string,
  model: string,
  baseURL?: string,
) {
  return providerType === 'anthropic'
    ? createAnthropicProvider(apiKey, model, baseURL)
    : createOpenAICompatProvider(apiKey, baseURL!, model);
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
    setResponseHeaders(event, { 'Content-Type': 'application/json' });
    return {
      error:
        'Missing required fields: sessionId, messages, systemPrompt, providerType, apiKey, model',
    };
  }

  const provider = createProviderHandle(body.providerType, body.apiKey, body.model, body.baseURL);
  const tools = createToolRegistry();
  for (const def of body.toolDefs ?? []) {
    const params = def.parameters ? { ...def.parameters } : { type: 'object' };
    delete (params as any).$schema;
    registerToolSchema(tools, def.name, JSON.stringify(params));
  }

  const prompt = body.messages[body.messages.length - 1]?.content ?? '';

  let session: AgentSession;

  if (body.teamMode || body.members?.length) {
    const concurrency = body.concurrency ?? 1;
    console.info(`[agent] creating team (teamMode=${!!body.teamMode}, concurrency=${concurrency})`);

    // Append team capabilities to system prompt when teamMode
    const teamSystemPrompt = body.teamMode && concurrency >= 2
      ? body.systemPrompt + buildTeamCapabilitiesPrompt(concurrency)
      : body.systemPrompt;

    const team = createTeam(provider, tools, teamSystemPrompt, body.maxTurns ?? 20);

    const memberHandles: Array<{ provider: ReturnType<typeof createProviderHandle>; tools: ReturnType<typeof createToolRegistry> }> = [];

    // Legacy path: pre-configured members from client
    if (body.members?.length) {
      for (const m of body.members) {
        const memberProvider = createProviderHandle(m.providerType, m.apiKey, m.model, m.baseURL);
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
    const priorMessages = body.messages.slice(0, -1).filter(
      (m) => (m.role === 'user' || m.role === 'assistant') && typeof m.content === 'string',
    );
    if (priorMessages.length > 0) {
      seedTeamMessages(team, JSON.stringify(priorMessages));
    }

    session = createSession({
      team, provider, tools, memberHandles,
      createdAt: Date.now(), lastActivity: Date.now(),
    });
  } else {
    // Single engine mode
    const engine = createQueryEngine({
      provider,
      tools,
      systemPrompt: body.systemPrompt,
      maxTurns: body.maxTurns ?? 20,
      cwd: process.cwd(),
    });

    // Seed conversation history
    const priorMessages = body.messages.slice(0, -1).filter(
      (m) => (m.role === 'user' || m.role === 'assistant') && typeof m.content === 'string',
    );
    if (priorMessages.length > 0) {
      seedMessages(engine, JSON.stringify(priorMessages));
    }

    session = createSession({
      engine, provider, tools,
      createdAt: Date.now(), lastActivity: Date.now(),
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
      const pingTimer = setInterval(() => {
        try {
          controller.enqueue(encoder.encode(': ping\n\n'));
        } catch {
          /* stream already closed */
        }
      }, 5_000);

      let iter;
      try {
        iter = session.team
          ? await runTeam(session.team, prompt)
          : await submitMessage(session.engine!, prompt);
        session.iter = iter;

        let raw: string | null;
        while ((raw = await nextEvent(iter)) !== null) {
          session.lastActivity = Date.now();

          if (session.team) {
            try {
              const evt = JSON.parse(raw);

              // ── spawn_member intercept ──
              if (evt.tool_use && evt.tool_use.name === 'spawn_member') {
                const toolUseId = evt.tool_use.id;
                const inputData = typeof evt.tool_use.input === 'string'
                  ? JSON.parse(evt.tool_use.input) : evt.tool_use.input;
                const memberId: string = inputData?.id;
                const role: string = inputData?.role;
                const memberModel: string | undefined = inputData?.model;

                if (!memberId || !role || !ROLE_TOOL_PRESETS[role]) {
                  resolveTeamToolResult(session.team, toolUseId, JSON.stringify({
                    success: false, error: `Invalid spawn_member args: id=${memberId}, role=${role}`,
                  }));
                  continue;
                }

                // Check duplicate
                if (session.memberRoles.has(memberId)) {
                  resolveTeamToolResult(session.team, toolUseId, JSON.stringify({
                    success: false, error: `Member "${memberId}" already exists`,
                  }));
                  continue;
                }

                // Create provider (use member model or lead's)
                const mProvider = createProviderHandle(
                  body.providerType, body.apiKey, memberModel ?? body.model, body.baseURL,
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
                  role, body.designMdContent, body.hasVariables,
                );

                addTeamMember(session.team, memberId, mProvider, mTools, memberPrompt, 20);
                if (!session.memberHandles) session.memberHandles = [];
                session.memberHandles.push({ provider: mProvider, tools: mTools });
                session.memberRoles.set(memberId, role);

                resolveTeamToolResult(session.team, toolUseId, JSON.stringify({
                  success: true, member_id: memberId, role, tools: presetNames,
                }));
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
                  } catch { /* fallback below */ }
                } else if (inputData && typeof inputData === 'object') {
                  memberIdRaw = inputData.member_id;
                  taskRaw = inputData.task;
                }

                if (memberIdRaw && taskRaw) {
                  // Resolve task-specific skills based on member role
                  const memberRole = session.memberRoles.get(memberIdRaw);
                  let enrichedTask = taskRaw;
                  if (memberRole) {
                    const phase = ROLE_SKILL_PHASE[memberRole] ?? 'generation';
                    const taskSkills = resolveSkills(phase, taskRaw, {
                      flags: {
                        hasDesignMd: !!body.designMdContent,
                        hasVariables: !!body.hasVariables,
                      },
                    });
                    const skillPrefix = taskSkills.skills.map((s) => s.content).join('\n\n');
                    if (skillPrefix) enrichedTask = skillPrefix + '\n\n' + taskRaw;
                  }

                  controller.enqueue(encoder.encode(
                    `event: member_start\ndata: ${JSON.stringify({ type: 'member_start', memberId: memberIdRaw, task: taskRaw })}\n\n`,
                  ));

                  let memberResult = '';
                  const memberIter = await runTeamMember(session.team, memberIdRaw, enrichedTask);
                  try {
                    let memberRaw: string | null;
                    while ((memberRaw = await nextEvent(memberIter)) !== null) {
                      session.lastActivity = Date.now();
                      try {
                        const mEvt = JSON.parse(memberRaw);

                        // Member tool_use → record owner, forward with source
                        if (mEvt.tool_use) {
                          const mToolId = mEvt.tool_use.id;
                          session.toolOwners.set(mToolId, memberIdRaw!);
                          // Forward as tool_call with source field
                          const level = TOOL_LEVEL_MAP[mEvt.tool_use.name as string] ?? 'read';
                          const toolCallEvt = {
                            type: 'tool_call',
                            id: mToolId,
                            name: mEvt.tool_use.name,
                            args: typeof mEvt.tool_use.input === 'string'
                              ? JSON.parse(mEvt.tool_use.input as string)
                              : (mEvt.tool_use.input ?? {}),
                            level,
                            source: memberIdRaw,
                          };
                          controller.enqueue(encoder.encode(
                            `event: tool_call\ndata: ${JSON.stringify(toolCallEvt)}\n\n`,
                          ));
                          continue;
                        }

                        // Collect text
                        if (mEvt.stream_event?.text && mEvt.stream_event.type === 'text_delta') {
                          memberResult += mEvt.stream_event.text;
                        }
                      } catch { /* ignore parse errors */ }
                      controller.enqueue(encoder.encode(zigEventToSSE(memberRaw)));
                    }
                  } finally {
                    destroyIterator(memberIter);
                    // Clean up any remaining toolOwner entries for this member
                    for (const [tid, mid] of session.toolOwners) {
                      if (mid === memberIdRaw) session.toolOwners.delete(tid);
                    }
                  }

                  controller.enqueue(encoder.encode(
                    `event: member_end\ndata: ${JSON.stringify({ type: 'member_end', memberId: memberIdRaw, result: '' })}\n\n`,
                  ));

                  resolveTeamToolResult(
                    session.team,
                    toolUseId,
                    JSON.stringify({ result: memberResult || 'Member completed task.' }),
                  );
                  continue;
                }
              }
            } catch { /* not JSON or not intercepted — fall through to normal forwarding */ }
          }

          controller.enqueue(encoder.encode(zigEventToSSE(raw)));
        }
      } catch (err: any) {
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
