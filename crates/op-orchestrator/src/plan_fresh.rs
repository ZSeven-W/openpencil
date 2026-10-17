//! Acceptance contract for fresh planner responses, before normalization.
//!
//! Stored plans remain backwards compatible. New model responses explicitly
//! declare whether they share content, and cannot enter generation with stale
//! section references. This module never invents entities or repairs their ids.

use crate::plan::{OrchestratorPlan, PlanSharedContext};
use crate::types::DesignRequest;
use serde_json::Value;
use std::collections::BTreeSet;

#[path = "plan_fresh_error.rs"]
mod error;
pub(crate) use error::FreshPlanError;

pub(crate) fn check_envelope(value: &Value) -> Result<(), FreshPlanError> {
    let context = value
        .get("sharedContext")
        .ok_or(FreshPlanError::MissingSharedContext)?;
    if !context.is_object() {
        return Err(FreshPlanError::InvalidSharedContextShape);
    }
    if context.get("subtasks").is_some() {
        return Err(FreshPlanError::NestedSubtasks);
    }
    crate::plan_shared_context::from_plan_value(value)
        .map_err(FreshPlanError::InvalidSharedContext)?;
    Ok(())
}

pub(crate) fn accept(
    plan: &mut OrchestratorPlan,
    request: &DesignRequest,
) -> Result<(), FreshPlanError> {
    let Some(context) = &mut plan.shared_context else {
        return Ok(());
    };
    let declared: BTreeSet<_> = plan.subtasks.iter().map(|task| task.id.as_str()).collect();
    for entity in &context.example_entities {
        if entity.used_by.is_empty() {
            return Err(FreshPlanError::UnassignedEntity {
                entity_id: entity.id.clone(),
            });
        }
        for id in &entity.used_by {
            if !declared.contains(id.as_str()) {
                return Err(FreshPlanError::UnknownSection {
                    entity_id: entity.id.clone(),
                    section_id: id.clone(),
                });
            }
        }
    }
    let removed = retain_verbatim_sources(context, &request.prompt);
    if removed > 0 {
        // Count only: planner claims may include private source text.
        tracing::warn!(removed, "filtered non-verbatim planner sourceFacts");
    }
    if context.source_facts.is_empty() && context.example_entities.is_empty() {
        plan.shared_context = None;
    }
    Ok(())
}

/// A paraphrase or approximate match is not evidence that the user supplied a
/// fact. The full original brief is sent separately, so dropping an unverified
/// duplicate does not drop the user's source content.
pub(crate) fn retain_verbatim_sources(context: &mut PlanSharedContext, brief: &str) -> usize {
    let before = context.source_facts.len();
    context
        .source_facts
        .retain(|fact| !fact.trim().is_empty() && brief.contains(fact.as_str()));
    before - context.source_facts.len()
}

#[cfg(test)]
#[path = "plan_fresh_tests.rs"]
mod tests;
