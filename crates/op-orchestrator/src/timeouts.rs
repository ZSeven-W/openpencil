//! 超时档位表 —— S3b-1b Task A2。
//!
//! Port of:
//! - `ai-runtime-config.ts` — `PROMPT_TIMEOUT_BUCKETS`,
//!   `ORCHESTRATOR_TIMEOUT_PROFILES`, `SUB_AGENT_TIMEOUT_PROFILES`
//! - `orchestrator-prompt-optimizer.ts` — `getOrchestratorTimeouts`,
//!   `getBuiltinPlanningTimeouts`, `getSubAgentTimeouts`
//! - `model-profiles.ts` — `applyProfileToTimeouts`

#![allow(dead_code)]

use crate::model_profile::ModelTier;
use std::time::Duration;

// ── 从 TS `ai-runtime-config.ts` 逐字搬来的毫秒常量 ────────────────────────

/// 规划 prompt 桶阈值 1 —— `PROMPT_OPTIMIZER_LIMITS.longPromptCharThreshold`。
/// `< SHORT_THRESH` → Short。
const SHORT_THRESH: usize = 2200;

/// 规划 prompt 桶阈值 2 —— `PROMPT_TIMEOUT_BUCKETS.mediumPromptMaxChars`。
/// `SHORT_THRESH <= len < MEDIUM_THRESH` → Medium; `>= MEDIUM_THRESH` → Long。
const MEDIUM_THRESH: usize = 4200;

// Orchestrator timeout profiles (ms) — verbatim from ORCHESTRATOR_TIMEOUT_PROFILES
const ORCH_SHORT_HARD_MS: u64 = 300_000;
const ORCH_SHORT_NO_TEXT_MS: u64 = 150_000;
const ORCH_SHORT_FIRST_TEXT_MS: u64 = 300_000;

const ORCH_MEDIUM_HARD_MS: u64 = 420_000;
const ORCH_MEDIUM_NO_TEXT_MS: u64 = 210_000;
const ORCH_MEDIUM_FIRST_TEXT_MS: u64 = 420_000;

const ORCH_LONG_HARD_MS: u64 = 600_000;
const ORCH_LONG_NO_TEXT_MS: u64 = 300_000;
const ORCH_LONG_FIRST_TEXT_MS: u64 = 600_000;

// Sub-agent timeout profiles (ms) — verbatim from SUB_AGENT_TIMEOUT_PROFILES
// Base: hardTimeoutMs=420_000, noTextTimeoutMs=210_000 (SUB_AGENT_TIMEOUT_BASE)
const SA_SHORT_HARD_MS: u64 = 420_000;
const SA_SHORT_NO_TEXT_MS: u64 = 210_000;
const SA_SHORT_FIRST_TEXT_MS: u64 = 420_000;

const SA_MEDIUM_HARD_MS: u64 = 600_000;
const SA_MEDIUM_NO_TEXT_MS: u64 = 300_000;
const SA_MEDIUM_FIRST_TEXT_MS: u64 = 600_000;

const SA_LONG_HARD_MS: u64 = 900_000;
const SA_LONG_NO_TEXT_MS: u64 = 480_000;
const SA_LONG_FIRST_TEXT_MS: u64 = 900_000;

// Basic-tier sub-agent clamps — from getSubAgentTimeouts:
//   noTextTimeoutMs  = Math.min(noTextTimeoutMs,  45_000)
//   firstTextTimeoutMs = Math.min(firstTextTimeoutMs, 75_000)
const SA_BASIC_NO_TEXT_MAX_MS: u64 = 45_000;
const SA_BASIC_FIRST_TEXT_MAX_MS: u64 = 75_000;

// Builtin planning base timeouts — from getBuiltinPlanningTimeouts
const BUILTIN_HARD_MS: u64 = 60_000;
const BUILTIN_NO_TEXT_MS: u64 = 30_000;
const BUILTIN_FIRST_TEXT_MS: u64 = 30_000;

// Basic-tier builtin planning floors (Math.max raises):
//   hardTimeoutMs       = Math.max(hard,       150_000)
//   noTextTimeoutMs     = Math.max(no_text,     75_000)
//   firstTextTimeoutMs  = Math.max(first_text,  75_000)
const BUILTIN_BASIC_HARD_MIN_MS: u64 = 150_000;
const BUILTIN_BASIC_NO_TEXT_MIN_MS: u64 = 75_000;
const BUILTIN_BASIC_FIRST_TEXT_MIN_MS: u64 = 75_000;

// ── 公开类型 ─────────────────────────────────────────────────────────────────

/// 一次 LLM 调用的三路超时。
///
/// - `hard` — 整体硬截止
/// - `no_text` — 从连接建立到第一个 text chunk 的超时(非 abort 中途超时)
/// - `first_text` — 从第一个 text chunk 到真正"文字内容"出现的超时
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeouts {
    pub hard: Duration,
    pub no_text: Duration,
    pub first_text: Duration,
}

impl Timeouts {
    fn from_ms(hard_ms: u64, no_text_ms: u64, first_text_ms: u64) -> Self {
        Self {
            hard: Duration::from_millis(hard_ms),
            no_text: Duration::from_millis(no_text_ms),
            first_text: Duration::from_millis(first_text_ms),
        }
    }
}

/// prompt 长度对应的桶 —— 决定采用哪组超时档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptBucket {
    /// `< 2200` 字符
    Short,
    /// `2200 <= len < 4200` 字符
    Medium,
    /// `>= 4200` 字符
    Long,
}

/// 根据 prompt 字节长度确定桶。
///
/// Port of the bucket selection in `getOrchestratorTimeouts` /
/// `getSubAgentTimeouts`:
/// ```ts
/// if (len < PROMPT_OPTIMIZER_LIMITS.longPromptCharThreshold) Short
/// else if (len < PROMPT_TIMEOUT_BUCKETS.mediumPromptMaxChars)  Medium
/// else                                                          Long
/// ```
pub fn bucket_for(prompt_len: usize) -> PromptBucket {
    if prompt_len < SHORT_THRESH {
        PromptBucket::Short
    } else if prompt_len < MEDIUM_THRESH {
        PromptBucket::Medium
    } else {
        PromptBucket::Long
    }
}

// ── 公开函数 ─────────────────────────────────────────────────────────────────

/// 规划阶段超时 —— port of `getOrchestratorTimeouts(promptLength, model)`.
///
/// `tier` 只影响通过 `apply_profile_to_timeouts` 注入的 `timeout_multiplier`。
/// 本函数不接 model_id,调用方提前 resolve 好 multiplier 传入即可。
/// 注:TS 原函数最后调用 `applyProfileToTimeouts` —— 此处分两步
/// (`orchestrator_timeouts` 取基值,再由调用方做 `apply_profile_to_timeouts`)
/// 保持函数接口纯净。
pub fn orchestrator_timeouts(prompt_len: usize) -> Timeouts {
    match bucket_for(prompt_len) {
        PromptBucket::Short => Timeouts::from_ms(
            ORCH_SHORT_HARD_MS,
            ORCH_SHORT_NO_TEXT_MS,
            ORCH_SHORT_FIRST_TEXT_MS,
        ),
        PromptBucket::Medium => Timeouts::from_ms(
            ORCH_MEDIUM_HARD_MS,
            ORCH_MEDIUM_NO_TEXT_MS,
            ORCH_MEDIUM_FIRST_TEXT_MS,
        ),
        PromptBucket::Long => Timeouts::from_ms(
            ORCH_LONG_HARD_MS,
            ORCH_LONG_NO_TEXT_MS,
            ORCH_LONG_FIRST_TEXT_MS,
        ),
    }
}

/// sub-agent 超时 —— port of `getSubAgentTimeouts(promptLength, model)`.
///
/// 基础档位由 prompt 长度决定;Basic tier 对 `no_text` / `first_text`
/// 施加上限(TS: `Math.min(..., 45_000)` / `Math.min(..., 75_000)`)。
pub fn sub_agent_timeouts(prompt_len: usize, tier: ModelTier) -> Timeouts {
    let mut t = match bucket_for(prompt_len) {
        PromptBucket::Short => Timeouts::from_ms(
            SA_SHORT_HARD_MS,
            SA_SHORT_NO_TEXT_MS,
            SA_SHORT_FIRST_TEXT_MS,
        ),
        PromptBucket::Medium => Timeouts::from_ms(
            SA_MEDIUM_HARD_MS,
            SA_MEDIUM_NO_TEXT_MS,
            SA_MEDIUM_FIRST_TEXT_MS,
        ),
        PromptBucket::Long => {
            Timeouts::from_ms(SA_LONG_HARD_MS, SA_LONG_NO_TEXT_MS, SA_LONG_FIRST_TEXT_MS)
        }
    };
    if tier == ModelTier::Basic {
        t.no_text = t
            .no_text
            .min(Duration::from_millis(SA_BASIC_NO_TEXT_MAX_MS));
        t.first_text = t
            .first_text
            .min(Duration::from_millis(SA_BASIC_FIRST_TEXT_MAX_MS));
    }
    t
}

/// 内建规划超时(Compact/Minimal 模式的轻量规划调用)——
/// port of `getBuiltinPlanningTimeouts(model)`.
///
/// 基础值固定;Basic tier 用 `Math.max` 提高下限
/// (Basic 模型较慢,需要更多时间思考但要禁 thinking)。
pub fn builtin_planning_timeouts(tier: ModelTier) -> Timeouts {
    let mut t = Timeouts::from_ms(BUILTIN_HARD_MS, BUILTIN_NO_TEXT_MS, BUILTIN_FIRST_TEXT_MS);
    if tier == ModelTier::Basic {
        t.hard = t.hard.max(Duration::from_millis(BUILTIN_BASIC_HARD_MIN_MS));
        t.no_text = t
            .no_text
            .max(Duration::from_millis(BUILTIN_BASIC_NO_TEXT_MIN_MS));
        t.first_text = t
            .first_text
            .max(Duration::from_millis(BUILTIN_BASIC_FIRST_TEXT_MIN_MS));
    }
    t
}

/// 按模型的 `timeout_multiplier` 缩放三路超时 —— port of
/// `applyProfileToTimeouts` (the timeout-scaling part only).
///
/// `multiplier == 1.0` 为 no-op。四舍五入到整毫秒(`Math.round` 对齐)。
pub fn apply_profile_to_timeouts(t: Timeouts, multiplier: f64) -> Timeouts {
    if (multiplier - 1.0).abs() < f64::EPSILON {
        return t;
    }
    Timeouts {
        hard: mul_ms(t.hard, multiplier),
        no_text: mul_ms(t.no_text, multiplier),
        first_text: mul_ms(t.first_text, multiplier),
    }
}

fn mul_ms(d: Duration, m: f64) -> Duration {
    let ms = (d.as_millis() as f64 * m).round() as u64;
    Duration::from_millis(ms)
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── bucket_for ────────────────────────────────────────────────────────────

    #[test]
    fn bucket_short_below_2200() {
        assert_eq!(bucket_for(0), PromptBucket::Short);
        assert_eq!(bucket_for(2199), PromptBucket::Short);
    }

    #[test]
    fn bucket_medium_at_2200_below_4200() {
        assert_eq!(bucket_for(2200), PromptBucket::Medium);
        assert_eq!(bucket_for(4199), PromptBucket::Medium);
    }

    #[test]
    fn bucket_long_at_and_above_4200() {
        assert_eq!(bucket_for(4200), PromptBucket::Long);
        assert_eq!(bucket_for(9999), PromptBucket::Long);
    }

    // ── orchestrator_timeouts ─────────────────────────────────────────────────

    #[test]
    fn orchestrator_timeouts_short_bucket() {
        let t = orchestrator_timeouts(100);
        assert_eq!(t.hard, Duration::from_millis(300_000));
        assert_eq!(t.no_text, Duration::from_millis(150_000));
        assert_eq!(t.first_text, Duration::from_millis(300_000));
    }

    #[test]
    fn orchestrator_timeouts_medium_bucket() {
        let t = orchestrator_timeouts(3000);
        assert_eq!(t.hard, Duration::from_millis(420_000));
        assert_eq!(t.no_text, Duration::from_millis(210_000));
        assert_eq!(t.first_text, Duration::from_millis(420_000));
    }

    #[test]
    fn orchestrator_timeouts_long_bucket() {
        let t = orchestrator_timeouts(5000);
        assert_eq!(t.hard, Duration::from_millis(600_000));
        assert_eq!(t.no_text, Duration::from_millis(300_000));
        assert_eq!(t.first_text, Duration::from_millis(600_000));
    }

    // ── sub_agent_timeouts ────────────────────────────────────────────────────

    #[test]
    fn sub_agent_timeouts_short_full_tier() {
        let t = sub_agent_timeouts(100, ModelTier::Full);
        assert_eq!(t.hard, Duration::from_millis(420_000));
        assert_eq!(t.no_text, Duration::from_millis(210_000));
        assert_eq!(t.first_text, Duration::from_millis(420_000));
    }

    #[test]
    fn sub_agent_timeouts_medium_standard_tier() {
        let t = sub_agent_timeouts(3000, ModelTier::Standard);
        assert_eq!(t.hard, Duration::from_millis(600_000));
        assert_eq!(t.no_text, Duration::from_millis(300_000));
        assert_eq!(t.first_text, Duration::from_millis(600_000));
    }

    #[test]
    fn sub_agent_timeouts_long_full_tier() {
        let t = sub_agent_timeouts(5000, ModelTier::Full);
        assert_eq!(t.hard, Duration::from_millis(900_000));
        assert_eq!(t.no_text, Duration::from_millis(480_000));
        assert_eq!(t.first_text, Duration::from_millis(900_000));
    }

    #[test]
    fn sub_agent_timeouts_basic_tier_clamps_no_text_and_first_text() {
        // Short bucket base: no_text=210_000, first_text=420_000
        // Basic clamp: no_text ≤ 45_000, first_text ≤ 75_000
        let t = sub_agent_timeouts(100, ModelTier::Basic);
        assert_eq!(t.hard, Duration::from_millis(420_000)); // hard unclamped
        assert_eq!(t.no_text, Duration::from_millis(45_000));
        assert_eq!(t.first_text, Duration::from_millis(75_000));
    }

    #[test]
    fn sub_agent_timeouts_basic_tier_long_bucket_still_clamped() {
        // Long bucket base: no_text=480_000, first_text=900_000
        let t = sub_agent_timeouts(5000, ModelTier::Basic);
        assert_eq!(t.hard, Duration::from_millis(900_000)); // hard unclamped
        assert_eq!(t.no_text, Duration::from_millis(45_000));
        assert_eq!(t.first_text, Duration::from_millis(75_000));
    }

    // ── builtin_planning_timeouts ─────────────────────────────────────────────

    #[test]
    fn builtin_planning_full_tier_base_values() {
        let t = builtin_planning_timeouts(ModelTier::Full);
        assert_eq!(t.hard, Duration::from_millis(60_000));
        assert_eq!(t.no_text, Duration::from_millis(30_000));
        assert_eq!(t.first_text, Duration::from_millis(30_000));
    }

    #[test]
    fn builtin_planning_standard_tier_base_values() {
        let t = builtin_planning_timeouts(ModelTier::Standard);
        assert_eq!(t.hard, Duration::from_millis(60_000));
        assert_eq!(t.no_text, Duration::from_millis(30_000));
        assert_eq!(t.first_text, Duration::from_millis(30_000));
    }

    #[test]
    fn builtin_planning_basic_tier_floors_raised() {
        // Basic: Math.max(hard, 150_000), Math.max(no_text, 75_000),
        //        Math.max(first_text, 75_000)
        let t = builtin_planning_timeouts(ModelTier::Basic);
        assert_eq!(t.hard, Duration::from_millis(150_000));
        assert_eq!(t.no_text, Duration::from_millis(75_000));
        assert_eq!(t.first_text, Duration::from_millis(75_000));
    }

    // ── apply_profile_to_timeouts ─────────────────────────────────────────────

    #[test]
    fn apply_multiplier_scales_all_three() {
        let base = Timeouts::from_ms(300_000, 150_000, 300_000);
        let scaled = apply_profile_to_timeouts(base, 2.0);
        assert_eq!(scaled.hard, Duration::from_millis(600_000));
        assert_eq!(scaled.no_text, Duration::from_millis(300_000));
        assert_eq!(scaled.first_text, Duration::from_millis(600_000));
    }

    #[test]
    fn apply_multiplier_one_is_noop() {
        let base = Timeouts::from_ms(300_000, 150_000, 300_000);
        let scaled = apply_profile_to_timeouts(base.clone(), 1.0);
        assert_eq!(scaled, base);
    }

    #[test]
    fn apply_multiplier_rounds_to_ms() {
        // 210_000 * 1.5 = 315_000 (exact)
        let base = Timeouts::from_ms(210_000, 210_000, 210_000);
        let scaled = apply_profile_to_timeouts(base, 1.5);
        assert_eq!(scaled.hard, Duration::from_millis(315_000));
    }
}
