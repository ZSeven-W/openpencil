//! 阶段 2 —— 单屏画布搭建。
//!
//! 产出一条 `InsertSubtree`:把根 frame(移动端再带一个固定状态
//! 栏 child)插到活动页根。根 frame 用 JSON 构建后反序列化为
//! canonical `PenNode` —— 避免在 Rust 侧硬写富 schema 的每个字段,
//! 且与 `parse` 模块的解析路径一致。
//!
//! # Concurrent (multi-root) scaffold
//!
//! [`build_scaffold_concurrent_mobile`] handles the N-root-frame concurrent path
//! (S3b-2 Task A2). One root frame per [`ScreenGroup`], laid out
//! left-to-right with gap 100. Returns `(cmds, root_ids, baselines)` where
//! `baselines[i]` is the scaffold descendant count for root `i` (0 for
//! desktop, 1 for mobile with status bar).

use crate::concurrent::ScreenGroup;
use crate::plan::{OrchestratorPlan, Subtask};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};

/// Mobile mockup status bar height. Mirrors
/// `apps/web/src/services/ai/mobile-status-bar.ts`.
const STATUS_BAR_HEIGHT: f64 = 62.0;

const CELLULAR_D: &str =
    "M19.2 1.14623c0-0.63304-0.47756-1.14623-1.06667-1.14623l-1.06666 0c-0.5891 0-1.06667 0.51318-1.06667 1.14623l0 9.93396c0 0.63304 0.47756 1.14623 1.06667 1.14622l1.06666 0c0.5891 0 1.06667-0.51318 1.06667-1.14622l0-9.93396z m-7.43411 1.29905l1.06666 0c0.5891 0 1.06667 0.5255 1.06667 1.17374l0 7.43366c0 0.64824-0.47756 1.17374-1.06667 1.17373l-1.06666 0c-0.5891 0-1.06667-0.5255-1.06667-1.17373l0-7.43366c0-0.64824 0.47756-1.17374 1.06667-1.17374z m-4.33178 2.64905l-1.06666 0c-0.5891 0-1.06667 0.53219-1.06667 1.18868l0 4.75472c0 0.65649 0.47756 1.18868 1.06667 1.18867l1.06666 0.00001c0.5891 0 1.06667-0.53219 1.06667-1.18868l0-4.75472c0-0.65649-0.47756-1.18868-1.06667-1.18868z m-5.30078 2.44529l-1.06666 0c-0.5891 0-1.06667 0.52459-1.06667 1.1717l0 2.3434c0 0.64711 0.47756 1.1717 1.06667 1.1717l1.06666 0c0.5891 0 1.06667-0.52459 1.06667-1.1717l0-2.3434c0-0.64711-0.47756-1.1717-1.06667-1.1717z";
const WIFI_D: &str =
    "M8.5713 2.46628c2.48711 0.00011 4.87912 0.92219 6.68163 2.57567 0.13573 0.12765 0.35269 0.12604 0.48637-0.00361l1.29749-1.26347c0.06769-0.06576 0.10543-0.15484 0.10487-0.24752-0.00056-0.09268-0.03938-0.18133-0.10786-0.24631-4.73101-4.37472-12.19473-4.37472-16.92574 0-0.06853 0.06494-0.10742 0.15356-0.10805 0.24624-0.00063 0.09268 0.03704 0.18178 0.10468 0.24759l1.29786 1.26347c0.1336 0.12985 0.35072 0.13146 0.48638 0.00361 1.80274-1.65359 4.19502-2.57567 6.68237-2.57567z m-0.00335 4.22028c1.35732-0.00008 2.6662 0.51165 3.67232 1.43578 0.13608 0.13116 0.35045 0.12831 0.4831-0.00641l1.28728-1.3193c0.06779-0.0692 0.1054-0.16308 0.10443-0.26063-0.00098-0.09755-0.04047-0.19063-0.10963-0.25843-3.06383-2.89085-7.80857-2.89085-10.8724 0-0.06921 0.06779-0.10869 0.16092-0.1096 0.2585-0.00091 0.09758 0.03684 0.19145 0.10477 0.26056l1.28691 1.3193c0.13265 0.13472 0.34702 0.13756 0.4831 0.00641 1.00545-0.92352 2.3133-1.43521 3.66972-1.43578z m2.52442 2.79355c0.00193 0.10535-0.03514 0.20692-0.10244 0.28073l-2.17666 2.45472c-0.06381 0.07214-0.1508 0.11274-0.24157 0.11275-0.09077 0-0.17776-0.0406-0.24157-0.11275l-2.17703-2.45472c-0.06725-0.07386-0.10425-0.17546-0.10225-0.28082 0.00199-0.10535 0.0428-0.20511 0.11279-0.27573 1.3901-1.31389 3.42602-1.31389 4.81612 0 0.06994 0.07067 0.11068 0.17047 0.11261 0.27582z";
const CAP_D: &str =
    "M0 0l0 4c0.80473-0.33878 1.32804-1.12687 1.32804-2 0-0.87313-0.52331-1.66122-1.32804-2";

/// 并发多屏路径的相邻 root frame 间距(像素)。
/// Port of `const gap = 100` in `orchestrator.ts:868`.
const CONCURRENT_GAP: f64 = 100.0;

/// 并发路径里每个 root 的最小高度(桌面端)。
/// Port of `Math.max(320, totalRegionHeight)` in `orchestrator.ts:883`.
const CONCURRENT_MIN_HEIGHT: f64 = 320.0;

/// 移动端无 rootFrame.height 时的默认视口高度。
/// Port of `plan.rootFrame.height || 812` in `orchestrator.ts:882`.
const MOBILE_DEFAULT_HEIGHT: f64 = 812.0;

/// Keep newly generated single-screen roots out from under the native
/// floating toolbar.
const SAFE_CANVAS_X: f64 = 80.0;
const SAFE_CANVAS_Y: f64 = 40.0;

/// Return type of [`build_scaffold_concurrent_mobile`]:
/// `(commands, root_frame_ids, per_root_scaffold_baselines)`.
///
/// - `Vec<EditorCommand>` — one `InsertSubtree` per screen group.
/// - `Vec<String>` — the root frame ID for each group (index-aligned).
/// - `Vec<usize>` — scaffold descendant-count baseline per root
///   (0 for desktop, 1 for mobile with status bar).
pub(crate) type ConcurrentScaffoldResult = (Vec<EditorCommand>, Vec<String>, Vec<usize>);

fn solid_fill_json(color: &str) -> serde_json::Value {
    serde_json::json!([{ "type": "solid", "color": color }])
}

fn status_bar_foreground(fill_hex: &str) -> &'static str {
    let hex = fill_hex.trim_start_matches('#');
    let Some(rgb) = hex.get(0..6) else {
        return "#000000ff";
    };
    if rgb.len() != 6 {
        return "#000000ff";
    }
    let Ok(r) = u8::from_str_radix(&rgb[0..2], 16) else {
        return "#000000ff";
    };
    let Ok(g) = u8::from_str_radix(&rgb[2..4], 16) else {
        return "#000000ff";
    };
    let Ok(b) = u8::from_str_radix(&rgb[4..6], 16) else {
        return "#000000ff";
    };
    let luminance = (0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)) / 255.0;
    if luminance < 0.5 {
        "#ffffffff"
    } else {
        "#000000ff"
    }
}

/// Status-bar chrome for a mobile root frame. `width` is the root frame's
/// width so the right-aligned levels group (cellular/wifi/battery) clamps
/// to the screen edge instead of overflowing on explicit narrow widths
/// (e.g. 320 × 568 iPhone SE). Right-edge inset = 26 to match the iOS
/// safe-area gutter the 390-wide reference is built against.
fn mobile_status_bar_json(root_id: &str, fill_hex: &str, width: f64) -> serde_json::Value {
    const LEVELS_WIDTH: f64 = 78.0;
    const LEVELS_RIGHT_INSET: f64 = 26.0;
    let levels_x = (width - LEVELS_WIDTH - LEVELS_RIGHT_INSET).max(0.0);
    let fg = status_bar_foreground(fill_hex);
    let fg_fill = solid_fill_json(fg);
    let time_label = serde_json::json!({
        "type": "text",
        "id": format!("{root_id}-status-bar-time-label"),
        "name": "Time",
        "x": 0,
        "y": 0,
        "width": 54,
        "height": 22,
        "content": "9:41",
        "fill": fg_fill.clone(),
        "fontFamily": "Inter",
        "fontSize": 17,
        "fontWeight": 600,
        "lineHeight": 1.2941176470588236,
        "textAlign": "center"
    });
    let time = serde_json::json!({
        "type": "frame",
        "id": format!("{root_id}-status-bar-time"),
        "name": "Time",
        "x": 34,
        "y": 21,
        "width": 54,
        "height": 22,
        "layout": "none",
        "children": [time_label]
    });
    let cellular = serde_json::json!({
        "type": "path",
        "id": format!("{root_id}-status-bar-cellular"),
        "name": "Cellular Connection",
        "d": CELLULAR_D,
        "x": 0,
        "y": 0.85,
        "width": 19.2,
        "height": 12.226,
        "fill": fg_fill.clone()
    });
    let wifi = serde_json::json!({
        "type": "path",
        "id": format!("{root_id}-status-bar-wifi"),
        "name": "Wifi",
        "d": WIFI_D,
        "x": 26.2,
        "y": 0.75,
        "width": 17.142,
        "height": 12.328,
        "fill": fg_fill.clone()
    });
    let battery_border = serde_json::json!({
        "type": "rectangle",
        "id": format!("{root_id}-status-bar-battery-border"),
        "name": "Border",
        "x": 0,
        "y": 0,
        "width": 25,
        "height": 13,
        "cornerRadius": 4.3,
        "opacity": 0.35,
        "stroke": { "align": "inside", "fill": fg_fill.clone(), "thickness": 1 }
    });
    let battery_cap = serde_json::json!({
        "type": "path",
        "id": format!("{root_id}-status-bar-battery-cap"),
        "name": "Cap",
        "d": CAP_D,
        "x": 26,
        "y": 4.5,
        "width": 1.328,
        "height": 4.075,
        "fill": fg_fill.clone(),
        "opacity": 0.4
    });
    let battery_capacity = serde_json::json!({
        "type": "rectangle",
        "id": format!("{root_id}-status-bar-battery-capacity"),
        "name": "Capacity",
        "x": 2,
        "y": 2,
        "width": 21,
        "height": 9,
        "cornerRadius": 2.5,
        "fill": fg_fill
    });
    let battery = serde_json::json!({
        "type": "frame",
        "id": format!("{root_id}-status-bar-battery"),
        "name": "Battery",
        "x": 50.3,
        "y": 0,
        "width": 27.328,
        "height": 13,
        "layout": "none",
        "children": [battery_border, battery_cap, battery_capacity]
    });
    let levels = serde_json::json!({
        "type": "frame",
        "id": format!("{root_id}-status-bar-levels"),
        "name": "Levels",
        "x": levels_x,
        "y": 24,
        "width": LEVELS_WIDTH,
        "height": 14,
        "layout": "none",
        "children": [cellular, wifi, battery]
    });

    serde_json::json!({
        "type": "frame",
        "id": format!("{root_id}-status-bar"),
        "name": "Status Bar",
        "role": "status-bar",
        "width": "fill_container",
        "height": STATUS_BAR_HEIGHT,
        "layout": "none",
        "children": [time, levels]
    })
}

/// 构建单个根 frame 的 `PenNode`,含可选状态栏子节点。
///
/// 内部辅助函数,被 `build_scaffold`(单屏)和
/// `build_scaffold_concurrent`(多屏)共用。
#[allow(clippy::too_many_arguments)]
/// Canonical inter-section spacing for the page-root vertical stack (Pencil's
/// reverse-engineered demo uses `gap: 20` between sections). The LLM frequently
/// omits the page gap, leaving it `0` so every section touches the next — the
/// cramped, no-breathing-room look that reads nothing like the TS references.
const SECTION_STACK_GAP: f64 = 20.0;

/// Resolve the page-root section gap: honor an explicit positive plan gap,
/// otherwise fall back to [`SECTION_STACK_GAP`]. Mirrors the dashboard
/// main-column gap fallback (`root.gap > 0 ? root.gap : 20`).
fn resolve_section_gap(plan_gap: Option<f64>) -> f64 {
    match plan_gap {
        Some(g) if g > 0.0 => g,
        _ => SECTION_STACK_GAP,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_root_frame_node(
    id: &str,
    name: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    layout: &str,
    gap: f64,
    fill_hex: &str,
    is_mobile: bool,
) -> Result<PenNode, String> {
    let children = if is_mobile {
        serde_json::json!([mobile_status_bar_json(id, fill_hex, width)])
    } else {
        serde_json::json!([])
    };

    let frame = serde_json::json!({
        "type": "frame",
        "id": id,
        "name": name,
        "x": x,
        "y": y,
        "width": width,
        "height": height,
        "layout": layout,
        "gap": gap,
        "fill": [{ "type": "solid", "color": fill_hex }],
        "children": children,
    });

    serde_json::from_value(frame).map_err(|e| format!("scaffold root frame `{id}`: {e}"))
}

/// 从 subtask 的 label 剥去括号后缀并 trim,用作 frame 名 fallback。
/// Port of `firstSt.label.replace(/\s*[（(].+$/, '').trim() || firstSt.label`
/// in `orchestrator.ts:888`.
#[allow(dead_code)]
fn short_label(st: &Subtask) -> String {
    let stripped = if let Some(pos) = st.label.find(['（', '(']) {
        st.label[..pos].trim().to_string()
    } else {
        st.label.trim().to_string()
    };
    if stripped.is_empty() {
        st.label.clone()
    } else {
        stripped
    }
}

/// 构建阶段 2 的画布命令(单屏顺序路径)。
///
/// `is_mobile` 为真时根 frame 带一个固定状态栏 child。
/// 返回 `Err` 表示根 frame JSON 模板有问题(实现 bug,非用户输入问题)。
pub fn build_scaffold(
    plan: &OrchestratorPlan,
    is_mobile: bool,
) -> Result<Vec<EditorCommand>, String> {
    build_scaffold_at(plan, is_mobile, SAFE_CANVAS_X, SAFE_CANVAS_Y)
}

/// Same as [`build_scaffold`], but places the newly inserted root at an
/// explicit canvas position. Used for follow-on screens so they land beside the
/// existing app screen instead of overlapping the starter coordinates.
pub(crate) fn build_scaffold_at(
    plan: &OrchestratorPlan,
    is_mobile: bool,
    x: f64,
    y: f64,
) -> Result<Vec<EditorCommand>, String> {
    let node = build_scaffold_root_node_at(plan, is_mobile, &plan.root_frame.id, x, y)?;
    Ok(vec![EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    }])
}

/// Reuse an existing EMPTY top-level frame (the fresh-canvas starter) as the
/// design root, mirroring TS `replaceEmptyFrame`: instead of clearing the
/// starter + inserting a brand-new root (the visible "delete + re-add" the
/// user flagged), REPLACE the starter frame's subtree in place. The scaffold
/// root takes the reused id, so its slot/identity is preserved and the canvas
/// fills smoothly rather than flashing empty.
pub fn build_scaffold_reusing(
    plan: &OrchestratorPlan,
    is_mobile: bool,
    reuse_id: &str,
) -> Result<Vec<EditorCommand>, String> {
    let node =
        build_scaffold_root_node_at(plan, is_mobile, reuse_id, SAFE_CANVAS_X, SAFE_CANVAS_Y)?;
    Ok(vec![EditorCommand::ReplaceSubtree {
        node_id: NodeId::new(reuse_id.to_string()),
        node: Box::new(node),
        drop_children: true,
        page_id: None,
    }])
}

/// Build the root-frame node (status-bar child injected when `is_mobile`),
/// stamping `root_id` as its id so the caller can either insert it fresh or
/// replace an existing frame's slot with it.
fn build_scaffold_root_node_at(
    plan: &OrchestratorPlan,
    is_mobile: bool,
    root_id: &str,
    x: f64,
    y: f64,
) -> Result<PenNode, String> {
    let rf = &plan.root_frame;
    let layout = rf.layout.as_deref().unwrap_or("vertical");
    let fill_hex = rf
        .first_solid_hex()
        .unwrap_or_else(|| "#FFFFFF".to_string());

    build_root_frame_node(
        root_id,
        &rf.name,
        x,
        y,
        rf.width,
        rf.height,
        layout,
        resolve_section_gap(rf.gap),
        &fill_hex,
        is_mobile,
    )
}

/// 构建 N-root-frame 并发画布搭建命令(多屏并发路径,S3b-2 Task A2)。
///
/// Port of `orchestrator.ts:856-937` (the `effectiveConcurrency > 1` scaffold
/// branch). One root frame per screen group, laid out left-to-right with
/// gap 100. Each group's subtasks get `parent_frame_id` = that group's root ID.
///
/// # Arguments
/// * `plan` — orchestrator plan (provides `root_frame` spec).
/// * `groups` — screen groups from [`crate::concurrent::group_subtasks_by_screen`].
///
/// # Returns
/// Mobile-aware variant; exposed as a separate entry point so tests can drive
/// the mobile path without going through `plan_normalize`.
///
/// Pass `is_mobile = false` for the desktop (non-mobile) path.
///
/// # Side effect on plan subtasks
/// The caller is responsible for assigning `parent_frame_id` on each subtask
/// from the returned `root_ids`. This function does NOT mutate `plan` — it
/// returns the mapping data needed for the caller to do so.
///
/// Callers land in S3b-2 Task C2 (`run.rs`).
pub(crate) fn build_scaffold_concurrent_mobile(
    plan: &OrchestratorPlan,
    groups: &[ScreenGroup],
    is_mobile: bool,
) -> Result<ConcurrentScaffoldResult, String> {
    build_scaffold_concurrent_inner(plan, groups, is_mobile)
}

fn build_scaffold_concurrent_inner(
    plan: &OrchestratorPlan,
    groups: &[ScreenGroup],
    is_mobile: bool,
) -> Result<ConcurrentScaffoldResult, String> {
    let rf = &plan.root_frame;
    let layout = rf.layout.as_deref().unwrap_or("vertical");
    let fill_hex = rf
        .first_solid_hex()
        .unwrap_or_else(|| "#FFFFFF".to_string());

    let mut cmds: Vec<EditorCommand> = Vec::with_capacity(groups.len());
    let mut root_ids: Vec<String> = Vec::with_capacity(groups.len());
    let mut baselines: Vec<usize> = Vec::with_capacity(groups.len());

    let mut next_x: f64 = 0.0;

    for (g, group) in groups.iter().enumerate() {
        // Original ID: `{root_frame.id}-{group.screen}` — mirrors TS `originalId`.
        // The first group uses the same id scheme; `run()` handles the
        // empty-canvas remap (only for g == 0) after applying the InsertSubtree.
        let original_id = format!("{}-{}", rf.id, group.screen);

        // Height: mobile uses fixed viewport; desktop = max(320, Σ region heights).
        let total_region_height: f64 = group
            .indices
            .iter()
            .map(|&i| {
                plan.subtasks
                    .get(i)
                    .map(|st| st.region.height)
                    .unwrap_or(0.0)
            })
            .sum();

        let frame_height = if is_mobile {
            if rf.height > 0.0 {
                rf.height
            } else {
                MOBILE_DEFAULT_HEIGHT
            }
        } else {
            total_region_height.max(CONCURRENT_MIN_HEIGHT)
        };

        // Frame name: use screen name if the first subtask has one,
        // else first subtask's short label. Port of `orchestrator.ts:886-888`.
        let frame_name =
            if let Some(first_st) = group.indices.first().and_then(|&i| plan.subtasks.get(i)) {
                if first_st.screen.is_some() {
                    group.screen.clone()
                } else {
                    short_label(first_st)
                }
            } else {
                group.screen.clone()
            };

        let node = build_root_frame_node(
            &original_id,
            &frame_name,
            next_x,
            0.0,
            rf.width,
            frame_height,
            layout,
            resolve_section_gap(rf.gap),
            &fill_hex,
            is_mobile,
        )?;

        // The first root frame uses InsertSubtree with NONE parent (same as the
        // single-root path) so the run() caller can handle the empty-canvas
        // ID-remap. Subsequent roots are plain InsertSubtree with NONE parent
        // (addNode into page root — mirrors TS `addNode(null, rootNode)`).
        let _ = g; // both paths use NodeId::NONE — kept for clarity
        cmds.push(EditorCommand::InsertSubtree {
            nodes: vec![node],
            parent_id: NodeId::NONE,
            page_id: None,
        });

        root_ids.push(original_id);
        // Baseline: mobile root has 1 scaffold child (status bar); desktop has 0.
        baselines.push(if is_mobile { 1 } else { 0 });

        next_x += rf.width + CONCURRENT_GAP;
    }

    Ok((cmds, root_ids, baselines))
}

#[cfg(test)]
#[path = "scaffold_tests.rs"]
mod tests;
