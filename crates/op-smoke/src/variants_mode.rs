//! `--variants N`: run one brief as N side-by-side design directions.
//!
//! This is NOT a reimplementation of the variants feature: it plans the
//! directions with the same `choose_variant_style_guides` the desktop's
//! `LaunchRoute::Variants` turn uses and drives them through the shared
//! `op_orchestrator::variants_run::run_design_variants` runner, landing
//! every direction on the smoke's own document. The caller then fills
//! images and saves the `.op` exactly as a single-design run does.

use op_orchestrator::variants::{choose_variant_style_guides, VariantPlan};
use op_orchestrator::{
    AbortFlag, DesignRequest, DocSink, LlmClient, OrchestratorError, Progress, RunSummary,
    ValidationProviders,
};

/// Upper bound on directions, matching what the product offers.
pub(crate) const MAX_VARIANTS: usize = 4;

/// The smoke's positional arguments after `--variants` parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmokeArgs {
    /// `Some(n)` when `--variants n` (or `--variants=n`) was given.
    pub variants: Option<usize>,
    pub prompt: String,
}

/// Parse `op-smoke [--variants N] <prompt>` (argv without the program name).
/// `Err` carries a one-line reason; the caller prints usage.
pub(crate) fn parse_smoke_args(args: &[String]) -> Result<SmokeArgs, String> {
    let mut variants = None;
    let mut prompt = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let count = if arg == "--variants" {
            Some(
                iter.next()
                    .ok_or_else(|| "--variants needs a count".to_string())?
                    .as_str(),
            )
        } else {
            arg.strip_prefix("--variants=")
        };
        if let Some(count) = count {
            let n: usize = count
                .trim()
                .parse()
                .map_err(|_| format!("--variants: not a number: {count:?}"))?;
            if !(2..=MAX_VARIANTS).contains(&n) {
                return Err(format!("--variants must be 2..={MAX_VARIANTS}, got {n}"));
            }
            variants = Some(n);
        } else if prompt.is_none() && !arg.is_empty() {
            prompt = Some(arg.clone());
        } else {
            return Err(format!("unexpected argument: {arg:?}"));
        }
    }
    let prompt = prompt.ok_or_else(|| "missing <prompt>".to_string())?;
    Ok(SmokeArgs { variants, prompt })
}

/// The directions the desktop would plan for this request.
pub(crate) fn plan_variants(request: &DesignRequest, count: usize) -> Vec<VariantPlan> {
    choose_variant_style_guides(
        &request.prompt,
        request.pinned_style_guide.as_deref(),
        count,
    )
}

/// Plan and run `count` directions through the shared variants runner,
/// landing them on `sink`. Progress events are dumped to stderr in the
/// same `[PROGRESS]` shape the single-design path prints, plus one
/// `[VARIANTS]` line per plan so a log reader can see the chosen guides.
pub(crate) async fn run_variants(
    count: usize,
    request: &DesignRequest,
    sink: &mut dyn DocSink,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    providers: &ValidationProviders<'_>,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<RunSummary, OrchestratorError> {
    let plans = plan_variants(request, count);
    for plan in &plans {
        eprintln!(
            "[VARIANTS] plan {} style_guide={} label={:?}",
            op_editor_core::variant_letter(plan.index),
            plan.style_guide,
            plan.style_label
        );
    }
    let base_state = sink.state().clone();
    let started = std::time::Instant::now();
    // Stamp each direction's landing with the wall time since launch so a
    // log reader gets per-direction timings without diffing timestamps.
    let mut timed = |event: Progress| {
        match &event {
            Progress::VariantReady(variant) => eprintln!(
                "[VARIANTS] ready {} after {:.1}s ({} board(s))",
                op_editor_core::variant_letter(variant.index),
                started.elapsed().as_secs_f64(),
                variant.root_ids.len()
            ),
            Progress::VariantFailed { index, error, .. } => eprintln!(
                "[VARIANTS] failed {} after {:.1}s: {error}",
                op_editor_core::variant_letter(*index),
                started.elapsed().as_secs_f64()
            ),
            _ => {}
        }
        on_progress(event);
    };
    op_orchestrator::variants_run::run_design_variants(
        request,
        &plans,
        &base_state,
        llm,
        sink,
        &mut timed,
        abort,
        providers,
        None,
    )
    .await
}

#[cfg(test)]
#[path = "variants_mode_tests.rs"]
mod tests;
