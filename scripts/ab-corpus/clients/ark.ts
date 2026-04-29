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
  // Kimi-K2.6 on Ark CP is markedly more brownout-prone than GLM-5.1
  // — ab-v5 (2026-05-02) cut its T arm garbage rate to 17% from
  // ab-v2's 42%, but it still contributed 9/14 of the T arm garbage
  // and ARK's empty-content / 120s-timeout were the only failure
  // modes. Bump the kimi branch to retries=3 (extra 4000ms backoff
  // attempt) + timeoutMs=180000 (60s headroom) so slow-but-eventually-
  // OK responses don't get clipped. GLM-5.1 keeps the existing
  // retries=2 + 120s defaults — its garbage rate is already <2%,
  // pushing those would just burn budget on healthy calls.
  const isKimi = /^kimi/i.test(args.model);
  return callOpenAICompat({
    baseURL: ARK_BASE_URL,
    apiKey,
    model: args.model,
    system: args.system,
    user: args.user,
    temperature: args.temperature,
    maxTokens: args.maxTokens,
    label: 'ark',
    retries: isKimi ? 3 : 2,
    timeoutMs: isKimi ? 180_000 : undefined,
  });
}
