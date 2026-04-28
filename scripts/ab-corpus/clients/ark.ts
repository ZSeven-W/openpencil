/**
 * 方舟 (Volcengine Ark) Coding Plan wrapper. One API key covers
 * multiple third-party models hosted on Ark's coding tier —
 * currently GLM-5.1 and KIMI-K2.6. Cribbed from builtin-provider-
 * presets.ts `ark-coding` preset.
 *
 * Default baseURL: https://ark.cn-beijing.volces.com/api/coding/v3
 * Override via `ARK_BASE_URL` if Volcengine ever ships a regional
 * variant (none published as of 2026-04-22).
 *
 * Key env var: `ARK_CODING_KEY` — Volcengine's UUID-format ARK
 * access key. NOT committed to the repo; export in your shell
 * before running `bun scripts/ab-corpus/run.ts --live`.
 */

import type { ChatCallResult } from './openai-compat';
import { callOpenAICompat } from './openai-compat';

const ARK_BASE_URL = process.env.ARK_BASE_URL ?? 'https://ark.cn-beijing.volces.com/api/coding/v3';
const ARK_API_KEY_ENV = 'ARK_CODING_KEY';

export interface CallArkArgs {
  model: string;
  system: string;
  user: string;
  temperature?: number;
  maxTokens?: number;
}

export async function callArk(args: CallArkArgs): Promise<ChatCallResult> {
  const apiKey = process.env[ARK_API_KEY_ENV];
  if (!apiKey) {
    throw new Error(
      `${ARK_API_KEY_ENV} not set — export it (Volcengine 方舟 CP UUID key) before running with --live`,
    );
  }
  return callOpenAICompat({
    baseURL: ARK_BASE_URL,
    apiKey,
    model: args.model,
    system: args.system,
    user: args.user,
    temperature: args.temperature,
    maxTokens: args.maxTokens,
    label: 'ark',
    // ab-v2 (2026-04-28) saw kimi-k2.6 garbage rate hit 17.5% — every
    // failure was Ark returning empty `choices[0].message.content` or
    // exceeding the 120s wall clock, not a model-quality issue (the
    // model itself routed to the right element tool 80% of the time
    // when it did respond). One retry is the minimum that flips most
    // of those into successes; bumping higher would mostly waste
    // budget on the genuinely broken minority.
    retries: 1,
  });
}
