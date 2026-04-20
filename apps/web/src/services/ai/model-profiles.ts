/**
 * Model capability profiles — adapt AI configs per model tier.
 *
 * Each profile matches a model ID pattern and overrides thinking mode,
 * effort, timeouts, and prompt complexity as needed. First match wins.
 */

import type { ThinkingMode, ThinkingEffort } from './ai-runtime-config';

export type ModelTier = 'full' | 'standard' | 'basic';

export interface ModelProfile {
  match: string | RegExp;
  tier: ModelTier;
  thinkingMode?: ThinkingMode;
  effort?: ThinkingEffort;
  timeoutMultiplier?: number;
  simplifiedPrompt?: boolean;
  label?: string;
}

const MODEL_PROFILES: ModelProfile[] = [
  // Full tier — defaults unchanged
  { match: 'claude-opus', tier: 'full', label: 'Claude Opus' },
  { match: 'claude-sonnet', tier: 'full', label: 'Claude Sonnet' },
  { match: 'claude-3-5', tier: 'full', label: 'Claude 3.5' },
  { match: 'claude-3.5', tier: 'full', label: 'Claude 3.5' },
  { match: 'claude-4', tier: 'full', label: 'Claude 4' },

  // Standard tier — disable thinking (unsupported or unhelpful)
  { match: 'gpt-4o', tier: 'standard', thinkingMode: 'disabled', label: 'GPT-4o' },
  { match: 'o1', tier: 'standard', thinkingMode: 'disabled', label: 'o1' },
  { match: 'o3', tier: 'standard', thinkingMode: 'disabled', label: 'o3' },
  { match: 'o4', tier: 'standard', thinkingMode: 'disabled', label: 'o4' },
  { match: 'gemini-3-pro', tier: 'full', thinkingMode: 'disabled', label: 'Gemini 3 Pro' },
  { match: 'gemini-3-flash', tier: 'standard', thinkingMode: 'disabled', label: 'Gemini 3 Flash' },
  { match: /^gemini-3/, tier: 'full', thinkingMode: 'disabled', label: 'Gemini 3' },
  { match: 'gemini-2.5-pro', tier: 'full', thinkingMode: 'disabled', label: 'Gemini 2.5 Pro' },
  {
    match: 'gemini-2.5-flash',
    tier: 'standard',
    thinkingMode: 'disabled',
    label: 'Gemini 2.5 Flash',
  },
  { match: 'gemini-pro', tier: 'standard', thinkingMode: 'disabled', label: 'Gemini Pro' },
  { match: /^gemini-2/, tier: 'standard', thinkingMode: 'disabled', label: 'Gemini 2' },
  { match: 'deepseek', tier: 'standard', thinkingMode: 'disabled', label: 'DeepSeek' },

  // Basic tier — disable thinking, use simplified prompt
  { match: 'claude-haiku', tier: 'basic', thinkingMode: 'disabled', label: 'Claude Haiku' },
  { match: 'gpt-4o-mini', tier: 'basic', thinkingMode: 'disabled', label: 'GPT-4o Mini' },
  { match: 'gpt-4.1-mini', tier: 'basic', thinkingMode: 'disabled', label: 'GPT-4.1 Mini' },
  { match: 'gpt-4.1-nano', tier: 'basic', thinkingMode: 'disabled', label: 'GPT-4.1 Nano' },
  { match: 'minimax', tier: 'basic', thinkingMode: 'disabled', label: 'MiniMax' },
  { match: 'qwen', tier: 'basic', thinkingMode: 'disabled', label: 'Qwen' },
  { match: 'llama', tier: 'basic', thinkingMode: 'disabled', label: 'Llama' },
  { match: 'mistral', tier: 'basic', thinkingMode: 'disabled', label: 'Mistral' },
  { match: 'gemma', tier: 'basic', thinkingMode: 'disabled', label: 'Gemma' },
  { match: 'glm', tier: 'basic', thinkingMode: 'disabled', label: 'GLM' },
];

const DEFAULT_PROFILE: ModelProfile = {
  match: '',
  tier: 'standard',
  thinkingMode: 'disabled',
  label: 'Unknown model',
};

/**
 * Resolve a model profile by ID. Strips `providerID/` prefix, first match wins.
 */
export function resolveModelProfile(modelId?: string): ModelProfile {
  if (!modelId)
    return {
      ...DEFAULT_PROFILE,
      tier: 'full',
      thinkingMode: undefined,
      label: 'Default (no model)',
    };

  // Strip provider prefix (e.g. "opencode/gpt-4o" → "gpt-4o")
  const normalized = modelId.includes('/') ? modelId.slice(modelId.indexOf('/') + 1) : modelId;
  const lower = normalized.toLowerCase();

  for (const profile of MODEL_PROFILES) {
    if (typeof profile.match === 'string') {
      if (lower.startsWith(profile.match) || lower.includes(profile.match)) {
        return profile;
      }
    } else {
      if (profile.match.test(lower)) {
        return profile;
      }
    }
  }

  return DEFAULT_PROFILE;
}

/**
 * Check if a profile requires the simplified sub-agent prompt.
 */
export function needsSimplifiedPrompt(profile: ModelProfile): boolean {
  return profile.simplifiedPrompt === true;
}

/**
 * Environment variable that gates N-tool element integration in the
 * embedded orchestrator. Default off so production behavior is
 * unchanged until the flag is explicitly flipped. Any truthy value
 * (`"1"`, `"true"`, `"yes"`, case-insensitive) enables the feature.
 *
 * Rollout per plan §4 (openpencil-docs
 * superpowers/plans/2026-04-21-element-tools-orchestrator-integration.md):
 *   Phase 1: ship with flag off (scaffolding only, zero behavior change)
 *   Phase 2: flip on for basic + standard tiers in a canary env
 *   Phase 3: remove the flag once stable across a monitoring window
 */
const ELEMENT_TOOLS_FLAG_ENV = 'ENABLE_ELEMENT_TOOLS_IN_ORCHESTRATOR';

function isElementToolsFlagEnabled(): boolean {
  const raw = process.env[ELEMENT_TOOLS_FLAG_ENV];
  if (!raw) return false;
  const lower = raw.trim().toLowerCase();
  return lower === '1' || lower === 'true' || lower === 'yes' || lower === 'on';
}

/**
 * Whether to inject elements.md + teach the `<op_tool>` output format
 * for a given model profile. Returns `true` iff the feature flag env
 * var is enabled AND the model is in a tier that benefits from the
 * element-tool surface.
 *
 * A/B v1 (openpencil-docs superpowers/notes/2026-04-20-ab-v1-results.md)
 * observed consistent wins on the basic + standard tiers (Δ M1 +8 to
 * +21pp on MiniMax/GLM5/GLM5.1) and a ceiling-effect regression on
 * the one full-tier weak model tested (Kimi K2.5: Δ M1 -12.5pp). We
 * therefore default full-tier models OFF; caller-level override is
 * anticipated but not yet wired (plan §7.2).
 */
export function needsElementTools(profile: ModelProfile): boolean {
  if (!isElementToolsFlagEnabled()) return false;
  return profile.tier === 'basic' || profile.tier === 'standard';
}

/**
 * Apply profile overrides to a timeout config object (mutates a copy).
 */
export function applyProfileToTimeouts<
  T extends {
    hardTimeoutMs: number;
    noTextTimeoutMs: number;
    firstTextTimeoutMs?: number;
    thinkingMode?: ThinkingMode;
    effort?: ThinkingEffort;
  },
>(base: T, profile: ModelProfile): T {
  const result = { ...base };

  if (profile.timeoutMultiplier != null && profile.timeoutMultiplier !== 1) {
    const m = profile.timeoutMultiplier;
    result.hardTimeoutMs = Math.round(result.hardTimeoutMs * m);
    result.noTextTimeoutMs = Math.round(result.noTextTimeoutMs * m);
    if (result.firstTextTimeoutMs != null) {
      result.firstTextTimeoutMs = Math.round(result.firstTextTimeoutMs * m);
    }
  }

  if (profile.thinkingMode != null) {
    result.thinkingMode = profile.thinkingMode;
  }

  if (profile.effort != null) {
    result.effort = profile.effort;
  }

  return result;
}
