//! 阶段 3 —— 单个 sub-agent 的顺序执行。
//!
//! 一个 subtask:构 prompt → 调 `LlmClient` → 收集文本 → 解析成
//! `PenNode` 树 → 经 `DocSink` 发一条 `InsertSubtree`。
//!
//! 返回的 [`SubtaskOutcome`] 用 `node_count` 区分(见 spec §6.2):
//! - `node_count == 0` —— 零节点失败,调用方应停止后续 subtask;
//! - `node_count > 0`(`error` 可带软错误)—— 部分产出,继续后续。

use crate::plan::{OrchestratorPlan, Subtask};
use crate::prompt::build_subagent_prompt_with_screen_routes;
use crate::types::{AbortFlag, DesignRequest, DocSink, LlmChunk, LlmClient, SubtaskOutcome};
use futures::StreamExt;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use std::time::{SystemTime, UNIX_EPOCH};

/// 执行一个 subtask。总是返回 [`SubtaskOutcome`];调用方据
/// `node_count` 决定继续/停止。
///
/// * `reduced_complexity` — Narrow the skill set to the `retryAllowed`
///   8-skill set when the model is Basic tier.  Pass `false` for the
///   first attempt; pass `true` on the second attempt of the retry
///   ladder (Task C3).
/// * `minimal_skills` — Strip the skill set to only `schema`; the output
///   protocol remains script-gen via `SCRIPT_FORMAT`. Pass `false` for the
///   first two attempts; pass `true` on the third attempt (Task C3).
#[allow(clippy::too_many_arguments)]
pub async fn run_subtask(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
) -> SubtaskOutcome {
    run_subtask_with_reveal_at(
        subtask,
        plan,
        req,
        llm,
        sink,
        abort,
        reduced_complexity,
        minimal_skills,
        None,
        reveal_now_millis(),
        None,
    )
    .await
}

/// Like [`run_subtask`] but accepts an optional progress sink that receives
/// [`crate::types::Progress::SubtaskSkills`] immediately after the prompt is built.
#[allow(clippy::too_many_arguments)]
pub async fn run_subtask_with_progress(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    on_progress: Option<&mut dyn FnMut(crate::types::Progress)>,
) -> SubtaskOutcome {
    run_subtask_with_reveal_at(
        subtask,
        plan,
        req,
        llm,
        sink,
        abort,
        reduced_complexity,
        minimal_skills,
        None,
        reveal_now_millis(),
        on_progress,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_subtask_with_reveal_at(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    indicator_epoch: Option<u64>,
    reveal_started_ms: u64,
    on_progress: Option<&mut dyn FnMut(crate::types::Progress)>,
) -> SubtaskOutcome {
    let fail = |msg: String| SubtaskOutcome {
        id: subtask.id.clone(),
        node_count: 0,
        error: Some(msg),
        inserted_root_ids: Vec::new(),
        // Persist the spec on every zero-node failure so the progress
        // panel's manual "Retry" button (see `crate::retry_subtask`) can
        // re-run this EXACT subtask later.
        subtask: Some(subtask.clone()),
    };

    // Snapshot document-wide prompt context before building the prompt.
    // Classic fan-out derives routes from normalized planning groups; loop
    // continuation (whose synthetic plan has no screen labels) falls back to
    // live screen markers. Both paths share navigation's route allocator.
    let screen_routes =
        crate::wire_screen_navigation::prompt_screen_route_inventory(plan, sink.state());
    let components = sink.state().components.clone();

    // 收集 LLM 文本输出。
    let (call_req, skill_report) = build_subagent_prompt_with_screen_routes(
        subtask,
        plan,
        req,
        abort.clone(),
        reduced_complexity,
        minimal_skills,
        &components,
        &screen_routes,
    );
    // Surface the per-subtask skill-load report to the chat UI immediately
    // after the prompt is built (spec Component 4).
    if let Some(cb) = on_progress {
        let (included, dropped, budget_used, budget_max) =
            crate::types::report_to_progress_parts(&skill_report);
        cb(crate::types::Progress::SubtaskSkills {
            id: subtask.id.clone(),
            included,
            dropped,
            budget_used,
            budget_max,
        });
    }
    tracing::debug!(subtask = %subtask.id, model = req.model.as_deref().unwrap_or(""), "subagent LLM call");
    let mut stream = llm.call(call_req);
    let mut text = String::new();
    let mut thinking_len = 0usize;
    while let Some(item) = stream.next().await {
        match item {
            Ok(LlmChunk::Text(t)) => text.push_str(&t),
            // Thinking models (glm-5.2 etc.) stream reasoning here. We don't
            // parse it, but track its size: a large reasoning blob with an
            // empty `text` is the classic "no JSON found" failure mode — the
            // model spent its budget thinking and never emitted the answer.
            Ok(LlmChunk::Thinking(t)) => thinking_len += t.len(),
            Err(e) => {
                return fail(if e.aborted {
                    "aborted".into()
                } else {
                    e.message
                });
            }
        }
    }
    tracing::debug!(
        subtask = %subtask.id,
        text_len = text.len(),
        thinking_len,
        "subagent text collected"
    );

    // Script-gen is THE protocol on every subagent rung. Reduced/minimal
    // retries narrow the skill set only; they never switch to flat JSONL.
    // `program_state` carries any doc-root `state` script-gen's underlying
    // `run_program_to_forest` hoisted on the SCRATCH document it builds the
    // forest against (see `program_gen`'s module doc).
    let (mut nodes, program_state) = match crate::script_gen::parse_script(&text) {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(
                subtask = %subtask.id,
                text_len = text.len(),
                thinking_len,
                raw = %text,
                "subagent script-gen parse failed"
            );
            return fail(e.to_string());
        }
    };
    if is_blank_container_forest(&nodes) {
        return fail("blank container root produced no content nodes".into());
    }
    // Semantic role inference + role-default injection (P2 I1/I2) on the parsed
    // subtree, BEFORE the fallback sizing normalize (semantic-before-fallback,
    // memory feedback_post_processing_order). Canvas width + theme come from the
    // plan's root frame — the page background drives light/dark default colors.
    let canvas_width = plan.root_frame.width;
    let theme = {
        let first_solid = plan
            .root_frame
            .fill
            .as_ref()
            .and_then(|fills| {
                fills
                    .iter()
                    .find(|f| f.kind == "solid" || f.kind.is_empty())
            })
            .map(|f| f.color.as_str())
            .filter(|c| !c.is_empty());
        crate::role_defaults::detect_theme_from_fill(first_solid)
    };
    // Weak-model deterministic floor (runs BEFORE role resolution so roles land
    // on the cleaned forest): a subtask is ONE section, but weak models split it
    // into sibling pieces — an empty wrapper + the content/leaves that belong
    // inside it. Reparent those back so they don't normalize into separate
    // full-width page bands (empty banner + floating invisible text, etc.).
    coalesce_subtask_section(&mut nodes);
    crate::role_infer::resolve_forest_roles(&mut nodes, canvas_width, theme);
    // Cross-node contrast post-pass (I3) runs AFTER role resolution (it keys off
    // the roles I1/I2 set) and before the fallback sizing normalize.
    // Repair-tier policy for this document (see `crate::repair_tier`): the
    // intent-tier passes below defer to authored template input. Read off the
    // sink's state so this path and the agentic loop reach the same answer.
    let tier = crate::repair_tier::RepairTierPolicy::for_document(sink.state());
    // The board/page/screen these sections will sit on — taken from the PLAN's
    // root frame, which is the artboard; the forest here is its content.
    let root_form = crate::design_type::classify_root_form(
        Some(plan.root_frame.width),
        Some(plan.root_frame.height),
    );
    let deck_echoes = crate::role_post_pass::post_pass_forest_with_tier(
        &mut nodes,
        canvas_width,
        &tier,
        root_form,
    );
    // A subtask has no `RepairSummary` to note against (it reports through
    // `SubtaskOutcome`, which counts nodes, not repairs), so the log is the
    // channel here. The whole-document finalize path echoes the same
    // violations into the user-visible summary.
    for echo in &deck_echoes {
        tracing::warn!(subtask = %subtask.id, echo = %echo.line(), "deck geometry left unrepaired");
    }
    // Promote explicitly-marked role frames to first-class widget nodes.
    // Must run AFTER post_pass_forest (which keys on `role` to set defaults)
    // and BEFORE variable binding (which resolves hex refs — widgets produced
    // here carry the same fill/stroke props that binding normalises).
    jian_ops_schema::promote::promote_forest(&mut nodes);
    // Post-streaming tree heuristics (TS applyPostStreamingTreeHeuristics parity):
    // nav-surface anchor, redundant section-fill strip, nested-card decoration
    // strip. Runs BEFORE binding — the section-fill strip matches literal hedge
    // HEX that binding would convert to refs. Each forest root is a page-root
    // child (a section).
    let page_bg = plan.root_frame.first_solid_hex();
    // Dominant brand accent from the already-assembled page (prior subtasks live
    // in the sink) — drives the invisible-band fill so it matches the screen's
    // real accent (often a chart token) instead of a clashing palette default.
    let prior_accent = {
        let st = sink.state();
        let roots = st
            .doc
            .pages
            .as_ref()
            .and_then(|p| p.get(st.ui.active_page_index))
            .map(|pg| pg.children.as_slice())
            .unwrap_or(st.doc.children.as_slice());
        crate::tree_heuristics::dominant_design_accent(roots)
    };
    if tier.runs_pass(crate::repair_tier::TieredPass::TreeHeuristics) {
        crate::tree_heuristics::apply_tree_heuristics(
            &mut nodes,
            page_bg.as_deref(),
            theme == crate::role_defaults::Theme::Light,
            prior_accent.as_deref(),
        );
    }
    if tier.runs_pass(crate::repair_tier::TieredPass::VariableBinding) {
        crate::variable_binding::bind_generated_color_variables(&mut nodes, sink.state());
    }
    // Surface-color discipline runs AFTER binding: glm emits raw hex, and binding
    // is what turns it into the `$color-danger-bg` / `$color-bg-deep` refs this
    // pass matches (recolor misused state-bg surfaces → neutral, strip the
    // page-bg token off inner wrappers). Pre-binding it only saw hex and missed.
    crate::role_post_pass::enforce_surface_color_discipline_with_tier(&mut nodes, &tier);
    normalize_section_roots_for_parent_layout(&mut nodes);
    let self_check = crate::orchestration_self_check::check_generated_nodes(&nodes, canvas_width);
    if self_check.has_fatal() {
        let fixed =
            crate::orchestration_self_check::auto_fix_fixable_issues(&mut nodes, canvas_width);
        let recheck = crate::orchestration_self_check::check_generated_nodes(&nodes, canvas_width);
        if recheck.has_fatal() {
            let message = recheck.failure_message();
            tracing::warn!(
                subtask = %subtask.id,
                issues = %message,
                "subagent self-check rejected generated nodes (unfixable after auto-fix)"
            );
            return fail(format!("self-check failed: {message}"));
        }
        if fixed {
            tracing::info!(
                subtask = %subtask.id,
                "subagent self-check auto-fixed fixable layout issues before insertion"
            );
        }
    }
    let node_count = nodes.len();

    // Hoist node-level `state` to one document-root MergeAppState so
    // `$app.*` references resolve globally (events stay on the nodes).
    let plan_idx = plan
        .subtasks
        .iter()
        .position(|s| s.id == subtask.id)
        .unwrap_or(0);
    let mut merge_state = op_editor_core::hoist_app_state(&mut nodes, plan_idx);
    // Union in whatever state script-gen's scratch-document run already
    // hoisted (`program_state`) — it was tagged "unplanned" there since
    // `program_gen`/`op-mcp` don't know this subtask's real plan_idx. Node-
    // drained state (above) takes priority on key collisions via `or_insert`;
    // in practice there's no real overlap since exactly one protocol runs per
    // attempt, but `or_insert` keeps the merge deterministic regardless.
    if let EditorCommand::MergeAppState { state, .. } = &mut merge_state {
        for (k, v) in program_state {
            state.entry(k).or_insert(v);
        }
    }
    let has_state =
        matches!(&merge_state, EditorCommand::MergeAppState { state, .. } if !state.is_empty());
    if has_state {
        apply_command_with_reveal(sink, merge_state, indicator_epoch, reveal_started_ms);
    }

    // Apply InsertSubtree via the root-id-returning path so we capture
    // the post-remap ids for Component 11 (append-mode cleanup scoping).
    let parent_id = match &subtask.parent_frame_id {
        Some(id) => NodeId::new(id.clone()),
        None => NodeId::NONE,
    };
    let Some(inserted_root_ids) = apply_insert_subtree_with_reveal(
        sink,
        nodes,
        parent_id.clone(),
        indicator_epoch,
        reveal_started_ms,
    ) else {
        let state = sink.state();
        let parent_status = if !parent_id.is_real() {
            "page-root"
        } else {
            match op_editor_core::walkers::find_node(state.active_children(), &parent_id) {
                None => "missing",
                Some(node) if node.is_container() => "container",
                Some(_) => "non-container",
            }
        };
        let active_page = state
            .doc
            .pages
            .as_ref()
            .and_then(|pages| pages.get(state.ui.active_page_index))
            .map(|page| format!("{} ({})", page.name, page.id))
            .unwrap_or_else(|| "legacy page 0".into());
        let error = format!(
            "InsertSubtree rejected: parent_id={} status={parent_status} active_page={active_page}",
            parent_id.as_str()
        );
        tracing::warn!(subtask = %subtask.id, error = %error, "subagent insert rejected");
        return fail(error);
    };

    SubtaskOutcome {
        id: subtask.id.clone(),
        node_count,
        error: None,
        inserted_root_ids,
        subtask: None,
    }
}

/// Block (worker thread, abort-aware) until the reveal overlay's scheduled
/// sweep for `epoch` has finished playing. The finalize passes swap subtrees
/// via `ReplaceSubtree`, whose fresh ids were never registered with the
/// overlay — restructuring mid-sweep snaps the still-animating tail of the
/// design in at once and orphans the agent cursor. Capped so a stuck clock
/// can't hang the run.
pub(crate) fn wait_for_reveal_drain(epoch: Option<u64>, abort: &crate::types::AbortFlag) {
    let Some(epoch) = epoch else {
        return;
    };
    let cap = reveal_now_millis().saturating_add(15_000);
    loop {
        if abort.is_set() {
            return;
        }
        let now = reveal_now_millis();
        let Some(end) = op_editor_core::agent_indicators::latest_reveal_end_ms(epoch) else {
            return;
        };
        if now >= end || now >= cap {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis((end - now).min(120)));
    }
}

pub(crate) fn reveal_now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

pub(crate) fn apply_command_with_reveal(
    sink: &mut dyn DocSink,
    cmd: EditorCommand,
    indicator_epoch: Option<u64>,
    reveal_started_ms: u64,
) -> bool {
    fn inserts_subtree(cmd: &EditorCommand) -> bool {
        match cmd {
            EditorCommand::InsertSubtree { .. } => true,
            EditorCommand::Batch { commands } => commands.iter().any(inserts_subtree),
            _ => false,
        }
    }

    if !inserts_subtree(&cmd) {
        return sink.apply(cmd);
    }
    let ids_before = indicator_epoch.map(|_| collect_active_node_ids(sink.state()));
    let applied = sink.apply(cmd);
    if applied {
        if let Some(ids_before) = ids_before.as_ref() {
            register_new_node_reveals(ids_before, sink.state(), indicator_epoch, reveal_started_ms);
        }
    }
    applied
}

/// Apply an `InsertSubtree` and return the **post-remap** root ids
/// (`None` = rejected). Same reveal bookkeeping as
/// [`apply_command_with_reveal`], but routes through the typed apply path
/// so it can surface the remapped ids onto the `SubtaskOutcome` (Component 11).
pub(crate) fn apply_insert_subtree_with_reveal(
    sink: &mut dyn DocSink,
    nodes: Vec<PenNode>,
    parent_id: NodeId,
    indicator_epoch: Option<u64>,
    reveal_started_ms: u64,
) -> Option<Vec<String>> {
    let ids_before = indicator_epoch.map(|_| collect_active_node_ids(sink.state()));
    let root_ids = sink.insert_subtree_returning_root_ids(nodes, &parent_id)?;
    if let Some(ids_before) = ids_before.as_ref() {
        register_new_node_reveals(ids_before, sink.state(), indicator_epoch, reveal_started_ms);
    }
    Some(root_ids)
}

// The reveal walk (before/after id diff + staggered reveal registration)
// is single-sourced in op-editor-core; re-exported so sibling modules and
// the reveal tests keep their `crate::subagent::*` paths.
pub(crate) use op_editor_core::agent_reveals::{
    collect_active_node_ids, register_new_node_reveals,
};

fn is_blank_container_forest(nodes: &[PenNode]) -> bool {
    !nodes.iter().any(has_content_node)
}

fn has_content_node(node: &PenNode) -> bool {
    match node.children() {
        Some(children) if !children.is_empty() => children.iter().any(has_content_node),
        // A childless rectangle is a visual in its own right (skeleton
        // line, divider, color block) — `is_container()` lumps it with
        // Frame/Group because it carries ContainerProps, which made
        // every skeleton-screen design read as "blank" (ab-v9: the
        // mobile-loading-skeleton prompt failed on all four models).
        _ => match node {
            PenNode::Rectangle(_) => true,
            // A childless frame with explicit paint renders exactly like
            // that rectangle — same pixels, different spelling. The
            // otp_input builder's await-input slots (stroked empty
            // boxes) made the whole manifest read as blank scaffolding,
            // forcing a retry into the hand-rolled raw fallback.
            PenNode::Frame(f) => {
                f.container.stroke.is_some()
                    || f.container
                        .fill
                        .as_ref()
                        .is_some_and(|fills| !fills.is_empty())
            }
            // A childless `ref` is a component instance — it expands to the
            // master's subtree at render (`ref_resolve`), so it is real
            // content even though `is_container()` lumps it with the empty
            // wrappers. Without this a design that reuses a component (the
            // whole point of refs) would be rejected as "blank scaffolding".
            PenNode::Ref(_) => true,
            _ => !node.is_container(),
        },
    }
}

/// The content slot a split-out section piece should be reparented into:
/// `primary` itself if it is an empty wrapper, else its FIRST empty direct-child
/// container (the "Dish Row" / "Card List" the model emitted but left empty).
/// Depth is capped at one on purpose — a deeply-nested incidental empty frame
/// (an await-input box, a spacer) must not swallow whole sections.
fn section_content_slot(primary: &mut PenNode) -> Option<&mut Vec<PenNode>> {
    let primary_empty = primary.children().map(|c| c.is_empty()).unwrap_or(true);
    if primary_empty {
        return primary.children_mut();
    }
    primary
        .children_mut()?
        .iter_mut()
        .find(|c| {
            matches!(c, PenNode::Frame(_) | PenNode::Group(_))
                && c.children().map(|cc| cc.is_empty()).unwrap_or(true)
        })
        .and_then(|c| c.children_mut())
}

/// Weak-model deterministic floor (TS `applyPostStreamingTreeHeuristics`
/// spirit): a subtask maps to ONE section, but glm-class models routinely split
/// a section into sibling pieces — an (often empty) wrapper plus the content
/// roots that belong inside it (e.g. an empty `Promo Banner` next to a
/// `Promo Content` text block and a `Promo Food Image`; an empty `Dish Row` next
/// to the dish cards). Each forest root is later normalized into a full-width
/// page band ([`normalize_section_roots_for_parent_layout`]), so the wrapper
/// renders as a blank gap while its content floats with no background — invisible
/// white-on-cream text, the broken Featured/Promo screens the user flagged.
///
/// Resolution, keyed off the first section container (Frame/Group) as the
/// "primary" section:
/// - **Wrapper slot present** — the primary is empty, or has an empty direct
///   child container waiting for content: reparent every orphan root into that
///   slot (forest order preserved). Fixes the empty-banner / empty-row splits.
/// - **No slot, orphans all leaves or EMPTY containers** — stray decorations a
///   header / section spilled outside itself (a bell icon, a heading, an empty
///   notification-badge frame): leaf/heading orphans fold into the primary
///   (leading prepended, trailing appended) and childless containers are dropped
///   (they would only normalize into a blank full-width band).
/// - **No slot, an orphan is a POPULATED container (≥1 child)** — a legitimate
///   multi-section forest with nowhere to nest: left untouched so real sections
///   are never collapsed into one another.
fn coalesce_subtask_section(nodes: &mut Vec<PenNode>) {
    if nodes.len() < 2 {
        return;
    }
    let Some(primary_idx) = nodes
        .iter()
        .position(|n| matches!(n, PenNode::Frame(_) | PenNode::Group(_)))
    else {
        return; // no section container to reparent into.
    };
    // An orphan is a stray DECORATION to fold (not a real sibling section) when
    // it's a non-container leaf (icon / badge text), OR an EMPTY container — a
    // childless `Notification Badge` frame the model hung OUTSIDE the header that
    // would otherwise normalize into a blank full-width band. A container that
    // actually holds content (≥1 child) is a genuine section → bail (never
    // collapse real sibling sections into one another).
    let kid_count = |n: &PenNode| n.children().map(|c| c.len()).unwrap_or(0);
    // A childless `ref` is a component instance, not an empty wrapper — it
    // expands to its master's subtree at render. Treat it as content so the
    // drop/fold passes below never discard it.
    let is_empty_wrapper =
        |n: &PenNode| n.is_container() && kid_count(n) == 0 && !matches!(n, PenNode::Ref(_));
    let orphans_all_foldable = nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != primary_idx)
        .all(|(_, n)| !n.is_container() || kid_count(n) == 0);

    // Split into before / primary / after, preserving forest order. Empty
    // containers (0 children — junk like a stray empty badge that would
    // normalize into a blank full-width band) are dropped rather than folded.
    let taken = std::mem::take(nodes);
    let mut before: Vec<PenNode> = Vec::new();
    let mut after: Vec<PenNode> = Vec::new();
    let mut primary: Option<PenNode> = None;
    let keep = |n: &PenNode| !is_empty_wrapper(n);
    for (i, node) in taken.into_iter().enumerate() {
        match i.cmp(&primary_idx) {
            std::cmp::Ordering::Less if keep(&node) => before.push(node),
            std::cmp::Ordering::Equal => primary = Some(node),
            std::cmp::Ordering::Greater if keep(&node) => after.push(node),
            _ => {} // dropped empty-container orphan
        }
    }
    let mut primary = primary.expect("primary index was computed from nodes");

    if section_content_slot(&mut primary).is_some() {
        // Wrapper slot: drop the split-out pieces into it, forest order kept.
        let slot = section_content_slot(&mut primary).expect("slot present");
        slot.extend(before);
        slot.extend(after);
    } else if orphans_all_foldable {
        // Stray decorations: heading-before prepends, badge/icon-after appends.
        if let Some(kids) = primary.children_mut() {
            let existing = std::mem::take(kids);
            kids.extend(before);
            kids.extend(existing);
            kids.extend(after);
        }
    } else {
        // Legitimate multi-section forest — restore original order, change nothing.
        nodes.extend(before);
        nodes.push(primary);
        nodes.extend(after);
        return;
    }
    nodes.push(primary);
}

fn normalize_section_roots_for_parent_layout(nodes: &mut [PenNode]) {
    for node in nodes {
        node.base_mut().x = None;
        node.base_mut().y = None;
        match node {
            PenNode::Frame(frame) => {
                frame.container.width = Some(SizingBehavior::Keyword(SizingKeyword::FillContainer));
                frame.container.height = Some(SizingBehavior::Keyword(SizingKeyword::FitContent));
            }
            PenNode::Group(group) => {
                group.container.width = Some(SizingBehavior::Keyword(SizingKeyword::FillContainer));
                group.container.height = Some(SizingBehavior::Keyword(SizingKeyword::FitContent));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "subagent_reveal_tests.rs"]
mod subagent_reveal_tests;
#[cfg(test)]
#[path = "subagent_tests.rs"]
mod tests;
