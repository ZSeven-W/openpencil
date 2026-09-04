//! Normalize planned sibling screens against the live canvas contract.

use std::collections::HashSet;

use crate::plan::{OrchestratorPlan, PlanFill, Region, Subtask};
use crate::types::DesignRequest;

/// Make the live canvas contract authoritative for sibling-screen
/// continuations, even when planning returned syntactically valid but generic
/// desktop output.
pub(super) fn apply(plan: &mut OrchestratorPlan, req: &DesignRequest) -> bool {
    let Some(context) = req.continuation_context.as_ref() else {
        return false;
    };
    if !context.screen_width.is_finite()
        || !context.screen_height.is_finite()
        || context.screen_width <= 0.0
        || context.screen_height <= 0.0
    {
        return false;
    }

    let mut screen_names = Vec::<String>::new();
    for raw in &context.screen_names {
        let name = raw.trim();
        if !name.is_empty()
            && !screen_names
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(name))
        {
            screen_names.push(name.to_string());
        }
    }
    if screen_names.is_empty() {
        return false;
    }

    plan.root_frame.width = context.screen_width;
    plan.root_frame.height = context.screen_height;
    if let Some(color) = context
        .background_color
        .as_deref()
        .map(str::trim)
        .filter(|color| !color.is_empty())
    {
        plan.root_frame.fill = Some(vec![PlanFill {
            kind: "solid".into(),
            color: color.to_string(),
        }]);
    }

    // Preserve detailed planning only where it can be assigned to one of the
    // exact promised screens. Generic/unknown sections are ambiguous in a
    // multi-root continuation and used to collapse into a single Section 1
    // board, so drop them and synthesize one complete-screen task for every
    // missing promise.
    let mut buckets = vec![Vec::<Subtask>::new(); screen_names.len()];
    for mut subtask in std::mem::take(&mut plan.subtasks) {
        let candidate = subtask.screen.as_deref().or_else(|| {
            screen_names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(subtask.label.trim()))
                .then_some(subtask.label.as_str())
        });
        let Some(index) = candidate.and_then(|candidate| {
            screen_names
                .iter()
                .position(|name| name.eq_ignore_ascii_case(candidate.trim()))
        }) else {
            continue;
        };
        subtask.screen = Some(screen_names[index].clone());
        subtask.region = Region {
            width: context.screen_width,
            height: context.screen_height,
        };
        // A planner-provided parent can point at its generic desktop root.
        // Screen-group scaffolding will bind this task to the real sibling
        // root after normalization, so never carry that stale parent through.
        subtask.parent_frame_id = None;
        buckets[index].push(subtask);
    }

    let mut reconciled = Vec::new();
    let mut used_ids = buckets
        .iter()
        .flatten()
        .map(|task| task.id.clone())
        .collect::<HashSet<_>>();
    for (index, (screen_name, mut tasks)) in screen_names
        .into_iter()
        .zip(buckets.into_iter())
        .enumerate()
    {
        if tasks.is_empty() {
            let base_id = format!("continuation-screen-{}", index + 1);
            let mut id = base_id.clone();
            let mut suffix = 2usize;
            while used_ids.contains(&id) {
                id = format!("{base_id}-{suffix}");
                suffix += 1;
            }
            tasks.push(Subtask {
                id: id.clone(),
                label: screen_name.clone(),
                region: Region {
                    width: context.screen_width,
                    height: context.screen_height,
                },
                id_prefix: id,
                parent_frame_id: None,
                insert_after_sibling_id: None,
                elements: Some(format!(
                    "the complete {screen_name} screen, continuing the existing product; reuse its established design system and shared navigation"
                )),
                screen: Some(screen_name),
                generated_root_id: None,
                existing_section_labels: None,
                retry_feedback: None,
            });
        }
        used_ids.extend(tasks.iter().map(|task| task.id.clone()));
        reconciled.extend(tasks);
    }
    plan.subtasks = reconciled;
    true
}
