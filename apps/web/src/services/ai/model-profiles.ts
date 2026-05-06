/**
 * Model 功能配置文件
 *
 * — 根据模型层调整 AI 配置。 Each 配置文件与模型 ID 模式匹配，并根据需要覆盖思维模式、工作量、超时和提示复
 * 杂性。 First
 比赛获胜。
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
  // Full 层 — 默认值不变
  { match: 'claude-opus', tier: 'full', label: 'Claude Opus' },
  { match: 'claude-sonnet', tier: 'full', label: 'Claude Sonnet' },
  { match: 'claude-3-5', tier: 'full', label: 'Claude 3.5' },
  { match: 'claude-3.5', tier: 'full', label: 'Claude 3.5' },
  { match: 'claude-4', tier: 'full', label: 'Claude 4' },

  // Standard tier — 禁止思考（不受支持或无帮助）
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
  // DeepSeek v4 series — v4-pro and v4-flash default to thinking enabled;
  // API 通过 `{"thinking":{"type":"disabled"}}` 切换它。 Mark
  // 这些已禁用，因此应用程序保留其 fast/non-thinking 默认值 —
  // 连接 DeepSeek 切换的服务器推理路径将遵循它。
  // The Zig openai-compat 路径尚未发出切换，因此调用
  // 通过这条路径仍然看到提供商默认的思维，直到
  // 参数连接在那里。
  {
    match: 'deepseek-v4-pro',
    tier: 'full',
    thinkingMode: 'disabled',
    // Until Zig openai-compat 路径实际上发送
    // `thinking:{type:disabled}`，v4-pro 在每个请求上保持启用推理，并且推理令牌在长期计划提示上
    // 爆炸。 Double 超时窗口，以便协调器规划不会依赖于第一个大请求。 Drop 连接开关后，此值恢复为 1。
    timeoutMultiplier: 2,
    label: 'DeepSeek V4 Pro',
  },
  {
    match: 'deepseek-v4-flash',
    tier: 'standard',
    thinkingMode: 'disabled',
    label: 'DeepSeek V4 Flash',
  },
  // Legacy 别名 - 仅精确匹配，以便未来的 deepseek-* 变体（例如具有本机推理的假设的
  // deepseek-r2）不会继承强制禁用的 thinkingMode。 These 两个日落 2026-07-24 和
  // DeepSeek 今天自动将它们路由到 v4-flash。
  {
    match: /^deepseek-(chat|reasoner)$/,
    tier: 'standard',
    thinkingMode: 'disabled',
    label: 'DeepSeek (legacy)',
  },

  // Basic tier — 禁用思考，使用简化提示
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
 * Resolve ID 的模型配置文件。 Strips `providerID/` 前缀，第一场比赛获胜。
 */
export function resolveModelProfile(modelId?: string): ModelProfile {
  if (!modelId)
    return {
      ...DEFAULT_PROFILE,
      tier: 'full',
      thinkingMode: undefined,
      label: 'Default (no model)',
    };

  // Strip 提供商前缀（例如“opencode/gpt-4o”→“gpt-4o”）
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
 * Check 如果配置文件需要简化的子代理提示符。
 */
export function needsSimplifiedPrompt(profile: ModelProfile): boolean {
  return profile.simplifiedPrompt === true;
}

/**
 * Apply 配置文件覆盖超时配置对象（改变副本）。
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
