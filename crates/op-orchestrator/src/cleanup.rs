//! 阶段 4 —— 清理 pass。
//!
//! [`run_cleanup_passes`] 在所有 subtask 插入完成后运行,是独立
//! 函数 —— 顺序路径在收尾时复用它(spec §9)。
//!
//! [`descendant_count`] 给 `run()` 的"零内容"判定提供基线:
//! scaffold 之后数一次,subtask 全跑完再数一次,没涨即零内容。

use crate::cleanup_layout::root_content_height;
use crate::cleanup_typography::repair_overbold_text_hierarchy;
use crate::plan::OrchestratorPlan;
use crate::types::DocSink;
use jian_ops_schema::node::{container::Padding, PenNode};
use jian_ops_schema::style::PenEffect;
use op_editor_core::{
    first_fill_type, first_solid_fill_hex, EditorCommand, EditorState, FillType, LayoutPropValue,
    NodeId, PenNodeExt,
};

#[path = "cleanup_desktop_dashboard.rs"]
mod cleanup_desktop_dashboard;
#[path = "cleanup_mobile_chrome.rs"]
mod cleanup_mobile_chrome;
#[path = "cleanup_mobile_dense.rs"]
mod cleanup_mobile_dense;

/// 递归统计 `node` 下的后代数(不含自身)。
///
/// Exposed `pub(crate)` so scaffold builders can pre-compute the same
/// baseline that [`descendant_count`] returns after applying the scaffold
/// commands — needed by the dashboard scaffold (Task C2) whose row/slot
/// frames inflate the live descendant count beyond a fixed baseline.
pub(crate) fn count_descendants(node: &PenNode) -> usize {
    match node.children() {
        Some(children) => children.len() + children.iter().map(count_descendants).sum::<usize>(),
        None => 0,
    }
}

/// 统计活动页里 id 为 `root_id` 的节点的后代总数。节点不存在
/// 时返回 0。
pub fn descendant_count(state: &EditorState, root_id: &str) -> usize {
    state
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
        .map(count_descendants)
        .unwrap_or(0)
}

/// 名称 / id 命中即视为"状态栏"节点。
fn is_status_bar(node: &PenNode) -> bool {
    let name = node.base().name.as_deref().unwrap_or("").to_lowercase();
    let id = node.id_str().to_lowercase();
    let hay = format!("{id} {name}");
    hay.contains("status bar") || hay.contains("status-bar") || hay.contains("statusbar")
}

/// Pass ①:移动端重复状态栏去重。scaffold 注入了一个固定状态栏,
/// 若某个 sub-agent 又生成了状态栏,根下就会有多个 —— 保留第一个,
/// 删掉其余。对齐 TS `removeDuplicateStatusBars`。
fn remove_duplicate_status_bars(sink: &mut dyn DocSink, root_id: &str) {
    let dupes: Vec<NodeId> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        let Some(children) = root.children() else {
            return;
        };
        children
            .iter()
            .filter(|c| is_status_bar(c))
            .skip(1) // 保留第一个
            .map(|c| NodeId::new(c.id_str().to_string()))
            .collect()
    };
    for id in dupes {
        sink.apply(EditorCommand::DeleteNode {
            node_id: id,
            page_id: None,
        });
    }
}

pub(crate) fn remove_duplicate_bottom_nav_sections_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        remove_duplicate_bottom_nav_sections(sink, &root_id);
    }
}

/// Mobile root-level bottom-nav dedupe. Weak-model Chinese prompts can produce
/// both a localized bottom nav section and an English normalized bottom nav.
/// Keep the bottom-most/last top-level nav and remove earlier duplicates.
fn remove_duplicate_bottom_nav_sections(sink: &mut dyn DocSink, root_id: &str) {
    let dupes: Vec<NodeId> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        if !is_mobile_root(root) {
            return;
        }
        let Some(children) = root.children() else {
            return;
        };
        let nav_indices: Vec<usize> = children
            .iter()
            .enumerate()
            .filter_map(|(index, child)| is_bottom_nav_section(child).then_some(index))
            .collect();
        if nav_indices.len() < 2 {
            return;
        }
        let keep_index = nav_indices
            .iter()
            .copied()
            .max_by(|a, b| compare_bottom_nav_position(children, *a, *b))
            .expect("nav_indices is non-empty");
        nav_indices
            .into_iter()
            .filter(|index| *index != keep_index)
            .map(|index| NodeId::new(children[index].id_str().to_string()))
            .collect()
    };
    for id in dupes {
        sink.apply(EditorCommand::DeleteNode {
            node_id: id,
            page_id: None,
        });
    }
}

fn is_bottom_nav_section(node: &PenNode) -> bool {
    cleanup_mobile_chrome::bottom_nav_surface_target(node, false).is_some()
}

fn compare_bottom_nav_position(
    children: &[PenNode],
    left_index: usize,
    right_index: usize,
) -> std::cmp::Ordering {
    let left_y = children[left_index].base().y.unwrap_or(left_index as f64);
    let right_y = children[right_index].base().y.unwrap_or(right_index as f64);
    left_y
        .total_cmp(&right_y)
        .then_with(|| left_index.cmp(&right_index))
}

/// Pass ②:移动端浅色 root 下的 nav surface 纠偏。弱模型常把
/// bottom nav / tab bar 套用成黑色安全模板,和当前浅色页面调性断裂。
/// TS 端只补"缺失 fill"的 nav;Rust cleanup 还需要兜住已写
/// safe-dark fill 的误生成。
fn repair_light_mobile_nav_surfaces(sink: &mut dyn DocSink, root_id: &str) {
    let repairs: Vec<NavSurfaceRepair> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        if !is_light_mobile_root(root) {
            return;
        }
        let root_surface_hex = first_solid_fill_hex(root);
        let Some(children) = root.children() else {
            return;
        };
        children
            .iter()
            .filter_map(nav_surface_target)
            .filter_map(|nav| nav_surface_repair(nav, root_surface_hex))
            .collect()
    };

    for repair in repairs {
        sink.apply(EditorCommand::SetNodeFillHex {
            node_id: repair.node_id.clone(),
            hex: repair.fill_hex,
        });
    }
}

#[derive(Debug, Clone)]
struct NavSurfaceRepair {
    node_id: NodeId,
    fill_hex: String,
}

const MOBILE_CONTENT_SIDE_PADDING: f64 = 24.0;

/// Pass ③:移动端内容 section 的安全内边距/宽度纠偏。
///
/// 弱模型经常生成 `root > section > card(width:390)` 或无 padding 的
/// section,导致标题贴边、卡片越出 390px 屏幕。这里只处理 root 的
/// 直接非 chrome 子 frame,保留 status/nav 全宽。
fn repair_mobile_content_sections(sink: &mut dyn DocSink, root_id: &str) {
    let repairs: MobileSectionRepairs = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        if !is_mobile_root(root) {
            return;
        }
        let Some(children) = root.children() else {
            return;
        };
        let max_content_width =
            (root.width_px().unwrap_or(0.0) - MOBILE_CONTENT_SIDE_PADDING * 2.0).max(1.0);
        let mut repairs = MobileSectionRepairs::default();

        for child in children {
            if !is_mobile_content_section(child) {
                continue;
            }
            if !has_horizontal_padding_at_least(child, 16.0) {
                repairs
                    .pad_sections
                    .push(NodeId::new(child.id_str().to_string()));
            }
            collect_overwide_mobile_descendants(child, max_content_width, &mut repairs);
        }

        repairs
    };

    for node_id in repairs.pad_sections {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id,
            property: "padding".to_string(),
            value: LayoutPropValue::NumberArray(vec![
                0.0,
                MOBILE_CONTENT_SIDE_PADDING,
                0.0,
                MOBILE_CONTENT_SIDE_PADDING,
            ]),
        });
    }
    for node_id in repairs.width_fill {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id,
            property: "width".to_string(),
            value: LayoutPropValue::Keyword("fill_container".to_string()),
        });
    }
    for (node_id, x) in repairs.clamp_x {
        sink.apply(EditorCommand::UpdateNode {
            node_id,
            x: Some(x),
            y: None,
            width: None,
            height: None,
            name: None,
            fill_hex: None,
            page_id: None,
        });
    }
    for node_id in repairs.placeholder_tiles {
        sink.apply(EditorCommand::SetNodeFillHex {
            node_id,
            hex: "#FF6B00".to_string(),
        });
    }
    for (node_id, side) in repairs.square_tiles {
        sink.apply(EditorCommand::UpdateNode {
            node_id,
            x: None,
            y: None,
            width: Some(side),
            height: Some(side),
            name: None,
            fill_hex: None,
            page_id: None,
        });
    }
}

#[derive(Debug, Default)]
struct MobileSectionRepairs {
    pad_sections: Vec<NodeId>,
    width_fill: Vec<NodeId>,
    clamp_x: Vec<(NodeId, i32)>,
    placeholder_tiles: Vec<NodeId>,
    square_tiles: Vec<(NodeId, i32)>,
}

/// Pass ③:根 frame 高度自适应到内容。把根 frame 的高度设为其
/// 直接子节点的内容高度之和；`fit_content` 容器会按子节点估算。
/// 对齐 TS `adjustRootFrameHeightToContent`。
fn adjust_root_height_to_content(sink: &mut dyn DocSink, root_id: &str) {
    let (total, current_height, mobile, has_fill_height_child) = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        (
            root_content_height(root),
            root.height_px(),
            is_mobile_root(root),
            root_has_fill_height_child(root),
        )
    };

    // A tall, scrolling mobile screen should hug its content. When the content
    // genuinely exceeds a standard phone viewport, a fixed frame height that
    // sits ABOVE the content leaves dead space at the bottom ("下面太长").
    // Switching the root to `fit_content` lets the layout engine size it
    // exactly (it measures real text/images, so it never clips). Gated on
    // content > a phone viewport so a SPARSE screen keeps its phone-height
    // frame instead of collapsing to a tiny estimate. Skip when a direct child
    // fills height (`fit_content` parent + `fill_container` child is circular).
    const STANDARD_MOBILE_VIEWPORT: f64 = 812.0;
    if mobile
        && !has_fill_height_child
        && total.is_some_and(|content| f64::from(content) > STANDARD_MOBILE_VIEWPORT)
    {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(root_id.to_string()),
            property: "height".to_string(),
            value: LayoutPropValue::Keyword("fit_content".to_string()),
        });
        return;
    }

    // Desktop / fill-height roots: only GROW a too-short fixed height to fit
    // overflowing content. Never shrink here — a desktop dashboard root's
    // height is `max(region heights)` on purpose.
    if let Some(height) = total.filter(|height| {
        current_height
            .map(|current| f64::from(*height) > current)
            .unwrap_or(true)
    }) {
        sink.apply(EditorCommand::UpdateNode {
            node_id: NodeId::new(root_id.to_string()),
            x: None,
            y: None,
            width: None,
            height: Some(height),
            name: None,
            fill_hex: None,
            page_id: None,
        });
    }
}

/// True when any DIRECT child of `root` sizes its height as `fill_container` —
/// making a `fit_content` parent a circular layout dependency.
fn root_has_fill_height_child(root: &PenNode) -> bool {
    root.children()
        .map(|children| children.iter().any(height_is_fill_container))
        .unwrap_or(false)
}

fn height_is_fill_container(node: &PenNode) -> bool {
    use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
    let height = match node {
        PenNode::Frame(n) => n.container.height.as_ref(),
        PenNode::Group(n) => n.container.height.as_ref(),
        PenNode::Rectangle(n) => n.container.height.as_ref(),
        _ => return false,
    };
    matches!(
        height,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    )
}

fn is_light_mobile_root(root: &PenNode) -> bool {
    let width = root.width_px().unwrap_or(f64::INFINITY);
    let height = root.height_px().unwrap_or(0.0);
    if width > 480.0 || height < 500.0 {
        return false;
    }
    first_solid_fill_hex(root)
        .and_then(relative_luminance)
        .map(|luminance| luminance >= 0.5)
        .unwrap_or(false)
}

fn is_mobile_root(root: &PenNode) -> bool {
    let width = root.width_px().unwrap_or(f64::INFINITY);
    let height = root.height_px().unwrap_or(0.0);
    width <= 480.0 && height >= 500.0
}

fn is_mobile_content_section(node: &PenNode) -> bool {
    node.is_container()
        && node
            .children()
            .map(|children| !children.is_empty())
            .unwrap_or(false)
        && !is_status_bar(node)
        && nav_surface_target(node).is_none()
}

fn has_horizontal_padding_at_least(node: &PenNode, min: f64) -> bool {
    let padding = match node {
        PenNode::Frame(n) => n.container.padding.as_ref(),
        PenNode::Group(n) => n.container.padding.as_ref(),
        PenNode::Rectangle(n) => n.container.padding.as_ref(),
        _ => None,
    };
    match padding {
        Some(Padding::Uniform(v)) => *v >= min,
        Some(Padding::XY([_, x])) => *x >= min,
        Some(Padding::LtrB([_, right, _, left])) => *right >= min && *left >= min,
        Some(Padding::Expression(_)) | None => false,
    }
}

fn collect_overwide_mobile_descendants(
    node: &PenNode,
    max_width: f64,
    repairs: &mut MobileSectionRepairs,
) {
    let Some(children) = node.children() else {
        return;
    };
    for child in children {
        if is_status_bar(child) || nav_surface_target(child).is_some() {
            continue;
        }
        if child
            .width_px()
            .map(|width| width > max_width + 1.0)
            .unwrap_or(false)
        {
            repairs
                .width_fill
                .push(NodeId::new(child.id_str().to_string()));
        } else if let (Some(x), Some(width)) = (child.base().x, child.width_px()) {
            if width <= max_width && x + width > max_width + 1.0 {
                let clamped = (max_width - width).max(0.0).round() as i32;
                repairs
                    .clamp_x
                    .push((NodeId::new(child.id_str().to_string()), clamped));
            }
        }
        if is_blank_gray_mobile_placeholder(child) {
            repairs
                .placeholder_tiles
                .push(NodeId::new(child.id_str().to_string()));
        }
        if let Some(side) = mobile_square_tile_side(child) {
            repairs
                .square_tiles
                .push((NodeId::new(child.id_str().to_string()), side));
        }
        collect_overwide_mobile_descendants(child, max_width, repairs);
    }
}

fn is_blank_gray_mobile_placeholder(node: &PenNode) -> bool {
    if node
        .children()
        .map(|children| !children.is_empty())
        .unwrap_or(false)
    {
        return false;
    }
    let Some(width) = node.width_px() else {
        return false;
    };
    let Some(height) = node.height_px() else {
        return false;
    };
    if !(36.0..=140.0).contains(&width) || !(36.0..=140.0).contains(&height) {
        return false;
    }
    let ratio = width / height;
    if !(0.65..=1.6).contains(&ratio) {
        return false;
    }
    first_solid_fill_hex(node)
        .map(is_soft_gray_placeholder_hex)
        .unwrap_or(false)
}

fn mobile_square_tile_side(node: &PenNode) -> Option<i32> {
    let width = node.width_px()?;
    let height = node.height_px()?;
    if !(36.0..=140.0).contains(&width) || !(36.0..=140.0).contains(&height) {
        return None;
    }
    let ratio = width / height;
    if (0.9..=1.1).contains(&ratio) || !(0.65..=1.6).contains(&ratio) {
        return None;
    }
    if !is_blank_gray_mobile_placeholder(node) && !is_likely_square_icon_or_media_tile(node) {
        return None;
    }
    Some(width.min(height).round() as i32)
}

fn is_likely_square_icon_or_media_tile(node: &PenNode) -> bool {
    let hay = node_identity_haystack(node);
    if contains_any(
        &hay,
        &[
            "avatar",
            "filter",
            "icon button",
            "icon-button",
            "image",
            "media",
            "photo",
            "sliders",
            "thumbnail",
            "thumb",
            "tile",
        ],
    ) {
        return true;
    }
    node.children()
        .map(|children| {
            !children.is_empty()
                && children
                    .iter()
                    .any(|child| matches!(child, PenNode::IconFont(_)))
                && !children
                    .iter()
                    .any(|child| matches!(child, PenNode::Text(_)))
        })
        .unwrap_or(false)
}

fn node_identity_haystack(node: &PenNode) -> String {
    [
        node.id_str(),
        node.base().name.as_deref().unwrap_or(""),
        node.base().role.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase()
}

fn is_soft_gray_placeholder_hex(hex: &str) -> bool {
    let Some((r, g, b)) = parse_hex_rgb(hex) else {
        return false;
    };
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    max.saturating_sub(min) <= 8 && (190..=245).contains(&max)
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn nav_surface_target(child: &PenNode) -> Option<&PenNode> {
    if is_nav_surface(child) {
        return Some(child);
    }

    let role = child.base().role.as_deref().unwrap_or("");
    let children = child.children()?;
    if !role.eq_ignore_ascii_case("section") || children.len() != 1 {
        return None;
    }
    children.first().filter(|inner| is_nav_surface(inner))
}

fn is_nav_surface(node: &PenNode) -> bool {
    let role = node.base().role.as_deref().unwrap_or("").to_lowercase();
    // Matches tree_heuristics::NAV_ROLES exactly. The TOP header roles
    // (`navbar` / `top-nav-bar` / `top-app-bar`) are deliberately EXCLUDED: on a
    // light mobile page the header is transparent (TS references), and re-filling
    // it with the root surface hex + a drop-shadow is exactly what re-boxed the
    // mobile header the user flagged. Only bottom navs / floating tab bars — which
    // float over scrolling content — need a surface to read against the page.
    if matches!(
        role.as_str(),
        "nav" | "tab-bar" | "bottom-tab-bar" | "tab-row"
    ) {
        return true;
    }

    let name = node.base().name.as_deref().unwrap_or("").to_lowercase();
    let id = node.id_str().to_lowercase();
    let hay = format!("{id} {name}");
    hay.contains("bottom nav")
        || hay.contains("bottom-nav")
        || hay.contains("bottom navigation")
        || hay.contains("bottom-navigation")
        || hay.contains("tab bar")
        || hay.contains("tab-bar")
        || hay.contains("bottom tab")
        || hay.contains("bottom-tab")
}

fn nav_surface_repair(nav: &PenNode, root_surface_hex: Option<&str>) -> Option<NavSurfaceRepair> {
    let solid = first_solid_fill_hex(nav);
    let has_paintable_fill = solid.map(|hex| !hex.trim().is_empty()).unwrap_or(false)
        || !matches!(first_fill_type(nav), FillType::Solid);
    let safe_dark = solid.map(is_safe_dark_hex).unwrap_or(false);
    let is_bottom_nav = is_bottom_nav_surface(nav);
    let fill_hex = root_surface_hex
        .filter(|hex| !hex.trim().is_empty())
        .unwrap_or("#FFFFFF");
    let default_white_on_tinted_root = is_bottom_nav
        && solid.map(is_default_white_surface_hex).unwrap_or(false)
        && !same_hex(solid.unwrap_or_default(), fill_hex)
        && !is_default_white_surface_hex(fill_hex);

    if has_paintable_fill && !safe_dark && !default_white_on_tinted_root {
        return None;
    }

    Some(NavSurfaceRepair {
        node_id: NodeId::new(nav.id_str().to_string()),
        fill_hex: fill_hex.to_string(),
    })
}

fn is_default_white_surface_hex(hex: &str) -> bool {
    matches!(
        normalize_hex6(hex).as_deref(),
        Some("#FFFFFF" | "#F9FAFB" | "#F8FAFC")
    )
}

fn same_hex(a: &str, b: &str) -> bool {
    normalize_hex6(a) == normalize_hex6(b)
}

fn normalize_hex6(hex: &str) -> Option<String> {
    let trimmed = hex.trim();
    let body = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if body.len() != 6 || !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", body.to_ascii_uppercase()))
}

fn is_bottom_nav_surface(node: &PenNode) -> bool {
    let role = node.base().role.as_deref().unwrap_or("").to_lowercase();
    if role == "bottom-tab-bar" {
        return true;
    }
    let name = node.base().name.as_deref().unwrap_or("").to_lowercase();
    let id = node.id_str().to_lowercase();
    let hay = format!("{id} {name}");
    hay.contains("bottom nav")
        || hay.contains("bottom-nav")
        || hay.contains("bottom navigation")
        || hay.contains("bottom-navigation")
        || hay.contains("bottom tab")
        || hay.contains("bottom-tab")
}

fn is_safe_dark_hex(hex: &str) -> bool {
    let Some((r, g, b)) = parse_hex_rgb(hex) else {
        return false;
    };
    let normalized = format!("#{r:02X}{g:02X}{b:02X}");
    matches!(
        normalized.as_str(),
        "#000000"
            | "#0A0A0A"
            | "#0F0F0F"
            | "#111111"
            | "#121212"
            | "#141414"
            | "#1A1A1A"
            | "#181818"
            | "#1C1C1C"
            | "#1E1E1E"
            | "#202020"
            | "#111827"
            | "#0F172A"
            | "#18181B"
            | "#1F2937"
    ) || relative_luminance_from_rgb(r, g, b) <= 0.035
}

fn relative_luminance(hex: &str) -> Option<f64> {
    let (r, g, b) = parse_hex_rgb(hex)?;
    Some(relative_luminance_from_rgb(r, g, b))
}

fn relative_luminance_from_rgb(r: u8, g: u8, b: u8) -> f64 {
    (0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)) / 255.0
}

fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let raw = hex.trim().trim_start_matches('#');
    match raw.len() {
        3 => {
            let r = u8::from_str_radix(&raw[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&raw[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&raw[2..3].repeat(2), 16).ok()?;
            Some((r, g, b))
        }
        6 | 8 => {
            let r = u8::from_str_radix(&raw[0..2], 16).ok()?;
            let g = u8::from_str_radix(&raw[2..4], 16).ok()?;
            let b = u8::from_str_radix(&raw[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

/// Find a node by id anywhere in the active-page tree (recursive).
///
/// Append-mode generation nests the new root under an existing target
/// frame rather than placing it at the top level, so a top-level-only
/// scan would miss it (Component 11c).
fn find_root<'a>(state: &'a EditorState, root_id: &str) -> Option<&'a PenNode> {
    op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(root_id.to_string()))
}

/// 阶段 4 清理 pass —— 在全部 subtask 插入完成后运行。
///
/// `root_ids` 是本轮产出的根 frame id(S3a 单屏只有一个;S3b 并发
/// 多屏会有多个 —— 该函数对每个 root 跑同样的 pass,故 S3b 直接
/// 复用,spec §9)。
///
/// 已实现:① 移动端重复状态栏去重 ② 移动浅色 nav surface 纠偏
/// ③ 过度粗体文本层级修正 ④ 根高度自适应。
/// 未做:单组件 section root unwrap(`unwrapSingleComponentSection`
/// Root`)—— 启发式强、对 parity 敏感,留作 S3a 后续细化。
/// Whole-root finalize stage — the single, idempotent public entry point the
/// orchestrator (and, in a later step, the agentic design loop) calls to run
/// every whole-root cleanup pass over the produced root frames.
///
/// This is a thin, behavior-preserving wrapper around
/// [`run_cleanup_passes`]: it forwards its inputs unchanged so the effect is
/// byte-for-byte identical to calling `run_cleanup_passes` directly. The
/// orchestrator's cleanup stage now routes through here so a future agentic
/// loop can reuse the exact same finalize surface at the end of its turn.
///
/// SCOPE NOTE (Step 4 concern, NOT folded in here): the per-subtask Stage-1
/// ordered passes (`role_infer` / `role_post_pass` / `tree_heuristics` in
/// `subagent.rs`) are intentionally left where they are. They depend on
/// per-subtask forest context that is not available at this whole-root
/// finalize boundary, so folding them in is deferred to a later step.
pub fn finalize_design(sink: &mut dyn DocSink, plan: &OrchestratorPlan, root_ids: &[&str]) {
    run_cleanup_passes(sink, plan, root_ids);
}

/// Env-gated (`OPENPENCIL_DEBUG_CLEANUP=1`) probe: log the named child's
/// current height under `root_id`, tagged with the pass that just ran.
fn debug_probe_child_height(sink: &dyn DocSink, root_id: &str, tag: &str) {
    if std::env::var("OPENPENCIL_DEBUG_CLEANUP").is_err() {
        return;
    }
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
    else {
        eprintln!("[CLEANUP-PROBE] {tag}: root {root_id} NOT FOUND");
        return;
    };
    let Ok(v) = serde_json::to_value(root) else {
        return;
    };
    for c in v
        .get("children")
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
    {
        let name = c.get("name").and_then(|n| n.as_str()).unwrap_or("?");
        if name.to_lowercase().contains("sidebar") {
            eprintln!("[CLEANUP-PROBE] {tag}: {name} height={:?}", c.get("height"));
        }
    }
}

pub fn run_cleanup_passes(sink: &mut dyn DocSink, plan: &OrchestratorPlan, root_ids: &[&str]) {
    let effective_root_ids: Vec<String> = if root_ids.is_empty() {
        Vec::new()
    } else {
        // Page-global root dedupe must happen before per-root passes so a
        // deleted sparse scaffold id can be replaced by the kept rich root id.
        let removal = crate::abandoned_duplicate_roots::remove_abandoned_duplicate_roots(sink);
        let mut seen = std::collections::BTreeSet::new();
        root_ids
            .iter()
            .map(|root_id| {
                let root_id = *root_id;
                removal
                    .kept_for_removed(root_id)
                    .unwrap_or(root_id)
                    .to_string()
            })
            .filter(|root_id| seen.insert(root_id.clone()))
            .collect()
    };

    // Doc-global (not per-root): heal theme-polarity splits in the variable
    // table BEFORE the per-root passes, so every pass that resolves `$refs`
    // (surface discipline, geometry text fills) sees the repaired palette.
    crate::loop_finalize::fix_theme_variable_polarity(sink);
    for root_id in &effective_root_ids {
        debug_probe_child_height(sink, root_id, "cleanup-entry");
        // FIRST: whole-root structural restructures, shared by BOTH the
        // orchestrator (per-subtask role passes already ran) and the agentic
        // loop (whole-doc role passes ran in `apply_loop_finalize`), so the
        // moved sections keep their resolved roles. These swap the root via
        // `ReplaceSubtree`, which allocates a FRESH root id — `apply_root_transform`
        // returns the new id so the per-root cleanup passes below don't look up a
        // stale id and silently no-op.
        let mut rid = root_id.to_string();
        // Flat-vertical / crammed-horizontal sidebar dashboard → [sidebar | content].
        rid = apply_root_transform(sink, &rid, crate::app_shell::reshape_sidebar_to_app_shell);
        debug_probe_child_height(sink, &rid, "reshape");
        // Already-split `[sidebar | main]` root that a model left WITHOUT a
        // horizontal layout (MiniMax-M3 in the agentic loop) — flip it to a row
        // so the columns sit side by side instead of stacking. `reshape` above
        // skips 2-child roots, so this catches the case it leaves behind.
        rid = apply_root_transform(sink, &rid, crate::app_shell::ensure_split_shell_is_row);
        debug_probe_child_height(sink, &rid, "ensure_row");
        // Relocate a data section stranded in the narrow sidebar column — the
        // `nav`/`menu` substring routing (or a sidebar subtask that over-emitted
        // a second root) can misfile a "Client Directory" table into the 260px
        // clipContent rail — back to the content column, BEFORE the passes below
        // repair / size it in its correct home.
        rid = apply_root_transform(
            sink,
            &rid,
            crate::app_shell::evict_content_from_sidebar_column,
        );
        debug_probe_child_height(sink, &rid, "evict");
        crate::sidebar_archetype::repair_sidebar_navbar_archetype(sink, &rid);
        debug_probe_child_height(sink, &rid, "sidebar_archetype");
        // Flat table cells → Table → Row → Cell.
        rid = apply_root_transform(sink, &rid, crate::table_repair::regroup_flat_table_rows);
        debug_probe_child_height(sink, &rid, "regroup_table");
        // Gap-less table rows → column gap (weak models omit it → columns touch,
        // "SPEND"+"STATUS" reads as "SPENDSTATUS").
        rid = apply_root_transform(sink, &rid, crate::table_repair::ensure_table_column_gap);
        debug_probe_child_height(sink, &rid, "table_gap");
        // Row gap on the table CONTAINER (rows already zebra'd / hairlined) →
        // flush rows, reference-grade rhythm comes from the rows themselves.
        rid = apply_root_transform(sink, &rid, crate::table_repair::flush_table_row_gap);
        // Card-level "-35%" tags meant for the image corner → adopt into the
        // image wrapper as an absolute 8,8 overlay.
        rid = apply_root_transform(sink, &rid, crate::chip_repair::adopt_corner_badges);
        // [bell icon, 8px square] flow pairs → round the dot and pin it on
        // the icon's top-right corner.
        rid = apply_root_transform(sink, &rid, crate::chip_repair::adopt_notification_dots);
        debug_probe_child_height(sink, &rid, "table_flush");
        // Transparent wrapper padding inside an already-padded/gapped column →
        // double inset: misaligned section edges + starved children (a padded
        // "Key Metrics" strip squeezed its KPI cards until label touched icon).
        rid = apply_root_transform(
            sink,
            &rid,
            crate::spacing_repair::strip_wrapper_double_inset,
        );
        debug_probe_child_height(sink, &rid, "double_inset");
        // Footer-sink floor: a vertical column that wants to PUSH content apart
        // (justifyContent space_*) or carries a flexible spacer, but hugs its
        // height, gets promoted to fill_container so the footer actually sinks.
        // Runs on the ASSEMBLED tree (the per-subtask role pass sees the section
        // in isolation, before the sidebar column gets its definite height).
        rid = apply_root_transform(
            sink,
            &rid,
            crate::role_layout_post_pass::sink_main_axis_distribution,
        );
        debug_probe_child_height(sink, &rid, "sink_main_axis");
        // Sidebar footer-sink for an ALREADY-STRUCTURED [sidebar | content] shell
        // whose nav column stacks flat (no space_between / spacer) with a
        // user/Pro footer last — the app-shell reshape above only handles the
        // flat-vertical-root case, so this catches the already-correct-shell case.
        rid = apply_root_transform(
            sink,
            &rid,
            crate::app_shell::sink_structured_sidebar_footers,
        );
        debug_probe_child_height(sink, &rid, "sink_footer");
        // The tree-shape `fit_content` parent ↔ `fill_container` child demoter
        // (`fix_circular_fill_height`) is RETIRED: the layout engine now resolves
        // a fill-height child of a hugging parent to its content size (vertical
        // main axis → grow, horizontal cross axis → stretch), so the collapse the
        // pass guessed at no longer exists — while its demotions actively broke
        // healthy shells (the app-shell's fill-height sidebar under a transient
        // fit_content root, equal-height KPI cards stretched by a numeric
        // sibling). A REAL collapse is still caught below by the geometry loop's
        // `collect_collapse_fixes`, which only fires on a resolved ~0 height.
        let rid = rid.as_str();

        remove_duplicate_status_bars(sink, rid);
        remove_duplicate_bottom_nav_sections(sink, rid);
        repair_light_mobile_nav_surfaces(sink, rid);
        repair_mobile_content_sections(sink, rid);
        cleanup_mobile_chrome::repair_mobile_structural_chrome(sink, rid);
        cleanup_mobile_dense::repair_dense_mobile_rows(sink, rid);
        cleanup_desktop_dashboard::repair_sparse_desktop_dashboard_rows(sink, plan, rid);
        repair_overbold_text_hierarchy(sink, rid);
        strip_decorative_filled_strokes(sink, rid);
        crate::radial_repair::repair_radial_stacks(sink, rid);
        crate::stub_repair::remove_empty_decorated_stubs(sink, rid);
        // Geometry-driven validation LOOP: run the REAL jian layout, detect +
        // fix what the resolved rects prove wrong (table columns overflowing
        // their row, fill containers collapsed to 0 height by a hugging ancestor
        // the tree-shape passes miss), then re-layout and repeat until clean —
        // the deterministic analogue of Pencil's per-batch snapshot_layout
        // feedback, catching what the tree-shape passes above cannot see.
        if let Ok(path) = std::env::var("OPENPENCIL_DEBUG_CLEANUP_DUMP") {
            if let Some(root) = sink
                .state()
                .active_children()
                .iter()
                .find(|n| n.id_str() == rid)
            {
                if let Ok(json) = serde_json::to_string_pretty(root) {
                    let _ = std::fs::write(&path, json);
                }
            }
        }
        crate::geometry_validation::geometry_validate_and_fix(sink, rid);
        debug_probe_child_height(sink, rid, "geometry");
        adjust_root_height_to_content(sink, rid);
        debug_probe_child_height(sink, rid, "adjust_root_height");
    }
}

/// Apply a whole-root transform (the serialize → mutate → deserialize round-trip
/// the structural passes use) to the page-root and commit it via `ReplaceSubtree`.
///
/// `ReplaceSubtree` allocates a FRESH id for the replaced node (see
/// `command_replace_tests`), so the root's id changes on every successful
/// transform. This returns the root's CURRENT id (re-resolved by its unchanged
/// position) so the caller threads it into the next pass — otherwise every
/// subsequent per-root cleanup pass would look up the stale id and no-op.
fn apply_root_transform(
    sink: &mut dyn DocSink,
    root_id: &str,
    transform: fn(&mut PenNode) -> bool,
) -> String {
    let Some(idx) = sink
        .state()
        .active_children()
        .iter()
        .position(|n| n.id_str() == root_id)
    else {
        // A silent no-op here means EVERY cleanup pass silently skips this
        // root — surface it loudly so a stale-root bug can't hide again.
        tracing::warn!(root = %root_id, "cleanup: root id not found — pass skipped");
        return root_id.to_string();
    };
    let mut new_root = sink.state().active_children()[idx].clone();
    if !transform(&mut new_root) {
        return root_id.to_string();
    }
    sink.apply(EditorCommand::ReplaceSubtree {
        node_id: NodeId::new(root_id.to_string()),
        node: Box::new(new_root),
        drop_children: true,
        page_id: None,
    });
    sink.state()
        .active_children()
        .get(idx)
        .map(|n| n.id_str().to_string())
        .unwrap_or_else(|| root_id.to_string())
}

/// Strip the REDUNDANT border off a filled, shadowed container. When a
/// frame / group / rectangle has a fill AND a drop shadow AND a stroke,
/// the stroke is a "莫名其妙" hairline — the shadow already separates the
/// surface, so the border adds nothing on a light page. Clearing it
/// (`stroke_width = 0` → `stroke = None`) is conservative on purpose:
/// a filled container WITHOUT a shadow keeps its stroke (there the border
/// is the intentional boundary), and unfilled outlines (dividers) +
/// `text_input` borders are never touched.
fn strip_decorative_filled_strokes(sink: &mut dyn DocSink, root_id: &str) {
    let targets: Vec<NodeId> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        let mut ids = Vec::new();
        collect_redundant_borders(root, &mut ids);
        ids
    };
    for node_id in targets {
        sink.apply(EditorCommand::SetNodeStrokeWidth {
            node_id,
            width: 0.0,
        });
    }
}

fn collect_redundant_borders(node: &PenNode, out: &mut Vec<NodeId>) {
    if has_redundant_shadowed_border(node) {
        out.push(NodeId::new(node.id_str().to_string()));
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_redundant_borders(child, out);
        }
    }
}

/// True when `node` is a Frame/Group/Rectangle carrying a non-empty fill,
/// a stroke, AND a drop shadow — the redundant-border case (the shadow,
/// not the stroke, separates the surface). A filled+stroked container
/// with NO shadow is left alone: there the border is intentional.
fn has_redundant_shadowed_border(node: &PenNode) -> bool {
    let container = match node {
        PenNode::Frame(n) => &n.container,
        PenNode::Group(n) => &n.container,
        PenNode::Rectangle(n) => &n.container,
        _ => return false,
    };
    let has_fill = container.fill.as_ref().is_some_and(|f| !f.is_empty());
    let has_shadow = container
        .effects
        .as_ref()
        .is_some_and(|fx| fx.iter().any(|e| matches!(e, PenEffect::Shadow(_))));
    has_fill && container.stroke.is_some() && has_shadow
}

#[cfg(test)]
#[path = "cleanup_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cleanup_abandoned_duplicate_roots_tests.rs"]
mod tests_abandoned_duplicate_roots;

#[cfg(test)]
#[path = "cleanup_mobile_dense_tests.rs"]
mod tests_mobile_dense;

#[cfg(test)]
#[path = "cleanup_mobile_chrome_tests.rs"]
mod tests_mobile_chrome;

#[cfg(test)]
#[path = "cleanup_mobile_bottom_nav_dedup_tests.rs"]
mod tests_mobile_bottom_nav_dedup;

#[cfg(test)]
#[path = "cleanup_desktop_dashboard_tests.rs"]
mod tests_desktop_dashboard;
