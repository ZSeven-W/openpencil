/**
 * Zhipu GLM (智谱) Coding Plan wrapper. Uses the official GLM coding
 * endpoint at open.bigmodel.cn/api/coding/paas/v4 (CN). Key is the
 * `api_key.secret` pair format (`xxx.yyy`) passed verbatim as the
 * Bearer token — GLM's own auth handles the split internally.
 *
 * Key env var: `GLM_OFFICIAL_CODING_KEY`. See
 * builtin-provider-presets.ts `glm-coding` preset.
 */

import { callOpenAICompat } from './openai-compat';

const GLM_BASE_URL = process.env.GLM_BASE_URL ?? 'https://open.bigmodel.cn/api/coding/paas/v4';
const GLM_API_KEY_ENV = 'GLM_OFFICIAL_CODING_KEY';

export interface CallGlmArgs {
  model: string;
  system: string;
  user: string;
  temperature?: number;
  maxTokens?: number;
}

export async function callGlm(args: CallGlmArgs): Promise<string> {
  const apiKey = process.env[GLM_API_KEY_ENV];
  if (!apiKey) {
    throw new Error(
      `${GLM_API_KEY_ENV} not set — export it (GLM official CP xxx.yyy key) before running with --live`,
    );
  }
  return callOpenAICompat({
    baseURL: GLM_BASE_URL,
    apiKey,
    model: args.model,
    system: args.system,
    user: args.user,
    temperature: args.temperature,
    maxTokens: args.maxTokens,
    label: 'glm',
  });
}
