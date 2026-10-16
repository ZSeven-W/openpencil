//! Reserve explicit above-the-fold image content and check section budgets.
//! Stacked map plans also use the same native-layout height contract.

use crate::{
    plan::{OrchestratorPlan, Subtask},
    types::DesignRequest,
};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, PenNodeExt};

const MARKER: &str = "OP FIRST VIEWPORT:";

pub(crate) fn reserve(plan: &mut OrchestratorPlan, req: &DesignRequest) {
    let brief = req.prompt.to_lowercase();
    let explicit = [
        "首屏包含",
        "首屏必须包含",
        "首屏显示全部",
        "全部在首屏",
        "all above the fold",
        "first screen must include",
    ]
    .iter()
    .any(|s| brief.contains(s));
    if !explicit
        || req.append_context.is_some()
        || req.continuation_context.is_some()
        || plan.root_frame.width > 480.0
        || plan.root_frame.height > 1000.0
        || crate::screen_groups::group_subtasks_by_screen(&plan.subtasks).len() > 1
        || plan
            .root_frame
            .layout
            .as_deref()
            .is_some_and(|s| s != "vertical")
        || plan
            .subtasks
            .iter()
            .any(|t| !t.region.height.is_finite() || t.region.height <= 0.0)
    {
        return;
    }
    let heroes: Vec<_> = plan
        .subtasks
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            // A planner may repeat the screen's archetype on its compact
            // brand header. Only a section with room above the minimum hero
            // height is an image-budget candidate; required small copy stays.
            (t.region.height > 160.0 && is_image_section(t)).then_some(i)
        })
        .collect();
    let [hero] = heroes.as_slice() else { return };
    let without_gaps = plan.root_frame.height
        - crate::scaffold::STATUS_BAR_HEIGHT
        - plan.root_frame.padding.unwrap_or(0.0).max(0.0) * 2.0
        - 24.0;
    let total: f64 = plan.subtasks.iter().map(|t| t.region.height).sum();
    let original_gap = plan.root_frame.gap.unwrap_or(0.0).max(0.0);
    let height_with_gap = |gap: f64| {
        let available = without_gaps - gap * plan.subtasks.len() as f64;
        (plan.subtasks[*hero].region.height - (total - available).max(0.0)).floor()
    };
    // Decorative section spacing yields before the image or required copy.
    // Keep the existing rhythm when it fits; compact to an 8px baseline when
    // the media would fall below the preferred 160px section height.
    let gap = if height_with_gap(original_gap) < 160.0 {
        original_gap.min(8.0)
    } else {
        original_gap
    };
    let hero_height = height_with_gap(gap);
    // A compact 128–159px media card is still usable. Do not silently skip the
    // whole contract because a dense first screen misses the preferred size.
    // Truly infeasible plans need a new composition, not squeezed required copy.
    if hero_height < 128.0 || !hero_height.is_finite() {
        return;
    }
    if gap != original_gap {
        plan.root_frame.gap = Some(gap);
    }
    plan.subtasks[*hero].region.height = hero_height;
    mark_tasks(plan, &(0..plan.subtasks.len()).collect::<Vec<_>>());
}

fn is_image_section(task: &Subtask) -> bool {
    let Some(elements) = task.elements.as_deref() else {
        return false;
    };
    let elements = elements.to_lowercase();
    if elements.contains("archetype: image-led") {
        return true;
    }
    // A short brand header may carry the screen-level archetype while the
    // actual hero is its next sibling. Require both a media-specific section
    // identity and image content; thumbnails in ordinary lists are not heroes.
    let identity = format!("{} {}", task.id, task.label).to_lowercase();
    ["hero", "主图", "头图", "首图", "cover image"]
        .iter()
        .any(|token| identity.contains(token))
        && ["photo", "image", "照片", "主图", "头图"]
            .iter()
            .any(|token| elements.contains(token))
}

pub(crate) fn mark_tasks(plan: &mut OrchestratorPlan, indices: &[usize]) {
    for &index in indices {
        let task = &mut plan.subtasks[index];
        let elements = task.elements.get_or_insert_default();
        if let Some(start) = elements.find(MARKER) {
            elements.truncate(start);
        }
        elements.truncate(elements.trim_end().len());
        elements.push_str(&format!("\n{MARKER} content budget at most {:.0}px including padding, title, media and gaps. Required sections must all be visible before scrolling; this overrides earlier approximate hero percentages and type sizes.",task.region.height));
    }
}

pub(crate) fn owns(task: &Subtask) -> bool {
    task.elements.as_deref().is_some_and(|e| e.contains(MARKER))
}

pub(crate) fn instruction(task: &Subtask) -> String {
    if !owns(task) {
        return String::new();
    }
    format!("\n\nFIRST-VIEWPORT CONTRACT: This complete section must fit within {:.0}px including all padding and text wrapping. It is a MAXIMUM, not a quota to fill. Other required sections have reserved space. Compute padding + title + rows/media + all gaps before emitting the script. Prefer compact media and less decorative whitespace; keep every supplied line and readable body text. Supplied content and usable controls take priority over optional large photos, duplicate headings and repeated notes; supporting sections must not become a second photo hero. Use a content-height section, not clipping or a forced height. Size a long/CJK brand to its actual text width: normally 28–36px, 1–2 balanced lines, never a lone final glyph. A 48px display preset is for short display copy or figures, not mandatory for a long brand. Do not add new claims or omit supporting notes or actions to make room.",task.region.height)
}

pub(crate) fn rejection(
    task: &Subtask,
    nodes: &[PenNode],
    context: &EditorState,
) -> Option<String> {
    if !owns(task) {
        return None;
    }
    // Resolve with the same native text/layout path used by final cleanup.
    // A scratch parent supplies fill_container width without touching the doc.
    let mut scratch = EditorState::new();
    scratch.doc.variables = context.doc.variables.clone();
    scratch.doc.children = vec![serde_json::from_value(serde_json::json!({
        "type":"frame", "id":"__first_viewport_measure", "width":task.region.width,
        "height":"fit_content", "layout":"vertical", "children":nodes
    }))
    .ok()?];
    let rects = crate::image_fallback_policy::resolved_rects(&scratch);
    let height = nodes
        .iter()
        .filter_map(|n| rects.get(n.id_str()))
        .map(|r| r.height)
        .sum::<f64>();
    if height <= task.region.height + 2.0 {
        return None;
    }
    // Show the measured contributors, not just an opaque rejection. A row's
    // native height includes padding/wrapped copy, which a guessed budget
    // often omits. These ids describe the rejected draft, not reusable nodes.
    let parts = nodes
        .iter()
        .flat_map(|node| node.children().into_iter().flatten())
        .take(12)
        .filter_map(|child| {
            rects
                .get(child.id_str())
                .map(|rect| format!("{}={:.0}px", child.id_str(), rect.height))
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "self-check failed: first-viewport-overflow: section {:?} measures {:.0}px but has at most {:.0}px reserved; recover at least {:.0}px. Measured immediate child heights (before parent padding/gaps): {}. These are diagnostics of the rejected draft; recreate the complete section. Recalculate padding + title + rows/media + all gaps. Reduce media height, decorative gaps and oversized title; preserve every supplied line and readable body text. Do not clip content or force the root height.", task.label, height, task.region.height, (height-task.region.height).ceil(), parts
    ))
}

#[cfg(test)]
#[path = "first_viewport_tests.rs"]
mod tests;
