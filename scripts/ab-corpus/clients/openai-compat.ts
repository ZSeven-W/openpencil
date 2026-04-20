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

interface ChatMessage {
  role: 'system' | 'user';
  content: string;
}

interface OpenAICompatResponse {
  choices?: Array<{
    message?: { content?: string };
    finish_reason?: string;
  }>;
  error?: { message?: string; code?: string };
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
}

export async function callOpenAICompat(args: CallOpenAICompatArgs): Promise<string> {
  const label = args.label ?? 'openai-compat';
  const messages: ChatMessage[] = [
    { role: 'system', content: args.system },
    { role: 'user', content: args.user },
  ];
  const res = await fetch(`${args.baseURL}/chat/completions`, {
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
  });
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
  return content;
}
