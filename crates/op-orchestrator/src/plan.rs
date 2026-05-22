//! `OrchestratorPlan` —— 规划阶段的产物。
//!
//! 规划 LLM 调用产出一段 JSON,`parse_plan` 解析它;调用失败 / 解析
//! 失败时 `build_fallback_plan` 给一个启发式的可跑 plan(对齐 TS
//! `buildFallbackPlanFromPrompt`)。

use crate::types::DesignRequest;
use serde::Deserialize;
use std::collections::BTreeMap;

/// 一个 subtask 区域的尺寸。
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Region {
    pub width: f64,
    pub height: f64,
}

/// 根 frame 规格。
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RootFrameSpec {
    pub id: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub gap: Option<f64>,
    #[serde(default)]
    pub padding: Option<f64>,
    #[serde(default)]
    pub fill: Option<String>,
}

/// 一个生成子任务 —— 对应设计里的一个区块。
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Subtask {
    pub id: String,
    pub label: String,
    pub region: Region,
    /// 规范化后赋值 = `id`。
    #[serde(default)]
    pub id_prefix: String,
    /// 规范化后赋值 = 根 frame id。
    #[serde(default)]
    pub parent_frame_id: Option<String>,
}

/// 设计系统提示 —— 调色板等。
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct StyleGuide {
    /// `名称 -> hex`,阶段 2 据此 seed `$color-*` 文档变量。
    #[serde(default)]
    pub palette: BTreeMap<String, String>,
}

/// 规划阶段的完整产物。
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OrchestratorPlan {
    pub root_frame: RootFrameSpec,
    pub subtasks: Vec<Subtask>,
    #[serde(default)]
    pub style_guide: Option<StyleGuide>,
}

/// plan 解析错误。
#[derive(Debug, Clone)]
pub struct PlanParseError(pub String);

impl std::fmt::Display for PlanParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "plan parse error: {}", self.0)
    }
}

/// 从规划 LLM 的文本输出里抽出 plan JSON 并反序列化。
///
/// 容忍 ```json 围栏 + 前后散文 —— 抽第一个平衡的 `{...}`。
/// `subtasks` 为空视为解析失败。
pub fn parse_plan(text: &str) -> Result<OrchestratorPlan, PlanParseError> {
    let json =
        extract_json_object(text).ok_or_else(|| PlanParseError("no JSON object found".into()))?;
    let plan: OrchestratorPlan =
        serde_json::from_str(json).map_err(|e| PlanParseError(format!("deserialize: {e}")))?;
    if plan.subtasks.is_empty() {
        return Err(PlanParseError("plan has no subtasks".into()));
    }
    Ok(plan)
}

/// 抽第一个平衡的 `{...}`(忽略字符串内的花括号)。
fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for i in start..bytes.len() {
        let c = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// 启发式兜底 plan —— 规划 LLM 不可用 / 解析失败时用,保证编排器
/// 仍能跑出点东西。对齐 TS `buildFallbackPlanFromPrompt`:固定宽度
/// 根 frame + 按 prompt 规模切 1-3 个等高区块。
pub fn build_fallback_plan(req: &DesignRequest) -> OrchestratorPlan {
    const WIDTH: f64 = 1200.0;
    const SECTION_HEIGHT: f64 = 360.0;

    // prompt 越长 → 区块越多,封顶 3(单屏骨架不做更复杂的切分)。
    let section_count = match req.prompt.chars().count() {
        0..=80 => 1,
        81..=200 => 2,
        _ => 3,
    };

    let subtasks: Vec<Subtask> = (0..section_count)
        .map(|i| {
            let id = format!("section-{}", i + 1);
            Subtask {
                id: id.clone(),
                label: format!("Section {}", i + 1),
                region: Region {
                    width: WIDTH,
                    height: SECTION_HEIGHT,
                },
                id_prefix: id,
                parent_frame_id: None,
            }
        })
        .collect();

    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Design".into(),
            width: WIDTH,
            height: SECTION_HEIGHT * section_count as f64,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: Some(0.0),
            fill: Some("#FFFFFF".into()),
        },
        subtasks,
        style_guide: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(prompt: &str) -> DesignRequest {
        DesignRequest {
            prompt: prompt.into(),
            model: None,
            provider: None,
        }
    }

    #[test]
    fn parse_plan_reads_fenced_json() {
        let text = r#"Here is the plan:
```json
{
  "root_frame": { "id": "root", "name": "Page", "width": 1200, "height": 800 },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } }
  ]
}
```
done."#;
        let plan = parse_plan(text).expect("parse");
        assert_eq!(plan.root_frame.id, "root");
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].id, "hero");
    }

    #[test]
    fn parse_plan_rejects_no_subtasks() {
        let text = r#"{ "root_frame": { "id": "r", "name": "P", "width": 1, "height": 1 }, "subtasks": [] }"#;
        assert!(parse_plan(text).is_err());
    }

    #[test]
    fn parse_plan_rejects_garbage() {
        assert!(parse_plan("the model refused to answer").is_err());
    }

    #[test]
    fn fallback_plan_is_usable() {
        let short = build_fallback_plan(&req("a button"));
        assert_eq!(short.subtasks.len(), 1);
        assert!(short.root_frame.width > 0.0);

        let long = build_fallback_plan(&req(&"x".repeat(300)));
        assert_eq!(long.subtasks.len(), 3);
        assert!(!long.subtasks.is_empty());
    }
}
