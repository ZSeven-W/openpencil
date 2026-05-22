//! Configuration constants for the vision-validation pipeline.
//!
//! Faithful port of `apps/web/src/services/ai/ai-runtime-config.ts:119-123`.

/// Minimum total node count in the active page to trigger the vision LLM
/// validation loop.  Designs producing fewer nodes than this threshold
/// skip vision (saves +30-90s + vision API tokens) — pre-validation
/// heuristics still run regardless.  Composite designs (multi-section
/// dashboards, full-page mockups) easily clear this; atomic single-tool
/// outputs (one badge, one chart) stay fast.
///
/// Port of `VALIDATION_NODE_COUNT_THRESHOLD = 30` in `ai-runtime-config.ts`.
pub const VALIDATION_NODE_COUNT_THRESHOLD: usize = 30;

/// Maximum number of vision-LLM validation rounds per generation.
///
/// Port of `MAX_VALIDATION_ROUNDS = 3` in `ai-runtime-config.ts`.
pub const MAX_VALIDATION_ROUNDS: u8 = 3;

/// Quality score (0–10) at or above which the validation loop exits early
/// without running the next round.
///
/// Port of `VALIDATION_QUALITY_THRESHOLD = 8` in `ai-runtime-config.ts`.
pub const VALIDATION_QUALITY_THRESHOLD: u8 = 8;

/// Per-round vision LLM call timeout in milliseconds.
///
/// Port of `VALIDATION_TIMEOUT_MS = 180_000` in `ai-runtime-config.ts`.
pub const VALIDATION_TIMEOUT_MS: u64 = 180_000;
