//! 阶段 3 —— 单个 sub-agent 的顺序执行。
//!
//! 一个 subtask:构 prompt → 调 `LlmClient` → 收集文本 → 解析成
//! `PenNode` 树 → 经 `DocSink` 发一条 `InsertSubtree`。
//!
//! 返回的 [`SubtaskOutcome`] 用 `node_count` 区分(见 spec §6.2):
//! - `node_count == 0` —— 零节点失败,调用方应停止后续 subtask;
//! - `node_count > 0`(`error` 可带软错误)—— 部分产出,继续后续。

use crate::parse::parse_nodes;
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
/// * `minimal_skills` — Strip the system prompt to only
///   `schema`+`jsonl-format` (last-ditch fallback).  Pass `false` for
///   the first two attempts; pass `true` on the third attempt (Task C3).
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
) -> SubtaskOutcome {
    let fail = |msg: String| SubtaskOutcome {
        id: subtask.id.clone(),
        node_count: 0,
        error: Some(msg),
    };

    // 收集 LLM 文本输出。
    let call_req = build_subagent_prompt(
        subtask,
        plan,
        req,
        abort.clone(),
        reduced_complexity,
        minimal_skills,
    );
    let mut stream = llm.call(call_req);
    let mut text = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(LlmChunk::Text(t)) => text.push_str(&t),
            Ok(LlmChunk::Thinking(_)) => {}
            Err(e) => {
                return fail(if e.aborted {
                    "aborted".into()
                } else {
                    e.message
                });
            }
        }
    }

    // 解析成 PenNode 树。manifest 模式（按模型路由 + `OPENPENCIL_MANIFEST`
    // override）先按元素清单解析；文本里没有清单行（如重试梯度回落到裸
    // JSONL prompt 后的输出）时回落到既有裸 PenNode 路径，两条路汇入同
    // 一套后处理。
    let mut nodes =
        if crate::manifest::manifest_enabled_for_model(req.model.as_deref().unwrap_or("")) {
            match crate::manifest::parse_manifest(&text) {
                Some(outcome) => {
                    for warning in &outcome.warnings {
                        eprintln!("[manifest] {warning}");
                    }
                    if outcome.nodes.is_empty() {
                        return fail("manifest parsed but produced no nodes".into());
                    }
                    outcome.nodes
                }
                None => match parse_nodes(&text) {
                    Ok(n) => n,
                    Err(e) => return fail(e.to_string()),
                },
            }
        } else {
            match parse_nodes(&text) {
                Ok(n) => n,
                Err(e) => return fail(e.to_string()),
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
    crate::role_infer::resolve_forest_roles(&mut nodes, canvas_width, theme);
    // Cross-node contrast post-pass (I3) runs AFTER role resolution (it keys off
    // the roles I1/I2 set) and before the fallback sizing normalize.
    crate::role_post_pass::post_pass_forest(&mut nodes, canvas_width);
    crate::variable_binding::bind_generated_color_variables(&mut nodes, sink.state());
    normalize_section_roots_for_parent_layout(&mut nodes);
    let node_count = nodes.len();

    // 经 DocSink 发 InsertSubtree。
    let parent_id = match &subtask.parent_frame_id {
        Some(id) => NodeId::new(id.clone()),
        None => NodeId::NONE,
    };
    let applied = apply_command_with_reveal(
        sink,
        EditorCommand::InsertSubtree {
            nodes,
            parent_id,
            page_id: None,
        },
        indicator_epoch,
        reveal_started_ms,
    );
    if !applied {
        return fail("InsertSubtree rejected by document".into());
    }

    SubtaskOutcome {
        id: subtask.id.clone(),
        node_count,
        error: None,
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
            _ => !node.is_container(),
        },
    }
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

    const NODE_JSON: &str = r#"[{"type":"frame","id":"hero-1","name":"Card","x":0,"y":0,"width":1200,"height":200,"children":[{"type":"text","id":"hero-title","content":"Hero","fontSize":18}]}]"#;

    #[test]
    fn run_subtask_ok_applies_insert_subtree() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_JSON.into())]);
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

    #[test]
    fn run_subtask_binds_generated_exact_color_to_document_variable() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            r##"[{"type":"rectangle","id":"card","width":100,"height":50,"fill":[{"type":"solid","color":"#F8FAFC"}]}]"##
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
            r##"[{"type":"rectangle","id":"card","width":100,"height":50,"fill":[{"type":"solid","color":"#FFF8F0"}]}]"##
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
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_JSON.into())]);
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

    /// 离线冒烟：manifest 模式下,stub LLM 输出元素清单 → 解析、修复、
    /// 组装、role 后处理、InsertSubtree 全链路走通。
    /// 对并行 runner 安全:flag 置位期间,其它测试的非清单输出在
    /// `parse_manifest` 返回 `None` 后照常回落 `parse_nodes`。
    #[test]
    fn run_subtask_manifest_mode_builds_elements_end_to_end() {
        let _env = crate::test_support::MANIFEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::env::set_var("OPENPENCIL_MANIFEST", "1");
        let manifest = concat!(
            "{\"el\":\"section\",\"gap\":16,\"role\":\"stats\"}\n",
            "{\"el\":\"heading\",\"in\":1,\"content\":\"Revenue Overview\"}\n",
            "{\"el\":\"stat_card\",\"in\":1,\"label\":\"MRR\",\"value\":\"$48k\"}",
        );
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(manifest.into())]);
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
        std::env::remove_var("OPENPENCIL_MANIFEST");

        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        assert_eq!(outcome.node_count, 1, "one section root");
        let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
            panic!("expected InsertSubtree, got {:?}", sink.applied.last());
        };
        assert_eq!(nodes.len(), 1);
        let jian_ops_schema::node::PenNode::Frame(section) = &nodes[0] else {
            panic!("section root must be a frame");
        };
        assert_eq!(
            section.children.as_ref().map(Vec::len),
            Some(2),
            "heading + stat_card nested under the section"
        );
    }

    /// ab-v9.2 现场:`{"el":"otp_input"}` 全空槽位 → builder 产出一排
    /// 带描边的无子 frame。带显式 paint 的无子 frame 与无子矩形像素
    /// 等价,必须算内容 —— 此前整树被判"空白容器",manifest 被拒后
    /// 重试降级到手搓 raw 路径。
    #[test]
    fn manifest_element_of_stroked_empty_frames_is_content_not_blank() {
        let _env = crate::test_support::MANIFEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::env::set_var("OPENPENCIL_MANIFEST", "1");
        let manifest = concat!(
            "{\"el\":\"section\",\"direction\":\"horizontal\",\"gap\":12}\n",
            "{\"el\":\"otp_input\",\"in\":1,\"length\":6,\"focused_index\":0}",
        );
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(manifest.into())]);
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
        std::env::remove_var("OPENPENCIL_MANIFEST");

        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        assert_eq!(
            outcome.node_count, 1,
            "one section root with the otp element"
        );
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
            r#"[{"type":"frame","id":"sk-root","name":"Skeleton","width":"fill_container","height":"fit_content","children":[{"type":"rectangle","id":"sk-1","width":"fill_container","height":16,"cornerRadius":8},{"type":"rectangle","id":"sk-2","width":205,"height":16,"cornerRadius":8}]}]"#
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
            r#"[{"type":"frame","id":"section-root","name":"Blank","x":0,"y":0,"width":390,"height":112,"children":[]}]"#
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
    fn run_subtask_normalizes_section_root_for_parent_layout() {
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            r#"[{"type":"frame","id":"section-root","name":"Section","x":0,"y":0,"width":390,"height":112,"children":[{"type":"text","id":"title","content":"Pizza","fontSize":18}]}]"#
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
}
