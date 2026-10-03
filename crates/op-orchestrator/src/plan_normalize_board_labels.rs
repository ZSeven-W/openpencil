//! Keep explicitly requested full-board tasks on independent artboards.

use crate::plan::OrchestratorPlan;
use crate::types::DesignRequest;
use crate::DesignType;

pub(super) fn apply(plan: &mut OrchestratorPlan, request: &DesignRequest) {
    let count = match crate::detect_design_type(&request.prompt).type_ {
        DesignType::Slides => crate::plan::explicit_slide_count(&request.prompt),
        DesignType::Card => crate::plan_fallback_card::explicit_card_count(&request.prompt),
        _ => None,
    };
    let Some(count) = count else {
        return;
    };
    if count != plan.subtasks.len() || count < 2 {
        return;
    }
    // Section-level tasks must continue to share their authored screen.
    if !plan.subtasks.iter().all(|task| {
        (task.region.width - plan.root_frame.width).abs() <= 1.0
            && (task.region.height - plan.root_frame.height).abs() <= 1.0
    }) {
        return;
    }
    if crate::screen_groups::group_subtasks_by_screen(&plan.subtasks).len() == count {
        return;
    }
    for (index, task) in plan.subtasks.iter_mut().enumerate() {
        task.screen = Some(format!("{:02} · {}", index + 1, task.label));
    }
}
