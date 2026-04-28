/**
 * DeepSeek wrapper. OpenAI-compatible endpoint at api.deepseek.com.
 * Key in `DEEPSEEK_API_KEY`. Latest flagship at time of wiring is
 * `deepseek-v4-pro`; `deepseek-v4-flash` is the lighter variant; the
 * older `deepseek-chat` / `deepseek-reasoner` IDs are deprecated
 * (sunset 2026-07-24) but the harness routes them anyway so corpus
 * comparisons against pre-deprecation runs still work.
 *
 * Docs: https://api-docs.deepseek.com/zh-cn/
 */

import type { ChatCallResult } from './openai-compat';
import { callOpenAICompat } from './openai-compat';

const DEEPSEEK_BASE_URL = process.env.DEEPSEEK_BASE_URL ?? 'https://api.deepseek.com';
const DEEPSEEK_API_KEY_ENV = 'DEEPSEEK_API_KEY';

export interface CallDeepSeekArgs {
  model: string;
  system: string;
  user: string;
  temperature?: number;
  maxTokens?: number;
}

export async function callDeepSeek(args: CallDeepSeekArgs): Promise<ChatCallResult> {
  const apiKey = process.env[DEEPSEEK_API_KEY_ENV];
  if (!apiKey) {
    throw new Error(
      `${DEEPSEEK_API_KEY_ENV} not set — export it before running with --live, or pass --dry-run to use fixtures`,
    );
  }
  return callOpenAICompat({
    baseURL: DEEPSEEK_BASE_URL,
    apiKey,
    model: args.model,
    system: args.system,
    user: args.user,
    temperature: args.temperature,
    maxTokens: args.maxTokens,
    label: 'deepseek',
    // ab-v2 garbage attribution: 6 of 40 deepseek-v4-pro runs returned
    // empty content (server-side flakiness, not model output). Same
    // rationale as ark.ts — one retry recovers the transient ones,
    // anything more would mostly burn budget on real failures.
    retries: 1,
  });
}
