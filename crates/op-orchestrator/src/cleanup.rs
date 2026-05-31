//! 阶段 4 —— 清理 pass。
//!
//! [`run_cleanup_passes`] 在所有 subtask 插入完成后运行,是独立
//! 函数 —— S3a 顺序路径与 S3b 并发路径都复用它(spec §9)。
//!
//! [`descendant_count`] 给 `run()` 的"零内容"判定提供基线:
//! scaffold 之后数一次,subtask 全跑完再数一次,没涨即零内容。
//!
//! ## Task C1 (S3b-2): concurrent failure policy + N-root cleanup
//!
//! [`aggregate_concurrent_verdict`] — run-all-aggregate failure check:
//! if EVERY collected outcome has 0 nodes → returns
//! `Err(OrchestratorError::AllFailed)` carrying the first non-empty error
//! string (or a fallback message); otherwise returns `Ok(())` (partial
//! success is accepted). Port of `orchestrator-sub-agent.ts:319-325`.
//!
//! [`cleanup_concurrent_roots`] — N-root cleanup after a concurrent run.
//! Per root: if `descendant_count <= baseline`, delete that root (scaffold-only).
//! Roll back plan-derived variables only when NOTHING survived across all roots.
//! Port of `orchestrator.ts:1101-1158`.

use crate::cleanup_typography::repair_overbold_text_hierarchy;
use crate::plan::OrchestratorPlan;
use crate::types::{DocSink, OrchestratorError, SubtaskOutcome};
use crate::variables::{rollback, VarSnapshot};
use jian_ops_schema::node::{container::Padding, PenNode};
use op_editor_core::{
    first_fill_type, first_solid_fill_hex, node_effects, EditorCommand, EditorState, EffectField,
    FillType, LayoutPropValue, NodeId, PenNodeExt,
};

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
        sink.apply(EditorCommand::DeleteNode { node_id: id });
    }
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
        if repair.add_shadow {
            add_nav_surface_shadow(sink, repair.node_id, repair.is_bottom_nav);
        }
    }
}

#[derive(Debug, Clone)]
struct NavSurfaceRepair {
    node_id: NodeId,
    fill_hex: String,
    add_shadow: bool,
    is_bottom_nav: bool,
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
/// 直接子节点的像素高度之和(无显式像素高度的子节点跳过)。
/// 对齐 TS `adjustRootFrameHeightToContent`。
fn adjust_root_height_to_content(sink: &mut dyn DocSink, root_id: &str) {
    let total: Option<i32> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        let Some(children) = root.children() else {
            return;
        };
        if children.is_empty() {
            return;
        }
        let sum: f64 = children.iter().filter_map(|c| c.height_px()).sum();
        if sum > 0.0 {
            Some(sum.round() as i32)
        } else {
            None
        }
    };
    let current_height = find_root(sink.state(), root_id).and_then(|root| root.height_px());
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
    if matches!(
        role.as_str(),
        "navbar" | "nav" | "tab-bar" | "bottom-tab-bar" | "top-nav-bar" | "top-app-bar" | "tab-row"
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
        || hay.contains("navbar")
        || hay.contains("nav bar")
        || hay.contains("navigation bar")
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
        add_shadow: !has_paintable_fill && node_effects(nav).is_empty(),
        is_bottom_nav,
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

fn add_nav_surface_shadow(sink: &mut dyn DocSink, node_id: NodeId, is_bottom_nav: bool) {
    sink.apply(EditorCommand::AddNodeEffect {
        node_id: node_id.clone(),
        kind: "shadow".to_string(),
    });
    sink.apply(EditorCommand::SetEffectParam {
        node_id: node_id.clone(),
        index: 0,
        field: EffectField::OffsetX,
        value: 0.0,
    });
    sink.apply(EditorCommand::SetEffectParam {
        node_id: node_id.clone(),
        index: 0,
        field: EffectField::OffsetY,
        value: if is_bottom_nav { -4.0 } else { 4.0 },
    });
    sink.apply(EditorCommand::SetEffectParam {
        node_id: node_id.clone(),
        index: 0,
        field: EffectField::Blur,
        value: 12.0,
    });
    sink.apply(EditorCommand::SetEffectParam {
        node_id: node_id.clone(),
        index: 0,
        field: EffectField::Spread,
        value: 0.0,
    });
    sink.apply(EditorCommand::SetEffectColor {
        node_id,
        index: 0,
        hex: "#0000000F".to_string(),
    });
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

/// 在活动页根里按 id 找节点。
fn find_root<'a>(state: &'a EditorState, root_id: &str) -> Option<&'a PenNode> {
    state
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
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
pub fn run_cleanup_passes(sink: &mut dyn DocSink, _plan: &OrchestratorPlan, root_ids: &[&str]) {
    for root_id in root_ids {
        remove_duplicate_status_bars(sink, root_id);
        repair_light_mobile_nav_surfaces(sink, root_id);
        repair_mobile_content_sections(sink, root_id);
        repair_overbold_text_hierarchy(sink, root_id);
        adjust_root_height_to_content(sink, root_id);
    }
}

// ── Task C1: concurrent failure policy + N-root cleanup ──────────────────────

/// Run-all-aggregate failure check.
///
/// Port of `orchestrator-sub-agent.ts:319-325`.
///
/// Called after all concurrent workers have finished and their outcomes have
/// been collected.  Rules:
///
/// - `collected` must be the non-None outcomes from `run_concurrent`'s
///   `Vec<Option<SubtaskOutcome>>` (i.e. outcomes that actually ran).
/// - `total_nodes = Σ outcome.node_count` across `collected`.
/// - If `total_nodes == 0 && !collected.is_empty()` → returns
///   `Err(OrchestratorError::NoContent)` carrying the FIRST non-empty
///   `error` string.  Fallback message when no outcome has an error:
///   `"The model failed to generate any design output."`.
/// - Partial success (any outcome has `node_count > 0`) → returns `Ok(())`.
/// - Empty collected (all aborted) → returns `Ok(())` (the caller handles
///   the abort case separately).
///
/// This is explicitly DIFFERENT from the sequential path's fail-fast: the
/// concurrent path runs ALL workers regardless of individual failures and only
/// fails the entire run when nothing was produced at all.
pub fn aggregate_concurrent_verdict(collected: &[SubtaskOutcome]) -> Result<(), OrchestratorError> {
    if collected.is_empty() {
        return Ok(());
    }
    let total_nodes: usize = collected.iter().map(|o| o.node_count).sum();
    if total_nodes == 0 {
        // Gather the first non-empty error string for the error context.
        // Port of `orchestrator-sub-agent.ts:322-324`:
        //   const errors = collected.filter(r => r.error).map(r => r.error!);
        //   const firstError = errors[0] ?? 'The model failed to generate any design output.';
        //   throw new Error(firstError);
        //
        // `OrchestratorError::NoContent` is the canonical error variant here.
        // The message is preserved in the `Internal` variant for callers that
        // need a human-readable string (e.g., the progress/error log).
        let first_error = collected
            .iter()
            .find_map(|o| o.error.as_deref().filter(|s| !s.is_empty()))
            .unwrap_or("The model failed to generate any design output.");
        return Err(OrchestratorError::AllFailed(first_error.to_string()));
    }
    Ok(())
}

/// N-root cleanup on the concurrent throw path.
///
/// Port of `orchestrator.ts:1101-1158` (the `catch (e)` block after
/// `executeSubAgents`).
///
/// For each root in `root_ids`:
/// - Compute `now_count = descendant_count(root_id)` on the live sink state.
/// - If `now_count <= baselines[i]` → root is scaffold-only (no sub-agent
///   content produced); delete it (`DeleteNode`).
/// - Otherwise → content survived; set `any_content_survived = true`.
///
/// After iterating all roots:
/// - Roll back plan-derived variables (via `rollback`) ONLY when
///   `!any_content_survived`.  With partial success the user still sees a
///   design whose colors should match the seeded palette; rolling back would
///   flip every `$color-*` ref to the default-palette blue.
///
/// `root_ids` and `baselines` must be index-aligned (both length N).
/// Missing/already-deleted roots are silently skipped (mirrors the TS
/// `try { store.removeNode(rn.id) } catch { /* already gone */ }` pattern).
pub fn cleanup_concurrent_roots(
    sink: &mut dyn DocSink,
    root_ids: &[&str],
    baselines: &[usize],
    var_snapshot: &VarSnapshot,
) {
    debug_assert_eq!(
        root_ids.len(),
        baselines.len(),
        "root_ids and baselines must be index-aligned"
    );

    let mut any_content_survived = false;

    for (i, &root_id) in root_ids.iter().enumerate() {
        let baseline = baselines.get(i).copied().unwrap_or(0);
        let now_count = descendant_count(sink.state(), root_id);

        if now_count <= baseline {
            // Scaffold-only root — remove it (mirrors TS `store.removeNode`).
            sink.apply(EditorCommand::DeleteNode {
                node_id: NodeId::new(root_id.to_string()),
            });
        } else {
            any_content_survived = true;
        }
    }

    // Roll back plan-derived variables only when NO content survived at all.
    if !any_content_survived {
        rollback(sink, var_snapshot);
    }
}

// Tests are split into sibling files: cleanup_tests.rs (general
// cleanup pass tests) + cleanup_tests_c1.rs (Task C1 tests).
#[cfg(test)]
#[path = "cleanup_tests_c1.rs"]
mod tests_c1;

#[cfg(test)]
#[path = "cleanup_tests.rs"]
mod tests;
