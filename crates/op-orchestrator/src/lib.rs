//! `op-orchestrator` — S3a 设计编排器(单屏顺序骨架)。
//!
//! 把 TS `apps/web/src/services/ai/orchestrator.ts` 的阶段 1-4
//! 单屏路径补回 Rust。副作用只走 [`DocSink`] / [`LlmClient`] 两个
//! trait,核心逻辑全是纯函数,不依赖 winit/casement/agent。
//!
//! Plan B 提供"零件"(类型 + intent/plan/parse/normalize/variables);
//! Plan C 在 `run` 模块接出四阶段主轴。

pub mod compact_prompt;
pub mod compact_skills;
pub mod dashboard_columns;
pub mod design_md_policy;
pub mod design_system;
pub mod design_type;
pub mod intent;
pub mod model_profile;
pub mod parse;
pub mod plan;
pub mod plan_normalize;
pub mod plan_repair;
pub mod retry;
pub mod stub_providers;
pub mod style_guide_context;
pub mod timeouts;
pub mod types;
pub mod validation;
pub mod validation_config;
pub mod validation_dump;
pub mod validation_fixes;
pub(crate) mod validation_fixes_b3;
pub mod variables;

pub mod append;
pub mod cleanup;
pub(crate) mod cleanup_typography;
pub mod concurrent;
pub mod prompt;
pub mod role_defaults;
pub mod role_infer;
pub mod role_post_pass;
pub mod run;
pub mod run_dashboard;
pub mod scaffold;
pub mod scaffold_dashboard;
pub mod subagent;

#[cfg(test)]
mod test_support;

pub use compact_prompt::{build_compact_planning_prompt, CompactPlanningPrompt};
pub use design_md_policy::{
    build_design_md_style_policy, guess_neutral_background_from_theme, infer_design_md_background,
};
pub use design_system::{
    default_design_system, design_system_to_prompt_context, design_system_to_seed_commands,
    generate_design_system, parse_design_system, DesignSystem, Spacing, Typography,
};
pub use design_type::{detect_design_type, DesignType, DesignTypePreset};
pub use intent::classify_intent;
pub use model_profile::{resolve_model_profile, ModelProfile, ModelTier};
pub use prompt::build_orchestrator_prompt;
pub use run::Orchestrator;
pub use stub_providers::{
    SkippedPreValidator, SkippedScreenshotProvider, SkippedVisionLlmClient,
    SkippedVisualRefProvider,
};
pub use types::*;
