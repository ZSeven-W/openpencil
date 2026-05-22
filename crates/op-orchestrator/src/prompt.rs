//! Prompt 构造 —— 规划 prompt 与 sub-agent prompt。
//!
//! skill 正文经 `op_ai_skills::resolve_skills` 解析:规划用
//! `Phase::Planning`,sub-agent 用 `Phase::Generation`。两段格式
//! 指令(产 OrchestratorPlan JSON / 产 PenNode JSON 数组)由本
//! 模块附加在 skill 正文之后。
//!
//! 注:格式指令文本是功能性的最小版;逐条对齐 TS
//! `orchestrator-prompt-optimizer.ts` / `orchestrator-sub-agent.ts`
//! 的细则(style-guide 注入、移动端禁 phone-wrapper 等)是后续
//! 细化项。

use crate::plan::{OrchestratorPlan, Subtask};
use crate::types::{AbortFlag, CallRequest, DesignRequest};
use std::time::Duration;

const PLANNING_TIMEOUT: Duration = Duration::from_secs(300);
const SUBAGENT_TIMEOUT: Duration = Duration::from_secs(420);

/// 规划阶段要求模型产出的 JSON 形状说明。
const PLAN_FORMAT: &str = r##"
Respond with a single JSON object describing the design plan:
{
  "rootFrame": { "id": "root", "name": "<name>", "width": <px>, "height": <px>,
                 "layout": "vertical", "gap": <px>,
                 "fill": [{ "type": "solid", "color": "#RRGGBB" }] },
  "styleGuideName": "<style-guide-name or omit>",
  "subtasks": [
    { "id": "<kebab-id>", "label": "<human label>",
      "region": { "width": <px>, "height": <px> } }
  ]
}
Each subtask is one visual section. Use 1-6 subtasks. Output ONLY the JSON object."##;

/// sub-agent 阶段要求模型产出的 JSON 形状说明。
const NODE_FORMAT: &str = r#"
Respond with a JSON array of canonical PenNode objects for THIS section only.
Each node is tagged by "type" (frame/group/rectangle/ellipse/line/polygon/path/
text/text_input/image/icon_font). Frames/groups nest children via "children".
Example: [ { "type": "frame", "id": "<prefix>-1", "name": "Card", "x": 0, "y": 0,
"width": 1200, "height": 200, "children": [] } ]
ALL field names are camelCase: cornerRadius, fontSize, fontWeight, justifyContent,
alignItems, clipContent. Geometry fields are x, y, width, height. Never snake_case.
Output ONLY the JSON array."#;

/// 把解析出的 skill 正文 join 成一段。
fn skill_preamble(phase: op_ai_skills::Phase, message: &str) -> String {
    let ctx =
        op_ai_skills::resolve_skills(phase, message, &op_ai_skills::ResolveOptions::default());
    ctx.skills
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 规划阶段的 LLM 调用输入。
pub fn build_orchestrator_prompt(req: &DesignRequest, abort: AbortFlag) -> CallRequest {
    let mut system_prompt = skill_preamble(op_ai_skills::Phase::Planning, &req.prompt);
    system_prompt.push_str("\n\n");
    system_prompt.push_str(PLAN_FORMAT);
    CallRequest {
        system_prompt,
        user_prompt: req.prompt.clone(),
        model: req.model.clone(),
        provider: req.provider.clone(),
        timeout: PLANNING_TIMEOUT,
        abort,
    }
}

/// 单个 sub-agent 的 LLM 调用输入。
pub fn build_subagent_prompt(
    subtask: &Subtask,
    _plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
) -> CallRequest {
    let mut system_prompt = skill_preamble(op_ai_skills::Phase::Generation, &subtask.label);
    system_prompt.push_str("\n\n");
    system_prompt.push_str(NODE_FORMAT);

    let user_prompt = format!(
        "Overall design: {}\n\nGenerate the section \"{}\" \
         (区块 id 前缀 `{}-`). Target region: {:.0}x{:.0} px.\nPalette: (default)",
        req.prompt, subtask.label, subtask.id_prefix, subtask.region.width, subtask.region.height,
    );

    CallRequest {
        system_prompt,
        user_prompt,
        model: req.model.clone(),
        provider: req.provider.clone(),
        timeout: SUBAGENT_TIMEOUT,
        abort,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Region, RootFrameSpec};

    fn req() -> DesignRequest {
        DesignRequest {
            prompt: "a pricing page".into(),
            model: Some("claude".into()),
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

    #[test]
    fn orchestrator_prompt_carries_request_and_format() {
        let cr = build_orchestrator_prompt(&req(), AbortFlag::new());
        assert!(cr.user_prompt.contains("a pricing page"));
        assert!(cr.system_prompt.contains("subtasks"));
        assert_eq!(cr.model.as_deref(), Some("claude"));
    }

    #[test]
    fn subagent_prompt_carries_subtask_and_node_format() {
        let st = Subtask {
            id: "hero".into(),
            label: "Hero".into(),
            region: Region {
                width: 1200.0,
                height: 400.0,
            },
            id_prefix: "hero".into(),
            parent_frame_id: Some("root".into()),
        };
        let cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new());
        assert!(cr.user_prompt.contains("Hero"));
        assert!(cr.user_prompt.contains("hero-"));
        assert!(cr.system_prompt.contains("PenNode"));
    }
}
