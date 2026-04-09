import { defineEventHandler, readBody, setResponseHeaders } from 'h3';
import { writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { resolveClaudeCli } from '../../utils/resolve-claude-cli';
import { runCodexExec } from '../../utils/codex-client';
import { startSSEKeepAlive } from '../../utils/sse-keepalive';
import {
  buildClaudeAgentEnv,
  buildSpawnClaudeCodeProcess,
  getClaudeAgentDebugFilePath,
  resolveAgentModel,
} from '../../utils/resolve-claude-agent-env';
import { normalizeOptionalBaseURL, requireOpenAICompatBaseURL } from './provider-url';
// SENSITIVE_LOG_PATTERN + readDebugTail are now canonical in @zseven-w/pen-mcp.
// Re-export here to keep existing consumers (tests, other modules) working.
import { SENSITIVE_LOG_PATTERN, readDebugTail } from '@zseven-w/pen-mcp';
export { SENSITIVE_LOG_PATTERN };

/** Allowed media types for image attachments */
export const ALLOWED_MEDIA_TYPES = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp']);

/** Resolve file extension from media type, falling back to 'png' for disallowed types */
export function resolveMediaExtension(mediaType: string): string {
  return ALLOWED_MEDIA_TYPES.has(mediaType) ? mediaType.split('/')[1] : 'png';
}

interface ChatAttachmentWire {
  name: string;
  mediaType: string;
  data: string; // base64
}

interface ChatBody {
  system: string;
  messages: Array<{
    role: 'user' | 'assistant';
    content: string;
    attachments?: ChatAttachmentWire[];
  }>;
  model?: string;
  provider?: 'anthropic' | 'openai' | 'opencode' | 'copilot' | 'gemini' | 'builtin';
  thinkingMode?: 'adaptive' | 'disabled' | 'enabled';
  thinkingBudgetTokens?: number;
  effort?: 'low' | 'medium' | 'high' | 'max';
  /** For builtin provider: direct API key (not CLI-based) */
  builtinApiKey?: string;
  /** For builtin provider: API root base URL (e.g. https://api.openai.com/v1) */
  builtinBaseURL?: string;
  /** For builtin provider: 'anthropic' or 'openai-compat' */
  builtinType?: 'anthropic' | 'openai-compat';
}

function buildClaudeExitHint(rawError: string, debugTail?: string[]): string | undefined {
  if (!/process exited with code 1/i.test(rawError)) return undefined;

  const hints: string[] = [];

  if (debugTail && debugTail.length > 0) {
    const text = debugTail.join('\n');
    if (
      /Failed to save config with lock: Error: EPERM|operation not permitted, .*\.claude\.json/i.test(
        text,
      )
    ) {
      hints.push(
        'Claude Code cannot write ~/.claude.json (permission denied). ' +
          'On Windows, try running as Administrator or manually create the file: echo {} > %USERPROFILE%\\.claude.json',
      );
    }
    if (
      /Connection error|Could not resolve host|Failed to connect|ECONNREFUSED|ETIMEDOUT/i.test(text)
    ) {
      hints.push(
        'Upstream API connection failed. Check proxy/DNS/network reachability to your ANTHROPIC_BASE_URL.',
      );
    }
    if (/ANTHROPIC_CUSTOM_HEADERS present: false, has Authorization header: false/i.test(text)) {
      hints.push(
        'No API auth header detected. Run "claude login" to authenticate, ' +
          'or set ANTHROPIC_API_KEY in ~/.claude/settings.json ' +
          '(env: { "ANTHROPIC_API_KEY": "sk-..." }).',
      );
    }
    if (/invalid.*api.?key|unauthorized|401|authentication/i.test(text)) {
      hints.push(
        'API key authentication failed. Verify your ANTHROPIC_API_KEY is correct and has not expired.',
      );
    }
    if (/ENOTFOUND|getaddrinfo/i.test(text)) {
      hints.push(
        'DNS resolution failed for the API endpoint. Check your ANTHROPIC_BASE_URL is correct.',
      );
    }
    if (/certificate|CERT_|ssl|tls/i.test(text)) {
      hints.push(
        'TLS/SSL certificate error. If using a corporate proxy, set NODE_TLS_REJECT_UNAUTHORIZED=0 in ~/.claude/settings.json env (not recommended for production).',
      );
    }
  }

  // Detect proxy/custom endpoint — most common cause of exit-code-1 on Windows
  const env = process.env;
  const hasProxy = !!(env.http_proxy || env.https_proxy || env.HTTP_PROXY || env.HTTPS_PROXY);
  const hasCustomBaseUrl = !!env.ANTHROPIC_BASE_URL;
  if ((hasProxy || hasCustomBaseUrl) && !env.NODE_TLS_REJECT_UNAUTHORIZED) {
    hints.push(
      'Proxy or custom ANTHROPIC_BASE_URL detected but NODE_TLS_REJECT_UNAUTHORIZED is not set. ' +
        'If your proxy uses a self-signed or corporate certificate, add ' +
        '"NODE_TLS_REJECT_UNAUTHORIZED": "0" to the env section of ~/.claude/settings.json.',
    );
  }

  // If no debug info available, provide generic Windows guidance
  if (hints.length === 0) {
    const isWin = process.platform === 'win32';
    if (isWin) {
      hints.push(
        'Claude Code process crashed on Windows. Common fixes: ' +
          '(1) Ensure ~/.claude.json exists: echo {} > %USERPROFILE%\\.claude.json ' +
          '(2) Check ANTHROPIC_API_KEY and ANTHROPIC_BASE_URL in ~/.claude/settings.json ' +
          '(3) If using a proxy, set NODE_TLS_REJECT_UNAUTHORIZED=0 in env.',
      );
    } else {
      return undefined;
    }
  }

  return `${rawError}\n${hints.join('\n')}`;
}

/**
 * Streaming chat endpoint.
 * Routes to the appropriate provider SDK based on the `provider` field.
 * Requires explicit provider and model; no fallback routing.
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<ChatBody>(event);

  if (!body?.messages || body?.system == null) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' });
    return { error: 'Missing required fields: system, messages' };
  }
  if (!body.provider) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' });
    return { error: 'Missing provider. Provider fallback is disabled.' };
  }
  if (!body.model?.trim()) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' });
    return { error: 'Missing model. Model fallback is disabled.' };
  }
  if (
    body.provider !== 'anthropic' &&
    body.provider !== 'openai' &&
    body.provider !== 'opencode' &&
    body.provider !== 'copilot' &&
    body.provider !== 'gemini' &&
    body.provider !== 'builtin'
  ) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' });
    return { error: 'Missing or unsupported provider. Provider fallback is disabled.' };
  }

  setResponseHeaders(event, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    Connection: 'keep-alive',
  });

  if (body.provider === 'builtin') return streamViaBuiltin(body);
  if (body.provider === 'anthropic') return streamViaAgentSDK(body, body.model);
  if (body.provider === 'opencode') return streamViaOpenCode(body, body.model);
  if (body.provider === 'copilot') return streamViaCopilot(body, body.model);
  if (body.provider === 'gemini') return streamViaGemini(body, body.model);
  return streamViaCodex(body, body.model);
});

// Keep-alive ping interval (ms) — must stay below Bun's 10s idle timeout,
// but shouldn't be so aggressive that long-lived nested SSE streams create
// unnecessary write pressure on Bun dev.
const KEEPALIVE_INTERVAL_MS = 5_000;
function getAgentThinkingConfig(
  body: ChatBody,
): { type: 'adaptive' | 'disabled' } | { type: 'enabled'; budgetTokens?: number } | undefined {
  if (!body.thinkingMode) return undefined;
  if (body.thinkingMode === 'enabled') {
    return { type: 'enabled', budgetTokens: body.thinkingBudgetTokens };
  }
  return { type: body.thinkingMode };
}

/**
 * Save base64 attachments to temp files. Returns { tempDir, files[] } — caller must clean up tempDir.
 *
 * When `insideProject` is true, files are saved under `.openpencil-tmp/` in the
 * current working directory so that Claude Code Agent SDK (which restricts reads
 * to the project directory in plan mode) can access them.
 */
async function saveAttachmentsToTempFiles(
  attachments: ChatAttachmentWire[],
  insideProject = false,
): Promise<{ tempDir: string; files: string[] }> {
  let tempDir: string;
  if (insideProject) {
    const { mkdirSync, chmodSync } = await import('node:fs');
    const baseDir = join(process.cwd(), '.openpencil-tmp');
    mkdirSync(baseDir, { recursive: true, mode: 0o700 });
    chmodSync(baseDir, 0o700);
    tempDir = await mkdtemp(join(baseDir, 'attach-'));
  } else {
    tempDir = await mkdtemp(join(tmpdir(), 'openpencil-attach-'));
  }
  const files: string[] = [];
  for (const att of attachments) {
    const ext = resolveMediaExtension(att.mediaType);
    const filePath = join(tempDir, `${files.length}.${ext}`);
    await writeFile(filePath, Buffer.from(att.data, 'base64'));
    files.push(filePath);
  }
  return { tempDir, files };
}

/** Collect all attachments from the last user message */
function getLastUserAttachments(body: ChatBody): ChatAttachmentWire[] {
  const lastUser = [...body.messages].reverse().find((m) => m.role === 'user');
  return lastUser?.attachments ?? [];
}

/**
 * Strip "NEVER use tools" and similar instructions from system prompt
 * when we need Claude Code Agent SDK to use its Read tool for image analysis.
 */
function stripNoToolsRestriction(systemPrompt: string): string {
  return systemPrompt.replace(/^.*NEVER use tools.*$/gim, '').replace(/\n{3,}/g, '\n\n');
}

/** Stream via Claude Agent SDK (uses local Claude Code OAuth login, no API key needed) */
function streamViaAgentSDK(body: ChatBody, requestedModel?: string) {
  let activeQuery: { close(): void } | undefined;
  let cancelled = false;
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder();
      const safeEnqueue = (payload: Record<string, unknown>) => {
        if (cancelled) return false;
        try {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify(payload)}\n\n`));
          return true;
        } catch {
          cancelled = true;
          return false;
        }
      };
      const safeClose = () => {
        if (cancelled) return;
        cancelled = true;
        try {
          controller.close();
        } catch {
          /* already closed */
        }
      };
      // Keep emitting pings for the full stream lifetime. Some providers pause
      // for >10s between text deltas, and Bun will otherwise kill the SSE socket.
      const pingTimer = startSSEKeepAlive(() => {
        safeEnqueue({ type: 'ping', content: '' });
      }, KEEPALIVE_INTERVAL_MS);
      let debugFile: string | undefined;
      let attachTempDir: string | undefined;

      try {
        const { query } = await import('@anthropic-ai/claude-agent-sdk');

        // Build prompt from the last user message
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user');
        let prompt = lastUserMsg?.content ?? '';

        // If the last user message has image attachments, save to temp files
        // inside the project directory so Claude Code has read permission.
        const attachments = getLastUserAttachments(body);
        const hasImageAttachments = attachments.length > 0;
        if (hasImageAttachments) {
          const saved = await saveAttachmentsToTempFiles(attachments, true);
          attachTempDir = saved.tempDir;
          const imageRefs = saved.files
            .map(
              (f) =>
                `First, use the Read tool to read the image file at "${f}". Then analyze it and respond to the user.`,
            )
            .join('\n');
          prompt = imageRefs + '\n\n' + (prompt || 'Describe what you see in the image.');
        }

        // Remove CLAUDECODE env to allow running from within a CC terminal
        const env = buildClaudeAgentEnv();
        debugFile = getClaudeAgentDebugFilePath();

        // When using a custom proxy (ANTHROPIC_BASE_URL), skip explicit model
        // so Claude Code uses ANTHROPIC_MODEL from env — the proxy may not
        // recognize standard Claude model IDs.
        const model = resolveAgentModel(requestedModel, env);

        const claudePath = resolveClaudeCli();
        const spawnProcess = buildSpawnClaudeCodeProcess();
        const thinking = getAgentThinkingConfig(body);

        // When images are attached, strip the "NEVER use tools" restriction from
        // the system prompt so Claude Code will use its Read tool to view images.
        const effectiveSystemPrompt = hasImageAttachments
          ? stripNoToolsRestriction(body.system)
          : body.system;

        // When images are attached, use result-based flow (like validate.ts):
        // let Claude Code read the image via its Read tool internally, then
        // only emit the final result text. This avoids streaming intermediate
        // tool-use preamble like "I need to read the file first".
        if (hasImageAttachments) {
          const runImageQuery = async (): Promise<string> => {
            const q = query({
              prompt,
              options: {
                systemPrompt: effectiveSystemPrompt,
                ...(model ? { model } : {}),
                maxTurns: 3,
                plugins: [],
                permissionMode: 'plan',
                persistSession: false,
                ...(body.effort ? { effort: body.effort } : {}),
                ...(thinking ? { thinking } : {}),
                env,
                ...(debugFile ? { debugFile } : {}),
                ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
                ...(spawnProcess ? { spawnClaudeCodeProcess: spawnProcess } : {}),
              },
            });
            activeQuery = q;

            try {
              for await (const message of q) {
                if (cancelled) return '';
                if (message.type === 'result') {
                  const isErrorResult =
                    'is_error' in message && Boolean((message as { is_error?: boolean }).is_error);
                  if (message.subtype === 'success' && !isErrorResult) {
                    return message.result ?? '';
                  }
                  const errors = 'errors' in message ? (message.errors as string[]) : [];
                  const resultText = 'result' in message ? String(message.result ?? '') : '';
                  const errContent =
                    errors.join('; ') || resultText || `Query ended with: ${message.subtype}`;
                  throw new Error(errContent);
                }
              }
              return '';
            } finally {
              activeQuery = undefined;
              q.close();
            }
          };

          const resultText = await runImageQuery();

          if (resultText) {
            safeEnqueue({ type: 'text', content: resultText });
          }
        } else {
          // Normal text-only chat: stream partial messages as before
          const runQuery = async () => {
            const q = query({
              prompt,
              options: {
                systemPrompt: effectiveSystemPrompt,
                ...(model ? { model } : {}),
                maxTurns: 1,
                includePartialMessages: true,
                tools: [],
                plugins: [],
                permissionMode: 'plan',
                persistSession: false,
                ...(body.effort ? { effort: body.effort } : {}),
                ...(thinking ? { thinking } : {}),
                env,
                ...(debugFile ? { debugFile } : {}),
                ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
                ...(spawnProcess ? { spawnClaudeCodeProcess: spawnProcess } : {}),
              },
            });
            activeQuery = q;

            try {
              for await (const message of q) {
                if (cancelled) return;
                if (message.type === 'stream_event') {
                  const ev = message.event;
                  if (ev.type === 'content_block_delta') {
                    if (ev.delta.type === 'text_delta') {
                      safeEnqueue({ type: 'text', content: ev.delta.text });
                    } else if (ev.delta.type === 'thinking_delta') {
                      safeEnqueue({
                        type: 'thinking',
                        content: (ev.delta as any).thinking,
                      });
                    }
                  }
                } else if (message.type === 'result') {
                  const isErrorResult =
                    'is_error' in message && Boolean((message as { is_error?: boolean }).is_error);
                  if (message.subtype !== 'success' || isErrorResult) {
                    const errors = 'errors' in message ? (message.errors as string[]) : [];
                    const resultText = 'result' in message ? String(message.result ?? '') : '';
                    const content =
                      errors.join('; ') || resultText || `Query ended with: ${message.subtype}`;
                    safeEnqueue({ type: 'error', content });
                  }
                }
              }
            } finally {
              activeQuery = undefined;
              q.close();
            }
          };

          await runQuery();
        }

        safeEnqueue({ type: 'done', content: '' });
      } catch (error) {
        const rawContent = error instanceof Error ? error.message : 'Unknown error';

        const tail = await readDebugTail(debugFile);

        const hintedContent = buildClaudeExitHint(rawContent, tail);
        // Append debug log tail so the user can see what Claude Code actually reported
        let content = hintedContent ?? rawContent;
        if (tail && tail.length > 0 && /process exited with code/i.test(rawContent)) {
          const debugSnippet = tail.slice(-10).join('\n');
          content += `\n\n[Debug log]:\n${debugSnippet}`;
        }
        safeEnqueue({ type: 'error', content });
      } finally {
        clearInterval(pingTimer);
        try {
          activeQuery?.close();
        } catch {
          /* ignore */
        }
        activeQuery = undefined;
        if (attachTempDir) {
          rm(attachTempDir, { recursive: true, force: true }).catch(() => {});
        }
        safeClose();
      }
    },
    cancel() {
      cancelled = true;
      try {
        activeQuery?.close();
      } catch {
        /* ignore */
      }
      activeQuery = undefined;
    },
  });

  return new Response(stream);
}

/** Error name → user-friendly label mapping */
const OPENCODE_ERROR_LABELS: Record<string, string> = {
  APIError: 'API error',
  ProviderAuthError: 'Authentication failed',
  UnknownError: 'Unknown error',
  MessageOutputLengthError: 'Response too long',
  MessageAbortedError: 'Request aborted',
  StructuredOutputError: 'Output format error',
  ContextOverflowError: 'Context too long',
};

/**
 * Extract a human-readable message from an OpenCode error object.
 * Handles structured errors like { name: "APIError", data: { message: "..." } }
 * and nested JSON in message strings.
 */
export function formatOpenCodeError(error: unknown): string {
  if (!error) return 'Unknown error';
  if (typeof error === 'string') return error;

  const err = error as Record<string, any>;

  // Structured OpenCode error: { name, data: { message, ... } }
  if (err.name && err.data?.message) {
    const label = OPENCODE_ERROR_LABELS[err.name] ?? err.name;
    let msg: string = err.data.message;

    // Try to extract nested error message from JSON in the message string
    // e.g. 'Unauthorized: {"error":{"code":"invalid_api_key","message":"invalid access token"}}'
    const jsonStart = msg.indexOf('{');
    if (jsonStart > 0) {
      try {
        const nested = JSON.parse(msg.slice(jsonStart));
        const nestedMsg = nested?.error?.message ?? nested?.message;
        if (nestedMsg) {
          const prefix = msg.slice(0, jsonStart).replace(/:\s*$/, '').trim();
          msg = prefix ? `${prefix}: ${nestedMsg}` : nestedMsg;
        }
      } catch {
        /* not JSON, use as-is */
      }
    }

    return `${label} — ${msg}`;
  }

  // Plain { message } object
  if (err.message) return err.message;

  // Fallback: truncated JSON
  const json = JSON.stringify(error);
  return json.length > 200 ? json.slice(0, 200) + '…' : json;
}

/** Parse an OpenCode model string ("providerID/modelID") into its parts */
function parseOpenCodeModel(model?: string): { providerID: string; modelID: string } | undefined {
  if (!model || !model.includes('/')) return undefined;
  const idx = model.indexOf('/');
  return { providerID: model.slice(0, idx), modelID: model.slice(idx + 1) };
}

// Note: OpenCode SDK does not support `reasoning` in promptAsync/prompt params.
// The `reasoning` field was silently dropped by buildClientParams. Removed.

/** Wrap an async generator with a timeout — yields values until timeout fires */
async function* streamWithTimeout<T>(
  stream: AsyncGenerator<T>,
  timeoutPromise: Promise<{ done: true; value: undefined }>,
): AsyncGenerator<T> {
  while (true) {
    const result = (await Promise.race([stream.next(), timeoutPromise])) as IteratorResult<T>;
    if (result.done) break;
    yield result.value;
  }
}

function streamViaCodex(body: ChatBody, model?: string) {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder();
      const pingTimer = startSSEKeepAlive(() => {
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`),
        );
      }, KEEPALIVE_INTERVAL_MS);

      let attachTempDir: string | undefined;
      try {
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user');
        const prompt = lastUserMsg?.content ?? '';

        // Save image attachments to temp files for Codex CLI
        const attachments = getLastUserAttachments(body);
        let imageFiles: string[] | undefined;
        if (attachments.length > 0) {
          const saved = await saveAttachmentsToTempFiles(attachments);
          attachTempDir = saved.tempDir;
          imageFiles = saved.files;
        }

        const result = await runCodexExec(prompt, {
          model,
          systemPrompt: body.system,
          thinkingMode: body.thinkingMode,
          thinkingBudgetTokens: body.thinkingBudgetTokens,
          effort: body.effort,
          imageFiles,
        });

        if (result.error) {
          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify({ type: 'error', content: result.error })}\n\n`),
          );
          return;
        }

        if (result.text) {
          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify({ type: 'text', content: result.text })}\n\n`),
          );
        }

        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'done', content: '' })}\n\n`),
        );
      } catch (error) {
        const content = error instanceof Error ? error.message : 'Unknown error';
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
        );
      } finally {
        clearInterval(pingTimer);
        if (attachTempDir) {
          rm(attachTempDir, { recursive: true, force: true }).catch(() => {});
        }
        controller.close();
      }
    },
  });

  return new Response(stream);
}

/** Stream via OpenCode SDK using event subscription for real-time streaming */
function streamViaOpenCode(body: ChatBody, model?: string) {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder();
      const pingTimer = startSSEKeepAlive(() => {
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`),
        );
      }, KEEPALIVE_INTERVAL_MS);

      let ocServer: { close(): void } | undefined;
      try {
        const { getOpencodeClient } = await import('../../utils/opencode-client');
        const oc = await getOpencodeClient();
        const ocClient = oc.client;
        ocServer = oc.server;

        // Create a session for this conversation
        const { data: session, error: sessionError } = await ocClient.session.create({
          title: 'OpenPencil Chat',
        });
        if (sessionError || !session) {
          throw new Error(
            `Failed to create OpenCode session: ${formatOpenCodeError(sessionError)}`,
          );
        }

        // Inject system prompt as context (no AI reply)
        const { error: sysPromptError } = (await ocClient.session.prompt({
          sessionID: session.id,
          noReply: true,
          parts: [{ type: 'text', text: body.system }],
        })) as any;
        if (sysPromptError) {
          console.error(
            '[AI] OpenCode system prompt injection failed:',
            formatOpenCodeError(sysPromptError),
          );
        }

        // Build prompt from the last user message
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user');
        const prompt = lastUserMsg?.content ?? '';

        const parsed = parseOpenCodeModel(model);
        if (model && !parsed) {
          console.warn(
            `[AI] OpenCode: could not parse model string "${model}", sending without model override`,
          );
        }

        // Build parts array, adding image attachments if present
        const attachments = getLastUserAttachments(body);
        const parts: Array<Record<string, unknown>> = [
          ...attachments.map((a) => ({
            type: 'image',
            url: `data:${a.mediaType};base64,${a.data}`,
          })),
          { type: 'text', text: prompt || 'Analyze these images.' },
        ];

        // Build prompt payload with optional model and reasoning
        const promptPayload: Record<string, unknown> = {
          sessionID: session.id,
          ...(parsed ? { model: parsed } : {}),
          parts,
        };

        // Subscribe to event stream for real-time deltas.
        // IMPORTANT: The SSE connection is lazy — it only connects when
        // iteration starts. We must start consuming BEFORE sending the
        // prompt to avoid a race where events are emitted before the
        // SSE connection is established.
        const eventResult = await ocClient.event.subscribe();
        const eventStream = eventResult.stream;

        const sessionId = session.id;
        const STREAM_TIMEOUT_MS = 180_000;

        // Start eagerly consuming the event stream into a buffer.
        // This triggers the SSE HTTP connection immediately.
        const eventBuffer: unknown[] = [];
        let streamDone = false;
        let notifyFn: (() => void) | null = null;

        const notify = () => {
          if (notifyFn) {
            const fn = notifyFn;
            notifyFn = null;
            fn();
          }
        };

        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        void (async () => {
          const timeoutPromise = new Promise<{ done: true; value: undefined }>((resolve) =>
            setTimeout(() => resolve({ done: true, value: undefined }), STREAM_TIMEOUT_MS),
          );
          try {
            for await (const event of streamWithTimeout(eventStream, timeoutPromise)) {
              eventBuffer.push(event);
              notify();
            }
          } finally {
            streamDone = true;
            notify();
          }
        })();

        // Give the SSE connection a moment to establish before sending prompt
        await new Promise<void>((resolve) => setTimeout(resolve, 100));

        // Now send the prompt — SSE connection should already be active
        const { error: asyncError } = await ocClient.session.promptAsync(promptPayload as any);
        if (asyncError) {
          const detail = formatOpenCodeError(asyncError);
          console.error('[AI] OpenCode promptAsync error:', detail);
          throw new Error(detail);
        }

        // Consume buffered events + wait for new ones
        let emittedText = false;
        let eventCount = 0;
        let shouldBreak = false;

        while (!shouldBreak) {
          // Wait for events if buffer is empty
          if (eventBuffer.length === 0) {
            if (streamDone) break;
            await new Promise<void>((resolve) => {
              notifyFn = resolve;
            });
            continue;
          }

          const event = eventBuffer.shift();
          if (!event || !('type' in (event as any))) continue;

          const eventType = (event as any).type as string;
          eventCount++;

          // Stream text deltas for our session
          if (eventType === 'message.part.delta') {
            const props = (event as any).properties;
            if (props?.sessionID === sessionId && props.field === 'text') {
              const data = JSON.stringify({ type: 'text', content: props.delta });
              controller.enqueue(encoder.encode(`data: ${data}\n\n`));
              emittedText = true;
            }
            // Forward reasoning deltas as thinking chunks
            if (props?.sessionID === sessionId && props.field === 'reasoning') {
              const data = JSON.stringify({ type: 'thinking', content: props.delta });
              controller.enqueue(encoder.encode(`data: ${data}\n\n`));
            }
            continue;
          }

          // Session went idle — response complete
          if (eventType === 'session.idle') {
            const props = (event as any).properties;
            if (props?.sessionID === sessionId) {
              shouldBreak = true;
            }
            continue;
          }

          // Session error
          if (eventType === 'session.error') {
            const props = (event as any).properties;
            if (props?.sessionID === sessionId || !props?.sessionID) {
              const errMsg = formatOpenCodeError(props?.error);
              console.error('[AI] OpenCode session error:', errMsg);
              const data = JSON.stringify({ type: 'error', content: errMsg });
              controller.enqueue(encoder.encode(`data: ${data}\n\n`));
              shouldBreak = true;
            }
            continue;
          }
        }

        // Fallback: if no text was streamed, try reading session messages directly
        if (!emittedText) {
          try {
            const { data: messages } = (await ocClient.session.messages({
              sessionID: sessionId,
            })) as any;
            if (messages && Array.isArray(messages)) {
              // Find the last assistant message (each item has { info, parts })
              const assistantMsg = [...messages]
                .reverse()
                .find((m: any) => m.info?.role === 'assistant');
              if (assistantMsg?.parts) {
                for (const part of assistantMsg.parts) {
                  if (part.type === 'text' && part.text) {
                    const data = JSON.stringify({ type: 'text', content: part.text });
                    controller.enqueue(encoder.encode(`data: ${data}\n\n`));
                    emittedText = true;
                  }
                }
              }
            }
          } catch {
            // fallback failed — will emit error below
          }
        }

        if (!emittedText) {
          const data = JSON.stringify({
            type: 'error',
            content:
              'OpenCode returned an empty response. The model may not have generated any output.',
          });
          controller.enqueue(encoder.encode(`data: ${data}\n\n`));
        }

        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'done', content: '' })}\n\n`),
        );
      } catch (error) {
        const content = error instanceof Error ? error.message : 'Unknown error';
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
        );
      } finally {
        const { releaseOpencodeServer } = await import('../../utils/opencode-client');
        releaseOpencodeServer(ocServer);
        clearInterval(pingTimer);
        controller.close();
      }
    },
  });

  return new Response(stream);
}

/** Map ChatBody effort to Copilot SDK ReasoningEffort */
function mapCopilotReasoningEffort(
  effort?: 'low' | 'medium' | 'high' | 'max',
): 'low' | 'medium' | 'high' | 'xhigh' | undefined {
  if (!effort) return undefined;
  if (effort === 'max') return 'xhigh';
  return effort;
}

/** Stream via Gemini CLI (`gemini -p -o stream-json`) — CLI handles its own auth */
function streamViaGemini(body: ChatBody, model?: string) {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder();
      const pingTimer = startSSEKeepAlive(() => {
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`),
        );
      }, KEEPALIVE_INTERVAL_MS);

      try {
        const { streamGeminiExec } = await import('../../utils/gemini-client');

        // Build prompt from messages
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user');
        const prompt = lastUserMsg?.content ?? '';

        const { stream: geminiStream } = streamGeminiExec(prompt, {
          model,
          systemPrompt: body.system,
        });

        for await (const event of geminiStream) {
          if (event.type === 'text') {
            const data = JSON.stringify({ type: 'text', content: event.content });
            try {
              controller.enqueue(encoder.encode(`data: ${data}\n\n`));
            } catch {
              /* stream closed */
            }
          } else if (event.type === 'error') {
            const data = JSON.stringify({ type: 'error', content: event.content });
            controller.enqueue(encoder.encode(`data: ${data}\n\n`));
          }
          // 'done' is handled after loop
        }

        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'done', content: '' })}\n\n`),
        );
      } catch (error) {
        const content = error instanceof Error ? error.message : 'Unknown error';
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
        );
      } finally {
        clearInterval(pingTimer);
        controller.close();
      }
    },
  });

  return new Response(stream);
}

/** Stream via GitHub Copilot SDK (@github/copilot-sdk) */
function streamViaCopilot(body: ChatBody, model?: string) {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder();
      const pingTimer = startSSEKeepAlive(() => {
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`),
        );
      }, KEEPALIVE_INTERVAL_MS);

      let copilotClient: { stop(): Promise<unknown> } | undefined;
      try {
        const { CopilotClient, approveAll } = await import('@github/copilot-sdk');
        // Use standalone copilot binary to avoid Bun's node:sqlite issue
        const { resolveCopilotCli, resolveCliPathForSdk } =
          await import('../../utils/copilot-client');
        const rawCliPath = resolveCopilotCli();
        // On Windows, .cmd wrappers cause "spawn EINVAL" — resolve to .js entry point
        const cliPath = rawCliPath ? resolveCliPathForSdk(rawCliPath) : undefined;
        const client = new CopilotClient({
          autoStart: true,
          ...(cliPath ? { cliPath } : {}),
        });
        copilotClient = client;
        await client.start();

        const session = await client.createSession({
          ...(model ? { model } : {}),
          streaming: true,
          onPermissionRequest: approveAll,
          systemMessage: { mode: 'replace', content: body.system },
          ...(body.effort ? { reasoningEffort: mapCopilotReasoningEffort(body.effort) } : {}),
        });

        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user');
        const prompt = lastUserMsg?.content ?? '';

        // Subscribe to streaming deltas
        session.on('assistant.message_delta', (event) => {
          const deltaContent = (event as any).data?.deltaContent ?? '';
          if (deltaContent) {
            const data = JSON.stringify({ type: 'text', content: deltaContent });
            try {
              controller.enqueue(encoder.encode(`data: ${data}\n\n`));
            } catch {
              /* stream closed */
            }
          }
        });

        // Wait for completion
        await session.sendAndWait({ prompt }, 120_000);
        await session.destroy();

        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'done', content: '' })}\n\n`),
        );
      } catch (error) {
        const content = error instanceof Error ? error.message : 'Unknown error';
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
        );
      } finally {
        clearInterval(pingTimer);
        if (copilotClient) {
          copilotClient.stop().catch(() => {});
        }
        controller.close();
      }
    },
  });

  return new Response(stream);
}

/**
 * Stream via builtin provider — direct API key, no CLI tool needed.
 * Uses Zig NAPI addon (agent-native) with Anthropic or OpenAI-compatible providers.
 */
function streamViaBuiltin(body: ChatBody) {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder();
      const BUILTIN_EVENT_IDLE_TIMEOUT_MS = 45_000;
      const pingTimer = startSSEKeepAlive(() => {
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`),
        );
      }, KEEPALIVE_INTERVAL_MS);

      try {
        const {
          createAnthropicProvider,
          createOpenAICompatProvider,
          createQueryEngine,
          seedMessages,
          submitMessage,
          nextEvent,
          abortEngine,
          destroyIterator,
          destroyQueryEngine,
          destroyProvider,
        } = await import('@zseven-w/agent-native');

        const apiKey = body.builtinApiKey;
        const rawModel = body.model?.trim() ?? '';
        // Model string may be "builtin:<providerId>:<actualModel>" — extract the actual model name
        const model = rawModel.startsWith('builtin:')
          ? rawModel.split(':').slice(2).join(':')
          : rawModel;
        if (!apiKey || !model) throw new Error('Builtin provider requires apiKey and model');

        const normalizedBuiltinBaseURL = normalizeOptionalBaseURL(body.builtinBaseURL);
        const builtinProvider =
          body.builtinType === 'anthropic'
            ? createAnthropicProvider(apiKey, model, normalizedBuiltinBaseURL)
            : createOpenAICompatProvider(
                apiKey,
                requireOpenAICompatBaseURL(normalizedBuiltinBaseURL),
                model,
              );

        // Pure streaming — no tools, maxTurns=1 prevents agentic looping
        const builtinEngine = createQueryEngine({
          provider: builtinProvider,
          systemPrompt: body.system,
          maxTurns: 1,
          maxOutputTokens: 16384,
          cwd: process.cwd(),
        });

        // Seed prior conversation history for multi-turn context
        const priorMsgs = body.messages
          .slice(0, -1)
          .filter(
            (m: any) =>
              (m.role === 'user' || m.role === 'assistant') && typeof m.content === 'string',
          );
        if (priorMsgs.length > 0) {
          seedMessages(builtinEngine, JSON.stringify(priorMsgs));
        }

        const lastMsg = body.messages[body.messages.length - 1]?.content ?? '';
        const builtinIter = await submitMessage(builtinEngine, lastMsg);

        // Abort engine if no events arrive within 60s (provider sent 200 but no SSE data)
        let gotFirstEvent = false;
        const firstEventTimer = setTimeout(() => {
          if (!gotFirstEvent) {
            console.warn('[builtin] No SSE events received within 60s — aborting engine');
            abortEngine(builtinEngine);
          }
        }, 60_000);

        try {
          let raw: string | null;
          while (
            (raw = await waitForBuiltinEvent(
              nextEvent,
              builtinIter,
              () => abortEngine(builtinEngine),
              BUILTIN_EVENT_IDLE_TIMEOUT_MS,
            )) !== null
          ) {
            if (!gotFirstEvent) {
              gotFirstEvent = true;
              clearTimeout(firstEventTimer);
            }
            const evt = JSON.parse(raw);
            // Zig events are tagged unions: {"stream_event":{...}} or {"result":{...}}
            const se = evt.stream_event;
            if (se?.type === 'text_delta' && se.text) {
              controller.enqueue(
                encoder.encode(`data: ${JSON.stringify({ type: 'text', content: se.text })}\n\n`),
              );
            } else if (se?.type === 'thinking_delta' && se.text) {
              controller.enqueue(
                encoder.encode(
                  `data: ${JSON.stringify({ type: 'thinking', content: se.text })}\n\n`,
                ),
              );
            } else if (evt.result?.is_error) {
              const errMsg = `Provider error: ${evt.result.subtype ?? 'unknown'}`;
              console.error('[builtin]', errMsg);
              controller.enqueue(
                encoder.encode(`data: ${JSON.stringify({ type: 'error', content: errMsg })}\n\n`),
              );
            }
          }

          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify({ type: 'done', content: '' })}\n\n`),
          );
        } finally {
          clearTimeout(firstEventTimer);
          destroyIterator(builtinIter);
          destroyQueryEngine(builtinEngine);
          destroyProvider(builtinProvider);
        }
      } catch (error) {
        const content = error instanceof Error ? error.message : 'Unknown error';
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
        );
      } finally {
        clearInterval(pingTimer);
        controller.close();
      }
    },
  });

  return new Response(stream);
}

async function waitForBuiltinEvent<TIterator>(
  nextEventFn: (iter: TIterator) => Promise<string | null>,
  iter: TIterator,
  onTimeout: () => void,
  timeoutMs: number,
): Promise<string | null> {
  return await new Promise<string | null>((resolve, reject) => {
    const timer = setTimeout(() => {
      try {
        onTimeout();
      } catch {
        /* ignore */
      }
      reject(new Error('Builtin provider stalled without output. Please retry.'));
    }, timeoutMs);

    nextEventFn(iter)
      .then((value) => {
        clearTimeout(timer);
        resolve(value);
      })
      .catch((err) => {
        clearTimeout(timer);
        reject(err);
      });
  });
}
