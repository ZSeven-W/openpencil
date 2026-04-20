/**
 * Live model dispatcher. Picks the right client per model id and
 * invokes it with the full system + user prompt. Exposed as
 * `realModelCall` with the same signature as `stubModelCall` so the
 * harness swaps them behind the `--live` flag without conditional
 * logic in the main loop.
 *
 * Router rules:
 *   - `minimax*` → clients/minimax.ts (needs MINIMAX_API_KEY in env)
 *   - `gpt-*` / `o1` / `o3` / `o4` → clients/codex-cli.ts (uses
 *     Codex CLI subscription)
 *   - anything else → throw with the supported list
 */

import { callMinimax } from './clients/minimax';
import { callCodex } from './clients/codex-cli';
import { callBailian } from './clients/bailian';
import { callGlm } from './clients/glm';
import { buildSystemPrompt } from './build-prompt';
import type { ModelCall } from './stub-model';

/**
 * Router rules (checked in priority order, first match wins):
 *
 *   - `minimax*`    → clients/minimax.ts       (MINIMAX_API_KEY)
 *   - `gpt-*` / o*  → clients/codex-cli.ts     (Codex CLI subscription)
 *   - `glm-5.1`     → clients/glm.ts           (GLM_OFFICIAL_CODING_KEY,
 *                                               GLM-official coding-plan endpoint)
 *   - `glm-*`       → clients/bailian.ts       (DASHSCOPE_BAILIAN_CODING_KEY,
 *                                               earlier GLM versions hosted on
 *                                               Bailian's DashScope aggregator)
 *   - `kimi-*`      → clients/bailian.ts       (DASHSCOPE_BAILIAN_CODING_KEY,
 *                                               Kimi K-series also on Bailian)
 *
 * Anything else → throw with the supported list.
 */
export async function realModelCall(call: ModelCall): Promise<string> {
  const built = buildSystemPrompt(call.variant);
  const user = call.prompt.prompt;
  const model = call.model;

  if (/^minimax/i.test(model)) {
    return callMinimax({
      model: mapMinimaxId(model),
      system: built.system,
      user,
    });
  }
  if (/^(gpt-|o1|o3|o4)/i.test(model)) {
    return callCodex({ model, system: built.system, user });
  }
  if (/^glm-5\.1/i.test(model)) {
    return callGlm({
      model: mapGlmOfficialId(model),
      system: built.system,
      user,
    });
  }
  if (/^glm-/i.test(model)) {
    return callBailian({
      model: mapGlmBailianId(model),
      system: built.system,
      user,
    });
  }
  if (/^kimi/i.test(model)) {
    return callBailian({
      model: mapKimiBailianId(model),
      system: built.system,
      user,
    });
  }
  throw new Error(
    `No live adapter for model "${model}". Supported: minimax-* (MINIMAX_API_KEY), gpt-*/o1/o3/o4 (Codex CLI), glm-5.1 (GLM_OFFICIAL_CODING_KEY), glm-5 / kimi-* (DASHSCOPE_BAILIAN_CODING_KEY).`,
  );
}

function mapMinimaxId(id: string): string {
  const lower = id.toLowerCase();
  if (lower.includes('m2.7') || lower === 'minimax-m2' || lower === 'minimax-m2.7') {
    return 'MiniMax-M2.7';
  }
  return id;
}

function mapGlmOfficialId(id: string): string {
  // GLM official CP ships "glm-4.7" as the current coding model
  // (per builtin-provider-presets.ts modelPlaceholder). User's
  // preferred short form "glm-5.1" maps to that id here.
  const lower = id.toLowerCase();
  if (lower === 'glm-5.1' || lower === 'glm-5.1-coding') return 'glm-4.7';
  return id;
}

function mapGlmBailianId(id: string): string {
  return mapGlmBailianIdInternal(id);
}

function mapKimiBailianId(id: string): string {
  // Bailian CP exposes Moonshot Kimi as `kimi-k2.5` — the `k` prefix
  // matters; plain `kimi-2.5` gets HTTP 400 "model not supported".
  // Source: Alibaba Cloud Model Studio Coding Plan FAQ (Apr 2026).
  const lower = id.toLowerCase();
  if (lower === 'kimi-2.5' || lower === 'kimi-k2.5' || lower === 'kimi') return 'kimi-k2.5';
  return id;
}

function mapGlmBailianIdInternal(id: string): string {
  // Bailian CP exposes Zhipu GLM as `glm-4.7` (current coding-plan
  // preset id). Harness id `glm-5` is accepted as an alias by the
  // server and produced responses in smoke tests, but standardize on
  // the documented id to avoid silent deprecation.
  const lower = id.toLowerCase();
  if (lower === 'glm-5' || lower === 'glm-4.7' || lower === 'glm-coding') return 'glm-4.7';
  return id;
}
