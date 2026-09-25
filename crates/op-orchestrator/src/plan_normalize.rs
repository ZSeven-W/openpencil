//! plan 规范化 —— 单屏路径。
//!
//! 行为忠实 TS,但**一次性算分类、干净派生**,不做 TS 那种
//! in-place strip-then-reclassify(`orchestrator.ts:838-845`)。

use crate::dashboard_columns::{
    infer_dashboard_section_height, infer_dashboard_section_width, is_dashboard_like_prompt,
};
use crate::plan::OrchestratorPlan;
use crate::types::DesignRequest;

#[path = "plan_home_intent.rs"]
mod plan_home_intent;
#[path = "plan_normalize_nav.rs"]
mod plan_normalize_nav;
use plan_normalize_nav::{ensure_requested_bottom_nav_subtask, is_bottom_nav_subtask};

#[path = "plan_normalize_side_rail.rs"]
mod plan_normalize_side_rail;

#[path = "plan_normalize_dimensions.rs"]
mod plan_normalize_dimensions;
#[path = "plan_normalize_root_name.rs"]
mod plan_normalize_root_name;

#[path = "plan_normalize_items.rs"]
mod plan_normalize_items;

#[path = "plan_normalize_hero.rs"]
mod plan_normalize_hero;

#[path = "plan_continuation_contract.rs"]
mod plan_continuation_contract;

// multiscreen-fanout-break tests live in a sibling to keep this file below 800 lines.
#[cfg(test)]
#[path = "plan_normalize_screen_groups_tests.rs"]
mod tests_screen_groups;

#[cfg(test)]
#[path = "plan_normalize_nav_tests.rs"]
mod tests_nav;

#[cfg(test)]
#[path = "plan_normalize_dimensions_tests.rs"]
mod tests_dimensions;

#[cfg(test)]
#[path = "plan_normalize_items_tests.rs"]
mod tests_items;

/// 规范化产出的派生信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormInfo {
    /// 根 frame 窄到移动端宽度 —— scaffold 阶段据此注入固定状态栏。
    pub is_mobile: bool,
    /// The request explicitly fixed both root dimensions, so fresh-root
    /// cleanup must preserve the requested height instead of growing it.
    pub preserve_requested_root_height: bool,
    /// A presentation deck. Its boards are fixed 16:9 surfaces, so cleanup
    /// centres their content instead of letting it pile up at the top edge.
    pub is_deck: bool,
}

/// 移动端宽度上限(含)—— ≤ 此值视为移动端单屏。
///
/// Aliases the band the tree-side classifier uses
/// ([`crate::design_type::classify_root_form`]) so the plan layer and the
/// repair layer cannot drift apart on where the phone band ends.
pub(crate) const MOBILE_MAX_WIDTH: f64 = op_design_lint::design_form::MOBILE_MAX_WIDTH;
const MOBILE_DEFAULT_HEIGHT: f64 = 812.0;
pub(crate) const MOBILE_DEFAULT_ROOT_GAP: f64 = 16.0;

/// subtask 的 id / label 命中即视为"状态栏"区块 —— 移动端由
/// scaffold 注入固定状态栏,plan 里若带状态栏 subtask 则剔除。
fn is_status_bar_subtask(id: &str, label: &str) -> bool {
    let hay = format!("{} {}", id.to_lowercase(), label.to_lowercase());
    hay.contains("status bar") || hay.contains("status-bar") || hay.contains("statusbar")
}

fn is_status_bar_fragment(fragment: &str) -> bool {
    let hay = fragment.to_lowercase();
    hay.contains("status bar")
        || hay.contains("status-bar")
        || hay.contains("statusbar")
        || hay.contains("system chrome")
        || hay.contains("os-level indicators")
}

fn strip_status_bar_fragments(text: &str) -> Option<String> {
    let kept: Vec<&str> = text
        .split(',')
        .map(str::trim)
        .filter(|fragment| !fragment.is_empty() && !is_status_bar_fragment(fragment))
        .collect();
    if kept.is_empty() {
        None
    } else {
        Some(kept.join(", "))
    }
}

/// 就地规范化 `plan`:
/// - 一次性判定 `is_mobile`(根 frame 宽度);
/// - 移动端剔除 plan 自带的状态栏 subtask(状态栏改由 scaffold 注入);
/// - 给每个 subtask 赋 `id_prefix = id`、`parent_frame_id = 根 id`;
/// - dashboard-like plan:每个 subtask 的 region 用推断宽高覆写
///   (宽度无条件覆写;高度在 [inferred*0.6, inferred*1.6] 窗口内保留
///   LLM 值,超出则取推断值 —— 忠实 TS `normalizeOrchestratorPlan`
///   `orchestrator.ts:259-272`)。
pub fn normalize(plan: &mut OrchestratorPlan, req: &DesignRequest) -> NormInfo {
    let requested_dimensions_applied =
        plan_normalize_dimensions::apply_requested_root_dimensions(plan, req);
    let continuation_contract_applied = plan_continuation_contract::apply(plan, req);
    let preserve_requested_root_height =
        requested_dimensions_applied || continuation_contract_applied;

    // A deck's board is the projector: 16:9, fixed, and never resized to fit
    // its content. Without this, `adjust_root_height_to_content` grew a cover
    // slide to 1920x2277 (measured 2026-08-02) — the aspect ratio the whole
    // slides contract rests on, gone. `preserve_requested_root_height` already
    // exists for prompt-stated sizes; a deck's height is stated by its design
    // type instead, and deserves the same protection.
    let is_deck = crate::design_type::detect_design_type(&req.prompt).type_
        == crate::design_type::DesignType::Slides;
    if is_deck {
        plan.root_frame.layout = Some("vertical".into());
        // Overwrite rather than fill a hole: a model that plans 1920x0 or
        // 1920x675 is proposing a board that is not 16:9, and the slide is
        // the one shape here that is not up for negotiation.
        let preset = crate::design_type::detect_design_type(&req.prompt);
        plan.root_frame.width = preset.width;
        plan.root_frame.height = preset.root_height;
    }
    // A card board is fixed for the same reason a slide is: 3:4 is the
    // contract the whole card system rests on (`card-system-0808.md` §5), and
    // a model planning 1080x0 hands `adjust_root_height_to_content` a
    // content-height root with the aspect gone. UNLIKE the deck this never
    // overrides an explicitly requested size — the system ships four
    // legitimate specs (3:4 / 1:1 / 公众号封面对 / 9:16) and the request is
    // how a user picks between them.
    let is_card = crate::design_type::detect_design_type(&req.prompt).type_
        == crate::design_type::DesignType::Card;
    if is_card && !requested_dimensions_applied {
        let preset = crate::design_type::detect_design_type(&req.prompt);
        plan.root_frame.layout = Some("vertical".into());
        plan.root_frame.width = preset.width;
        plan.root_frame.height = preset.root_height;
    }
    let preserve_requested_root_height = preserve_requested_root_height || is_deck || is_card;

    let folded_side_progress_rail = plan_normalize_side_rail::fold_side_progress_rail(plan);
    tracing::info!(
        count = folded_side_progress_rail,
        "plan normalization folded side progress rail subtasks"
    );

    let is_mobile = plan.root_frame.width <= MOBILE_MAX_WIDTH;

    if is_mobile {
        plan.root_frame.layout = Some("vertical".into());
        if plan.root_frame.gap.unwrap_or(0.0) <= 0.0 {
            plan.root_frame.gap = Some(MOBILE_DEFAULT_ROOT_GAP);
        }
        plan.root_frame.padding = Some(0.0);
        if plan.root_frame.height <= 0.0 {
            plan.root_frame.height = MOBILE_DEFAULT_HEIGHT;
        }
        plan.subtasks
            .retain(|st| !is_status_bar_subtask(&st.id, &st.label));
        for st in &mut plan.subtasks {
            if let Some(elements) = st.elements.as_deref() {
                st.elements = strip_status_bar_fragments(elements);
            }
            if is_bottom_nav_subtask(st) {
                st.region.width = plan.root_frame.width;
                st.region.height = 78.0;
            }
        }
        ensure_requested_bottom_nav_subtask(plan, req);
    }

    let root_width = plan.root_frame.width;
    let dashboard_like = is_dashboard_like_prompt(&req.prompt, plan);

    let root_id = plan.root_frame.id.clone();

    // Distinct screen labels get distinct placeholder roots; zero labels or a
    // single shared label retain the original single-root assignment.
    let groups = crate::screen_groups::group_subtasks_by_screen(&plan.subtasks);
    if groups.len() > 1 {
        for group in &groups {
            let group_root_id = format!("{root_id}-{}", group.screen);
            for &idx in &group.indices {
                if let Some(st) = plan.subtasks.get_mut(idx) {
                    st.parent_frame_id = Some(group_root_id.clone());
                }
            }
        }
    } else {
        for st in &mut plan.subtasks {
            st.parent_frame_id = Some(root_id.clone());
        }
    }

    for st in &mut plan.subtasks {
        st.id_prefix = st.id.clone();

        // Continuation regions describe complete sibling artboards, not
        // dashboard sections inside one root. Their live-canvas contract is
        // authoritative and must not be shrunk again by section heuristics.
        if dashboard_like && !continuation_contract_applied {
            let inferred_width = infer_dashboard_section_width(st, root_width);
            let inferred_height = infer_dashboard_section_height(st);

            st.region.width = inferred_width;

            if st.region.height <= 0.0 {
                st.region.height = inferred_height;
            } else {
                let min_height = (inferred_height * 0.6).round();
                let max_height = (inferred_height * 1.6).round();
                st.region.height = f64::max(min_height, f64::min(st.region.height, max_height));
            }
        }
    }

    // Repeated item-family bundling is gated to the deepseek model family and
    // runs last so the merged subtask inherits normalized fields.
    plan_normalize_items::bundle_repeated_item_families(plan, req.model.as_deref().unwrap_or(""));
    plan_normalize_hero::mark_bleed_hero_subtasks(plan);
    // Last, so the name heuristics above (home-screen detection reads the
    // root name) see the planner's own words.
    plan_normalize_root_name::localize_root_name(plan, req);

    NormInfo {
        is_mobile,
        preserve_requested_root_height,
        is_deck,
    }
}

#[cfg(test)]
#[path = "plan_normalize_tests.rs"]
mod tests;
