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
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

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

    // 解析成 PenNode 树。
    let mut nodes = match parse_nodes(&text) {
        Ok(n) => n,
        Err(e) => return fail(e.to_string()),
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
    crate::role_post_pass::post_pass_forest(&mut nodes);
    normalize_section_roots_for_parent_layout(&mut nodes);
    let node_count = nodes.len();

    // 经 DocSink 发 InsertSubtree。
    let parent_id = match &subtask.parent_frame_id {
        Some(id) => NodeId::new(id.clone()),
        None => NodeId::NONE,
    };
    let applied = sink.apply(EditorCommand::InsertSubtree {
        nodes,
        parent_id,
        page_id: None,
    });
    if !applied {
        return fail("InsertSubtree rejected by document".into());
    }

    SubtaskOutcome {
        id: subtask.id.clone(),
        node_count,
        error: None,
    }
}

fn is_blank_container_forest(nodes: &[PenNode]) -> bool {
    !nodes.iter().any(has_content_node)
}

fn has_content_node(node: &PenNode) -> bool {
    match node.children() {
        Some(children) if !children.is_empty() => children.iter().any(has_content_node),
        _ => !node.is_container(),
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
