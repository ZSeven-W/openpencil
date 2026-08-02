//! 阶段 2 —— 单屏画布搭建。
//!
//! 产出一条 `InsertSubtree`:把根 frame(移动端再带一个固定状态
//! 栏 child)插到活动页根。根 frame 用 JSON 构建后反序列化为
//! canonical `PenNode` —— 避免在 Rust 侧硬写富 schema 的每个字段,
//! 且与 `parse` 模块的解析路径一致。

use crate::plan::{OrchestratorPlan, Subtask};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};

/// Scaffold-template failure. Every variant means "the JSON template this
/// module hard-codes did not deserialize into a canonical `PenNode`" — an
/// implementation bug, never bad user input. `detail` carries the
/// `serde_json::Error` rendering (that error is neither `Clone` nor `Eq`,
/// so it is kept as its display text to leave this enum comparable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScaffoldError {
    /// The mobile status-bar chrome failed to deserialize.
    MobileStatusBar { root_id: String, detail: String },
    /// A scaffold root frame failed to deserialize.
    RootFrame { id: String, detail: String },
    /// The pre-built two-column app-shell root failed to deserialize.
    TwoColumnRoot { root_id: String, detail: String },
}

impl std::fmt::Display for ScaffoldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MobileStatusBar { root_id, detail } => {
                write!(f, "mobile status bar for `{root_id}`: {detail}")
            }
            Self::RootFrame { id, detail } => write!(f, "scaffold root frame `{id}`: {detail}"),
            Self::TwoColumnRoot { root_id, detail } => {
                write!(f, "two-column scaffold root `{root_id}`: {detail}")
            }
        }
    }
}

impl std::error::Error for ScaffoldError {}

/// Keeps `?`-into-`Result<_, String>` call sites (in this crate and in
/// hosts that consume the scaffold API) compiling unchanged.
impl From<ScaffoldError> for String {
    fn from(error: ScaffoldError) -> Self {
        error.to_string()
    }
}

/// Mobile mockup status bar height. Mirrors
/// `apps/web/src/services/ai/mobile-status-bar.ts`.
const STATUS_BAR_HEIGHT: f64 = 62.0;

const CELLULAR_D: &str =
    "M19.2 1.14623c0-0.63304-0.47756-1.14623-1.06667-1.14623l-1.06666 0c-0.5891 0-1.06667 0.51318-1.06667 1.14623l0 9.93396c0 0.63304 0.47756 1.14623 1.06667 1.14622l1.06666 0c0.5891 0 1.06667-0.51318 1.06667-1.14622l0-9.93396z m-7.43411 1.29905l1.06666 0c0.5891 0 1.06667 0.5255 1.06667 1.17374l0 7.43366c0 0.64824-0.47756 1.17374-1.06667 1.17373l-1.06666 0c-0.5891 0-1.06667-0.5255-1.06667-1.17373l0-7.43366c0-0.64824 0.47756-1.17374 1.06667-1.17374z m-4.33178 2.64905l-1.06666 0c-0.5891 0-1.06667 0.53219-1.06667 1.18868l0 4.75472c0 0.65649 0.47756 1.18868 1.06667 1.18867l1.06666 0.00001c0.5891 0 1.06667-0.53219 1.06667-1.18868l0-4.75472c0-0.65649-0.47756-1.18868-1.06667-1.18868z m-5.30078 2.44529l-1.06666 0c-0.5891 0-1.06667 0.52459-1.06667 1.1717l0 2.3434c0 0.64711 0.47756 1.1717 1.06667 1.1717l1.06666 0c0.5891 0 1.06667-0.52459 1.06667-1.1717l0-2.3434c0-0.64711-0.47756-1.1717-1.06667-1.1717z";
const WIFI_D: &str =
    "M8.5713 2.46628c2.48711 0.00011 4.87912 0.92219 6.68163 2.57567 0.13573 0.12765 0.35269 0.12604 0.48637-0.00361l1.29749-1.26347c0.06769-0.06576 0.10543-0.15484 0.10487-0.24752-0.00056-0.09268-0.03938-0.18133-0.10786-0.24631-4.73101-4.37472-12.19473-4.37472-16.92574 0-0.06853 0.06494-0.10742 0.15356-0.10805 0.24624-0.00063 0.09268 0.03704 0.18178 0.10468 0.24759l1.29786 1.26347c0.1336 0.12985 0.35072 0.13146 0.48638 0.00361 1.80274-1.65359 4.19502-2.57567 6.68237-2.57567z m-0.00335 4.22028c1.35732-0.00008 2.6662 0.51165 3.67232 1.43578 0.13608 0.13116 0.35045 0.12831 0.4831-0.00641l1.28728-1.3193c0.06779-0.0692 0.1054-0.16308 0.10443-0.26063-0.00098-0.09755-0.04047-0.19063-0.10963-0.25843-3.06383-2.89085-7.80857-2.89085-10.8724 0-0.06921 0.06779-0.10869 0.16092-0.1096 0.2585-0.00091 0.09758 0.03684 0.19145 0.10477 0.26056l1.28691 1.3193c0.13265 0.13472 0.34702 0.13756 0.4831 0.00641 1.00545-0.92352 2.3133-1.43521 3.66972-1.43578z m2.52442 2.79355c0.00193 0.10535-0.03514 0.20692-0.10244 0.28073l-2.17666 2.45472c-0.06381 0.07214-0.1508 0.11274-0.24157 0.11275-0.09077 0-0.17776-0.0406-0.24157-0.11275l-2.17703-2.45472c-0.06725-0.07386-0.10425-0.17546-0.10225-0.28082 0.00199-0.10535 0.0428-0.20511 0.11279-0.27573 1.3901-1.31389 3.42602-1.31389 4.81612 0 0.06994 0.07067 0.11068 0.17047 0.11261 0.27582z";
const CAP_D: &str =
    "M0 0l0 4c0.80473-0.33878 1.32804-1.12687 1.32804-2 0-0.87313-0.52331-1.66122-1.32804-2";

/// Keep newly generated single-screen roots out from under the native
/// floating toolbar.
const SAFE_CANVAS_X: f64 = 80.0;
const SAFE_CANVAS_Y: f64 = 40.0;

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

/// Build the mobile status-bar chrome as a canonical `PenNode`, for callers
/// outside the scaffold pipeline (the design-agent loop's root-seed guard
/// injects the SAME chrome the orchestrator scaffold pre-inserts, so a
/// loop-generated mobile screen ships with an identical status bar).
pub fn mobile_status_bar_node(
    root_id: &str,
    fill_hex: &str,
    width: f64,
) -> Result<PenNode, ScaffoldError> {
    serde_json::from_value(mobile_status_bar_json(root_id, fill_hex, width)).map_err(|e| {
        ScaffoldError::MobileStatusBar {
            root_id: root_id.to_string(),
            detail: e.to_string(),
        }
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
pub(crate) const SECTION_STACK_GAP: f64 = 20.0;

/// Resolve the page-root section gap: honor an explicit positive plan gap,
/// otherwise fall back to [`SECTION_STACK_GAP`]. Mirrors the dashboard
/// main-column gap fallback (`root.gap > 0 ? root.gap : 20`).
fn resolve_section_gap(plan_gap: Option<f64>) -> f64 {
    match plan_gap {
        Some(g) if g > 0.0 => g,
        _ => SECTION_STACK_GAP,
    }
}

/// A plan's `rootFrame.height` is often 0 — the model's "compute it from
/// content". A LITERAL 0-height root makes every `fill_container` descendant
/// resolve to 0px for the whole pipeline (measured: the sidebar footer
/// floated mid-page on three consecutive user runs), and the old
/// `fit_content` mapping made the artboard START as a thin strip that
/// jerked taller with every subtask (user: "web 输出应该一开始就预设整体
/// 高度，参考 Pencil"). Preset a full artboard by device class instead —
/// mobile 844 / desktop 900 — so the canvas reads as a complete page being
/// filled in; `adjust_root_height_to_content` (the LAST pass) still writes
/// the definitive number at the end.
fn root_height_json(height: f64, width: f64) -> serde_json::Value {
    if height > 0.0 {
        serde_json::json!(height)
    } else if width <= 480.0 {
        serde_json::json!(844)
    } else {
        serde_json::json!(900)
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
) -> Result<PenNode, ScaffoldError> {
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
        "height": root_height_json(height, width),
        "layout": layout,
        "gap": gap,
        "fill": [{ "type": "solid", "color": fill_hex }],
        "children": children,
    });

    serde_json::from_value(frame).map_err(|e| ScaffoldError::RootFrame {
        id: id.to_string(),
        detail: e.to_string(),
    })
}

/// 从 subtask 的 label 剥去括号后缀并 trim,用作 frame 名 fallback。
/// Port of `firstSt.label.replace(/\s*[（(].+$/, '').trim() || firstSt.label`
/// in `orchestrator.ts:888`. Consumed by
/// [`build_screen_group_scaffold`]'s per-group frame-name fallback.
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
) -> Result<Vec<EditorCommand>, ScaffoldError> {
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
) -> Result<Vec<EditorCommand>, ScaffoldError> {
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
) -> Result<Vec<EditorCommand>, ScaffoldError> {
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
) -> Result<PenNode, ScaffoldError> {
    let rf = &plan.root_frame;
    let fill_hex = rf
        .first_solid_hex()
        .unwrap_or_else(|| "#FFFFFF".to_string());

    // Desktop sidebar dashboard → pre-build the two-column app-shell so the
    // sidebar is a narrow left column from the first subtask (matching Pencil),
    // instead of filling the root until the finalize `app_shell` reshape. The
    // sidebar subtask then generates into the left column and the rest into the
    // content column (routed in `run.rs`).
    if plan_is_sidebar_dashboard(plan, is_mobile) {
        return build_two_column_root_node(
            root_id,
            &rf.name,
            x,
            y,
            rf.width,
            rf.height,
            &fill_hex,
            resolve_section_gap(rf.gap),
        );
    }

    let layout = rf.layout.as_deref().unwrap_or("vertical");
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

/// Horizontal gap between adjacent screen-group root frames — mirrors the
/// gap `run::next_root_insert_position` already uses to land a single
/// follow-on root beside existing canvas content, so every sibling screen
/// (pre-existing canvas content, group 0, group 1, …) reads with one
/// consistent gutter instead of two different magic numbers for the same
/// "next screen goes here" idea.
pub(crate) const SCREEN_GROUP_GAP: f64 = 80.0;

/// `(commands, placeholder_ids, baselines)` — mirrors the deleted concurrent
/// path's own `ConcurrentScaffoldResult` alias (same shape, same reason:
/// clippy's `type_complexity` on the raw 3-tuple-of-Vecs).
pub(crate) type ScreenGroupScaffoldResult = (Vec<EditorCommand>, Vec<String>, Vec<usize>);

/// Build one scaffold root PER screen group (multiscreen-fanout-break fix,
/// item A) — the per-screen N-root structure `aca0d3a0` deleted alongside
/// the concurrent worker machinery it was bundled with. This revival is
/// STRUCTURE ONLY: `run.rs` still runs every group's subtasks strictly
/// SEQUENTIALLY (no concurrency revived), so this just returns N `InsertSubtree`
/// commands, one per group, laid out left-to-right starting at `(start_x, y)`.
///
/// Each root inherits `plan.root_frame`'s width/height/layout/gap/fill
/// VERBATIM (no per-group size inference) — a screen-group scaffold is
/// exactly the single-root scaffold, just repeated per screen, so the
/// existing `adjust_root_height_to_content` finalize pass sizes it the same
/// way it already sizes the single-root case.
///
/// Returns `(commands, placeholder_ids, baselines)`:
/// - `placeholder_ids[g]` is the id stamped on group `g`'s node BEFORE
///   insertion (`"{rootFrame.id}-{screen}"`) — the caller resolves the REAL
///   post-insert id by diffing `active_children()` before/after, exactly
///   like the single-root path already does for its one root.
/// - `baselines[g]` is that root's scaffold-only descendant count (0, or 1
///   for the injected mobile status bar) — the pre-subtask content floor
///   `run.rs`'s zero-content check subtracts off per root.
pub(crate) fn build_screen_group_scaffold(
    plan: &OrchestratorPlan,
    groups: &[crate::screen_groups::ScreenGroup],
    is_mobile: bool,
    start_x: f64,
    y: f64,
) -> Result<ScreenGroupScaffoldResult, ScaffoldError> {
    let rf = &plan.root_frame;
    let layout = rf.layout.as_deref().unwrap_or("vertical");
    let fill_hex = rf
        .first_solid_hex()
        .unwrap_or_else(|| "#FFFFFF".to_string());

    let mut nodes = Vec::with_capacity(groups.len());
    let mut placeholder_ids = Vec::with_capacity(groups.len());
    let mut baselines = Vec::with_capacity(groups.len());
    let mut next_x = start_x;

    for group in groups {
        // Placeholder id: `{root_frame.id}-{screen}` — mirrors the deleted
        // concurrent path's `original_id` scheme. Only used as a join key
        // until the caller remaps it to the real post-insert id.
        let placeholder_id = format!("{}-{}", rf.id, group.screen);

        // Frame name: the screen label, UNLESS this group's first subtask
        // has no `screen` of its own (the untagged-subtasks-fall-back-to-
        // first_screen case in `group_subtasks_by_screen`) — then fall back
        // to that subtask's own short label, so a synthetic "page" default
        // group never paints as a frame literally named "page". Port of
        // `orchestrator.ts:886-888`.
        let frame_name = group
            .indices
            .first()
            .and_then(|&i| plan.subtasks.get(i))
            .map(|first_st| {
                if first_st.screen.is_some() {
                    group.screen.clone()
                } else {
                    short_label(first_st)
                }
            })
            .unwrap_or_else(|| group.screen.clone());

        let node = build_root_frame_node(
            &placeholder_id,
            &frame_name,
            next_x,
            y,
            rf.width,
            rf.height,
            layout,
            resolve_section_gap(rf.gap),
            &fill_hex,
            is_mobile,
        )?;

        nodes.push(node);
        placeholder_ids.push(placeholder_id);
        // Baseline: mobile root has 1 scaffold child (status bar); desktop
        // has 0 — mirrors the deleted concurrent path's identical baseline.
        baselines.push(usize::from(is_mobile));
        next_x += rf.width + SCREEN_GROUP_GAP;
    }

    // ONE insert carrying every root, not one insert per root. A top-level
    // `InsertSubtree` of a single frame is treated as "replace the empty
    // fresh-canvas starter" (`command_root_replace::prepare_root_frame_replacement`,
    // which matches ANY empty root) — so inserting N roots one at a time made
    // each new root swallow the previous one, which is still empty at scaffold
    // time. Six slides arrived as one board, the last one (measured
    // 2026-08-01: six `applied=true` inserts, one surviving root).
    //
    // The replacement path bails on `nodes.len() != 1`, and these roots are one
    // scaffold anyway, so batching them is both the fix and the truer shape.
    let cmds = vec![EditorCommand::InsertSubtree {
        nodes,
        parent_id: NodeId::NONE,
        page_id: None,
    }];
    Ok((cmds, placeholder_ids, baselines))
}

/// Fixed left-column width for a pre-built dashboard app-shell. Mirrors
/// `dashboard_columns`' sidebar width and `app_shell::SIDEBAR_WIDTH`.
const SIDEBAR_COLUMN_WIDTH: f64 = 260.0;

/// Names stamped on the two pre-built columns. The run loop re-resolves the
/// (remapped-on-insert) column ids BY NAME, and `app_shell` skips a root that
/// already carries a "Main Content" child — so the pre-built shell is never
/// double-restructured at finalize.
pub(crate) const SIDEBAR_COLUMN_NAME: &str = "Sidebar";
pub(crate) const CONTENT_COLUMN_NAME: &str = "Main Content";

/// True when the plan is a DESKTOP dashboard with a sidebar subtask plus at
/// least two other sections — the shape worth pre-building as a two-column
/// app-shell. Narrow on purpose (mobile / non-sidebar / single-section plans
/// stay on the single-root path, so parity is unaffected).
pub(crate) fn plan_is_sidebar_dashboard(plan: &OrchestratorPlan, is_mobile: bool) -> bool {
    use crate::dashboard_columns::{
        is_dashboard_content_subtask, is_sidebar_subtask, is_strong_sidebar_subtask,
        plan_has_landing_anatomy,
    };
    if is_mobile || plan.root_frame.width < 900.0 {
        return false;
    }
    let Some(first) = plan.subtasks.first() else {
        return false;
    };
    if !is_sidebar_subtask(first) {
        return false;
    }
    if !is_strong_sidebar_subtask(first) && plan_has_landing_anatomy(plan) {
        return false;
    }
    let sidebars = plan
        .subtasks
        .iter()
        .filter(|s| is_sidebar_subtask(s))
        .count();
    if sidebars == 0 {
        return false;
    }
    // There must be real content for the right column.
    let others = plan.subtasks.len().saturating_sub(sidebars);
    if others < 1 {
        return false;
    }
    // A STRONG sidebar signal ("sidebar"/"rail"/"side nav") is unambiguously a
    // left rail → pre-build the two-column shell regardless of what the content
    // sections are named (a desktop sidebar layout is two-column whether its
    // sections are tables, a directory, a schedule, …). This is the common gap:
    // a sidebar dashboard whose sections lack table/metric/chart keywords used
    // to fall through to the single-root path and fill the sidebar full-width
    // during streaming.
    if is_strong_sidebar_subtask(first) {
        return true;
    }
    // Only an AMBIGUOUS nav/menu signal → require >=2 data-content sections so a
    // landing/marketing page with a stray "Navigation" subtask is never mistaken
    // for a sidebar dashboard.
    plan.subtasks
        .iter()
        .filter(|s| is_dashboard_content_subtask(s))
        .count()
        >= 2
}

/// Build the pre-built two-column app-shell root: `horizontal [Sidebar(260,
/// vertical, clipped) | Main Content(fill, vertical)]`, both columns empty
/// (subtasks fill them). The sidebar `height: fill_container` stretches it to
/// the row (cross-axis) height; `clipContent` keeps a sub-agent's full-width
/// content from bleeding past the 260 column.
#[allow(clippy::too_many_arguments)]
fn build_two_column_root_node(
    root_id: &str,
    name: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill_hex: &str,
    gap: f64,
) -> Result<PenNode, ScaffoldError> {
    let sidebar = serde_json::json!({
        "type": "frame",
        "id": format!("{root_id}-sidebar"),
        "name": SIDEBAR_COLUMN_NAME,
        "width": SIDEBAR_COLUMN_WIDTH,
        "height": "fill_container",
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": fill_hex }],
        "clipContent": true,
        "children": [],
    });
    let content = serde_json::json!({
        "type": "frame",
        "id": format!("{root_id}-content"),
        "name": CONTENT_COLUMN_NAME,
        "width": "fill_container",
        // fill, not fit: the root now presets a full artboard height, and
        // an EMPTY fit_content column collapsed to its padding (a 940x64
        // strip next to a full-height sidebar - user report 2026-07-12).
        // fill stretches the empty column to the artboard from frame one;
        // the height-adjust finalize pass still sizes the root off the
        // column's CONTENT once sections land.
        "height": "fill_container",
        "layout": "vertical",
        "gap": gap,
        // Outer page gutter [vertical, horizontal] so sections don't run
        // edge-to-edge into the viewport — parity with the app-shell reshape
        // path (`app_shell::CONTENT_PADDING`). Without it a section the model
        // authored with no self-padding (KPI row, table) touches the right edge.
        "padding": [32, 40],
        "children": [],
    });
    let frame = serde_json::json!({
        "type": "frame",
        "id": root_id,
        "name": name,
        "x": x,
        "y": y,
        "width": width,
        "height": root_height_json(height, width),
        "layout": "horizontal",
        "gap": 0,
        "alignItems": "stretch",
        "fill": [{ "type": "solid", "color": fill_hex }],
        "children": [sidebar, content],
    });
    serde_json::from_value(frame).map_err(|e| ScaffoldError::TwoColumnRoot {
        root_id: root_id.to_string(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
#[path = "scaffold_tests.rs"]
mod tests;
