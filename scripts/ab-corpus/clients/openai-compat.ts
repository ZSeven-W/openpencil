/**
 * Generic OpenAI-compatible chat client. Most Chinese providers (MiniMax,
 * Bailian/DashScope, Zhipu/GLM, Moonshot/Kimi) expose an OpenAI-compatible
 * `/chat/completions` endpoint — we route every one of them through this
 * single helper so the per-provider wrappers stay 3-5 lines each and the
 * harness doesn't hardcode a different fetch shape for every vendor.
 *
 * Treats API errors as fatal for the single call (throws); the harness's
 * loop catches and records them as apply-phase failures so one bad call
 * never aborts a corpus sweep.
 */

import type { TokenUsage } from '@zseven-w/pen-ai-skills';

interface ChatMessage {
  role: 'system' | 'user';
  content: string;
}

interface OpenAICompatResponse {
  choices?: Array<{
    message?: { content?: string };
    finish_reason?: string;
  }>;
  usage?: {
    prompt_tokens?: number;
    completion_tokens?: number;
    total_tokens?: number;
  };
  error?: { message?: string; code?: string };
}

/**
 * Standardized provider response carrying the assistant message plus
 * usage stats. All ab-corpus clients (openai-compat, codex-cli, stub)
 * normalize to this shape so the harness can plumb token counts into
 * ScoreRow without per-client special cases. usage = 0/0 means
 * "provider didn't return usage", not "actually used 0 tokens" —
 * aggregate.avgUsage skips zero rows so codex-cli columns don't
 * appear free.
 */
export interface ChatCallResult {
  content: string;
  usage: TokenUsage;
}

export interface CallOpenAICompatArgs {
  /** Full endpoint base URL, e.g. `https://api.minimaxi.com/v1`. No trailing `/chat/completions`. */
  baseURL: string;
  /** Bearer token. Passed verbatim as `Authorization: Bearer <key>`. */
  apiKey: string;
  /** Provider-specific model id, e.g. `MiniMax-M2.7`, `glm-5`, `kimi-k2.5`. */
  model: string;
  system: string;
  user: string;
  temperature?: number;
  maxTokens?: number;
  /** Label for error messages so "<label> HTTP 401" points at the right provider. */
  label?: string;
  /**
   * Per-call wall-clock timeout in ms. Default 120000 (2 minutes).
   * Override via `AB_CORPUS_CALL_TIMEOUT_MS` env var.
   *
   * Why this matters: without a timeout, a hung provider connection
   * freezes the whole corpus sweep. Happened 2026-04-25 on a 48-run
   * live sweep — 9/12 prompts completed, then one call on Ark Kimi
   * never responded and stalled the remaining 3 prompts + 12 extra
   * runs. With a timeout, the harness records it as a
   * `__HARNESS_ERROR__` garbage run and moves on.
   */
  timeoutMs?: number;
  /**
   * Retry attempts on transient errors. Default 0 (no retry). Set to 2
   * for providers known to flake on empty content / timeout (Ark hosting
   * GLM-5.1+Kimi-K2.6, DeepSeek api.deepseek.com). 1 is enough for
   * MiniMax (saw 4 timeouts in 104 ab-v3 runs); Codex / Bailian don't
   * enable this — their ab-v2 failures were model-quality (truncation,
   * malformed DSL), where retry burns budget for nothing.
   *
   * Retried: empty `choices[0].message.content`, abort/timeout, HTTP 5xx,
   * HTTP 429. NOT retried: HTTP 4xx other than 429 (auth / bad request).
   *
   * Backoff: exponential, 250ms × 4^attempt — 250ms before retry 1,
   * 1000ms before retry 2, 4000ms before retry 3. Linear backoff was
   * too tight when retries=2 (500ms then 750ms is < a typical Ark
   * recovery window); exponential gives the provider room to settle.
   */
  retries?: number;
}

const DEFAULT_TIMEOUT_MS = (() => {
  const env = process.env.AB_CORPUS_CALL_TIMEOUT_MS;
  if (env && /^\d+$/.test(env)) {
    const n = Number(env);
    if (n > 0) return n;
  }
  return 120_000;
})();

export async function callOpenAICompat(args: CallOpenAICompatArgs): Promise<ChatCallResult> {
  const retries = args.retries ?? 0;
  const label = args.label ?? 'openai-compat';
  let lastErr: Error | undefined;
  for (let attempt = 0; attempt <= retries; attempt++) {
    try {
      return await callOpenAICompatOnce(args);
    } catch (err) {
      lastErr = err instanceof Error ? err : new Error(String(err));
      if (attempt === retries || !isTransientError(lastErr)) {
        throw lastErr;
      }
      const delayMs = 250 * 4 ** attempt;
      console.error(
        `[${label}] transient error on attempt ${attempt + 1}/${retries + 1}, retrying in ${delayMs}ms: ${truncate(
          lastErr.message,
          200,
        )}`,
      );
      await sleep(delayMs);
    }
  }
  throw lastErr ?? new Error(`${label}: callOpenAICompat exited loop without result`);
}

async function callOpenAICompatOnce(args: CallOpenAICompatArgs): Promise<ChatCallResult> {
  const label = args.label ?? 'openai-compat';
  const timeoutMs = args.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const messages: ChatMessage[] = [
    { role: 'system', content: args.system },
    { role: 'user', content: args.user },
  ];
  const ctrl = new AbortController();
  const timer = setTimeout(
    () => ctrl.abort(new Error(`${label} call exceeded ${timeoutMs}ms (model=${args.model})`)),
    timeoutMs,
  );
  let res: Response;
  try {
    res = await fetch(`${args.baseURL}/chat/completions`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${args.apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: args.model,
        messages,
        temperature: args.temperature ?? 0.2,
        max_tokens: args.maxTokens ?? 4096,
        stream: false,
      }),
      signal: ctrl.signal,
    });
  } finally {
    clearTimeout(timer);
  }
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`${label} HTTP ${res.status}: ${body.slice(0, 300)}`);
  }
  const data: OpenAICompatResponse = await res.json();
  if (data.error) {
    throw new Error(`${label} API error: ${data.error.message ?? data.error.code ?? 'unknown'}`);
  }
  const content = data.choices?.[0]?.message?.content;
  if (typeof content !== 'string' || content.length === 0) {
    throw new Error(`${label} returned no content in choices[0].message.content`);
  }
  return {
    content,
    usage: {
      promptTokens: typeof data.usage?.prompt_tokens === 'number' ? data.usage.prompt_tokens : 0,
      completionTokens:
        typeof data.usage?.completion_tokens === 'number' ? data.usage.completion_tokens : 0,
    },
  };
}

function isTransientError(err: Error): boolean {
  const msg = err.message ?? '';
  if (/returned no content/.test(msg)) return true;
  if (/exceeded \d+ms/.test(msg)) return true;
  if (/HTTP 5\d\d:/.test(msg)) return true;
  if (/HTTP 429:/.test(msg)) return true;
  if (err.name === 'AbortError') return true;
  return false;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function truncate(s: string, n: number): string {
  return s.length <= n ? s : `${s.slice(0, n)}…`;
}
