//! `Orchestrator::run()` —— 四阶段编排主轴(spec §4)。
//!
//! 规划 → 画布搭建 → 顺序子 agent → 清理。副作用全经
//! [`DocSink`] / [`LlmClient`]。错误 / abort / 零内容语义见
//! spec §6。

use crate::cleanup::{descendant_count, run_cleanup_passes};
use crate::plan::{build_fallback_plan, parse_plan};
use crate::plan_normalize::normalize;
use crate::prompt::build_orchestrator_prompt;
use crate::scaffold::build_scaffold;
use crate::subagent::run_subtask;
use crate::types::{
    AbortFlag, DesignRequest, DocSink, LlmChunk, LlmClient, OrchestratorError, Progress,
    RunSummary, SubtaskOutcome,
};
use crate::variables::{rollback, seed_commands, snapshot_plan_vars};
use futures::StreamExt;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

/// 设计编排器。S3a 阶段无构造期配置 —— 保留 struct 以便 S3b/S3c
/// 挂选项。
#[derive(Debug, Default, Clone, Copy)]
pub struct Orchestrator;

impl Orchestrator {
    pub fn new() -> Self {
        Orchestrator
    }

    /// 跑一次完整编排。见 spec §4 数据流。
    pub async fn run(
        &self,
        request: DesignRequest,
        sink: &mut dyn DocSink,
        llm: &dyn LlmClient,
        on_progress: &mut dyn FnMut(Progress),
        abort: &AbortFlag,
    ) -> Result<RunSummary, OrchestratorError> {
        // -- 阶段 1:规划 --
        on_progress(Progress::Planning);
        let plan_call = build_orchestrator_prompt(&request, abort.clone());
        let mut plan = match collect_text(llm.call(plan_call)).await {
            Ok(text) => parse_plan(&text).unwrap_or_else(|_| build_fallback_plan(&request)),
            Err(aborted) => {
                if aborted {
                    // abort 在规划阶段:尚未动文档,直接返回。
                    return Err(OrchestratorError::Aborted);
                }
                // 非 abort 失败 → 启发式兜底 plan。
                build_fallback_plan(&request)
            }
        };

        // -- 阶段 1.5:规范化 --
        let norm = normalize(&mut plan, &request);
        let planned_root_id = plan.root_frame.id.clone();

        // -- 进入"已动文档"区,全程 undo batch 包裹 --
        sink.begin_undo_batch();
        let var_snapshot = snapshot_plan_vars(sink, &plan);

        // -- 阶段 2:画布搭建 --
        for cmd in seed_commands(&plan) {
            sink.apply(cmd);
        }
        let scaffold_root_index = sink.state().active_children().len();
        match build_scaffold(&plan, norm.is_mobile) {
            Ok(cmds) => {
                for cmd in cmds {
                    if !sink.apply(cmd) {
                        rollback(sink, &var_snapshot);
                        sink.end_undo_batch();
                        return Err(OrchestratorError::Internal(
                            "scaffold insert rejected by document".into(),
                        ));
                    }
                }
            }
            Err(e) => {
                // scaffold 模板 bug —— 收尾后报内部错误。
                rollback(sink, &var_snapshot);
                sink.end_undo_batch();
                return Err(OrchestratorError::Internal(e));
            }
        }
        let Some(root_id) = sink
            .state()
            .active_children()
            .get(scaffold_root_index)
            .map(|n| n.id_str().to_string())
        else {
            rollback(sink, &var_snapshot);
            sink.end_undo_batch();
            return Err(OrchestratorError::Internal(format!(
                "scaffold root `{planned_root_id}` was not inserted"
            )));
        };
        for subtask in &mut plan.subtasks {
            subtask.parent_frame_id = Some(root_id.clone());
        }
        let scaffold_baseline = descendant_count(sink.state(), &root_id);
        on_progress(Progress::ScaffoldDone);

        // -- 阶段 3:顺序子 agent --
        let mut outcomes: Vec<SubtaskOutcome> = Vec::new();
        let mut aborted_mid = false;
        let mut zero_node_failure = false;
        for subtask in &plan.subtasks {
            if abort.is_set() {
                aborted_mid = true;
                break;
            }
            on_progress(Progress::SubtaskStarted {
                id: subtask.id.clone(),
                label: subtask.label.clone(),
            });
            let outcome = run_subtask(subtask, &plan, &request, llm, sink, abort).await;
            let zero = outcome.node_count == 0;
            let node_count = outcome.node_count;
            let err_msg = outcome.error.clone();
            outcomes.push(outcome);

            // abort 在 run_subtask 期间被置位 —— 优先于零节点判定归
            // abort 路径(否则 mid-stream abort 会被误判为错误路径,
            // 错误地移除 scaffold root 并返回 NoContent 而非 Aborted)。
            if abort.is_set() {
                aborted_mid = true;
                if zero {
                    on_progress(Progress::SubtaskFailed {
                        id: subtask.id.clone(),
                        error: err_msg.unwrap_or_else(|| "aborted".into()),
                    });
                } else {
                    on_progress(Progress::SubtaskDone {
                        id: subtask.id.clone(),
                        node_count,
                    });
                }
                break;
            }
            if zero {
                // 零节点失败(非 abort)—— 停止后续 subtask(spec §6.2)。
                on_progress(Progress::SubtaskFailed {
                    id: subtask.id.clone(),
                    error: err_msg.unwrap_or_default(),
                });
                zero_node_failure = true;
                break;
            }
            on_progress(Progress::SubtaskDone {
                id: subtask.id.clone(),
                node_count,
            });
        }

        // -- 阶段 4:清理 --
        run_cleanup_passes(sink, &plan);
        on_progress(Progress::CleanupDone);

        // -- 阶段 4.5:收尾(spec §6.3 三路径)--
        let zero_content = descendant_count(sink.state(), &root_id) <= scaffold_baseline;
        if zero_content {
            // 错误路径才移除空 scaffold root;abort / 正常零内容只回滚变量。
            if zero_node_failure {
                sink.apply(EditorCommand::DeleteNode {
                    node_id: NodeId::new(root_id.clone()),
                });
            }
            rollback(sink, &var_snapshot);
        }
        sink.end_undo_batch();

        // -- 返回 --
        if zero_content {
            return Err(if aborted_mid {
                OrchestratorError::Aborted
            } else {
                OrchestratorError::NoContent
            });
        }
        let total_nodes = outcomes.iter().map(|o| o.node_count).sum();
        Ok(RunSummary {
            root_frame_id: root_id,
            subtasks: outcomes,
            total_nodes,
        })
    }
}

/// 消费一次 LLM 调用的流 —— 拼接所有 `Text` chunk,丢弃 `Thinking`。
/// `Err(true)` 表示中止,`Err(false)` 表示真实错误。
async fn collect_text(
    mut stream: futures::stream::BoxStream<'static, Result<LlmChunk, crate::types::LlmError>>,
) -> Result<String, bool> {
    let mut text = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(LlmChunk::Text(t)) => text.push_str(&t),
            Ok(LlmChunk::Thinking(_)) => {}
            Err(e) => return Err(e.aborted),
        }
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};

    fn req() -> DesignRequest {
        DesignRequest {
            prompt: "a landing page".into(),
            model: None,
            provider: None,
        }
    }

    const PLAN_JSON: &str = r##"{
      "root_frame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                      "layout": "vertical", "gap": 0, "fill": "#FFFFFF" },
      "subtasks": [
        { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
        { "id": "feat", "label": "Features", "region": { "width": 1200, "height": 400 } }
      ]
    }"##;

    fn node_json(prefix: &str) -> String {
        format!(
            r#"[{{"type":"frame","id":"{prefix}-1","name":"Sec","x":0,"y":0,"width":1200,"height":300,"children":[]}}]"#
        )
    }

    #[test]
    fn run_happy_path_applies_scaffold_and_subtasks() {
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text(PLAN_JSON.into()),
            ScriptResponse::Text(node_json("hero")),
            ScriptResponse::Text(node_json("feat")),
        ]);
        let mut sink = VecDocSink::new();
        let mut events: Vec<Progress> = Vec::new();
        let mut on_progress = |p: Progress| events.push(p);

        let summary = futures::executor::block_on(Orchestrator::new().run(
            req(),
            &mut sink,
            &llm,
            &mut on_progress,
            &AbortFlag::new(),
        ))
        .expect("run ok");

        // root_frame_id 是 InsertSubtree 重映射后的真实 id —— 不是
        // plan 里的 "root" 字面值,只断言它非空。
        assert!(!summary.root_frame_id.is_empty());
        assert_eq!(summary.subtasks.len(), 2);
        assert!(summary.total_nodes >= 2);
        // undo batch 配对。
        assert_eq!(sink.batch_depth, 0);
        // 至少有 scaffold + 两个 subtask 的 InsertSubtree。
        let inserts = sink
            .applied
            .iter()
            .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
            .count();
        assert!(inserts >= 3, "expected >=3 InsertSubtree, got {inserts}");
        assert!(matches!(events.first(), Some(Progress::Planning)));
        assert!(matches!(events.last(), Some(Progress::CleanupDone)));
    }

    #[test]
    fn run_zero_node_subtask_stops_and_errors() {
        // 规划 OK,但第一个 subtask 吐垃圾 → 零节点失败 → 停止。
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text(PLAN_JSON.into()),
            ScriptResponse::Text("the model refused".into()),
        ]);
        let mut sink = VecDocSink::new();
        let mut on_progress = |_p: Progress| {};
        let result = futures::executor::block_on(Orchestrator::new().run(
            req(),
            &mut sink,
            &llm,
            &mut on_progress,
            &AbortFlag::new(),
        ));
        assert!(matches!(result, Err(OrchestratorError::NoContent)));
        // undo batch 仍配对。
        assert_eq!(sink.batch_depth, 0);
    }

    #[test]
    fn run_planning_failure_uses_fallback_plan() {
        // 规划吐垃圾 → fallback plan;subtask 正常 → 成功。
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text("no json here".into()),
            ScriptResponse::Text(node_json("section-1")),
        ]);
        let mut sink = VecDocSink::new();
        let mut on_progress = |_p: Progress| {};
        let summary = futures::executor::block_on(Orchestrator::new().run(
            req(),
            &mut sink,
            &llm,
            &mut on_progress,
            &AbortFlag::new(),
        ))
        .expect("fallback run ok");
        assert!(summary.total_nodes >= 1);
    }
}
