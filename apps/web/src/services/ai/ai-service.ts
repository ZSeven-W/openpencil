import type { AIStreamChunk } from './ai-types';
import type { AIModelInfo } from '@/stores/ai-store';
import {
  DEFAULT_GENERATE_TIMEOUT_MS,
  DEFAULT_STREAM_HARD_TIMEOUT_MS,
  DEFAULT_STREAM_NO_TEXT_TIMEOUT_MS,
  STREAM_TIMEOUT_MIN_MS,
} from './ai-runtime-config';
import {
  estimateChatPayloadChars,
  formatChatPayloadTooLargeError,
  MAX_CHAT_REQUEST_CHARS,
} from './context-optimizer';

interface StreamChatOptions {
  hardTimeoutMs?: number;
  noTextTimeoutMs?: number;
  /**
   * `thinking` 事件是否应当重置“长时间无文本输出”的超时。
   * 默认为 `true`，保持向后兼容。
   * 如果你希望快速失败，可以设为 `false`，
   * 这样模型即便还在思考，也不会阻止无文本超时触发。
   */
  thinkingResetsTimeout?: boolean;
  /**
   * keep-alive `ping` 是否应当重置无文本超时。
   * 默认为 `true`，保持向后兼容。
   * 设为 `false` 可以避免服务器只发 ping 时无限等待。
   */
  pingResetsTimeout?: boolean;
  /**
   * 最长等待第一个非空文本 token 的时间。
   * 这个超时与 keep-alive ping / thinking 块无关。
   */
  firstTextTimeoutMs?: number;
  /**
   * 控制提供方的思考模式：
   * - `adaptive`：由模型自行决定思考深度
   * - `disabled`：关闭扩展思考，优先更快地返回首个文本
   * - `enabled`：显式开启扩展思考
   */
  thinkingMode?: 'adaptive' | 'disabled' | 'enabled';
  /** 思考预算，仅在 `thinkingMode === 'enabled'` 时使用。 */
  thinkingBudgetTokens?: number;
  /** 模型 effort 等级，通常 `low` 会更快。 */
  effort?: 'low' | 'medium' | 'high' | 'max';
}

/**
 * 以流式方式消费服务端 AI 端点返回的聊天响应。
 * 服务端会把请求路由到对应的 provider SDK，
 * 因此客户端不需要直接持有这些 provider 的密钥。
 */
export async function* streamChat(
  systemPrompt: string,
  messages: Array<{
    role: 'user' | 'assistant';
    content: string;
    attachments?: Array<{ name: string; mediaType: string; data: string }>;
  }>,
  model?: string,
  options?: StreamChatOptions,
  provider?: string,
  abortSignal?: AbortSignal,
): AsyncGenerator<AIStreamChunk> {
  const hardTimeoutMs = Math.max(
    STREAM_TIMEOUT_MIN_MS,
    options?.hardTimeoutMs ?? DEFAULT_STREAM_HARD_TIMEOUT_MS,
  );
  const noTextTimeoutMs = Math.max(
    STREAM_TIMEOUT_MIN_MS,
    options?.noTextTimeoutMs ?? DEFAULT_STREAM_NO_TEXT_TIMEOUT_MS,
  );
  const thinkingResetsTimeout = options?.thinkingResetsTimeout ?? true;
  const pingResetsTimeout = options?.pingResetsTimeout ?? true;
  const firstTextTimeoutMs = options?.firstTextTimeoutMs
    ? Math.max(STREAM_TIMEOUT_MIN_MS, options.firstTextTimeoutMs)
    : null;

  const controller = new AbortController();
  let abortReason: 'hard_timeout' | 'no_text_timeout' | 'first_text_timeout' | null = null;
  let noTextTimeout: ReturnType<typeof setTimeout> | null = null;
  let firstTextTimeout: ReturnType<typeof setTimeout> | null = null;
  let sawText = false;

  const clearNoTextTimeout = () => {
    if (noTextTimeout) {
      clearTimeout(noTextTimeout);
      noTextTimeout = null;
    }
  };

  const clearFirstTextTimeout = () => {
    if (firstTextTimeout) {
      clearTimeout(firstTextTimeout);
      firstTextTimeout = null;
    }
  };

  const resetActivityTimeout = () => {
    clearNoTextTimeout();
    noTextTimeout = setTimeout(() => {
      abortReason = 'no_text_timeout';
      controller.abort();
    }, noTextTimeoutMs);
  };

  const hardTimeout = setTimeout(() => {
    abortReason = 'hard_timeout';
    controller.abort();
  }, hardTimeoutMs);

  if (firstTextTimeoutMs) {
    firstTextTimeout = setTimeout(() => {
      if (sawText) return;
      abortReason = 'first_text_timeout';
      controller.abort();
    }, firstTextTimeoutMs);
  }

  resetActivityTimeout();

  try {
    const fetchSignal = abortSignal
      ? AbortSignal.any([controller.signal, abortSignal])
      : controller.signal;

    // 对 builtin provider，额外从代理设置里补上 API Key 和配置。
    let builtinFields: Record<string, unknown> = {};
    if (provider === 'builtin') {
      const { useAgentSettingsStore } = await import('@/stores/agent-settings-store');
      const { useAIStore } = await import('@/stores/ai-store');
      const currentModel = useAIStore.getState().model;
      if (currentModel.startsWith('builtin:')) {
        const bpId = currentModel.split(':')[1];
        const bp = useAgentSettingsStore.getState().builtinProviders.find((p) => p.id === bpId);
        if (bp) {
          builtinFields = {
            builtinApiKey: bp.apiKey,
            builtinBaseURL: bp.baseURL,
            builtinType: bp.type,
          };
        }
      }
    }

    const requestPayload = {
      system: systemPrompt,
      messages: messages.map((m) => ({
        role: m.role,
        content: m.content,
        ...(m.attachments?.length ? { attachments: m.attachments } : {}),
      })),
      model,
      provider,
      thinkingMode: options?.thinkingMode,
      thinkingBudgetTokens: options?.thinkingBudgetTokens,
      effort: options?.effort,
      ...builtinFields,
    };

    const payloadChars = estimateChatPayloadChars(requestPayload);
    if (payloadChars > MAX_CHAT_REQUEST_CHARS) {
      yield {
        type: 'error',
        content: formatChatPayloadTooLargeError(payloadChars),
      };
      clearTimeout(hardTimeout);
      clearNoTextTimeout();
      clearFirstTextTimeout();
      return;
    }

    const response = await fetch('/api/ai/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(requestPayload),
      signal: fetchSignal,
    });

    if (!response.ok) {
      const errBody = await response.text();
      yield { type: 'error', content: `Server error: ${response.status} ${errBody}` };
      clearTimeout(hardTimeout);
      clearNoTextTimeout();
      clearFirstTextTimeout();
      return;
    }

    // 服务端返回了 JSON，而不是 SSE 流；按错误响应处理。
    const contentType = response.headers.get('content-type') ?? '';
    if (contentType.includes('application/json')) {
      const body = await response.text();
      try {
        const jsonBody = JSON.parse(body);
        yield {
          type: 'error',
          content:
            jsonBody.error || jsonBody.message || `Unexpected JSON response: ${body.slice(0, 200)}`,
        };
      } catch {
        yield { type: 'error', content: `Unexpected server response: ${body.slice(0, 200)}` };
      }
      clearTimeout(hardTimeout);
      clearNoTextTimeout();
      clearFirstTextTimeout();
      return;
    }

    const reader = response.body?.getReader();
    if (!reader) {
      yield { type: 'error', content: 'No response stream available' };
      clearTimeout(hardTimeout);
      clearNoTextTimeout();
      clearFirstTextTimeout();
      return;
    }

    const decoder = new TextDecoder();
    let buffer = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        if (buffer.trim().length > 0) {
          // 剩余缓冲区可能是非 SSE 响应，例如一段 JSON 错误。
          try {
            const jsonErr = JSON.parse(buffer.trim());
            if (jsonErr.error) {
              yield { type: 'error', content: jsonErr.error } as AIStreamChunk;
            }
          } catch {
            // 不是 JSON，就忽略这段尾部内容。
          }
        }
        break;
      }

      buffer += decoder.decode(value, { stream: true });

      // 从缓冲区中解析 SSE 事件
      const lines = buffer.split('\n');
      buffer = lines.pop() ?? '';

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6).trim();
          if (!data) continue;
          try {
            const chunk = JSON.parse(data) as AIStreamChunk;

            if (chunk.type === 'done') {
              clearTimeout(hardTimeout);
              clearNoTextTimeout();
              clearFirstTextTimeout();
              try {
                await reader.cancel();
              } catch {
                // 忽略取消读取时的异常
              }
              return;
            }

            // 服务端 keep-alive ping：只重置活跃超时，不向上层产出 chunk。
            if (chunk.type === 'ping') {
              if (pingResetsTimeout) {
                resetActivityTimeout();
              }
              continue;
            }

            if (chunk.type === 'thinking' && !chunk.content) {
              continue;
            }

            // 任何非空文本都算“有活动”。
            // `thinking` 是否重置超时，则取决于 `thinkingResetsTimeout`。
            if (chunk.type === 'text' && chunk.content.trim().length > 0) {
              sawText = true;
              clearFirstTextTimeout();
              resetActivityTimeout();
            } else if (
              chunk.type === 'thinking' &&
              chunk.content.trim().length > 0 &&
              thinkingResetsTimeout
            ) {
              // 主动思考也意味着模型没有“卡死”，
              // 所以可以让“首个文本迟迟未到”的看门狗先退下。
              // 真正的兜底仍由 `noTextTimeout` 和 `hardTimeout` 提供。
              clearFirstTextTimeout();
              resetActivityTimeout();
            }

            yield chunk;
            if (chunk.type === 'error') {
              clearTimeout(hardTimeout);
              clearNoTextTimeout();
              clearFirstTextTimeout();
              try {
                await reader.cancel();
              } catch {
                // 忽略取消读取时的异常
              }
              return;
            }
          } catch {
            // 跳过格式错误的行
          }
        }
      }
    }

    // 处理最后剩下的缓冲区内容
    if (buffer.startsWith('data: ')) {
      const data = buffer.slice(6).trim();
      if (data) {
        try {
          const chunk = JSON.parse(data) as AIStreamChunk;
          if (chunk.type === 'done') {
            clearTimeout(hardTimeout);
            clearNoTextTimeout();
            clearFirstTextTimeout();
            return;
          }
          if (chunk.type === 'thinking' && !chunk.content) {
            clearTimeout(hardTimeout);
            clearNoTextTimeout();
            clearFirstTextTimeout();
            return;
          }
          if (chunk.type === 'text' && chunk.content.trim().length > 0) {
            sawText = true;
            clearFirstTextTimeout();
          }
          clearTimeout(hardTimeout);
          clearNoTextTimeout();
          clearFirstTextTimeout();
          yield chunk;
          if (chunk.type === 'error') {
            return;
          }
        } catch {
          // 跳过空块
        }
      }
    }
  } catch (error) {
    // 外部中止信号触发的用户主动停止
    if (abortSignal?.aborted && !abortReason) {
      clearTimeout(hardTimeout);
      clearNoTextTimeout();
      clearFirstTextTimeout();
      return;
    }

    if (controller.signal.aborted) {
      if (abortReason === 'no_text_timeout') {
        yield {
          type: 'error',
          content: 'AI has been thinking too long without output. Request stopped, please retry.',
        };
      } else if (abortReason === 'hard_timeout') {
        yield {
          type: 'error',
          content: 'AI request timed out. Please retry.',
        };
      } else if (abortReason === 'first_text_timeout') {
        yield {
          type: 'error',
          content:
            'AI spent too long thinking without producing output. Request stopped, please retry.',
        };
      } else {
        yield {
          type: 'error',
          content: 'AI request was aborted.',
        };
      }
      clearTimeout(hardTimeout);
      clearNoTextTimeout();
      clearFirstTextTimeout();
      return;
    }

    const message = error instanceof Error ? error.message : 'Unknown error occurred';
    yield { type: 'error', content: message };
  } finally {
    clearTimeout(hardTimeout);
    clearNoTextTimeout();
    clearFirstTextTimeout();
  }
}

/**
 * 消费一个 SSE 端点，并把完整文本结果拼接出来。
 * 适用于不需要逐 chunk 处理的调用方。
 */
export async function consumeSSEAsText(response: Response): Promise<string> {
  if (!response.body) throw new Error('No response body');
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  let accumulated = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    const chunks = buffer.split('\n\n');
    buffer = chunks.pop() ?? '';
    for (const chunk of chunks) {
      const dataMatch = chunk.match(/^data:\s*(.+)$/m);
      if (!dataMatch) continue;
      try {
        const evt = JSON.parse(dataMatch[1]);
        if (evt.type === 'text') accumulated += evt.content;
        if (evt.type === 'error') throw new Error(evt.content);
      } catch (e) {
        if (e instanceof Error && e.message !== dataMatch[1]) throw e;
      }
    }
  }
  return accumulated;
}

/**
 * 面向设计 / 代码生成场景的非流式 completion 调用。
 * 服务端会把请求路由到合适的 provider SDK。
 */
export async function generateCompletion(
  systemPrompt: string,
  userMessage: string,
  model?: string,
  provider?: string,
): Promise<string> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), DEFAULT_GENERATE_TIMEOUT_MS);

  try {
    const response = await fetch('/api/ai/generate', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'text/event-stream',
      },
      body: JSON.stringify({ system: systemPrompt, message: userMessage, model, provider }),
      signal: controller.signal,
    });
    if (!response.ok) throw new Error(`Server error: ${response.status}`);
    return await consumeSSEAsText(response);
  } catch (error) {
    if (controller.signal.aborted) {
      throw new Error('AI generation request timed out. Please retry.');
    }
    throw error;
  } finally {
    clearTimeout(timeout);
  }
}

/**
 * 从服务端拉取可用模型列表。
 * 当前由服务端去查询 Claude Agent SDK 支持的模型集合。
 */
export async function fetchAvailableModels(): Promise<AIModelInfo[]> {
  try {
    const response = await fetch('/api/ai/models');
    if (!response.ok) return [];
    const data = await response.json();
    return data.models ?? [];
  } catch {
    return [];
  }
}
