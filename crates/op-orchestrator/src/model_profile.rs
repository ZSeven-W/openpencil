//! 模型能力 tier —— port of `apps/web/src/services/ai/model-profiles.ts`。
//!
//! 首个命中胜出。S3b-1a 只消费 `tier`(决定 style-guide snippet
//! 数);`thinking_disabled` / `timeout_multiplier` 移植但留 S3b-1b。

/// 模型 tier —— 决定 planner 重试档数与 style-guide snippet 上限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    Full,
    Standard,
    Basic,
}

/// 一个模型的能力画像。
#[derive(Debug, Clone)]
pub struct ModelProfile {
    pub tier: ModelTier,
    /// TS `thinkingMode: 'disabled'`。
    pub thinking_disabled: bool,
    /// TS `timeoutMultiplier ?? 1`。
    pub timeout_multiplier: f64,
    pub label: &'static str,
}

/// 表项的匹配方式 —— 对齐 TS `match: string | RegExp`。
enum Match {
    /// TS string:`lower.starts_with(s) || lower.contains(s)`。
    Sub(&'static str),
    /// TS `/^xxx/`:前缀。
    Prefix(&'static str),
    /// TS `/^deepseek-(chat|reasoner)$/`:精确命中其一。
    Exact(&'static [&'static str]),
}

struct Entry {
    matcher: Match,
    tier: ModelTier,
    thinking_disabled: bool,
    timeout_multiplier: f64,
    label: &'static str,
}

/// 默认 profile —— 有 id 但无命中(TS `DEFAULT_PROFILE`)。
const DEFAULT_PROFILE: ModelProfile = ModelProfile {
    tier: ModelTier::Standard,
    thinking_disabled: true,
    timeout_multiplier: 1.0,
    label: "Unknown model",
};

/// 模型表 —— verbatim 移植自 `model-profiles.ts:22-95`,首个命中胜出。
const MODEL_PROFILES: &[Entry] = &[
    // Full tier
    e(
        Match::Sub("claude-opus"),
        ModelTier::Full,
        false,
        "Claude Opus",
    ),
    e(
        Match::Sub("claude-sonnet"),
        ModelTier::Full,
        false,
        "Claude Sonnet",
    ),
    e(
        Match::Sub("claude-3-5"),
        ModelTier::Full,
        false,
        "Claude 3.5",
    ),
    e(
        Match::Sub("claude-3.5"),
        ModelTier::Full,
        false,
        "Claude 3.5",
    ),
    e(Match::Sub("claude-4"), ModelTier::Full, false, "Claude 4"),
    // Standard tier — thinking disabled
    e(Match::Sub("gpt-4o"), ModelTier::Standard, true, "GPT-4o"),
    e(Match::Sub("o1"), ModelTier::Standard, true, "o1"),
    e(Match::Sub("o3"), ModelTier::Standard, true, "o3"),
    e(Match::Sub("o4"), ModelTier::Standard, true, "o4"),
    e(
        Match::Sub("gemini-3-pro"),
        ModelTier::Full,
        true,
        "Gemini 3 Pro",
    ),
    e(
        Match::Sub("gemini-3-flash"),
        ModelTier::Standard,
        true,
        "Gemini 3 Flash",
    ),
    e(Match::Prefix("gemini-3"), ModelTier::Full, true, "Gemini 3"),
    e(
        Match::Sub("gemini-2.5-pro"),
        ModelTier::Full,
        true,
        "Gemini 2.5 Pro",
    ),
    e(
        Match::Sub("gemini-2.5-flash"),
        ModelTier::Standard,
        true,
        "Gemini 2.5 Flash",
    ),
    e(
        Match::Sub("gemini-pro"),
        ModelTier::Standard,
        true,
        "Gemini Pro",
    ),
    e(
        Match::Prefix("gemini-2"),
        ModelTier::Standard,
        true,
        "Gemini 2",
    ),
    Entry {
        matcher: Match::Sub("deepseek-v4-pro"),
        tier: ModelTier::Full,
        thinking_disabled: true,
        timeout_multiplier: 2.0,
        label: "DeepSeek V4 Pro",
    },
    e(
        Match::Sub("deepseek-v4-flash"),
        ModelTier::Standard,
        true,
        "DeepSeek V4 Flash",
    ),
    e(
        Match::Exact(&["deepseek-chat", "deepseek-reasoner"]),
        ModelTier::Standard,
        true,
        "DeepSeek (legacy)",
    ),
    // Basic tier
    e(
        Match::Sub("claude-haiku"),
        ModelTier::Basic,
        true,
        "Claude Haiku",
    ),
    e(
        Match::Sub("gpt-4o-mini"),
        ModelTier::Basic,
        true,
        "GPT-4o Mini",
    ),
    e(
        Match::Sub("gpt-4.1-mini"),
        ModelTier::Basic,
        true,
        "GPT-4.1 Mini",
    ),
    e(
        Match::Sub("gpt-4.1-nano"),
        ModelTier::Basic,
        true,
        "GPT-4.1 Nano",
    ),
    e(Match::Sub("minimax"), ModelTier::Basic, true, "MiniMax"),
    e(Match::Sub("qwen"), ModelTier::Basic, true, "Qwen"),
    e(Match::Sub("llama"), ModelTier::Basic, true, "Llama"),
    e(Match::Sub("mistral"), ModelTier::Basic, true, "Mistral"),
    e(Match::Sub("gemma"), ModelTier::Basic, true, "Gemma"),
    e(Match::Sub("glm"), ModelTier::Basic, true, "GLM"),
];

/// `Entry` 构造简写(默认 `timeout_multiplier = 1.0`)。
const fn e(matcher: Match, tier: ModelTier, thinking_disabled: bool, label: &'static str) -> Entry {
    Entry {
        matcher,
        tier,
        thinking_disabled,
        timeout_multiplier: 1.0,
        label,
    }
}

/// 解析模型 id → profile。strip `provider/` 前缀 → 小写 → 首个命中。
/// 空 id → 强制 `Full`(TS 行为);无命中 → `DEFAULT_PROFILE`。
pub fn resolve_model_profile(model_id: &str) -> ModelProfile {
    if model_id.is_empty() {
        return ModelProfile {
            tier: ModelTier::Full,
            thinking_disabled: false,
            timeout_multiplier: 1.0,
            label: "Default (no model)",
        };
    }
    let normalized = match model_id.find('/') {
        Some(i) => &model_id[i + 1..],
        None => model_id,
    };
    let lower = normalized.to_lowercase();
    for entry in MODEL_PROFILES {
        let hit = match &entry.matcher {
            Match::Sub(s) => lower.starts_with(s) || lower.contains(s),
            Match::Prefix(p) => lower.starts_with(p),
            Match::Exact(list) => list.contains(&lower.as_str()),
        };
        if hit {
            return ModelProfile {
                tier: entry.tier,
                thinking_disabled: entry.thinking_disabled,
                timeout_multiplier: entry.timeout_multiplier,
                label: entry.label,
            };
        }
    }
    DEFAULT_PROFILE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_tier_models() {
        assert_eq!(
            resolve_model_profile("claude-opus-4-1").tier,
            ModelTier::Full
        );
        assert_eq!(
            resolve_model_profile("claude-sonnet-4").tier,
            ModelTier::Full
        );
        assert_eq!(
            resolve_model_profile("gemini-2.5-pro").tier,
            ModelTier::Full
        );
    }

    #[test]
    fn basic_tier_models() {
        assert_eq!(resolve_model_profile("claude-haiku").tier, ModelTier::Basic);
        assert_eq!(resolve_model_profile("minimax-01").tier, ModelTier::Basic);
        assert_eq!(resolve_model_profile("glm-4-plus").tier, ModelTier::Basic);
        assert_eq!(resolve_model_profile("qwen-max").tier, ModelTier::Basic);
    }

    #[test]
    fn standard_tier_models() {
        assert_eq!(resolve_model_profile("gpt-4o").tier, ModelTier::Standard);
        assert_eq!(
            resolve_model_profile("gemini-2.5-flash").tier,
            ModelTier::Standard
        );
    }

    #[test]
    fn provider_prefix_is_stripped() {
        assert_eq!(
            resolve_model_profile("opencode/gpt-4o").tier,
            ModelTier::Standard
        );
    }

    #[test]
    fn regex_entries_match() {
        assert_eq!(
            resolve_model_profile("gemini-3-ultra").tier,
            ModelTier::Full
        );
        assert_eq!(
            resolve_model_profile("deepseek-chat").tier,
            ModelTier::Standard
        );
        assert_eq!(
            resolve_model_profile("deepseek-v4-pro").timeout_multiplier,
            2.0
        );
    }

    #[test]
    fn empty_id_forces_full() {
        assert_eq!(resolve_model_profile("").tier, ModelTier::Full);
    }

    #[test]
    fn unknown_id_defaults_standard() {
        assert_eq!(
            resolve_model_profile("some-unknown-model").tier,
            ModelTier::Standard
        );
    }
}
