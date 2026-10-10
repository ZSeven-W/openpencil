//! A bounded re-plan for supplied lines lost before multi-section generation.

use super::*;

pub(super) async fn ensure(
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    on_progress: &mut dyn FnMut(Progress),
    mut plan: OrchestratorPlan,
) -> Result<(OrchestratorPlan, NormInfo), OrchestratorError> {
    // Check the actual normalized owners, after umbrella folding and other
    // structural repairs have removed or combined planned sections.
    let norm = normalize_logging_coverage_drops(&mut plan, request);
    // A single section is already checked against the entire source before
    // insertion. Multi-section runs need explicit, non-duplicated ownership.
    if plan.subtasks.len() <= 1 {
        return Ok((plan, norm));
    }
    let contract = crate::source_copy::SourceCopy::from_brief(&request.prompt);
    let missing = contract.missing_in_plan(&plan);
    if missing.is_empty() {
        return Ok((plan, norm));
    }
    eprintln!(
        "[PLAN] source-copy: {} missing lines; re-planning",
        missing.len()
    );
    on_progress(Progress::PlanCoverageRetry { missing });
    let mut prompt = build_orchestrator_prompt(request, PlanningMode::Rich, abort.clone());
    prompt.call_request.user_prompt.push_str(
        "\n\nSOURCE COPY FIX REQUIRED: The previous plan omitted or paraphrased supplied lines. Return a complete plan with every complete source line in its owning section's `elements`, keeping the requested page count and dimensions.",
    );
    let raw = collect_text(llm.call(prompt.call_request))
        .await
        .map_err(|error| {
            if error.aborted {
                OrchestratorError::Aborted
            } else {
                OrchestratorError::AllFailed(error.message)
            }
        })?;
    if abort.is_set() {
        return Err(OrchestratorError::Aborted);
    }
    let Some((mut revised, _)) = parse_orchestrator_response(&raw, request) else {
        return Err(OrchestratorError::AllFailed(
            "source-copy-missing: could not plan all supplied text".into(),
        ));
    };
    apply_plan_pins(&mut revised, prompt.forced_style_guide_name, request);
    let required = crate::plan_coverage::required_sections(&request.prompt);
    let details = crate::plan_coverage::required_section_details(&request.prompt);
    crate::plan_coverage_append::append_missing_sections(&mut revised, &required, &details);
    let norm = normalize_logging_coverage_drops(&mut revised, request);
    let missing = contract.missing_in_plan(&revised);
    if !missing.is_empty() {
        return Err(OrchestratorError::AllFailed(
            crate::source_copy::SourceCopy::retry_feedback(&missing),
        ));
    }
    Ok((revised, norm))
}
