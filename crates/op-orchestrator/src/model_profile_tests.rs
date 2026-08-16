//! Tests for the model capability profile table (model_profile).
//!
//! Sibling of `model_profile.rs` so the spine stays under the 800-line cap;
//! the `#[path]` module keeps the `model_profile::tests` path stable.

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
        "OpenAI-compatible strict endpoints reject unknown body fields \
             with 400; the model has no `thinking` body-field contract",
    ),
    (
        "gemini-3-flash-preview",
        "Google's OpenAI-compat shim is not documented to accept it",
    ),
    ("qwen-plus", "no verified thinking-disable field"),
    ("qwen3-coder-plus", "no verified thinking-disable field"),
    (
        "doubao-seed-2-0-pro-260215",
        "no verified thinking-disable field",
    ),
    ("Qwen/Qwen3.5-35B-A3B", "no verified thinking-disable field"),
    ("ark-code-latest", "no verified thinking-disable field"),
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

/// Version-lane comparator unit tests: numeric dotted comparison with
/// missing segments read as 0, the version tail rule, and the
/// prefix-locating rule.
#[test]
fn version_lane_comparator_and_tail_rule() {
    // [6] >= [5,2] holds (glm-6 inherits); [5,1] < [5,2] does not.
    assert!(version_at_least(&[6], &[5, 2]));
    assert!(!version_at_least(&[5, 1], &[5, 2]));
    assert!(version_at_least(&[5, 2], &[5, 2]));
    assert!(version_at_least(&[5, 3], &[5, 2]));
    assert!(version_at_least(&[5, 20], &[5, 2]));

    // Unknown variant suffix after the version does not match.
    assert!(!version_lane_matches("glm-6-air", "glm-", &[5, 2], None));
    assert!(version_lane_matches("glm-6", "glm-", &[5, 2], None));
    // Prefix absent (or followed by a non-version) does not match.
    assert!(!version_lane_matches(
        "claude-sonnet-5",
        "glm-",
        &[5, 2],
        None
    ));
    assert!(!version_lane_matches("glm-x", "glm-", &[5, 2], None));
    // A required suffix must match exactly; None requires end-of-string.
    assert!(version_lane_matches(
        "deepseek-v5-pro",
        "deepseek-v",
        &[4],
        Some("-pro")
    ));
    assert!(!version_lane_matches(
        "deepseek-v5-pro-max",
        "deepseek-v",
        &[4],
        Some("-pro")
    ));
    assert!(!version_lane_matches("glm-5.2-fp8", "glm-", &[5, 2], None));
    assert!(version_lane_matches("glm-5.2", "glm-", &[5, 2], None));
    // Version below the floor does not match, even with a clean tail.
    assert!(!version_lane_matches("glm-5.1", "glm-", &[5, 2], None));
    // Malformed versions do not match.
    assert!(!version_lane_matches("glm-5.", "glm-", &[5, 2], None));
    assert!(!version_lane_matches("glm-.5", "glm-", &[5, 2], None));
}

/// GLM lane: explicit glm-5.3 override (×3), lane inheritance (×2),
/// below-floor and unknown-variant fall-through to the default.
#[test]
fn glm_version_lane_entries() {
    let five_three = resolve_model_profile("glm-5.3");
    assert_eq!(five_three.tier, ModelTier::Full);
    assert!(five_three.thinking_disabled);
    assert_eq!(five_three.timeout_multiplier, 3.0);
    // Vendor prefix (ark/) hits the same rows.
    assert_eq!(resolve_model_profile("ark/glm-5.3").tier, ModelTier::Full);
    assert_eq!(resolve_model_profile("ark/glm-5.3").timeout_multiplier, 3.0);

    // glm-6 inherits the lane (numeric comparison: [6] >= [5,2]).
    let six = resolve_model_profile("glm-6");
    assert_eq!(six.tier, ModelTier::Full);
    assert_eq!(six.timeout_multiplier, 2.0);

    // glm-5.2 keeps its old lane behaviour (no regression).
    let five_two = resolve_model_profile("glm-5.2");
    assert_eq!(five_two.tier, ModelTier::Full);
    assert_eq!(five_two.timeout_multiplier, 2.0);

    // Below the floor and unknown variants fall to the default Standard.
    for below in ["glm-5.1", "glm-4.7", "glm-6-air"] {
        assert_eq!(
            resolve_model_profile(below).tier,
            ModelTier::Standard,
            "model={below}"
        );
    }
}

/// Kimi lane: K3 floor; k3.1-preview keeps its shipped Full tier via an
/// explicit row (the strict tail rule would otherwise drop it).
#[test]
fn kimi_version_lane_entries() {
    assert_eq!(resolve_model_profile("kimi-k4").tier, ModelTier::Full);
    assert_eq!(resolve_model_profile("kimi-k3").tier, ModelTier::Full);
    assert_eq!(
        resolve_model_profile("kimi-k3.1-preview").tier,
        ModelTier::Full
    );
    // K2.x stays below the floor (existing behaviour).
    assert_eq!(resolve_model_profile("kimi-k2.5").tier, ModelTier::Standard);
}

/// MiniMax lane: M3 floor; older M2.x / abab keep their existing tiers.
#[test]
fn minimax_version_lane_entries() {
    assert_eq!(resolve_model_profile("minimax-m4").tier, ModelTier::Full);
    assert_eq!(resolve_model_profile("MiniMax-M3").tier, ModelTier::Full);
    assert_eq!(resolve_model_profile("MiniMax-M2.7").tier, ModelTier::Basic);
    // abab naming has no lane: it keeps falling to the default.
    assert_eq!(
        resolve_model_profile("abab6.5s-chat").tier,
        ModelTier::Standard
    );
}

/// DeepSeek dual lanes: the variant suffix is the lane boundary.
#[test]
fn deepseek_variant_lane_entries() {
    let v5_pro = resolve_model_profile("deepseek-v5-pro");
    assert_eq!(v5_pro.tier, ModelTier::Full);
    assert_eq!(v5_pro.timeout_multiplier, 2.0);
    assert_eq!(
        resolve_model_profile("deepseek-v5-flash").tier,
        ModelTier::Standard
    );
    // v4 rows do not regress.
    let v4_pro = resolve_model_profile("deepseek-v4-pro");
    assert_eq!(v4_pro.tier, ModelTier::Full);
    assert_eq!(v4_pro.timeout_multiplier, 2.0);
    assert_eq!(
        resolve_model_profile("deepseek-v4-flash").tier,
        ModelTier::Standard
    );
    // The legacy exact ids are untouched.
    assert_eq!(
        resolve_model_profile("deepseek-chat").tier,
        ModelTier::Standard
    );
    assert_eq!(
        resolve_model_profile("deepseek-reasoner").tier,
        ModelTier::Standard
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
