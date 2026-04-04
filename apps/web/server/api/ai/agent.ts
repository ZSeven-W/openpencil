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
} from '@zseven-w/agent-native';
import type { AuthLevel } from '../../../src/types/agent';
import { agentSessions, cleanup, abortSession, type AgentSession } from '../../utils/agent-sessions';

const TOOL_LEVEL_MAP: Record<string, AuthLevel> = {
  batch_get: 'read',
  snapshot_layout: 'read',
  find_empty_space: 'read',
  generate_design: 'create',
  insert_node: 'create',
  update_node: 'modify',
  delete_node: 'delete',
};

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
      if (session.team) {
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

  if (body.members?.length) {
    // Team mode: create team with lead + members
    console.info(`[agent] creating team with ${body.members.length} member(s)`);
    const team = createTeam(provider, tools, body.systemPrompt, body.maxTurns ?? 20);

    const memberHandles: Array<{ provider: ReturnType<typeof createProviderHandle>; tools: ReturnType<typeof createToolRegistry> }> = [];

    for (const m of body.members) {
      const memberProvider = createProviderHandle(m.providerType, m.apiKey, m.model, m.baseURL);
      // Members get an EMPTY tool registry — they output JSONL text directly,
      // not via tool calls. If we registered external tools (generate_design etc.),
      // the member would call them, but tool results can only be resolved on the
      // leader engine, not the member — causing the member to block forever.
      const memberTools = createToolRegistry();
      addTeamMember(team, m.id, memberProvider, memberTools, m.systemPrompt ?? '', 20);
      memberHandles.push({ provider: memberProvider, tools: memberTools });
    }

    // Register delegate tool in leader's registry so the LLM can call
    // delegate({member_id, task}) to dispatch work to members
    teamRegisterDelegate(team);

    session = { team, provider, tools, memberHandles, createdAt: Date.now(), lastActivity: Date.now() };
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

    session = { engine, provider, tools, createdAt: Date.now(), lastActivity: Date.now() };
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

          // Intercept delegate tool_use in team mode — run member engine
          // instead of forwarding to client
          if (session.team) {
            try {
              const evt = JSON.parse(raw);
              if (evt.tool_use && evt.tool_use.name === 'delegate') {
                const toolUseId = evt.tool_use.id;
                let memberIdRaw: string | undefined;
                let taskRaw: string | undefined;

                // Parse delegate args from input (may be JSON string or object)
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
                  // Emit member_start to client
                  controller.enqueue(encoder.encode(
                    `event: member_start\ndata: ${JSON.stringify({ type: 'member_start', memberId: memberIdRaw, task: taskRaw })}\n\n`,
                  ));

                  // Run member engine
                  let memberResult = '';
                  const memberIter = await runTeamMember(session.team, memberIdRaw, taskRaw);
                  try {
                    let memberRaw: string | null;
                    while ((memberRaw = await nextEvent(memberIter)) !== null) {
                      session.lastActivity = Date.now();
                      // Forward member events to client (text, thinking, tool_call, etc.)
                      controller.enqueue(encoder.encode(zigEventToSSE(memberRaw)));
                      // Collect text for delegate tool result
                      try {
                        const mEvt = JSON.parse(memberRaw);
                        if (mEvt.stream_event?.text && mEvt.stream_event.type === 'text_delta') {
                          memberResult += mEvt.stream_event.text;
                        }
                      } catch { /* ignore parse errors */ }
                    }
                  } finally {
                    destroyIterator(memberIter);
                  }

                  // Emit member_end to client
                  controller.enqueue(encoder.encode(
                    `event: member_end\ndata: ${JSON.stringify({ type: 'member_end', memberId: memberIdRaw, result: '' })}\n\n`,
                  ));

                  // Resolve delegate tool result back to leader engine
                  resolveTeamToolResult(
                    session.team,
                    toolUseId,
                    JSON.stringify({ result: memberResult || 'Member completed task.' }),
                  );
                  continue; // skip normal forwarding for this event
                }
              }
            } catch { /* not JSON or not delegate — fall through to normal forwarding */ }
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
