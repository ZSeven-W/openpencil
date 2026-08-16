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
use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::types::DocSink;
use jian_ops_schema::node::{
    container::{AlignItems, ContainerProps, JustifyContent, LayoutMode, Padding},
    PenNode,
};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::PenEffect;
use op_editor_core::{
    fills::node_stroke_width, first_fill_type, first_solid_fill_hex, EditorCommand, EditorState,
    FillType, LayoutPropValue, NodeId, PenNodeExt,
};

#[path = "cleanup_desktop_dashboard.rs"]
mod cleanup_desktop_dashboard;
#[path = "cleanup_mobile_chrome.rs"]
mod cleanup_mobile_chrome;
#[path = "cleanup_root_patches.rs"]
mod cleanup_root_patches;
pub(crate) use cleanup_mobile_chrome::{
    anchor_bottom_nav_last_for_all_roots, repair_mobile_structural_chrome_for_all_roots,
};
#[path = "cleanup_mobile_dense.rs"]
mod cleanup_mobile_dense;

// Repair-pass submodules: this file keeps the public surface (`finalize_design`
// / `run_cleanup_passes` / the `*_for_all_roots` drivers) plus the shared
// predicates; each repair family lives in its own file and is re-imported here
// so the drivers (and the test modules mounted below) see the same flat
// namespace as before.
#[path = "cleanup_bottom_nav_repairs.rs"]
mod cleanup_bottom_nav_repairs;
#[path = "cleanup_clip_row_stroke.rs"]
mod cleanup_clip_row_stroke;
#[path = "cleanup_container_geometry.rs"]
mod cleanup_container_geometry;
#[path = "cleanup_equalize_siblings.rs"]
mod cleanup_equalize_siblings;
#[path = "cleanup_image_slots.rs"]
mod cleanup_image_slots;
#[path = "cleanup_root_and_nav.rs"]
mod cleanup_root_and_nav;
#[path = "cleanup_root_transform.rs"]
mod cleanup_root_transform;
#[path = "cleanup_section_margins.rs"]
mod cleanup_section_margins;
#[path = "cleanup_section_sizing.rs"]
mod cleanup_section_sizing;
#[path = "cleanup_slide_padding.rs"]
mod cleanup_slide_padding;

use cleanup_bottom_nav_repairs::*;
use cleanup_clip_row_stroke::*;
use cleanup_container_geometry::*;
use cleanup_equalize_siblings::*;
use cleanup_image_slots::*;
use cleanup_root_and_nav::*;
use cleanup_root_patches::*;
use cleanup_root_transform::*;
use cleanup_section_margins::*;
use cleanup_section_sizing::*;
use cleanup_slide_padding::*;

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

/// Explicit status-bar role or an English/Chinese name/id match identifies
/// status-bar chrome. The role path keeps generated/custom-named bars stable;
/// Chinese aliases cover direct local edits such as "顶部状态栏".
pub(crate) fn is_status_bar(node: &PenNode) -> bool {
    if node
        .base()
        .role
        .as_deref()
        .is_some_and(|role| role.eq_ignore_ascii_case("status-bar"))
    {
        return true;
    }
    let name = node.base().name.as_deref().unwrap_or("").to_lowercase();
    let id = node.id_str().to_lowercase();
    let hay = format!("{id} {name}");
    hay.contains("status bar")
        || hay.contains("status-bar")
        || hay.contains("statusbar")
        || hay.contains("状态栏")
        || hay.contains("系统栏")
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

pub(crate) fn distribute_bottom_nav_tabs_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        distribute_bottom_nav_tabs(sink, &root_id);
    }
}

pub(crate) fn collapse_nested_horizontal_padding_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        collapse_nested_horizontal_padding(sink, &root_id);
    }
}

pub(crate) fn expand_absolute_container_to_children_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        expand_absolute_container_to_children(sink, &root_id);
    }
}

pub(crate) fn pad_clipping_horizontal_row_for_stroke_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        pad_clipping_horizontal_row_for_stroke(sink, &root_id);
    }
}

pub(crate) fn collapse_fill_container_content_sections_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        collapse_fill_container_content_sections(sink, &root_id);
    }
}

pub(crate) fn equalize_horizontal_card_heights_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        equalize_horizontal_card_heights(sink, &root_id);
    }
}

pub(crate) fn is_bottom_nav_section(node: &PenNode) -> bool {
    cleanup_mobile_chrome::bottom_nav_surface_target(node, false).is_some()
}

/// Position-gated bottom-nav recognition for a root's literal last child.
///
/// Unlike [`is_bottom_nav_section`], this may use the strict structural
/// fallback (3-5 icon+label tabs). Keeping that fallback at the call site that
/// has already proven "last mobile root child" avoids turning an ordinary top
/// tab row into bottom chrome.
pub(crate) fn is_trailing_bottom_nav_section(node: &PenNode) -> bool {
    cleanup_mobile_chrome::bottom_nav_surface_target(node, true).is_some()
        || cleanup_mobile_chrome::is_pencil_trailing_tab_section(node)
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

/// Find a node by id anywhere in the active-page tree (recursive).
///
/// Append-mode generation nests the new root under an existing target
/// frame rather than placing it at the top level, so a top-level-only
/// scan would miss it (Component 11c).
fn find_root<'a>(state: &'a EditorState, root_id: &str) -> Option<&'a PenNode> {
    op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(root_id.to_string()))
}

/// Cleanup intent that exists outside the generated document tree.
///
/// The default deliberately preserves the historical cleanup behavior. Only
/// the fresh-root orchestrator path may opt into the request-derived height
/// contract; append, section, and whole-document loop finalization stay false.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CleanupPolicy {
    pub(crate) preserve_requested_root_height: bool,
    /// The REQUEST asked for a deck (prompt keywords, via
    /// `plan_normalize::NormInfo`). Deck boards are fixed 16:9 surfaces, so
    /// their content is centred rather than left to stack from the top edge,
    /// which on a 1080-tall board reads as a half-empty slide.
    ///
    /// This flag is only HALF the judgement — see [`root_is_deck_board`], the
    /// geometric half, which is unioned in at the point of use and covers
    /// every path that has no prompt to read.
    pub(crate) is_deck: bool,
    /// `root_ids` are roots THIS RUN produced, not pre-existing content the
    /// user may have arranged by hand.
    ///
    /// Gates the geometric half of the deck judgement only. Centring a board
    /// is an intent-tier move: an asymmetric composition can be exactly what
    /// the author wanted (a 70/30 board is good design, not a top-stacked
    /// defect), and the "explicit `justifyContent`" guard does not catch an
    /// author who simply placed their content and never set a distribution.
    /// The prompt half is deliberately NOT gated on this — a user who typed
    /// "PPT" stated the intent themselves.
    ///
    /// Defaults to `false` so a caller that cannot prove provenance gets the
    /// safe answer. The orchestrator's fresh and append paths both pass their
    /// own inserted root ids and set it; `loop_finalize` passes every
    /// top-level root in the document and cannot, so it leaves it false.
    pub(crate) roots_are_run_output: bool,
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
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(sink, plan, root_ids, &mut summary);
}

/// [`finalize_design`] that also reports what it checked and repaired, for the
/// user-facing quality credential. Identical behaviour — the summary is a
/// pure measurement of the same passes (see `crate::repair_summary`).
pub fn finalize_design_with_summary(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    root_ids: &[&str],
    summary: &mut RepairSummary,
) {
    run_cleanup_passes_with_summary(sink, plan, root_ids, summary);
}

/// Fresh-root orchestrator variant carrying request-derived cleanup intent.
pub(crate) fn finalize_design_with_summary_and_policy(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    root_ids: &[&str],
    summary: &mut RepairSummary,
    policy: CleanupPolicy,
) {
    run_cleanup_passes_with_summary_and_policy(sink, plan, root_ids, summary, policy);
}

pub fn run_cleanup_passes(sink: &mut dyn DocSink, plan: &OrchestratorPlan, root_ids: &[&str]) {
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(sink, plan, root_ids, &mut summary);
}

/// The real cleanup driver. `summary` accumulates what each contiguous group
/// of passes checked and how many document edits it applied — see
/// `crate::repair_summary` for what one "repair" counts as. The measurement
/// is non-invasive: `sink` is shadowed once by a counting decorator and the
/// `counter.checkpoint(...)` lines between pass groups are the only additions
/// to what is otherwise the unchanged pass sequence.
pub fn run_cleanup_passes_with_summary(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    root_ids: &[&str],
    summary: &mut RepairSummary,
) {
    run_cleanup_passes_with_summary_and_policy(
        sink,
        plan,
        root_ids,
        summary,
        CleanupPolicy::default(),
    );
}

/// Test-only alias so policy-dependent passes can be driven directly.
#[cfg(test)]
pub(crate) fn run_cleanup_passes_with_summary_and_policy_for_tests(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    root_ids: &[&str],
    summary: &mut RepairSummary,
    policy: CleanupPolicy,
) {
    run_cleanup_passes_with_summary_and_policy(sink, plan, root_ids, summary, policy);
}

fn run_cleanup_passes_with_summary_and_policy(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    root_ids: &[&str],
    summary: &mut RepairSummary,
    policy: CleanupPolicy,
) {
    let mut counter = RepairCounter::new();
    let mut counting = counter.wrap(sink);
    let sink: &mut dyn DocSink = &mut counting;

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
    counter.checkpoint(summary, CheckCategory::Structure, "duplicate-root-dedupe");

    // Doc-global (not per-root): heal theme-polarity splits in the variable
    // table BEFORE the per-root passes, so every pass that resolves `$refs`
    // (surface discipline, geometry text fills) sees the repaired palette.
    intent_theme_variable_polarity(sink, summary);
    counter.checkpoint(summary, CheckCategory::Palette, "theme-variable-polarity");
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
        counter.checkpoint(summary, CheckCategory::Structure, "app-shell+table-regroup");
        // Gap-less table rows → column gap (weak models omit it → columns touch,
        // "SPEND"+"STATUS" reads as "SPENDSTATUS").
        rid = apply_root_transform(sink, &rid, crate::table_repair::ensure_table_column_gap);
        debug_probe_child_height(sink, &rid, "table_gap");
        // Row gap on the table CONTAINER (rows already zebra'd / hairlined) →
        // flush rows, reference-grade rhythm comes from the rows themselves.
        rid = apply_root_transform(sink, &rid, crate::table_repair::flush_table_row_gap);
        counter.checkpoint(summary, CheckCategory::Layout, "table-gap");
        // Card-level "-35%" tags meant for the image corner → adopt into the
        // image wrapper as an absolute 8,8 overlay.
        rid = apply_root_transform(sink, &rid, crate::chip_repair::adopt_corner_badges);
        // [bell icon, 8px square] flow pairs → round the dot and pin it on
        // the icon's top-right corner.
        rid = apply_root_transform(sink, &rid, crate::chip_repair::adopt_notification_dots);
        // A progress ring's track + progress arc authored as FLEX SIBLINGS of a
        // general container (a padded card, a section with its own heading) →
        // extract them into a dedicated concentric `layout:none` wrapper.
        // `radial_repair` below only converts a parent that IS the ring's own
        // wrapper, so without this the arcs stay in flow and render as two
        // circles side by side. Runs HERE — a structural restructure, before
        // the geometry loop and before `repair_radial_stacks`, which then
        // finishes the wrapper's concentric geometry against resolved rects.
        rid = apply_root_transform(sink, &rid, crate::ring_repair::wrap_ring_fragments);
        debug_probe_child_height(sink, &rid, "table_flush");
        counter.checkpoint(summary, CheckCategory::Structure, "chip+ring-extract");
        // Chip/badge text contrast (DS P1-a): the specific, provable chip
        // branch runs BEFORE the generic contrast repair so the chip-scoped
        // proof (solid chip fill, chip shape) wins the repair and the generic
        // pass then sees the fixed text as already readable.
        crate::text_contrast_repair::repair_chip_text_contrast(sink, &rid);
        counter.checkpoint(summary, CheckCategory::Layout, "chip-text-contrast");
        // Inter-section gap the planner would have set. Runs BEFORE the
        // wrapper-inset pass below, which keys off the parent column's gaps
        // (`>= 12`) — repairing the gap first hands both paths the same column.
        patch_root_section_gap(sink, &rid);
        debug_probe_child_height(sink, &rid, "root_gap");
        if policy.is_deck || (policy.roots_are_run_output && root_is_deck_board(sink, &rid)) {
            centre_deck_board_content(sink, &rid);
        }
        // Text that resolves to ~1:1 against its own background is not
        // styled, it is missing. The lint crate has detected this since
        // 2026-05, but the generation path called exactly one of its
        // detectors, so it only ever fired for a user running
        // `lint_document` by hand.
        crate::text_contrast_repair::repair_text_contrast(sink, &rid);
        // Section-margin ownership (DS P1.5) runs BEFORE the wrapper-double-inset
        // stripper below: unifying first hands the stripper the group already
        // normalized, and the floor afterwards then sees no flush content left.
        unify_transparent_section_margins(sink, &rid);
        counter.checkpoint(summary, CheckCategory::Layout, "unify-section-margins");
        // Transparent wrapper padding inside an already-padded/gapped column →
        // double inset: misaligned section edges + starved children (a padded
        // "Key Metrics" strip squeezed its KPI cards until label touched icon).
        rid = strip_wrapper_double_inset_if_intent(sink, &rid);
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
        counter.checkpoint(summary, CheckCategory::Layout, "spacing+footer-sink");
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
        counter.checkpoint(summary, CheckCategory::Structure, "chrome-dedupe");
        distribute_bottom_nav_tabs(sink, rid);
        collapse_nested_horizontal_padding(sink, rid);
        expand_absolute_container_to_children(sink, rid);
        pad_clipping_horizontal_row_for_stroke(sink, rid);
        equalize_horizontal_card_heights(sink, rid);
        collapse_fill_container_content_sections(sink, rid);
        counter.checkpoint(summary, CheckCategory::Layout, "container-geometry");
        repair_light_mobile_nav_surfaces(sink, rid);
        counter.checkpoint(summary, CheckCategory::Palette, "light-mobile-nav-surface");
        cleanup_mobile_chrome::repair_mobile_structural_chrome(sink, rid);
        // Normalize the late-nav construction shell before geometry validation
        // can grow the root around the stale numeric wrapper and erase the
        // evidence that the wrapper used to consume the entire viewport.
        cleanup_mobile_chrome::anchor_bottom_nav_last(sink, rid);
        crate::mobile_content_rail::repair_mobile_content_rails(sink, rid);
        // The newly-established section rail may expose a redundant transparent
        // inner wrapper carrying the same horizontal inset. Re-run the
        // existing ownership collapse after rail repair so only one layer owns
        // the gutter.
        collapse_nested_horizontal_padding(sink, rid);
        // …and re-run the double-inset stripper for the same reason. Its first
        // run above sits before the mobile chrome / content-rail passes, so a
        // section that only BECOMES a padded rail there was still unpadded when
        // it was checked, and any transparent wrapper under it kept its own
        // gutter. `collapse_nested_horizontal_padding` above cannot cover the
        // gap: it only fires on a rail whose wrapper is its ONLY child, so a
        // section holding the wrapper plus any sibling (a tab-bar spacer, a
        // second module) stayed double-inset — measured on `0727-1-gm`, where
        // the wrapped card came out 279px wide against 327px siblings.
        let rid_owned = strip_wrapper_double_inset_if_intent(sink, rid);
        let rid = rid_owned.as_str();
        crate::mobile_reflow::repair_mobile_trailing_nav_reflow_for_root_in_sink(sink, rid);
        cleanup_mobile_dense::repair_dense_mobile_rows(sink, rid);
        cleanup_desktop_dashboard::repair_sparse_desktop_dashboard_rows(sink, plan, rid);
        counter.checkpoint(summary, CheckCategory::Layout, "mobile-chrome+content-rail");
        repair_overbold_text_hierarchy(sink, rid);
        strip_decorative_filled_strokes(sink, rid);
        counter.checkpoint(summary, CheckCategory::Hierarchy, "text-hierarchy+strokes");
        crate::radial_repair::repair_radial_stacks(sink, rid);
        crate::stub_repair::remove_empty_decorated_stubs(sink, rid);
        // A section's header row whose second child is the ENTIRE content
        // body (not a chevron/badge), with the body redundantly repeating
        // the header's own title as its own first child — the loop's
        // freeform fill step drew a title the section already had and
        // misnested the body as the header's flex sibling instead of the
        // header's own sibling. Runs before geometry validation so it sees
        // the corrected tree, not the pre-repair one.
        crate::section_shell_fill_repair::repair_section_shell_fill_ownership(sink, rid);
        counter.checkpoint(summary, CheckCategory::Structure, "radial+stub+shell");
        // Weak-model "image slots" authored as a childless frame/rect with one
        // still-empty image fill become real Image nodes BEFORE the geometry
        // passes below, so the slot resolves and validates like any other.
        materialize_empty_image_fill_slots(sink, rid);
        counter.checkpoint(summary, CheckCategory::Structure, "materialize-image-slots");
        // Sibling-item scalar alignment (DS P1-a) runs AFTER slot
        // materialization: an empty image-slot rect becomes an Image node
        // above, so the structure comparison below sees the FINAL tree shape
        // instead of treating the not-yet-materialized slot as drift.
        equalize_sibling_items(sink, rid);
        counter.checkpoint(summary, CheckCategory::Structure, "equalize-sibling-items");
        // No-nav mobile screens share one deterministic closing contract:
        // 24-32px of bottom room. The repair reads the same resolved geometry
        // as the diagnostic and grows only root padding, never business nodes.
        crate::geometry_validation::repair_mobile_bottom_breathing(sink, rid);
        counter.checkpoint(summary, CheckCategory::Layout, "mobile-bottom-breathing");
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
        let preserve_root_height = policy.preserve_requested_root_height
            || find_root(sink.state(), rid).is_some_and(|root| {
                root_has_explicit_fit_content_height(root)
                    || has_explicit_mobile_viewport_contract(root)
                    || crate::mobile_reflow::has_mobile_trailing_nav_reflow_contract(root)
            });
        if preserve_root_height {
            let mut guarded = PreserveRootHeightSink {
                inner: sink,
                root_id: rid,
            };
            crate::geometry_validation::geometry_validate_and_fix(&mut guarded, rid);
        } else {
            crate::geometry_validation::geometry_validate_and_fix(sink, rid);
        }
        debug_probe_child_height(sink, rid, "geometry");
        counter.checkpoint(summary, CheckCategory::Overflow, "geometry-validation");
        // Deck/card safe-margin floor (P1-a pass 3 + P1.5 card gate, and the
        // P2-b card vertical floor): AFTER geometry, evidence re-parsed from
        // the current tree, so it stands down once section margins pulled
        // content off the edge. The hook also runs the board text wrap
        // (P2-c B) on the settled margins, checkpointed under Overflow.
        enforce_slide_padding_floor_and_board_text_wrap(sink, rid, summary, &mut counter);
        // Card trailing-void centre (DS P2-b B): after the floor, on settled
        // margins; deck centring stays mounted earlier.
        centre_card_board_content(sink, rid);
        counter.checkpoint(summary, CheckCategory::Layout, "card-board-centre");
        // Geometry validation can flip a repaired radial wrapper to
        // `fill_container`; re-centre against the final resolved bounds so
        // arc/label coordinates do not drift off-centre.
        crate::radial_repair::repair_radial_stacks(sink, rid);
        adjust_root_height_to_content(sink, rid, preserve_root_height);
        debug_probe_child_height(sink, rid, "adjust_root_height");
        counter.checkpoint(summary, CheckCategory::Layout, "radial+root-height");
    }

    crate::avatar_repair::repair_avatar_slots_for_all_roots(sink);

    // LAST: structural chrome contract — bottom nav is the mobile root's
    // final child. Runs AFTER the per-root passes (incl. bottom-nav dedup,
    // which keeps the bottom-most duplicate: anchoring first would reorder
    // duplicates and flip which one dedup keeps). A late "catch-up" section
    // appended after the nav is repaired by moving the nav back to the end;
    // where that section belongs is intent — the geometry echo handles it.
    anchor_bottom_nav_last_for_all_roots(sink);
    counter.checkpoint(summary, CheckCategory::Structure, "avatar+nav-anchor");
    crate::mobile_reflow::repair_mobile_trailing_nav_reflow_in_sink(sink);
    counter.checkpoint(summary, CheckCategory::Layout, "mobile-trailing-nav-reflow");

    // Multi-screen root position deconfliction: screen-shaped top-level
    // roots that overlap because one or more never got a canvas position
    // (loop path model didn't call `find_empty_space`, or any other
    // producer skipped positioning) get spread into a left-to-right row.
    // Runs BEFORE `unify_shared_nav` below — a document whose screens are
    // still stacked on top of each other at diagnosis time reads (from a
    // canvas screenshot) like duplicate nav bars crammed into one frame,
    // but the roots are already correctly separated; they just need to
    // stop overlapping before the passes below reason about "which screen
    // owns which nav".
    crate::spread_screen_roots::spread_overlapping_screen_roots(sink);

    // Cross-screen shared-chrome unification: each screen's independently
    // re-generated bottom-nav drifts in icons/labels (measured: Home screen
    // "Home/Search/Library/Premium" vs Library screen's own redraw
    // "Home/Search/Your Library/Premium"). Runs BEFORE `wire_screen_
    // navigation` below so Track A's label↔screen tab-matching sees the
    // POST-unification tree (every screen sharing one tab-label set), not a
    // stale per-screen one.
    crate::unify_shared_nav::unify_shared_nav(sink);

    // Sibling pass to the above: screens missing the pinned status bar
    // entirely (measured: 0718-1-k3-1 — two of three screens had no
    // status-bar subtree at all) get one cloned in from whichever screen
    // already has it. Same "reuse, don't redraw" shape, same shared choke
    // point, so both the classic and loop-finalize paths pick it up.
    crate::unify_shared_status_bar::unify_shared_status_bar(sink);

    // Establish final screen routes first. The cleanup-only semantic pass can
    // then persist only fact-proven back/card interactions against those real
    // routes, before the label-matching nav fallback. Keeping the semantic pass
    // outside public `wire_screen_navigation` prevents Cmd+P's cloned-state
    // fallback from creating preview-only interactions that never reach the
    // saved document.
    crate::wire_screen_navigation::ensure_screen_routes(sink);
    crate::geometry_validation::wire_interaction_backfill(sink);

    // Track A fallback: wire bottom-nav/sidebar tabs after final chrome shape
    // and semantic interactions are settled. Whole-doc (scans `sink.state()`,
    // not `root_ids`) so it also links pre-existing screens from earlier turns.
    crate::wire_screen_navigation::wire_screen_navigation(sink);
    counter.checkpoint(summary, CheckCategory::Structure, "shared-chrome+nav");
}

#[cfg(test)]
#[path = "cleanup_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cleanup_repair_summary_tests.rs"]
mod tests_repair_summary;

#[cfg(test)]
#[path = "cleanup_repair_tier_tests.rs"]
mod tests_repair_tier;

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
#[path = "cleanup_bottom_nav_tests.rs"]
mod tests_bottom_nav;

#[cfg(test)]
#[path = "cleanup_nested_horizontal_padding_tests.rs"]
mod tests_nested_horizontal_padding;

#[cfg(test)]
#[path = "cleanup_rail_wrapper_gutter_tests.rs"]
mod tests_rail_wrapper_gutter;

#[cfg(test)]
#[path = "cleanup_absolute_container_tests.rs"]
mod tests_absolute_container;

#[cfg(test)]
#[path = "cleanup_fill_container_content_tests.rs"]
mod tests_fill_container_content;

#[cfg(test)]
#[path = "cleanup_clip_row_stroke_tests.rs"]
mod tests_clip_row_stroke;

#[cfg(test)]
#[path = "cleanup_image_slots_tests.rs"]
mod cleanup_image_slots_tests;

#[cfg(test)]
#[path = "cleanup_card_height_equalize_tests.rs"]
mod tests_card_height_equalize;

#[cfg(test)]
#[path = "cleanup_desktop_dashboard_tests.rs"]
mod tests_desktop_dashboard;

#[cfg(test)]
#[path = "cleanup_deck_geometry_tests.rs"]
mod tests_deck_geometry;
