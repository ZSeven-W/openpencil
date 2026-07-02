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
import { callArk } from './clients/ark';
import { callDeepSeek } from './clients/deepseek';
import type { ChatCallResult } from './clients/openai-compat';
import { buildSystemPrompt } from './build-prompt';
import type { ModelCall } from './stub-model';

/**
 * Router rules (checked in priority order, first match wins):
 *
 *   - `minimax*`    → clients/minimax.ts       (MINIMAX_API_KEY)
 *   - `gpt-*` / o*  → clients/codex-cli.ts     (Codex CLI subscription)
 *   - `glm-5.1`     → clients/ark.ts           (ARK_CODING_KEY;
 *                                               Volcengine 方舟 now hosts
 *                                               GLM-5.1 on its coding plan
 *                                               as of 2026-04-22. Replaces
 *                                               the earlier GLM-official
 *                                               CP route via open.bigmodel.cn
 *                                               — clients/glm.ts is kept on
 *                                               disk for historical reference
 *                                               but no longer auto-routed.)
 *   - `kimi-k2.6`   → clients/ark.ts           (ARK_CODING_KEY; Volcengine
 *                                               added K2.6 to 方舟 CP on
 *                                               2026-04-22)
 *   - `glm-*`       → clients/bailian.ts       (DASHSCOPE_BAILIAN_CODING_KEY,
 *                                               earlier GLM versions hosted on
 *                                               Bailian's DashScope aggregator)
 *   - `kimi-*`      → clients/bailian.ts       (DASHSCOPE_BAILIAN_CODING_KEY,
 *                                               Kimi K2.5 and earlier on Bailian)
 *
 * Anything else → throw with the supported list.
 *
 * When benchmarking across routes (e.g. to compare the new Ark GLM-5.1
 * vs. the legacy GLM-official route), import `callGlm` from
 * `./clients/glm.ts` directly and wire it here under a different model
 * alias (say `glm-5.1-legacy`). The default router picks Ark because
 * that's the path we're actively promoting.
 */
export async function realModelCall(call: ModelCall): Promise<ChatCallResult> {
  const built = buildSystemPrompt(call.variant, {
    difficulty: call.prompt.difficulty,
    category: call.prompt.category,
  });
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
  // Anchored match: `glm-5.1`, `glm-5.1-coding`, `glm-5.1-ark`.
  // Without `$` + suffix allowlist, `glm-5.10` (a hypothetical later
  // minor) would prefix-match and wrongly route here.
  if (/^glm-5\.1(-coding|-ark)?$/i.test(model)) {
    return callArk({
      model: mapGlmArkId(model),
      system: built.system,
      user,
    });
  }
  // Match both `kimi-k2.6` (canonical) and `kimi-2.6` (no-prefix
  // alias). Without the optional `k` the alias falls through to the
  // generic `/^kimi/i` branch below and wrongly lands on Bailian
  // (which doesn't serve K2.6). Trailing `-ark` suffix also allowed.
  if (/^kimi-k?2\.6(-ark)?$/i.test(model)) {
    return callArk({
      model: mapKimiArkId(model),
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
  // DeepSeek direct (api.deepseek.com, OpenAI-compat). Latest flagship
  // is `deepseek-v4-pro`; `deepseek-v4-flash` is the lighter variant.
  // The pre-2026-07-24 ids `deepseek-chat` / `deepseek-reasoner` are
  // deprecated upstream but routed here too so older corpus runs stay
  // reproducible until cutoff.
  if (/^deepseek/i.test(model)) {
    return callDeepSeek({
      model: mapDeepSeekId(model),
      system: built.system,
      user,
    });
  }
  throw new Error(
    `No live adapter for model "${model}". Supported: minimax-* (MINIMAX_API_KEY), gpt-*/o1/o3/o4 (Codex CLI), glm-5.1 / kimi-k2.6 (ARK_CODING_KEY), glm-5 / kimi-k2.5 (DASHSCOPE_BAILIAN_CODING_KEY), deepseek-* (DEEPSEEK_API_KEY).`,
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
  // GLM official CP (open.bigmodel.cn) used to ship "glm-4.7" as the
  // current coding model. The harness no longer routes `glm-5.1` here
  // (Ark took over, see mapGlmArkId) but this mapper is kept so
  // callers who manually import `./clients/glm.ts` for A/B comparison
  // can still get the canonical id.
  const lower = id.toLowerCase();
  if (lower === 'glm-5.1' || lower === 'glm-5.1-coding') return 'glm-4.7';
  return id;
}

function mapGlmArkId(id: string): string {
  // 方舟 CP exposes Zhipu GLM-5.1 on the coding-plan endpoint. The
  // on-Ark model name is "glm-5.1" verbatim (no endpoint-ID alias
  // required as of 2026-04-22 — earlier Ark models needed `ep-xxx`
  // but GLM-5.1 was onboarded with the promoted short name).
  const lower = id.toLowerCase();
  if (lower === 'glm-5.1' || lower === 'glm-5.1-coding' || lower === 'glm-5.1-ark') {
    return 'glm-5.1';
  }
  return id;
}

function mapKimiArkId(id: string): string {
  // 方舟 CP exposes Moonshot Kimi K2.6 on the coding-plan endpoint.
  // Volcengine accepts the promoted short name `kimi-k2.6` directly.
  // Anything that matched the Ark router regex (kimi-k?2.6[-ark]?)
  // normalizes to the canonical on-Ark id.
  const lower = id.toLowerCase();
  if (
    lower === 'kimi-k2.6' ||
    lower === 'kimi-2.6' ||
    lower === 'kimi-k2.6-ark' ||
    lower === 'kimi-2.6-ark'
  ) {
    return 'kimi-k2.6';
  }
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

function mapDeepSeekId(id: string): string {
  // DeepSeek's current flagship is `deepseek-v4-pro`. Harness aliases
  // `deepseek` (bare) and `deepseek-pro` resolve to that. The lighter
  // `deepseek-v4-flash` and the deprecated `deepseek-chat` /
  // `deepseek-reasoner` ids pass through verbatim — server still
  // accepts them until 2026-07-24 sunset per docs.
  const lower = id.toLowerCase();
  if (lower === 'deepseek' || lower === 'deepseek-pro' || lower === 'deepseek-v4-pro') {
    return 'deepseek-v4-pro';
  }
  return id;
}
