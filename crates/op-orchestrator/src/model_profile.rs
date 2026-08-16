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
    /// Family lane with a version floor (new policy, no TS equivalent):
    /// locate `prefix` with `contains` semantics (vendor prefixes such as
    /// `ark/` are allowed), parse the dotted numeric version that follows
    /// it, and match when that version is >= `min` AND the tail after the
    /// version is exactly `suffix` (or empty). Same-family same-variant
    /// newer versions inherit this entry's tier; an unknown variant tail
    /// (e.g. `glm-6-air`) deliberately does NOT match and falls through to
    /// the conservative default. See [`version_lane_matches`].
    VersionLane {
        prefix: &'static str,
        min: &'static [u32],
        suffix: Option<&'static str>,
    },
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
    // DeepSeek V `-pro` lane, floor 4 — same-family-same-variant newer
    // versions (deepseek-v5-pro, …) inherit Full automatically; an unknown
    // variant suffix (e.g. `deepseek-v5-pro-max`) falls through to the
    // conservative default. The `-flash` lane below is a separate entry
    // because variant suffixes are lane boundaries (v4-pro=Full vs
    // v4-flash=Standard is settled fact). Wire-control whitelists
    // deliberately do NOT inherit this way (Moonshot k3 lesson).
    Entry {
        matcher: Match::VersionLane {
            prefix: "deepseek-v",
            min: &[4],
            suffix: Some("-pro"),
        },
        tier: ModelTier::Full,
        thinking_disabled: true,
        timeout_multiplier: 2.0,
        label: "DeepSeek V4+ Pro",
    },
    // GLM-5.3 — explicit override BEFORE the glm lane so its ×3 beats the
    // lane's ×2. 0814 measurement: always thinks (`thinking:disabled` is
    // silently ignored, not a 400), one card took 635s ≈ 10× DeepSeek,
    // hence ×3.
    Entry {
        matcher: Match::Sub("glm-5.3"),
        tier: ModelTier::Full,
        thinking_disabled: true,
        timeout_multiplier: 3.0,
        label: "GLM-5.3",
    },
    // GLM family lane, floor 5.2 — measured strong on the ab-v9 manifest arm
    // (M3 96%, composite 5/5), and the user ruled only 5.2+ is strong.
    // Same-family-same-variant newer versions (glm-6, …) inherit Full
    // automatically; below the floor (glm-5 / glm-5.1) and unknown variant
    // suffixes (glm-6-air) fall through to the conservative default.
    // Thinking-off is tier-independent: the builtin HTTP route forces
    // `thinking:{type:disabled}` on ALL glm ids (see reasoning_wire_control)
    // because they are reasoning models that otherwise burn the whole
    // content budget. GLM is slow even with thinking off, hence ×2.
    // Wire-control whitelists deliberately do NOT inherit this way
    // (Moonshot k3 lesson): wire behaviour stays per model id.
    Entry {
        matcher: Match::VersionLane {
            prefix: "glm-",
            min: &[5, 2],
            suffix: None,
        },
        tier: ModelTier::Full,
        thinking_disabled: true,
        timeout_multiplier: 2.0,
        label: "GLM 5.2+",
    },
    // Kimi K3.1 preview — explicit row so the strict lane tail rule (an
    // unknown variant suffix after the version does not inherit) keeps the
    // shipped baseline: `kimi-k3.1-preview` was Full under the old
    // `Sub("kimi-k3")` row and must stay Full.
    e(
        Match::Sub("kimi-k3.1-preview"),
        ModelTier::Full,
        true,
        "Kimi K3.1 Preview",
    ),
    // Kimi K family lane, floor 3 — user verdict: K3+ is strong, K2.x is
    // not (K2.x falls through to the conservative default). Same-family
    // same-variant newer versions (kimi-k4, kimi-k3.x, …) inherit Full via
    // the numeric floor; unknown variant suffixes fall through. No measured
    // latency data yet, so ×1 for now. Wire-control whitelists deliberately
    // do NOT inherit this way — Moonshot changed control fields within the
    // family (k3 lesson), so wire behaviour stays per model id in
    // `reasoning_wire_control`.
    e(
        Match::VersionLane {
            prefix: "kimi-k",
            min: &[3],
            suffix: None,
        },
        ModelTier::Full,
        true,
        "Kimi K3+",
    ),
    // MiniMax M family lane, floor 3 — 2026-07-18 3×2 A/B (mobile
    // multi-screen / dashboard / landing): Full cut geometry issues 5→1,
    // ~30% faster, fewer calls, better completeness — a non-marginal win on
    // every axis. Only M3+ is strong; older M2.x / abab keep falling
    // through to the generic minimax Basic row below (order unchanged).
    // Same-family-same-variant newer versions (minimax-m4, …) inherit Full;
    // unknown variant suffixes fall through to the conservative default.
    // `thinking` is kept by ChatProviderLlmClient's m3_keeps_thinking
    // special case (Adaptive), so thinking_disabled is effectively inactive
    // for M3 and kept only for in-family consistency. Wire-control
    // whitelists deliberately do NOT inherit this way (Moonshot k3 lesson).
    e(
        Match::VersionLane {
            prefix: "minimax-m",
            min: &[3],
            suffix: None,
        },
        ModelTier::Full,
        true,
        "MiniMax M3+",
    ),
    // DeepSeek V `-flash` lane, floor 4 — the variant suffix is a lane
    // boundary: `-pro` is Full (row above), `-flash` stays Standard.
    // Same-family-same-variant newer versions (deepseek-v5-flash, …)
    // inherit; unknown variant suffixes fall through to the conservative
    // default. Wire-control whitelists deliberately do NOT inherit this way
    // (Moonshot k3 lesson).
    e(
        Match::VersionLane {
            prefix: "deepseek-v",
            min: &[4],
            suffix: Some("-flash"),
        },
        ModelTier::Standard,
        true,
        "DeepSeek V4+ Flash",
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
    // Legacy GLM-4.x rows — the former blanket `Sub("glm")` catch-all is
    // gone: under the lane policy every glm id not caught by a lane (below
    // the floor, or an unknown variant suffix such as glm-6-air) must fall
    // through to DEFAULT_PROFILE (Standard), not inherit a family-wide
    // Basic tier. These three explicit ids stay Basic to preserve shipped
    // behaviour (glm-4-plus is the Basic-tier fixture in orchestrator retry
    // tests; glm-4.6 / glm-4 are the Basic arm in skill-budget and deck
    // skill tests).
    e(
        Match::Sub("glm-4-plus"),
        ModelTier::Basic,
        true,
        "GLM 4 Plus",
    ),
    e(Match::Sub("glm-4.6"), ModelTier::Basic, true, "GLM 4.6"),
    e(Match::Exact(&["glm-4"]), ModelTier::Basic, true, "GLM 4"),
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

/// Parses the dotted numeric version at the start of `s`. Returns the
/// segment values and the number of bytes consumed. Requires at least one
/// digit and rejects a trailing `.` without a following digit; the first
/// character that is neither a digit nor a `.` ends the version.
fn parse_dotted_version(s: &str) -> Option<(Vec<u32>, usize)> {
    let mut segments: Vec<u32> = Vec::new();
    let mut current: Option<u32> = None;
    let mut consumed = 0usize;
    for (offset, ch) in s.char_indices() {
        match ch {
            '0'..='9' => {
                let digit = ch.to_digit(10).expect("matched digit range");
                current = Some(current.unwrap_or(0).checked_mul(10)?.checked_add(digit)?);
                consumed = offset + ch.len_utf8();
            }
            '.' => {
                segments.push(current?);
                current = None;
                consumed = offset + ch.len_utf8();
            }
            _ => break,
        }
    }
    segments.push(current?);
    Some((segments, consumed))
}

/// Numeric comparison of two dotted versions with missing segments read as
/// 0: `[6] >= [5,2]` holds, `[5,1] < [5,2]` does not.
fn version_at_least(version: &[u32], min: &[u32]) -> bool {
    let longest = version.len().max(min.len());
    for i in 0..longest {
        let a = version.get(i).copied().unwrap_or(0);
        let b = min.get(i).copied().unwrap_or(0);
        match a.cmp(&b) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {}
        }
    }
    true
}

/// [`Match::VersionLane`] semantics on the already-normalized lowercase id:
/// locate `prefix` anywhere in the id, parse the dotted numeric version
/// right after it, require version >= `min`, and require the tail after the
/// version to be exactly `suffix` (or empty). An unknown variant tail (e.g.
/// `glm-6-air`) therefore does not match and the id falls through to the
/// conservative default.
fn version_lane_matches(lower: &str, prefix: &str, min: &[u32], suffix: Option<&str>) -> bool {
    let Some(start) = lower.find(prefix) else {
        return false;
    };
    let after_prefix = &lower[start + prefix.len()..];
    let Some((version, consumed)) = parse_dotted_version(after_prefix) else {
        return false;
    };
    if !version_at_least(&version, min) {
        return false;
    }
    let tail = &after_prefix[consumed..];
    match suffix {
        Some(suffix) => tail == suffix,
        None => tail.is_empty(),
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
            Match::VersionLane {
                prefix,
                min,
                suffix,
            } => version_lane_matches(&lower, prefix, min, *suffix),
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
#[path = "model_profile_tests.rs"]
mod tests;
