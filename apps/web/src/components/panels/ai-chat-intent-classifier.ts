import { consumeSSEAsText } from '@/services/ai/ai-service';

/** Intent 分类提示——轻量级 LLM 调用来确定消息路由 */
const CLASSIFY_PROMPT = `You are a UI design tool assistant. Classify the user's message intent.
Reply with EXACTLY one of these tags, nothing else:
- DESIGN_NEW — user wants to create or generate a NEW design, screen, page, or component from scratch
- DESIGN_MODIFY — user wants to modify, adjust, refine, or iterate on an EXISTING design (e.g. change colors, resize, restyle, add/remove elements)
- CHAT — user is asking a question, seeking help, or having a conversation`;

export type DesignIntent = 'new' | 'modify' | 'chat';

/** Classify 通过轻量级 LLM 调用而不是硬编码关键字匹配来表达用户意图 */
export async function classifyIntent(
  text: string,
  model: string,
  provider?: string,
): Promise<{ intent: DesignIntent }> {
  // Builtin 提供程序无法使用 /api/ai/generate （它不解析内置 API 密钥）。 Use 改为基于关键字的分类。
  if (model.startsWith('builtin:') || provider === 'builtin') {
    return classifyByKeywords(text);
  }

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 8_000);

    const response = await fetch('/api/ai/generate', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'text/event-stream',
      },
      body: JSON.stringify({
        system: CLASSIFY_PROMPT,
        message: text,
        model,
        provider,
      }),
      signal: controller.signal,
    });
    clearTimeout(timeout);

    if (!response.ok) throw new Error('classify failed');
    const resultText = await consumeSSEAsText(response);
    const upper = resultText.trim().toUpperCase();

    if (upper.includes('DESIGN_MODIFY')) return { intent: 'modify' };
    if (upper.includes('DESIGN_NEW') || upper.includes('DESIGN')) return { intent: 'new' };
    if (upper.includes('CHAT')) return { intent: 'chat' };
    return { intent: 'new' };
  } catch {
    // Fallback：在设计工具中，默认为新设计模式
    return { intent: 'new' };
  }
}

const MODIFY_KEYWORDS =
  /\b(change|modify|update|adjust|resize|move|restyle|refine|fix|tweak|edit|replace|remove|delete|add to|smaller|larger|bigger|wider|taller)\b/i;
const CHAT_KEYWORDS = /\b(what is|how do|explain|tell me|help|why|can you|question|describe)\b/i;

function classifyByKeywords(text: string): { intent: DesignIntent } {
  if (CHAT_KEYWORDS.test(text) && !MODIFY_KEYWORDS.test(text)) return { intent: 'chat' };
  if (MODIFY_KEYWORDS.test(text)) return { intent: 'modify' };
  return { intent: 'new' };
}
