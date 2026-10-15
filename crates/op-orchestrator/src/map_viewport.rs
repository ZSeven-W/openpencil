//! A full-screen navigation map and its overlays share one coordinate space.
//! Splitting them into independent flow sections puts controls below the screen.

use crate::{
    plan::{OrchestratorPlan, Subtask},
    types::DesignRequest,
};

const MARKER: &str = "OP NAVIGATION VIEWPORT:";
const NODE_MARKER: &str = "op-navigation-viewport";

pub(crate) fn is_viewport(node: &serde_json::Value) -> bool {
    node["layout"] == "none"
        && node["explain"]
            .as_str()
            .is_some_and(|s| s.split(';').any(|part| part.trim() == NODE_MARKER))
}

pub(crate) fn is_map_background(node: &serde_json::Value) -> bool {
    node["role"] == "map"
        && node["explain"]
            .as_str()
            .is_some_and(|s| s.starts_with("map-template:"))
}

pub(crate) fn owns(task: &Subtask) -> bool {
    task.elements.as_deref().is_some_and(|s| s.contains(MARKER))
}

fn is_map_control(label: &str) -> bool {
    ["地图", "map", "图层", "定位", "layers", "locate"]
        .iter()
        .any(|word| label.contains(word))
        && ["按钮", "控件", "操作", "control", "toolbar"]
            .iter()
            .any(|word| label.contains(word))
}

fn is_overlay(task: &Subtask) -> bool {
    let label = task.label.to_lowercase();
    if matches!(
        label.as_str(),
        "底部导航栏"
            | "底部tab栏"
            | "底部 tab 栏"
            | "bottom navigation"
            | "bottom tab bar"
            | "tab bar"
    ) {
        return false;
    }
    if ["订单", "骑手", "列表", "profile", "feed", "list"]
        .iter()
        .any(|word| label.contains(word))
    {
        return false;
    }
    let floating = [
        "悬浮",
        "浮动",
        "overlay",
        "direction",
        "方向提示",
        "行程摘要",
        "trip summary",
        "图层",
    ]
    .iter()
    .any(|word| label.contains(word));
    // Coverage repair can split a navigation action from its summary. Its
    // unknown region height must not turn the whole map into stacked sections.
    let navigation_action = [
        "结束导航",
        "停止导航",
        "退出导航",
        "end navigation",
        "stop navigation",
        "exit navigation",
    ]
    .iter()
    .any(|word| label.contains(word));
    // Navigation information is a trip summary. A bottom navigation/tab bar
    // with destinations is different chrome and must not be folded here.
    let trip_information = ["信息", "数据", "摘要"]
        .iter()
        .any(|word| label.contains(word))
        && (label.contains("导航") || label.contains("行程"));
    // Coverage repair may name a missing trip datum verbatim from the brief
    // instead of assigning it a section label. It still belongs to the map's
    // summary, never to a second full-width page band.
    let content = format!(
        "{label} {}",
        task.elements.as_deref().unwrap_or("").to_lowercase()
    );
    let trip_datum = ["剩余", "remaining", "预计", "estimated"]
        .iter()
        .any(|s| content.contains(s))
        && ["公里", "分钟", "distance", "time", "km", "minutes"]
            .iter()
            .any(|s| content.contains(s));
    floating || is_map_control(&label) || navigation_action || trip_information || trip_datum
}

pub(crate) fn fold(plan: &mut OrchestratorPlan, req: &DesignRequest) {
    let brief = req.prompt.to_lowercase();
    if req.append_context.is_some()
        || req.continuation_context.is_some()
        || req.reference_skeleton.is_some()
        || ![
            "全屏地图",
            "full-screen map",
            "fullscreen map",
            "full screen map",
        ]
        .iter()
        .any(|s| brief.contains(s))
        || [
            "不要全屏",
            "不使用全屏",
            "no full-screen",
            "not full-screen",
            "no fullscreen",
            "not fullscreen",
        ]
        .iter()
        .any(|s| brief.contains(s))
        || !["骑行导航", "行车导航", "步行导航", "导航构图", "navigation"]
            .iter()
            .any(|s| brief.contains(s))
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
    let mut remove = Vec::new();
    for group in groups {
        if group.indices.iter().any(|&i| owns(&plan.subtasks[i])) {
            continue;
        }
        let maps: Vec<_> = group
            .indices
            .iter()
            .copied()
            .filter(|&i| {
                let task = &plan.subtasks[i];
                let label = task.label.to_lowercase();
                (label.contains("地图") || label.contains("map"))
                    // A control may be allocated the whole viewport region;
                    // that does not make it a second map background.
                    && !is_map_control(&label)
                    && task.region.height >= 200.0
                    && task.region.width >= plan.root_frame.width * 0.65
            })
            .collect();
        // The composition hint is attached to the first planned section,
        // which can be the direction banner after coverage replanning. The
        // explicit full-screen navigation brief plus one large named map is
        // the ownership signal, not the hint's incidental section placement.
        let [map] = maps.as_slice() else { continue };
        let layers: Vec<_> = group
            .indices
            .iter()
            .copied()
            .filter(|&i| i == *map || is_overlay(&plan.subtasks[i]))
            .collect();
        let outside: Vec<_> = group
            .indices
            .iter()
            .copied()
            .filter(|i| !layers.contains(i))
            .collect();
        // Footnotes, tab bars and unrelated content stay in normal flow. They
        // reserve real space instead of preventing ALL map overlays from
        // composing, or being silently swallowed by the viewport.
        if layers.len() > 6
            || outside
                .iter()
                .any(|&i| plan.subtasks[i].region.height <= 0.0)
        {
            continue;
        }
        let outside_height: f64 = outside
            .iter()
            .map(|&i| plan.subtasks[i].region.height)
            .sum();
        let height = plan.root_frame.height
            - crate::scaffold::STATUS_BAR_HEIGHT
            - plan.root_frame.gap.unwrap_or(0.0).max(0.0) * (outside.len() + 1) as f64
            - outside_height;
        if height < (plan.root_frame.height * 2.0 / 3.0).max(400.0) || !height.is_finite() {
            continue;
        }
        let descriptions = layers
            .iter()
            .map(|&i| {
                let task = &plan.subtasks[i];
                format!("{}: {}", task.label, task.elements.as_deref().unwrap_or(""))
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut covers = Vec::new();
        for &i in &layers {
            for entry in plan.subtasks[i].covers.iter().flatten() {
                if !covers.contains(entry) {
                    covers.push(entry.clone());
                }
            }
        }
        let template = if ["骑行", "步行", "cycling", "walking", "running"]
            .iter()
            .any(|s| brief.contains(s))
        {
            "running"
        } else {
            "delivery"
        };
        let owner = &mut plan.subtasks[*map];
        owner.label = "地图与悬浮导航".into();
        owner.region.width = plan.root_frame.width;
        owner.region.height = height;
        owner.covers = (!covers.is_empty()).then_some(covers);
        owner.elements = Some(format!("ARCHETYPE: route-map\n{descriptions}\n{MARKER} One {height:.0}px navigation viewport owns ALL the content above. Keep the map and all controls in the SAME absolute viewport, not separate vertical sections. Use map-template:{template} so the requested route is present. Supersede the earlier section heights; preserve the original brief's supplied directions and remaining distance/time. Repeated mentions of the same distance/time are coverage reminders: render each datum once in ONE summary, not duplicate cards."));
        remove.extend(layers.into_iter().filter(|i| i != map));
    }
    let mut index = 0;
    plan.subtasks.retain(|_| {
        let keep = !remove.contains(&index);
        index += 1;
        keep
    });
}

pub(crate) fn instruction(task: &Subtask) -> String {
    if !owns(task) {
        return String::new();
    }
    format!("\nNAVIGATION VIEWPORT CONTRACT: The section is a fixed {:.0}px-tall layout:none viewport with zero padding. Its ONE map image fills that viewport. Put the requested direction card at the top with 16px insets, the compact 40-48px controls at the right below the direction card, and the trip summary/action card near the bottom with 16px insets and at least 24px bottom room. ALL overlays remain inside the viewport. They use x/y ONLY under layout:none; their internal text/control rows use auto layout and real gaps. Create overlay siblings FIRST and the map image LAST (first child paints on top in OP). Here authored layer/location controls are allowed as compact siblings; do not draw extra terrain, paths or markers. Do not append a second card/toolbar below the viewport or add content just to fill its height. The map template supplies the route. No location-only template for navigation.", task.region.height)
}

/// Ordinary section roots hug their content. An absolute map viewport must
/// retain its measured height or its floating controls become its flow size.
#[derive(Debug)]
pub(crate) enum ViewportError {
    MultipleRoots,
    FlowLayout,
}

impl std::fmt::Display for ViewportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self {
            Self::MultipleRoots => "generate one frame containing the map and ALL overlays",
            Self::FlowLayout => "the navigation map and its overlays must share ONE layout:none viewport, not a vertical stack",
        };
        write!(f, "self-check failed: navigation-viewport: {reason}")
    }
}

impl std::error::Error for ViewportError {}

pub(crate) fn restore_height(
    nodes: &mut [jian_ops_schema::node::PenNode],
    task: &Subtask,
) -> Result<(), ViewportError> {
    if !owns(task) {
        return Ok(());
    }
    let [jian_ops_schema::node::PenNode::Frame(frame)] = nodes else {
        return Err(ViewportError::MultipleRoots);
    };
    if frame.container.layout != Some(jian_ops_schema::node::container::LayoutMode::None) {
        return Err(ViewportError::FlowLayout);
    }
    frame.container.height = Some(jian_ops_schema::sizing::SizingBehavior::Number(
        task.region.height,
    ));
    frame.container.padding = Some(jian_ops_schema::node::container::Padding::Uniform(0.0));
    let hint = frame.base.explain.get_or_insert_default();
    if !hint.split(';').any(|part| part.trim() == NODE_MARKER) {
        if !hint.is_empty() {
            hint.push_str("; ");
        }
        hint.push_str(NODE_MARKER);
    }
    Ok(())
}

#[cfg(test)]
#[path = "map_viewport_tests.rs"]
mod tests;
