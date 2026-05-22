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
use op_editor_core::{EditorCommand, NodeId};

/// 执行一个 subtask。总是返回 [`SubtaskOutcome`];调用方据
/// `node_count` 决定继续/停止。
pub async fn run_subtask(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
) -> SubtaskOutcome {
    let fail = |msg: String| SubtaskOutcome {
        id: subtask.id.clone(),
        node_count: 0,
        error: Some(msg),
    };

    // 收集 LLM 文本输出。
    let call_req = build_subagent_prompt(subtask, plan, req, abort.clone());
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
    let nodes = match parse_nodes(&text) {
        Ok(n) => n,
        Err(e) => return fail(e.to_string()),
    };
    let node_count = nodes.len();

    // 经 DocSink 发 InsertSubtree。
    let parent_id = match &subtask.parent_frame_id {
        Some(id) => NodeId::new(id.clone()),
        None => NodeId::NONE,
    };
    let applied = sink.apply(EditorCommand::InsertSubtree { nodes, parent_id });
    if !applied {
        return fail("InsertSubtree rejected by document".into());
    }

    SubtaskOutcome {
        id: subtask.id.clone(),
        node_count,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{OrchestratorPlan, Region, RootFrameSpec};
    use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
    use crate::types::LlmError;
    use futures::executor::block_on;

    fn req() -> DesignRequest {
        DesignRequest {
            prompt: "a page".into(),
            model: None,
            provider: None,
            design_md: None,
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
        }
    }

    const NODE_JSON: &str = r#"[{"type":"frame","id":"hero-1","name":"Card","x":0,"y":0,"width":1200,"height":200,"children":[]}]"#;

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
        ));
        assert_eq!(outcome.node_count, 0);
        assert!(outcome.error.is_some());
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
        ));
        assert_eq!(outcome.node_count, 0);
        assert_eq!(outcome.error.as_deref(), Some("rate limited"));
    }
}
