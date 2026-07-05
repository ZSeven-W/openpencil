//! 阶段 3 —— 单个 sub-agent 的顺序执行。
//!
//! 一个 subtask:构 prompt → 调 `LlmClient` → 收集文本 → 解析成
//! `PenNode` 树 → 经 `DocSink` 发一条 `InsertSubtree`。
//!
//! 返回的 [`SubtaskOutcome`] 用 `node_count` 区分(见 spec §6.2):
//! - `node_count == 0` —— 零节点失败,调用方应停止后续 subtask;
//! - `node_count > 0`(`error` 可带软错误)—— 部分产出,继续后续。

use crate::plan::{OrchestratorPlan, Subtask};
use crate::prompt::build_subagent_prompt;
use crate::types::{AbortFlag, DesignRequest, DocSink, LlmChunk, LlmClient, SubtaskOutcome};
use futures::StreamExt;
use jian_ops_schema::node::{ContainerProps, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use std::collections::HashSet;
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
    };

    // Snapshot the document's reusable-component registry before the prompt
    // build so the AVAILABLE COMPONENTS manifest reflects whatever masters were
    // merged into the doc (e.g. a loaded `.lib.op`). Cloned to release the
    // shared `sink` borrow before the later mutable inserts. Empty registry ⇒
    // `build_subagent_prompt` leaves the prompt unchanged.
    let components = sink.state().components.clone();

    // 收集 LLM 文本输出。
    let (call_req, skill_report) = build_subagent_prompt(
        subtask,
        plan,
        req,
        abort.clone(),
        reduced_complexity,
        minimal_skills,
        &components,
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
            return fail(e);
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
    crate::role_post_pass::post_pass_forest(&mut nodes, canvas_width);
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
    crate::tree_heuristics::apply_tree_heuristics(
        &mut nodes,
        page_bg.as_deref(),
        theme == crate::role_defaults::Theme::Light,
        prior_accent.as_deref(),
    );
    crate::variable_binding::bind_generated_color_variables(&mut nodes, sink.state());
    // Surface-color discipline runs AFTER binding: glm emits raw hex, and binding
    // is what turns it into the `$color-danger-bg` / `$color-bg-deep` refs this
    // pass matches (recolor misused state-bg surfaces → neutral, strip the
    // page-bg token off inner wrappers). Pre-binding it only saw hex and missed.
    crate::role_post_pass::enforce_surface_color_discipline(&mut nodes);
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
                "subagent self-check auto-fixed product-row overflow (fixed-width cards -> fill_container)"
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
        parent_id,
        indicator_epoch,
        reveal_started_ms,
    ) else {
        return fail("InsertSubtree rejected by document".into());
    };

    SubtaskOutcome {
        id: subtask.id.clone(),
        node_count,
        error: None,
        inserted_root_ids,
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
    if !matches!(cmd, EditorCommand::InsertSubtree { .. }) {
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

fn collect_active_node_ids(state: &op_editor_core::EditorState) -> HashSet<String> {
    let mut out = HashSet::new();
    for node in state.active_children() {
        collect_node_ids(node, &mut out);
    }
    out
}

fn collect_node_ids(node: &PenNode, out: &mut HashSet<String>) {
    out.insert(node.id_str().to_string());
    if let Some(children) = node.children() {
        for child in children {
            collect_node_ids(child, out);
        }
    }
}

pub(crate) fn register_new_node_reveals(
    ids_before: &HashSet<String>,
    state: &op_editor_core::EditorState,
    indicator_epoch: Option<u64>,
    reveal_started_ms: u64,
) {
    let Some(epoch) = indicator_epoch else {
        return;
    };
    let mut stream = RevealStream {
        index: 0,
        next_start_ms: reveal_started_ms,
    };
    for node in state.active_children() {
        register_node_reveals(
            node,
            ids_before,
            epoch,
            reveal_started_ms,
            0,
            None,
            &mut stream,
        );
    }
}

struct RevealStream {
    index: u64,
    next_start_ms: u64,
}

fn register_node_reveals(
    node: &PenNode,
    ids_before: &HashSet<String>,
    epoch: u64,
    reveal_started_ms: u64,
    depth: u64,
    parent_reveal_start_ms: Option<u64>,
    stream: &mut RevealStream,
) {
    let id = node.id_str();
    let mut own_reveal_start_ms = parent_reveal_start_ms;
    if !ids_before.contains(id) && should_reveal_node(node, depth) {
        let own_stream_index = stream.index;
        stream.index += 1;
        let base_start = reveal_started_ms
            + op_editor_core::agent_indicators::reveal_offset_ms(depth, own_stream_index);
        let child_runway_start = parent_reveal_start_ms
            .map(|started_at| {
                started_at.saturating_add(op_editor_core::agent_indicators::REVEAL_CHILD_RUNWAY_MS)
            })
            .unwrap_or(reveal_started_ms);
        let started_at = base_start.max(child_runway_start).max(stream.next_start_ms);
        op_editor_core::agent_indicators::add_reveal(epoch, id, started_at);
        stream.next_start_ms =
            started_at.saturating_add(op_editor_core::agent_indicators::REVEAL_STAGGER_MS);
        own_reveal_start_ms = Some(started_at);
    }
    if let Some(children) = node.children() {
        for child in children {
            register_node_reveals(
                child,
                ids_before,
                epoch,
                reveal_started_ms,
                depth + 1,
                own_reveal_start_ms,
                stream,
            );
        }
    }
}

fn should_reveal_node(node: &PenNode, depth: u64) -> bool {
    depth == 0 || node_has_own_visual(node) || node_is_named_structure(node)
}

fn node_has_own_visual(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(n) => {
            container_has_own_visual(&n.container) || n.image_search_query.is_some()
        }
        PenNode::Group(n) => container_has_own_visual(&n.container),
        PenNode::Rectangle(n) => container_has_own_visual(&n.container),
        PenNode::Ref(_) => false,
        PenNode::Text(n) => match &n.content {
            jian_ops_schema::node::TextContent::Plain(s) => !s.is_empty(),
            jian_ops_schema::node::TextContent::Styled(segments) => !segments.is_empty(),
        },
        _ => true,
    }
}

fn container_has_own_visual(container: &ContainerProps) -> bool {
    container
        .fill
        .as_ref()
        .is_some_and(|fills| !fills.is_empty())
        || container.stroke.is_some()
        || container
            .effects
            .as_ref()
            .is_some_and(|effects| !effects.is_empty())
}

fn node_is_named_structure(node: &PenNode) -> bool {
    if !node.is_container() {
        return false;
    }
    let base = node.base();
    base.role.as_deref().is_some_and(|role| !role.is_empty())
        || base.name.as_deref().is_some_and(|name| !name.is_empty())
}

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
mod tests {
    use super::*;
    use crate::plan::{OrchestratorPlan, Region, RootFrameSpec};
    use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
    use crate::types::LlmError;
    use futures::executor::block_on;
    use jian_ops_schema::node::PenNode;

    fn req() -> DesignRequest {
        DesignRequest {
            prompt: "a page".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: true,

            visual_ref_enabled: false,
        }
    }

    fn plan() -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "P".into(),
                width: 1200.0,
                height: 800.0,
                layout: None,
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks: vec![],
            style_guide_name: None,
        }
    }

    fn subtask() -> Subtask {
        Subtask {
            id: "hero".into(),
            label: "Hero".into(),
            region: Region {
                width: 1200.0,
                height: 400.0,
            },
            id_prefix: "hero".into(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
        }
    }

    // A single I(null, {...}) call whose node object nests its children inline
    // (batch_design's insert accepts a whole subtree per call). Authored ids
    // are dropped: the batch_design executor reassigns fresh ids to every
    // inserted node regardless, so tests that use this constant must not assert
    // on literal id strings.
    const NODE_SCRIPT: &str = r#"I(null, {"type":"frame","name":"Card","x":0,"y":0,"width":1200,"height":200,"children":[{"type":"text","content":"Hero","fontSize":18}]});"#;

    #[test]
    fn coalesce_folds_trailing_badge_leaf_into_lone_section() {
        // glm shape: a populated Top Bar section frame + a stray cart-count
        // badge "3" emitted as a SIBLING. No empty slot exists, the orphan is a
        // leaf → it appends into the section, not survive as a floating "3" band.
        let json = r#"[
            {"type":"frame","id":"s1","name":"Top Bar","width":"fill_container","height":"fit_content","layout":"horizontal","children":[
                {"type":"text","id":"loc","content":"Home"}
            ]},
            {"type":"text","id":"badge","content":"3"}
        ]"#;
        let mut nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        coalesce_subtask_section(&mut nodes);
        assert_eq!(nodes.len(), 1, "badge must fold into the lone section");
        let kids = nodes[0].children().expect("section children");
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].id_str(), "loc");
        assert_eq!(
            kids[1].id_str(),
            "badge",
            "trailing badge nests as last child"
        );
    }

    #[test]
    fn coalesce_prepends_leading_leaf_as_section_heading() {
        // A heading text emitted BEFORE a populated content frame folds in as the
        // FIRST child (preserving the intended heading-above-content order).
        let json = r#"[
            {"type":"text","id":"head","content":"Featured"},
            {"type":"frame","id":"sec","name":"List","children":[{"type":"text","id":"item","content":"x"}]}
        ]"#;
        let mut nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        coalesce_subtask_section(&mut nodes);
        assert_eq!(nodes.len(), 1);
        let kids = nodes[0].children().expect("section children");
        assert_eq!(
            kids[0].id_str(),
            "head",
            "leading leaf prepended as heading"
        );
        assert_eq!(kids[1].id_str(), "item");
    }

    #[test]
    fn coalesce_fills_empty_wrapper_with_split_pieces() {
        // tt5 Promo shape: an EMPTY `Promo Banner` wrapper emitted first, with
        // its `Promo Content` (text) + `Promo Food Image` hung as sibling roots.
        // They must reparent INTO the empty banner (forest order kept) so the
        // banner stops rendering as a blank gap with floating invisible text.
        let json = r#"[
            {"type":"frame","id":"banner","name":"Promo Banner","layout":"vertical","children":[]},
            {"type":"frame","id":"content","name":"Promo Content","children":[{"type":"text","id":"title","content":"Get 30% off"}]},
            {"type":"image","id":"img","name":"Promo Food Image","src":""}
        ]"#;
        let mut nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        coalesce_subtask_section(&mut nodes);
        assert_eq!(nodes.len(), 1, "split pieces fold into the empty wrapper");
        assert_eq!(nodes[0].id_str(), "banner");
        let kids = nodes[0].children().expect("banner children");
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].id_str(), "content");
        assert_eq!(kids[1].id_str(), "img");
    }

    #[test]
    fn coalesce_fills_empty_direct_child_row_with_orphan_cards() {
        // tt5 Popular Dishes shape: section with a Header + an EMPTY `Dish Row`
        // direct child, and the dish cards hung as sibling roots. The cards must
        // land inside the empty row, not flatten into separate page bands.
        let json = r#"[
            {"type":"frame","id":"sec","name":"Popular Dishes","children":[
                {"type":"frame","id":"header","name":"Header","children":[{"type":"text","id":"t","content":"Popular Dishes"}]},
                {"type":"frame","id":"row","name":"Dish Row","children":[]}
            ]},
            {"type":"image","id":"pizza","name":"Margherita Pizza","src":""},
            {"type":"frame","id":"pinfo","name":"Info","children":[{"type":"text","id":"pn","content":"Margherita"}]}
        ]"#;
        let mut nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        coalesce_subtask_section(&mut nodes);
        assert_eq!(
            nodes.len(),
            1,
            "cards fold into the section, not the page root"
        );
        let sec_kids = nodes[0].children().expect("section children");
        assert_eq!(sec_kids.len(), 2, "Header + Dish Row preserved");
        let row = &sec_kids[1];
        assert_eq!(row.id_str(), "row");
        let row_kids = row.children().expect("dish row children");
        assert_eq!(
            row_kids.len(),
            2,
            "both orphan cards landed in the empty row"
        );
        assert_eq!(row_kids[0].id_str(), "pizza");
        assert_eq!(row_kids[1].id_str(), "pinfo");
    }

    #[test]
    fn coalesce_leaves_populated_multi_section_forest_untouched() {
        // Two POPULATED section containers with no empty slot → a legitimate
        // multi-section forest; never collapse real sections into one another.
        let json = r#"[
            {"type":"frame","id":"a","name":"A","children":[{"type":"text","id":"ta","content":"A"}]},
            {"type":"frame","id":"b","name":"B","children":[{"type":"text","id":"tb","content":"B"}]}
        ]"#;
        let mut nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        coalesce_subtask_section(&mut nodes);
        assert_eq!(nodes.len(), 2, "two populated sections must be left as-is");
    }

    #[test]
    fn ref_only_forest_is_not_rejected_as_blank() {
        // A subtask that reuses a component is a lone childless `ref`. It has no
        // children pre-resolution but expands to the master's subtree — so it
        // must NOT count as a blank-scaffolding forest (which would `fail()`).
        let json = r#"[{"type":"ref","id":"inst","ref":"comp-card","x":0,"y":0}]"#;
        let nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        assert!(
            has_content_node(&nodes[0]),
            "a ref is content (it expands to the master subtree)"
        );
        assert!(
            !is_blank_container_forest(&nodes),
            "a ref-only forest must survive the blank-container guard"
        );
    }

    #[test]
    fn childless_frame_with_stroke_is_content_not_blank() {
        // The otp_input await-input slots: a childless Frame carrying an
        // explicit stroke renders exactly like a bare rectangle — same
        // pixels, different spelling. It must NOT count as blank scaffolding.
        let json = r##"[
            {"type":"frame","id":"root","name":"Root","children":[
                {"type":"frame","id":"box","name":"Await Input","children":[],
                 "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#E2E8F0"}]}}
            ]}
        ]"##;
        let nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        assert!(
            has_content_node(&nodes[0]),
            "a childless frame with a stroke paints real pixels"
        );
        assert!(
            !is_blank_container_forest(&nodes),
            "a stroked childless-frame forest must survive the blank-container guard"
        );
    }

    #[test]
    fn childless_frame_with_fill_is_content_not_blank() {
        // Same shape as above but the paint comes from `fill` instead of
        // `stroke` — both count as explicit paint on an otherwise-empty
        // container.
        let json = r##"[
            {"type":"frame","id":"root","name":"Root","children":[
                {"type":"frame","id":"box","name":"Color Block","children":[],
                 "fill":[{"type":"solid","color":"#111111"}]}
            ]}
        ]"##;
        let nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        assert!(
            has_content_node(&nodes[0]),
            "a childless frame with a fill paints real pixels"
        );
        assert!(
            !is_blank_container_forest(&nodes),
            "a filled childless-frame forest must survive the blank-container guard"
        );
    }

    #[test]
    fn childless_frame_without_paint_is_blank() {
        // Same shape, no stroke/fill: genuinely empty scaffolding. This is
        // the case the blank-container guard exists to catch, so it must
        // still be rejected.
        let json = r#"[
            {"type":"frame","id":"root","name":"Root","children":[
                {"type":"frame","id":"box","name":"Empty","children":[]}
            ]}
        ]"#;
        let nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        assert!(
            !has_content_node(&nodes[0]),
            "a childless frame with no paint is not content"
        );
        assert!(
            is_blank_container_forest(&nodes),
            "an unpainted childless-frame forest must be rejected as blank"
        );
    }

    #[test]
    fn coalesce_keeps_ref_orphan_instead_of_dropping_it() {
        // A populated section plus a sibling component instance (`ref`). The ref
        // is childless but is real content — it must fold into the section, not
        // be silently dropped as an "empty container" orphan.
        let json = r#"[
            {"type":"frame","id":"sec","name":"Section","children":[{"type":"text","id":"t","content":"Hi"}]},
            {"type":"ref","id":"inst","ref":"comp-card"}
        ]"#;
        let mut nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        coalesce_subtask_section(&mut nodes);
        // The ref folds into the section (no empty slot, orphan is foldable) —
        // the key assertion is that it is NOT discarded.
        let surviving: Vec<&str> = collect_ids(&nodes);
        assert!(
            surviving.contains(&"inst"),
            "the ref instance must survive coalesce, got ids {surviving:?}"
        );
    }

    /// Depth-first id collection for the ref-survival assertion.
    fn collect_ids(nodes: &[PenNode]) -> Vec<&str> {
        let mut out = Vec::new();
        fn walk<'a>(nodes: &'a [PenNode], out: &mut Vec<&'a str>) {
            for n in nodes {
                out.push(n.id_str());
                if let Some(kids) = n.children() {
                    walk(kids, out);
                }
            }
        }
        walk(nodes, &mut out);
        out
    }

    #[test]
    fn coalesce_folds_stray_icon_and_drops_empty_badge() {
        // tt5 header: the Bell icon (leaf) + an EMPTY Notification Badge frame
        // were emitted as top-level SIBLINGS of the header (bell floated below
        // the search; the empty badge would normalize into a blank full-width
        // band). The icon folds into the header; the empty badge is dropped —
        // neither survives as a floating page section.
        let json = r#"[
            {"type":"frame","id":"hdr","name":"Header & Search","children":[
                {"type":"frame","id":"loc","name":"Location & Actions","children":[{"type":"text","id":"l","content":"NYC"}]},
                {"type":"frame","id":"sb","name":"Search Bar","children":[{"type":"text_input","id":"si","placeholder":"Search"}]}
            ]},
            {"type":"icon_font","id":"bell","iconFontName":"bell"},
            {"type":"frame","id":"badge","name":"Notification Badge","children":[]}
        ]"#;
        let mut nodes: Vec<PenNode> = serde_json::from_str(json).expect("parse forest");
        coalesce_subtask_section(&mut nodes);
        assert_eq!(
            nodes.len(),
            1,
            "only the header section survives at top level"
        );
        let kids = nodes[0].children().expect("header children");
        assert!(
            kids.iter().any(|k| matches!(k, PenNode::IconFont(_))),
            "stray bell icon folded into the header"
        );
        let names: Vec<&str> = kids
            .iter()
            .filter_map(|k| k.base().name.as_deref())
            .collect();
        assert!(
            !names.contains(&"Notification Badge"),
            "empty badge dropped, not folded as a blank band"
        );
    }

    #[test]
    fn run_subtask_hoists_node_state_before_insert_subtree() {
        // A frame whose LLM output carries a `state` block should emit
        // a MergeAppState command BEFORE the InsertSubtree, and the inserted
        // nodes must carry no residual `state`.
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            r#"I(null, {"type":"frame","name":"Card","x":0,"y":0,"width":1200,"height":200,
                  "state":{"count":{"type":"int","default":0}},
                  "children":[{"type":"text","content":"Hero","fontSize":18}]});"#
                .into(),
        )]);
        let mut plan = plan();
        plan.subtasks = vec![subtask()];
        let mut sink = VecDocSink::new();
        let outcome = block_on(run_subtask(
            &subtask(),
            &plan,
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));
        assert!(
            outcome.error.is_none(),
            "subtask must succeed: {:?}",
            outcome.error
        );
        // MergeAppState must precede InsertSubtree.
        let merge_pos = sink
            .applied
            .iter()
            .position(|c| matches!(c, EditorCommand::MergeAppState { .. }));
        let insert_pos = sink
            .applied
            .iter()
            .position(|c| matches!(c, EditorCommand::InsertSubtree { .. }));
        assert!(merge_pos.is_some(), "MergeAppState must be emitted");
        assert!(
            merge_pos.unwrap() < insert_pos.unwrap(),
            "MergeAppState must precede InsertSubtree"
        );
        // The inserted nodes must have state drained.
        let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
            panic!("last command must be InsertSubtree");
        };
        let PenNode::Frame(f) = &nodes[0] else {
            panic!()
        };
        assert!(f.state.is_none(), "inserted node must have state stripped");
    }

    #[test]
    fn run_subtask_ok_applies_insert_subtree() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_SCRIPT.into())]);
        let mut sink = VecDocSink::new();
        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));
        assert_eq!(outcome.node_count, 1);
        assert!(outcome.error.is_none());
        assert!(matches!(
            sink.applied.last(),
            Some(EditorCommand::InsertSubtree { .. })
        ));
    }

    /// Reduced-complexity retry rung still uses script-gen; it narrows the
    /// skill set only. The parser must therefore accept the same nested
    /// `I(parent, node)` forest as the full attempt.
    #[test]
    fn run_subtask_reduced_complexity_still_uses_script_gen_nested_forest() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_SCRIPT.into())]);
        let mut sink = VecDocSink::new();
        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            true,
            false,
        ));
        assert_eq!(outcome.node_count, 1);
        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
            panic!("expected InsertSubtree, got {:?}", sink.applied.last());
        };
        assert_eq!(nodes.len(), 1);
        let children = nodes[0].children().expect("script-gen frame has children");
        assert_eq!(children.len(), 1);
        assert!(
            !nodes[0].id_str().is_empty(),
            "script-gen must assign a fresh root id"
        );
        assert_ne!(
            nodes[0].id_str(),
            "hero-1",
            "reduced retry must not use the retired flat-JSONL parser"
        );
    }

    #[test]
    fn run_subtask_binds_generated_exact_color_to_document_variable() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            r##"I(null, {"type":"rectangle","width":100,"height":50,"fill":[{"type":"solid","color":"#F8FAFC"}]});"##
                .into(),
        )]);
        let mut sink = VecDocSink::new();
        sink.apply(EditorCommand::MergeThemePreset {
            variables: crate::semantic_palette::palette_variables(),
            themes: crate::semantic_palette::palette_themes(),
        });

        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));

        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
            panic!("expected InsertSubtree, got {:?}", sink.applied.last());
        };
        assert_eq!(
            op_editor_core::fills::first_solid_fill_hex(&nodes[0]),
            Some("$color-bg-deep")
        );
    }

    #[test]
    fn run_subtask_binds_generated_near_color_to_document_variable() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            r##"I(null, {"type":"rectangle","width":100,"height":50,"fill":[{"type":"solid","color":"#FFF8F0"}]});"##
                .into(),
        )]);
        let mut sink = VecDocSink::new();
        sink.apply(EditorCommand::MergeThemePreset {
            variables: crate::semantic_palette::palette_variables(),
            themes: crate::semantic_palette::palette_themes(),
        });

        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));

        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
            panic!("expected InsertSubtree, got {:?}", sink.applied.last());
        };
        assert_eq!(
            op_editor_core::fills::first_solid_fill_hex(&nodes[0]),
            Some("$color-surface-3")
        );
    }

    #[test]
    fn run_subtask_staggers_reveals_for_live_inserted_nodes() {
        let _guard = crate::agent_indicator_test_support::lock();
        let epoch = op_editor_core::agent_indicators::begin();
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_SCRIPT.into())]);
        let mut sink = VecDocSink::new();

        let outcome = block_on(run_subtask_with_reveal_at(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
            Some(epoch),
            1_234,
            None,
        ));

        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        assert_eq!(outcome.node_count, 1);
        let live_ids = collect_active_node_ids(sink.state());
        let snapshot = op_editor_core::agent_indicators::snapshot_at(1_250);
        assert_eq!(snapshot.reveals.len(), 2);
        assert!(
            snapshot
                .reveals
                .keys()
                .all(|id| live_ids.contains(id.as_str())),
            "reveals must reference live document ids"
        );
        let first = snapshot.reveals.values().min().copied().unwrap();
        let last = snapshot.reveals.values().max().copied().unwrap();
        assert_eq!(first, 1_234);
        assert!(last > first, "subtree nodes should reveal progressively");
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    }

    #[test]
    fn run_subtask_zero_node_on_garbage() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text("the model refused".into())]);
        let mut sink = VecDocSink::new();
        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));
        assert_eq!(outcome.node_count, 0);
        assert!(outcome.error.is_some());
    }

    #[test]
    fn skeleton_of_bare_rectangles_is_content_not_blank() {
        // 骨架屏 = frame 根 + 一排无子矩形线条;矩形虽带 ContainerProps
        // 但它是视觉本体,不能整批判空(ab-v9 全模型踩中)。
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            r#"I(null, {"type":"frame","name":"Skeleton","width":"fill_container","height":"fit_content","children":[{"type":"rectangle","width":"fill_container","height":16,"cornerRadius":8},{"type":"rectangle","width":205,"height":16,"cornerRadius":8}]});"#
                .into(),
        )]);
        let mut sink = VecDocSink::new();
        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));
        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        assert_eq!(outcome.node_count, 1);
    }

    #[test]
    fn run_subtask_rejects_blank_container_root() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            r#"I(null, {"type":"frame","name":"Blank","x":0,"y":0,"width":390,"height":112,"children":[]});"#
                .into(),
        )]);
        let mut sink = VecDocSink::new();
        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));

        assert_eq!(outcome.node_count, 0);
        assert!(outcome
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("blank container"));
        assert!(sink.applied.is_empty());
    }

    #[test]
    fn run_subtask_auto_fixes_self_check_product_overflow() {
        let mut mobile_plan = plan();
        mobile_plan.root_frame.width = 390.0;
        mobile_plan.root_frame.height = 844.0;
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            r#"I(null, {
              "type":"frame","name":"Popular Section","width":"fill_container","height":"fit_content","layout":"vertical","children":[
                {"type":"frame","name":"Popular Now Cards","width":"fill_container","height":"fit_content","layout":"horizontal","gap":20,"children":[
                  {"type":"frame","role":"card","width":170,"height":220,"children":[
                    {"type":"image","width":170,"height":120,"imageSearchQuery":"pasta plate"},
                    {"type":"text","content":"Truffle Carbonara"}
                  ]},
                  {"type":"frame","role":"card","width":170,"height":220,"children":[
                    {"type":"image","width":170,"height":120,"imageSearchQuery":"burger plate"},
                    {"type":"text","content":"Smash Deluxe"}
                  ]},
                ]}
              ]
            });"#
            .into(),
        )]);
        let mut sink = VecDocSink::new();

        let outcome = block_on(run_subtask(
            &subtask(),
            &mobile_plan,
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));

        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        assert_eq!(outcome.node_count, 1);
        let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
            panic!("expected InsertSubtree");
        };
        let fixed_json = serde_json::to_value(nodes).expect("serialize nodes");
        let row = &fixed_json[0]["children"][0];
        assert_eq!(row["gap"].as_f64(), Some(12.0));
        assert_eq!(
            row["children"][0]["width"],
            serde_json::json!("fill_container")
        );
        assert_eq!(
            row["children"][1]["width"],
            serde_json::json!("fill_container")
        );
    }

    #[test]
    fn run_subtask_normalizes_section_root_for_parent_layout() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            r#"I(null, {"type":"frame","name":"Section","x":0,"y":0,"width":390,"height":112,"children":[{"type":"text","content":"Pizza","fontSize":18}]});"#
                .into(),
        )]);
        let mut sink = VecDocSink::new();
        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));

        assert_eq!(outcome.node_count, 1);
        let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
            panic!("expected InsertSubtree");
        };
        let PenNode::Frame(frame) = &nodes[0] else {
            panic!("expected frame root");
        };
        assert!(frame.base.x.is_none());
        assert!(frame.base.y.is_none());
        assert!(matches!(
            frame.container.width,
            Some(jian_ops_schema::sizing::SizingBehavior::Keyword(
                jian_ops_schema::sizing::SizingKeyword::FillContainer
            ))
        ));
        assert!(matches!(
            frame.container.height,
            Some(jian_ops_schema::sizing::SizingBehavior::Keyword(
                jian_ops_schema::sizing::SizingKeyword::FitContent
            ))
        ));
    }

    #[test]
    fn run_subtask_zero_node_on_llm_error() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
            message: "rate limited".into(),
            aborted: false,
        })]);
        let mut sink = VecDocSink::new();
        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));
        assert_eq!(outcome.node_count, 0);
        assert_eq!(outcome.error.as_deref(), Some("rate limited"));
    }

    #[test]
    fn run_subtask_emits_subtask_skills_progress() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_SCRIPT.into())]);
        let mut sink = VecDocSink::new();
        let mut events: Vec<crate::types::Progress> = Vec::new();
        let mut on_progress = |p: crate::types::Progress| events.push(p);
        let outcome = block_on(run_subtask_with_progress(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
            Some(&mut on_progress),
        ));
        assert_eq!(outcome.node_count, 1);
        assert!(
            events.iter().any(|p| matches!(
                p,
                crate::types::Progress::SubtaskSkills { id, .. } if id == "hero"
            )),
            "expected a SubtaskSkills event, got {events:?}"
        );
    }

    /// End-to-end: when the LLM emits a frame with role="input", run_subtask
    /// must insert a text_input node (not a frame) into the document.
    /// promote_forest runs AFTER post_pass_forest and BEFORE binding, so the
    /// widget lands in the live document tree.
    #[test]
    fn run_subtask_promotes_role_input_frame_to_text_input() {
        // The LLM output contains a section frame whose only child is a
        // role="input" field with a muted placeholder and icon children.
        let llm_script = r##"I(null, {
          "type":"frame","name":"Login Form","width":1200,"height":400,
           "layout":"vertical","children":[
             {"type":"frame","role":"input","width":320,"height":48,
              "fill":[{"type":"solid","color":"#f3f4f6"}],"children":[
                {"type":"icon_font","iconFontName":"mail","width":20,"height":20},
                {"type":"text","content":"Email address",
                 "fill":[{"type":"solid","color":"#9ca3af"}]}
              ]}
           ]
        });"##;
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(llm_script.into())]);
        let mut sink = VecDocSink::new();
        let outcome = block_on(run_subtask(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
        ));

        assert!(
            outcome.error.is_none(),
            "unexpected error: {:?}",
            outcome.error
        );
        assert_eq!(outcome.node_count, 1);

        let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
            panic!("expected InsertSubtree, got {:?}", sink.applied.last());
        };
        // The outer section frame is NOT a widget — it stays a frame.
        let PenNode::Frame(section) = &nodes[0] else {
            panic!("outer section must remain a frame");
        };
        // The inner role="input" child must have been promoted to text_input.
        let children = section.children.as_ref().expect("section has children");
        assert_eq!(children.len(), 1, "exactly one child (the promoted input)");
        let PenNode::TextInput(ti) = &children[0] else {
            panic!("role=input child must become TextInput after promotion");
        };
        // script-gen's batch_design executor remaps every id to a fresh one
        // (unlike the flat-JSONL path, which keeps the model-authored id), so
        // this only asserts an id WAS assigned, not its literal value.
        assert!(!ti.base.id.is_empty(), "promoted node must carry an id");
        assert!(ti.base.role.is_none(), "role cleared after promotion");
        assert_eq!(ti.leading_icon.as_deref(), Some("mail"));
        assert_eq!(ti.placeholder.as_deref(), Some("Email address"));
    }
}
