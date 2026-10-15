//! Pure layout whitespace is deterministic, not an LLM content-generation task.

use crate::plan::{OrchestratorPlan, Subtask};
use jian_ops_schema::node::PenNode;

fn spacing_phrase(text: &str) -> bool {
    matches!(
        text.trim().to_lowercase().as_str(),
        "spacer"
            | "bottom spacer"
            | "bottom safe area"
            | "bottom safe area spacer"
            | "whitespace only"
            | "empty safe area"
            | "no content"
            | "no buttons or tab bar"
            | "底部留白"
            | "底部安全区"
            | "底部安全区留白"
            | "安全区留白"
            | "纯留白安全区"
            | "纯留白"
            | "无内容"
            | "无任何按钮或 tab 栏"
            | "无任何按钮或tab栏"
            | "无按钮或 tab 栏"
            | "无按钮或tab栏"
    )
}

pub(super) fn node(task: &Subtask, plan: &OrchestratorPlan) -> Option<PenNode> {
    // A deliberately empty whole page, a painted footer, or a named indicator
    // must still use normal generation. Only explicit, small blank sections
    // with no other promised content belong to this path.
    if plan.subtasks.len() < 2
        || !spacing_phrase(&task.label)
        || !(1.0..=96.0).contains(&task.region.height)
        || !task.elements.as_deref().is_some_and(|text| {
            let parts: Vec<_> = text
                .split([',', '，', ';', '；', '.', '。', '\n'])
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect();
            !parts.is_empty() && parts.iter().all(|part| spacing_phrase(part))
        })
        || task
            .covers
            .as_ref()
            .is_some_and(|covers| covers.iter().any(|s| !spacing_phrase(s)))
    {
        return None;
    }
    serde_json::from_value(serde_json::json!({
        "type":"frame", "id":format!("{}-spacing", task.id),
        "name":task.label, "width":"fill_container", "height":task.region.height,
        "layout":"vertical", "children":[]
    }))
    .ok()
}
