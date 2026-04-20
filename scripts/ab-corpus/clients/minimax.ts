/**
 * MiniMax M-series wrapper. Uses the OpenAI-compatible endpoint at
 * api.minimaxi.com/v1 (CN) by default; override via `MINIMAX_BASE_URL`.
 * Key must be in `MINIMAX_API_KEY`. See builtin-provider-presets.ts
 * `minimax` for the canonical endpoint list.
 */

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

export async function callMinimax(args: CallMinimaxArgs): Promise<string> {
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
    maxTokens: args.maxTokens,
    label: 'minimax',
  });
}
