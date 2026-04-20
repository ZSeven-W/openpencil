/**
 * 百炼 (Alibaba DashScope Bailian) Coding Plan wrapper. Exposes
 * third-party models — including GLM-5 and Kimi K2.5 — through a
 * single DashScope endpoint, so the same API key (`DASHSCOPE_BAILIAN_CODING_KEY`)
 * covers multiple target models. Cribbed from builtin-provider-presets.ts
 * `bailian-coding` preset.
 *
 * Default baseURL: https://coding.dashscope.aliyuncs.com/v1 (CN).
 * Override via `BAILIAN_BASE_URL` if you need the -intl variant.
 */

import { callOpenAICompat } from './openai-compat';

// User supplied the 百炼 CP (Coding Plan) key, which targets the
// coding-plan endpoint (not the regular compatible-mode endpoint).
// See builtin-provider-presets.ts `bailian-coding` preset for the
// canonical URL — this is separate from plain `bailian` tier.
const BAILIAN_BASE_URL = process.env.BAILIAN_BASE_URL ?? 'https://coding.dashscope.aliyuncs.com/v1';
const BAILIAN_API_KEY_ENV = 'DASHSCOPE_BAILIAN_CODING_KEY';

export interface CallBailianArgs {
  model: string;
  system: string;
  user: string;
  temperature?: number;
  maxTokens?: number;
}

export async function callBailian(args: CallBailianArgs): Promise<string> {
  const apiKey = process.env[BAILIAN_API_KEY_ENV];
  if (!apiKey) {
    throw new Error(
      `${BAILIAN_API_KEY_ENV} not set — export it (the 百炼 CP sk-sp-... key) before running with --live`,
    );
  }
  return callOpenAICompat({
    baseURL: BAILIAN_BASE_URL,
    apiKey,
    model: args.model,
    system: args.system,
    user: args.user,
    temperature: args.temperature,
    maxTokens: args.maxTokens,
    label: 'bailian',
  });
}
