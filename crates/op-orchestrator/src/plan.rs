//! `OrchestratorPlan` —— 规划阶段的产物。
//!
//! 规划 LLM 调用产出一段 JSON,`parse_plan` 解析它;调用失败 / 解析
//! 失败时 `build_fallback_plan` 给一个启发式的可跑 plan(对齐 TS
//! `buildFallbackPlanFromPrompt`)。

use crate::types::DesignRequest;
use serde::{Deserialize, Serialize};

/// 根 frame 的一个 fill —— 对齐 TS canonical `[{type,color}]`。
/// 规划只需 solid 色;非 solid 项保留 `color` 字段(可能为空)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanFill {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub color: String,
}

/// 一个 subtask 区域的尺寸。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub width: f64,
    pub height: f64,
}

/// 根 frame 规格。字段对齐规划语料 `decomposition.md` 的 `rootFrame`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// TS canonical fill 数组 `[{type:"solid",color:"#hex"}]`。
    #[serde(default)]
    pub fill: Option<Vec<PlanFill>>,
}

impl RootFrameSpec {
    /// fill 数组里首个 `type == "solid"` 的颜色;无则 `None`。
    pub fn first_solid_hex(&self) -> Option<String> {
        self.fill
            .as_ref()?
            .iter()
            .find(|f| f.kind == "solid" && !f.color.is_empty())
            .map(|f| f.color.clone())
    }
}

/// 一个生成子任务 —— 对应设计里的一个区块。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    /// 本区块包含的元素描述 —— port of TS `SubTask.elements`。
    #[serde(default)]
    pub elements: Option<String>,
    /// 对应的屏幕 / 页面名 —— port of TS `SubTask.screen`。
    #[serde(default)]
    pub screen: Option<String>,
    /// sub-agent 运行后记录下的顶层节点 id —— port of TS
    /// `SubTask.generatedRootId`。仪器由 run.rs 填写;规划阶段为 None。
    #[serde(skip)]
    pub generated_root_id: Option<String>,
    /// 从 AppendContext 传播而来 —— 告知 sub-agent 哪些已有区块不要重复。
    /// port of TS `SubTask.existingSectionLabels` (`ai-types.ts:134`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub existing_section_labels: Option<Vec<String>>,
}

/// 规划阶段的完整产物。字段对齐规划语料 `decomposition.md`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorPlan {
    #[serde(rename = "rootFrame")]
    pub root_frame: RootFrameSpec,
    pub subtasks: Vec<Subtask>,
    /// 规划选中的 style guide 名(catalog 引用)。S3b-1a 写进 plan
    /// 但暂无消费方(变量播种入眠、sub-agent 注入推迟)。
    #[serde(rename = "styleGuideName", default)]
    pub style_guide_name: Option<String>,
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
                elements: None,
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
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
            fill: Some(vec![PlanFill {
                kind: "solid".into(),
                color: "#FFFFFF".into(),
            }]),
        },
        subtasks,
        style_guide_name: None,
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
            design_md: None,
            concurrency: 1,
            append_context: None,
        }
    }

    #[test]
    fn parse_plan_reads_ts_native_shape() {
        // decomposition.md FORMAT 行的形状:rootFrame / fill 数组 / styleGuideName
        let text = r##"```json
{
  "rootFrame": { "id": "page", "name": "Page", "width": 1200, "height": 0,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#0A0F1C" }] },
  "styleGuideName": "terminal-minimal-dark",
  "subtasks": [
    { "id": "hero", "label": "Hero", "elements": "headline, CTA",
      "region": { "width": 1200, "height": 560 } }
  ]
}
```"##;
        let plan = parse_plan(text).expect("parse");
        assert_eq!(plan.root_frame.id, "page");
        assert_eq!(
            plan.style_guide_name.as_deref(),
            Some("terminal-minimal-dark")
        );
        // fill 数组 → first_solid_hex 取首个 solid 色
        assert_eq!(
            plan.root_frame.first_solid_hex().as_deref(),
            Some("#0A0F1C")
        );
        // 未知字段 elements 被 serde 忽略,不报错
        assert_eq!(plan.subtasks.len(), 1);
    }

    #[test]
    fn parse_plan_rejects_no_subtasks() {
        let text = r#"{ "rootFrame": { "id": "r", "name": "P", "width": 1, "height": 1 }, "subtasks": [] }"#;
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

    // ── Task A1: Subtask.existing_section_labels ──────────────────────────────

    /// Subtask accepts existing_section_labels: None without breaking compilation.
    #[test]
    fn subtask_existing_section_labels_none_compiles() {
        let st = Subtask {
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
        };
        assert!(st.existing_section_labels.is_none());
    }

    /// Subtask accepts a populated existing_section_labels vec.
    #[test]
    fn subtask_existing_section_labels_some_compiles() {
        let st = Subtask {
            id: "features".into(),
            label: "Features".into(),
            region: Region {
                width: 1200.0,
                height: 400.0,
            },
            id_prefix: "features".into(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: Some(vec!["Hero".into(), "About".into()]),
        };
        let labels = st.existing_section_labels.as_ref().unwrap();
        assert_eq!(labels[0], "Hero");
        assert_eq!(labels[1], "About");
    }

    /// Subtask with existing_section_labels = Some([]) serializes without the field.
    #[test]
    fn subtask_existing_section_labels_omitted_when_none() {
        let st = Subtask {
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
        };
        let json = serde_json::to_string(&st).expect("serialize");
        assert!(
            !json.contains("existingSectionLabels"),
            "existingSectionLabels should be omitted when None, got: {json}"
        );
    }
}
