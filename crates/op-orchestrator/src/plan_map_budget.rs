//! Account for scaffold chrome before sizing a stacked tracking map.
//! Only the map's flexible area is reduced; text and controls keep their space.

use crate::{plan::OrchestratorPlan, types::DesignRequest};

const BUDGET_MARKER: &str = "OP MAP BUDGET:";

pub(super) fn reserve_supporting_content(plan: &mut OrchestratorPlan, req: &DesignRequest) {
    let brief = req.prompt.to_lowercase();
    // Tab/page navigation is app chrome, not turn-by-turn map navigation.
    // Likewise, 地图占位区 requests a placeholder, not an authored map ratio.
    let layout_brief = [
        "底部导航",
        "顶部导航",
        "导航栏",
        "导航菜单",
        "bottom navigation",
        "top navigation",
        "navigation bar",
        "navigation menu",
        "tab navigation",
        "地图占位",
    ]
    .iter()
    .fold(brief.clone(), |text, chrome| text.replace(chrome, ""));
    // Navigation, authored sizing and reference-driven compositions can put
    // content over a large map instead of stacking it in the first viewport.
    // Do not turn those intentional compositions into compact tracking cards.
    if req.append_context.is_some()
        || req.continuation_context.is_some()
        || [
            "导航",
            "navigation",
            "全屏",
            "fullscreen",
            "full-screen",
            "full screen",
            "滚动",
            "scroll",
            "参考",
            "复刻",
            "reference",
            "replicate",
            "地图高度",
            "map height",
            "地图占",
            "map occupies",
        ]
        .iter()
        .any(|word| layout_brief.contains(word))
    {
        return;
    }
    let mut groups = crate::screen_groups::group_subtasks_by_screen(&plan.subtasks);
    if groups.is_empty() {
        groups.push(crate::screen_groups::ScreenGroup {
            screen: "page".into(),
            indices: (0..plan.subtasks.len()).collect(),
        });
    }
    for group in groups {
        let maps: Vec<_> = group
            .indices
            .iter()
            .copied()
            .filter(|&index| {
                let task = &plan.subtasks[index];
                let label = task.label.to_lowercase();
                let elements = task.elements.as_deref().unwrap_or("");
                (label.contains("地图") || crate::design_type::contains_word(&label, "map"))
                    && elements.contains("ARCHETYPE: route-map")
            })
            .collect();
        let [map_index] = maps.as_slice() else {
            continue;
        };
        if group.indices.len() < 2 {
            continue;
        }
        let total: f64 = group
            .indices
            .iter()
            .map(|&i| plan.subtasks[i].region.height)
            .sum();
        let gap = plan.root_frame.gap.unwrap_or(0.0).max(0.0);
        let available = plan.root_frame.height
            - crate::scaffold::STATUS_BAR_HEIGHT
            - gap * group.indices.len() as f64;
        let overflow = total - available;
        let remaining = (plan.subtasks[*map_index].region.height - overflow).floor();
        // If content is too long even for a usable map, a new plan/scrolling
        // composition is needed. Never collapse a map into a decorative strip.
        if overflow <= 0.0 || remaining < 180.0 || !remaining.is_finite() {
            continue;
        }
        let task = &mut plan.subtasks[*map_index];
        task.region.height = remaining;
        let elements = task.elements.get_or_insert_default();
        if let Some(start) = elements.find(BUDGET_MARKER) {
            elements.truncate(start);
        }
        elements.push_str(&format!("\n{BUDGET_MARKER} This section has at most {remaining:.0}px INCLUDING its padding and labels after reserving the status bar, gaps and other requested sections. This measured budget overrides earlier approximate map heights/percentages. Keep the map slot within it. Do not shrink supporting text or contact controls, add filler, or move their content onto the map."));
        // A map-only hint cannot protect the CTA when another subagent grows
        // beyond its reservation. Enforce every section with native text layout.
        crate::first_viewport::mark_tasks(plan, &group.indices);
    }
}

#[cfg(test)]
#[path = "plan_map_budget_tests.rs"]
mod tests;
