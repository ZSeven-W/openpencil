/**
 * MiniMax M-series wrapper. Uses the OpenAI-compatible endpoint at
 * api.minimaxi.com/v1 (CN) by default; override via `MINIMAX_BASE_URL`.
 * Key must be in `MINIMAX_API_KEY`. See builtin-provider-presets.ts
 * `minimax` for the canonical endpoint list.
 */

import type { ChatCallResult } from './openai-compat';
import { callOpenAICompat } from './openai-compat';

const MINIMAX_BASE_URL = process.env.MINIMAX_BASE_URL ?? 'https://api.minimax.io/v1';
const MINIMAX_API_KEY_ENV = 'MINIMAX_API_KEY';

export interface CallMinimaxArgs {
  model: string;
  system: string;
  user: string;
  temperature?: number;
  maxTokens?: number;
}

export async function callMinimax(args: CallMinimaxArgs): Promise<ChatCallResult> {
  const apiKey = process.env[MINIMAX_API_KEY_ENV];
  if (!apiKey) {
    throw new Error(
      `${MINIMAX_API_KEY_ENV} not set — export it before running with --live, or pass --dry-run to use fixtures`,
    );
  }
  return callOpenAICompat({
    baseURL: MINIMAX_BASE_URL,
    apiKey,
    model: args.model,
    system: args.system,
    user: args.user,
    temperature: args.temperature,
    // M2.7 emits chain-of-thought inside the same `content` channel
    // (we strip <think> blocks in output-parser.ts); the openai-compat
    // default 4096 is fine for obvious prompts but cuts close on
    // composite multi-tool outputs (12-tag responses + thinking can
    // approach 3-4k easy). Double the cap defensively — we already pay
    // for thinking either way, and most replies still come in well
    // under 1k completion tokens (ab-v4 avg 697), so the bigger cap
    // costs nothing on the happy path and only matters when the model
    // would otherwise truncate mid-output.
    maxTokens: args.maxTokens ?? 8192,
    label: 'minimax',
    // ab-v3 saw 4 minimax timeouts on 104 runs — all wall-clock aborts,
    // none were content-quality issues. One retry picks up the
    // recoverable ones without burning budget on the truncation /
    // <think> chatter that characterizes minimax model-side failures.
    retries: 1,
  });
}
