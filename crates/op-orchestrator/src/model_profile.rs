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

/// Provider-specific control used to minimize hidden reasoning on structured
/// design turns. The transport layer maps this semantic policy to exactly one
/// OpenAI-compatible request field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningWireControl {
    /// `thinking: { "type": "disabled" }`.
    ThinkingDisabled,
    /// `reasoning_effort: "low"` (Kimi K3; `thinking` is unsupported).
    ReasoningEffortLow,
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

/// ACP agents do not expose their backing model to OpenPencil. Treat their
/// catalog identity conservatively instead of promoting a missing model id to
/// the Full-tier default.
const ACP_PROFILE: ModelProfile = ModelProfile {
    tier: ModelTier::Basic,
    thinking_disabled: true,
    timeout_multiplier: 1.0,
    label: "ACP agent",
};

/// Whether `model_id` is the catalog identity of an ACP agent rather than a
/// concrete provider model. Hosts may carry this marker through
/// [`crate::DesignRequest`] for capability policy, but must not forward it as a
/// transport model override.
pub fn is_acp_capability_marker(model_id: &str) -> bool {
    model_id
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("acp:"))
}

/// 模型表 —— verbatim 移植自 `model-profiles.ts:22-95`,首个命中胜出。
const MODEL_PROFILES: &[Entry] = &[
    // Full tier
    e(
        Match::Sub("claude-fable"),
        ModelTier::Full,
        false,
        "Claude Fable",
    ),
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
    // GLM-5.2（及以后更高版本）—— ab-v9 manifest arm 实测很强（M3 96%、composite
    // 5/5），给 Full tier。**只 5.2 起算强**（用户判定：glm-5 / glm-5.1 不够，不命中
    // 这条 → 落 default Standard）。关思考与 tier 无关：builtin HTTP 路由按
    // is_glm_model 对**所有** glm（含 5 / 5.1）强制下发 thinking:{type:disabled}，
    // 因为它们都是推理模型、不关都会烧光 content。GLM 体量大、即便关思考也偏慢，
    // timeout ×2。未来 glm-5.3 / glm-6 等更高版本同样算强，届时各加一条 matcher。
    Entry {
        matcher: Match::Sub("glm-5.2"),
        tier: ModelTier::Full,
        thinking_disabled: true,
        timeout_multiplier: 2.0,
        label: "GLM-5.2 (Coding Plan)",
    },
    // Kimi K3（及以后）—— 用户判定 K3 起算强；K2.x 不命中这条 → 落 default
    // Standard。与 GLM 同理是推理模型,关思考防烧 content;时长无实测数据,先 ×1,
    // 慢再调。未来 K4 等更高代际届时各加一条 matcher。
    e(Match::Sub("kimi-k3"), ModelTier::Full, true, "Kimi K3"),
    // MiniMax M3（及以后）—— 2026-07-18 3×2 A/B 实测（mobile 多屏/dashboard/
    // landing）：Full 档几何 issue 5→1、总耗时快 30%、调用数更少、完整性更优，
    // 全维度非边际胜出 → 升 Full。仅 M3 起算强（老 M 系/abab 落后面的泛 minimax
    // Basic 条目）。thinking 由 ChatProviderLlmClient 的 m3_keeps_thinking 特判
    // 保留（Adaptive），此处 thinking_disabled 对 M3 实际不生效，仅作族内一致。
    e(
        Match::Sub("minimax-m3"),
        ModelTier::Full,
        true,
        "MiniMax M3",
    ),
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

/// Resolve a model id to its capability profile. ACP catalog ids use the
/// conservative `Basic` default. Other ids strip a `provider/` prefix, then
/// match the lower-cased model table. An empty id keeps the legacy forced
/// `Full` behavior; an unmatched non-empty id uses [`DEFAULT_PROFILE`].
pub fn resolve_model_profile(model_id: &str) -> ModelProfile {
    if model_id.is_empty() {
        return ModelProfile {
            tier: ModelTier::Full,
            thinking_disabled: false,
            timeout_multiplier: 1.0,
            label: "Default (no model)",
        };
    }
    // Check before stripping a provider prefix: ACP ids are opaque and may
    // themselves contain `/` (for example `acp:vendor/custom-agent`).
    if is_acp_capability_marker(model_id) {
        return ACP_PROFILE;
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

/// Resolve the provider-specific wire control for reducing hidden reasoning.
///
/// 这是**协议能力**,不是 [`ModelProfile::thinking_disabled`](ModelProfile)
/// 那条"该不该关思考"的策略:后者说意图,这里说这条意图应如何在线级表达。
/// Kimi K3 使用 `reasoning_effort:"low"`;其余已验证家族使用
/// `thinking:{"type":"disabled"}`。调用方明确要求降低推理时才真正下发。
///
/// 单一来源。此前这份知识以 `is_minimax_model` / `is_glm_model` 两个谓词的形式
/// 散在传输层(`chat_builtin_http`)、agent tool-loop(`chat_agent_loop::openai`)
/// 和 headless harness(`op-smoke::llm_clients`)**三处**,每接一家推理模型都要
/// 三处同改。代价已经兑现两次:
///
/// 1. loop 侧漏加,glm-5.2 的每个 design turn 都在泄漏 reasoning,把
///    `batch_design` 的 JSON 截断在半路;
/// 2. harness 侧的副本 drift 成 `starts_with("glm")`(生产是 `contains`);
/// 3. deepseek-v4-pro 的 profile 明写 `thinking_disabled: true`,却因为不在这张
///    名单里而被静默丢弃 —— 只读工具的参数短,思考吃剩的预算还够吐出来所以全绿,
///    唯独上万 token 的 `batch_design` 必然截断,表现为"探索完就没下文"。
///
/// 收录依据(每一条都要有出处,不靠猜):
/// - MiniMax M 系 / 旧 abab:MiniMax 专属 `thinking` 字段,线上实测接受。
/// - GLM-4.5+ / GLM-5.x:curl 对 ark glm-5.2 验证,关思考后 reasoning_tokens=0、
///   content 为干净 JSON。
/// - DeepSeek V4 系:官方文档 <https://api-docs.deepseek.com/guides/thinking_mode/>
///   给出同形字段 `{"thinking":{"type":"enabled|disabled"}}`,并写明思考**默认开启
///   且默认 effort=high** —— 不关就是最重的那一档。
///
/// 不做无条件下发:内置 provider 允许用户把 base_url 指向任意 openai-compat 端点
/// (含 OpenAI 官方),那里的未知 body 字段会 400。名单是这条风险的边界,新增一家
/// 只改这里一处。
pub fn reasoning_wire_control(model_id: &str) -> Option<ReasoningWireControl> {
    let normalized = match model_id.find('/') {
        Some(i) => &model_id[i + 1..],
        None => model_id,
    };
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("kimi-k3") {
        return Some(ReasoningWireControl::ReasoningEffortLow);
    }
    (lower.starts_with("minimax")
        || lower.starts_with("abab")
        || lower.contains("glm")
        || lower.starts_with("deepseek")
        // Kimi 是**逐 model id**,不是整族 —— Moonshot 在族内换了控制字段。
        // 只有 K2.5 / K2.6 走同形的 `thinking:{type}`(官方 API 参考的逐模型
        // 表列出 enabled/disabled 两值,默认 enabled)。
        //
        // 故意排除、且**绝不能**放宽成 `starts_with("kimi")` 的三类:
        // - `kimi-k3`(正是内置预设的默认模型):不认 `thinking`,改用顶层
        //   `reasoning_effort`(low/high/max,默认 max),且文档写明"始终推理"
        //   关不掉;两个字段同时下发直接 400
        //   (`cannot specify both 'thinking' and 'reasoning_effort'`)。
        // - `kimi-k2.7-code*`:只接受 `type:"enabled"`,发 disabled 报
        //   `only type=enabled is allowed for this model`。
        // - `kimi-k2-thinking*`:强制开启,要非思考只能换 model id。
        // 同理不收 `moonshot-*` —— 那是老一代无思考模型,没有可关的东西。
        //
        // 出处:platform.kimi.ai/docs/api/chat 的逐模型 thinking /
        // reasoning_effort 对照表 + docs/models 的在售模型列表。
        || lower.contains("kimi-k2.5")
        || lower.contains("kimi-k2.6"))
    .then_some(ReasoningWireControl::ThinkingDisabled)
}

/// Whether a model accepts `thinking: { "type": "disabled" }`.
///
/// Kept as the narrow compatibility predicate for callers that specifically
/// need that field. New request builders should match on
/// [`reasoning_wire_control`] so Kimi K3 receives its mutually-exclusive
/// `reasoning_effort` control instead of being mistaken for an unsupported
/// model.
pub fn accepts_thinking_body_field(model_id: &str) -> bool {
    matches!(
        reasoning_wire_control(model_id),
        Some(ReasoningWireControl::ThinkingDisabled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_body_field_covers_every_reasoning_family_we_ship() {
        // MiniMax (current + legacy naming).
        assert!(accepts_thinking_body_field("MiniMax-M3"));
        assert!(accepts_thinking_body_field("abab6.5s-chat"));
        // GLM — `contains`, not `starts_with`: 方舟 ids carry a vendor prefix.
        assert!(accepts_thinking_body_field("glm-5.2"));
        assert!(accepts_thinking_body_field("ark/glm-5.1"));
        // DeepSeek — the family this table was missing (2026-07-31).
        assert!(accepts_thinking_body_field("deepseek-v4-pro"));
        assert!(accepts_thinking_body_field("deepseek-v4-flash"));
        assert!(accepts_thinking_body_field("deepseek-reasoner"));
        // Endpoints that would 400 on an unknown body field stay out.
        assert!(!accepts_thinking_body_field("gpt-5.6-sol"));
        assert!(!accepts_thinking_body_field("qwen3-coder-plus"));
        assert!(!accepts_thinking_body_field("claude-opus-5"));
        assert!(!accepts_thinking_body_field(""));
    }

    #[test]
    fn kimi_k3_uses_low_reasoning_effort_instead_of_thinking() {
        for model in ["kimi-k3", "moonshot/kimi-k3", "kimi-k3.1-preview"] {
            let profile = resolve_model_profile(model);
            assert_eq!(profile.tier, ModelTier::Full, "model={model}");
            assert!(profile.thinking_disabled, "model={model}");
            assert_eq!(
                reasoning_wire_control(model),
                Some(ReasoningWireControl::ReasoningEffortLow),
                "model={model}"
            );
            assert!(!accepts_thinking_body_field(model), "model={model}");
        }
    }

    /// The capability table must cover every model whose profile asks for
    /// thinking off — otherwise the profile's intent is silently dropped at
    /// the wire, which is exactly how deepseek-v4-pro regressed.
    #[test]
    fn every_reasoning_model_we_reduce_can_express_it() {
        for model in ["deepseek-v4-pro", "deepseek-v4-flash", "glm-5.2", "kimi-k3"] {
            assert!(
                resolve_model_profile(model).thinking_disabled,
                "{model} profile should ask for thinking off"
            );
            assert!(
                reasoning_wire_control(model).is_some(),
                "{model} asks for reduced reasoning but cannot express it on the wire"
            );
        }
    }

    /// Models we ship a built-in preset for whose profile asks for thinking
    /// off but which deliberately do NOT get the body field, each with the
    /// reason. Being on this list is a decision, not an oversight — that
    /// distinction is the whole point of the sweep below.
    const REASONING_CONTROL_WITHHELD: &[(&str, &str)] = &[
        (
            "gpt-5.4",
            "OpenAI's official endpoint rejects unknown body fields",
        ),
        (
            "gpt-5.6",
            "OpenAI's official endpoint rejects unknown body fields",
        ),
        (
            "gemini-3-flash-preview",
            "Google's OpenAI-compat shim is not documented to accept it",
        ),
        ("qwen-plus", "no verified thinking-disable field"),
        ("qwen3-coder-plus", "no verified thinking-disable field"),
        ("Qwen/Qwen3.5-35B-A3B", "no verified thinking-disable field"),
        ("doubao-seed-2.0-pro", "no verified thinking-disable field"),
        (
            "doubao-seed-2-0-pro-260215",
            "no verified thinking-disable field",
        ),
        ("ark-code-latest", "no verified thinking-disable field"),
        ("mimo-v2-pro", "no verified thinking-disable field"),
        ("mimo-v2.5-pro", "no verified thinking-disable field"),
        ("step-3.5-flash", "no verified thinking-disable field"),
        ("step-3-coding", "no verified thinking-disable field"),
        (
            "nvidia/llama-3.1-nemotron-70b-instruct",
            "no verified thinking-disable field",
        ),
    ];

    /// The blind spot that let kimi-k3 through, closed mechanically.
    ///
    /// The previous guard walked a hand-written list of three model ids, so
    /// it could only catch families someone had already remembered — which
    /// is not a guard at all. Walking the profile table instead does not
    /// work either: `thinking_disabled` is set to `true` on nearly every
    /// entry, including models with no thinking mode to disable, so it
    /// cannot distinguish "we mean this" from "the constructor defaulted".
    ///
    /// The models we actually ship *are* enumerable: the built-in provider
    /// presets. Every one whose profile asks for thinking off must be
    /// explicitly classified — either the wire table carries it, or it is
    /// listed above with a reason. Adding a preset with a new default model
    /// fails here until someone decides which.
    #[test]
    fn every_shipped_preset_model_is_classified_for_reasoning_control() {
        for preset in op_editor_core::BUILTIN_AGENT_PRESETS {
            // The Custom preset's `model` is a form placeholder, not an id.
            if preset.key == op_editor_core::BuiltinAgentPresetKey::Custom {
                continue;
            }
            let model = preset.model;
            if !resolve_model_profile(model).thinking_disabled {
                continue;
            }
            let on_the_wire = reasoning_wire_control(model).is_some();
            let withheld = REASONING_CONTROL_WITHHELD
                .iter()
                .any(|(listed, _)| *listed == model);
            assert!(
                on_the_wire != withheld,
                "preset `{}` ships model `{model}`, whose profile asks for \
                 thinking off, but it is {} — add it to \
                 `reasoning_wire_control` (with a source) or to \
                 REASONING_CONTROL_WITHHELD (with a reason)",
                preset.display_name,
                if on_the_wire {
                    "both sent on the wire AND listed as withheld"
                } else {
                    "neither sent on the wire nor listed as withheld"
                },
            );
        }
    }

    /// Kimi is per-model, not per-family: the preset default (`kimi-k3`)
    /// must stay OFF the wire table while the two ids that document the
    /// field stay on it.
    #[test]
    fn kimi_thinking_field_is_scoped_to_the_models_that_accept_it() {
        assert!(accepts_thinking_body_field("kimi-k2.5"));
        assert!(accepts_thinking_body_field("kimi-k2.6"));
        assert!(accepts_thinking_body_field("moonshot/kimi-k2.6"));
        // k3 rejects `thinking` outright (mutually exclusive with
        // `reasoning_effort`), so a blanket `kimi` prefix would 400 every
        // request from the shipped Kimi preset.
        assert!(!accepts_thinking_body_field("kimi-k3"));
        assert!(!accepts_thinking_body_field("kimi-k2.7-code"));
        assert!(!accepts_thinking_body_field("kimi-k2-thinking"));
        assert!(!accepts_thinking_body_field("moonshot-v1-128k"));
    }

    #[test]
    fn full_tier_models() {
        let fable = resolve_model_profile("claude-fable-5");
        assert_eq!(fable.tier, ModelTier::Full);
        assert!(!fable.thinking_disabled);
        assert_eq!(resolve_model_profile("kimi-k3").tier, ModelTier::Full);
        // K2.x stays below the strong line (falls to the unknown default).
        assert_eq!(resolve_model_profile("kimi-k2.5").tier, ModelTier::Standard);
        // M3 measured full-tier (2026-07-18 A/B); older MiniMax stays Basic.
        assert_eq!(resolve_model_profile("MiniMax-M3").tier, ModelTier::Full);
        assert_eq!(resolve_model_profile("MiniMax-M2.7").tier, ModelTier::Basic);
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
        assert_eq!(
            resolve_model_profile("acp:vendor/custom-agent").tier,
            ModelTier::Basic
        );
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
