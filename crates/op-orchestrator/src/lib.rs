//! `op-orchestrator` — S3a 设计编排器(单屏顺序骨架)。
//!
//! 把 TS `apps/web/src/services/ai/orchestrator.ts` 的阶段 1-4
//! 单屏路径补回 Rust。副作用只走 [`DocSink`] / [`LlmClient`] 两个
//! trait,核心逻辑全是纯函数,不依赖 winit/casement/agent。
//!
//! Plan B 提供"零件"(类型 + intent/plan/parse/normalize/variables);
//! Plan C 在 `run` 模块接出四阶段主轴。

pub mod agent_identity;
pub mod compact_prompt;
pub mod compact_skills;
pub mod dashboard_columns;
pub mod design_md_policy;
// (run_dashboard / scaffold_dashboard removed: dashboards flow through the
//  generic sequential path; dashboard_columns keeps only normalizer predicates.)
pub mod design_system;
pub mod design_type;
pub mod intent;
pub mod model_profile;
pub(crate) mod orchestration_self_check;
pub mod palette_harmonize;
pub mod parse;
pub mod plan;
pub mod plan_normalize;
pub mod plan_repair;
pub mod program_gen;
pub mod retry;
pub mod script_gen;
pub mod semantic_palette;
pub mod stub_providers;
pub mod style_guide_context;
pub mod timeouts;
pub mod types;
pub mod validation;
pub mod validation_config;
pub mod validation_dump;
pub mod validation_fixes;
pub(crate) mod validation_fixes_b3;
mod variable_binding;
pub mod variables;

pub mod app_shell;
pub mod append;
pub(crate) mod chip_repair;
pub mod cleanup;
pub(crate) mod cleanup_layout;
pub(crate) mod cleanup_typography;
pub mod concurrent;
pub mod geometry_validation;
pub mod loop_finalize;
pub mod prompt;
pub mod radial_repair;
pub mod role_defaults;
pub mod role_infer;
pub(crate) mod role_layout_post_pass;
pub mod role_post_pass;
pub mod run;
pub mod scaffold;
pub mod spacing_repair;
pub mod spawn_concurrent;
pub mod stub_repair;
pub mod subagent;
pub mod table_repair;
pub mod tree_heuristics;

#[cfg(test)]
mod geometry_chip_tests;
#[cfg(test)]
mod radial_stub_tests;
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
pub use loop_finalize::apply_loop_finalize;
pub use model_profile::{resolve_model_profile, ModelProfile, ModelTier};
pub use prompt::build_orchestrator_prompt;
pub use run::Orchestrator;
pub use spawn_concurrent::{run_spawned_agents_concurrent, SpawnAgentResult, SpawnAgentSpec};
pub use stub_providers::{
    SkippedPreValidator, SkippedScreenshotProvider, SkippedVisionLlmClient,
    SkippedVisualRefProvider,
};
pub use types::*;
pub use validation::{run_post_generation_validation, ValidationSummary};

#[cfg(test)]
pub(crate) mod agent_indicator_test_support {
    use std::sync::{LazyLock, Mutex, MutexGuard};

    static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    pub(crate) fn lock() -> MutexGuard<'static, ()> {
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }
}
