//! `op-orchestrator` — S3a 设计编排器(单屏顺序骨架)。
//!
//! 把 TS `apps/web/src/services/ai/orchestrator.ts` 的阶段 1-4
//! 单屏路径补回 Rust。副作用只走 [`DocSink`] / [`LlmClient`] 两个
//! trait,核心逻辑全是纯函数,不依赖 winit/casement/agent。
//!
//! Plan B 提供"零件"(类型 + intent/plan/parse/normalize/variables);
//! Plan C 在 `run` 模块接出四阶段主轴。

pub mod intent;
pub mod parse;
pub mod plan;
pub mod plan_normalize;
pub mod types;
pub mod variables;

pub mod cleanup;
pub mod prompt;
pub mod run;
pub mod scaffold;
pub mod subagent;

#[cfg(test)]
mod test_support;

pub use intent::classify_intent;
pub use run::Orchestrator;
pub use types::*;
