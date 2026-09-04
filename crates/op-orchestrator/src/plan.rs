//! `OrchestratorPlan` —— 规划阶段的产物。
//!
//! 规划 LLM 调用产出一段 JSON,`parse_plan` 解析它;调用失败 / 解析
//! 失败时 `build_fallback_plan` 给一个启发式的可跑 plan(对齐 TS
//! `buildFallbackPlanFromPrompt`)。

use crate::design_type::{detect_design_type, DesignType, DesignTypePreset};
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
    /// The current sibling after which a later interactive retry should land.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insert_after_sibling_id: Option<String>,
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
    /// Set by the retry ladder (`concurrent::run_subtask_retry_ladder`) to
    /// echo a reason back into the SAME-tier retry prompt instead of
    /// silently narrowing the skill set — see [`RetryFeedback`] for the two
    /// cases and their exact wording (`prompt.rs`'s injection). Execution-
    /// only, like `generated_root_id`: never persisted, never present on a
    /// freshly planned subtask.
    #[serde(skip)]
    pub retry_feedback: Option<RetryFeedback>,
}

/// Why a subtask is being retried WITH its rejection reason echoed into the
/// prompt (as opposed to a plain zero-node retry, which carries no
/// feedback) — drives the exact wording `prompt.rs`'s `retry_feedback`
/// block injects, so the model can tell "geometry problem in real content
/// you should keep" apart from "structural problem caught before your
/// content even landed".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryFeedback {
    /// `orchestration_self_check` fatally rejected otherwise-parsed content
    /// BEFORE insertion — carries the self-check `failure_message()`.
    SelfCheck(String),
    /// The REAL resolved layout of an already-INSERTED subtree proved a
    /// structural violation (`geometry_validation::geometry_diagnostics_for_roots`,
    /// the `geometry_echo` step) — carries the joined diagnostic lines.
    Geometry(String),
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

    if let Some(context) = req
        .continuation_context
        .as_ref()
        .filter(|context| !context.screen_names.is_empty())
    {
        let fill = context
            .background_color
            .clone()
            .unwrap_or_else(|| "#FFFFFF".into());
        let subtasks = context
            .screen_names
            .iter()
            .enumerate()
            .map(|(index, screen_name)| {
                let id = format!("continuation-screen-{}", index + 1);
                Subtask {
                    id: id.clone(),
                    label: screen_name.clone(),
                    region: Region {
                        width: context.screen_width,
                        height: context.screen_height,
                    },
                    id_prefix: id,
                    parent_frame_id: None,
                    insert_after_sibling_id: None,
                    elements: Some(format!(
                        "the complete {screen_name} screen, continuing the existing product; reuse its established design system and shared navigation"
                    )),
                    screen: Some(screen_name.clone()),
                    generated_root_id: None,
                    existing_section_labels: None,
                    retry_feedback: None,
                }
            })
            .collect();
        return OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "continuation".into(),
                name: "Continuation".into(),
                width: context.screen_width,
                height: context.screen_height,
                layout: Some("vertical".into()),
                gap: Some(0.0),
                padding: Some(0.0),
                fill: Some(vec![PlanFill {
                    kind: "solid".into(),
                    color: fill,
                }]),
            },
            subtasks,
            style_guide_name: None,
        };
    }

    let preset = detect_design_type(&req.prompt);
    if preset.type_ == DesignType::Slides {
        return build_fallback_deck_plan(req, preset);
    }
    if preset.type_ == DesignType::Card {
        return crate::plan_fallback_card::build_fallback_card_plan(req, preset);
    }
    if preset.type_ == DesignType::MobileScreen {
        let (width, height) = explicit_mobile_size(&req.prompt)
            .unwrap_or((preset.width, preset.root_height.max(preset.height)));
        let top_h = (height * 0.24).round().clamp(140.0, 220.0);
        let main_h = (height - top_h).max(320.0);
        return OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "page".into(),
                name: "Page".into(),
                width,
                height,
                layout: Some("vertical".into()),
                gap: Some(0.0),
                padding: Some(0.0),
                fill: Some(vec![PlanFill {
                    kind: "solid".into(),
                    color: "#FFFFFF".into(),
                }]),
            },
            subtasks: vec![
                Subtask {
                    id: "top-summary".into(),
                    label: "Top Summary".into(),
                    region: Region {
                        width,
                        height: top_h,
                    },
                    id_prefix: "top-summary".into(),
                    parent_frame_id: Some("page".into()),
                    insert_after_sibling_id: None,
                    elements: Some(
                        "the screen's header / context for this product (title or greeting, \
                         and the key top-level action[s] this app needs); no status bar. \
                         Choose what fits the prompt — not a fixed delivery-app header."
                            .into(),
                    ),
                    screen: None,
                    generated_root_id: None,
                    existing_section_labels: None,
                    retry_feedback: None,
                },
                Subtask {
                    id: "main-content".into(),
                    label: "Main Content".into(),
                    region: Region {
                        width,
                        height: main_h,
                    },
                    id_prefix: "main-content".into(),
                    parent_frame_id: Some("page".into()),
                    insert_after_sibling_id: None,
                    elements: Some(
                        "this screen's primary content for the product — the main job-to-be-done \
                         and whatever modules genuinely fit it; do not repeat the top summary. \
                         Vary the composition per prompt, not a fixed search + banner + card stack."
                            .into(),
                    ),
                    screen: None,
                    generated_root_id: None,
                    existing_section_labels: None,
                    retry_feedback: None,
                },
            ],
            style_guide_name: None,
        };
    }

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
                insert_after_sibling_id: None,
                elements: None,
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
                retry_feedback: None,
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

/// Heuristic deck fallback — the shape a deck plan MUST have, built without
/// the planning LLM.
///
/// Before this branch existed, a deck request whose planning call failed fell
/// through to the generic 1200-wide "1-3 stacked sections" skeleton below: the
/// user asked for a presentation and got one scrolling page. The three things
/// that make a deck a deck are all decided here:
///
/// 1. every board is the projector-shaped 1920x1080 preset, never 1200x0;
/// 2. ONE subtask per slide, each carrying its OWN `screen` label — that label
///    is what makes `plan_normalize` fan the subtasks into separate root
///    frames instead of collapsing them onto one board;
/// 3. the slide count comes from the request when it states one, and from the
///    prompt's size otherwise — a fallback must not re-introduce the fixed
///    page count the corpus was just de-anchored from.
fn build_fallback_deck_plan(req: &DesignRequest, preset: DesignTypePreset) -> OrchestratorPlan {
    let count = explicit_slide_count(&req.prompt).unwrap_or(
        // No count in the request: scale with how much the prompt describes,
        // the same signal the generic branch uses for its section count.
        match req.prompt.chars().count() {
            0..=80 => 5,
            81..=200 => 6,
            _ => 8,
        },
    );

    let subtasks = fallback_slide_titles(count)
        .into_iter()
        .enumerate()
        .map(|(i, title)| {
            let id = format!("slide-{}", i + 1);
            Subtask {
                id: id.clone(),
                label: title.clone(),
                region: Region {
                    width: preset.width,
                    height: preset.root_height,
                },
                id_prefix: id,
                // Left to `plan_normalize`, which rewrites it per screen group.
                parent_frame_id: None,
                insert_after_sibling_id: None,
                elements: Some(fallback_slide_elements(&title).to_string()),
                screen: Some(title),
                generated_root_id: None,
                existing_section_labels: None,
                retry_feedback: None,
            }
        })
        .collect();

    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "deck".into(),
            name: "Deck".into(),
            width: preset.width,
            height: preset.root_height,
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

/// A generic running order that grows with the count instead of repeating a
/// fixed six-step outline: cover, optional agenda, N key points, optional
/// evidence slide, closing. Every title is distinct, which the `screen`
/// tagging depends on — two slides sharing a label would land on one board.
fn fallback_slide_titles(count: usize) -> Vec<String> {
    let mut titles: Vec<String> = vec!["Cover".into()];
    let mut tail: Vec<String> = vec!["Closing".into()];
    if count >= 5 {
        tail.insert(0, "Evidence".into());
    }
    if count >= 4 {
        titles.push("Agenda".into());
    }
    let middle = count.saturating_sub(titles.len() + tail.len());
    titles.extend((1..=middle).map(|i| format!("Key Point {i}")));
    titles.extend(tail);
    titles
}

/// Per-slide `elements` in the deck corpus's own terms — one takeaway plus its
/// supporting parts, never a list of paragraphs.
fn fallback_slide_elements(title: &str) -> &'static str {
    match title {
        "Cover" => {
            "the deck's title on one line, a one-line subtitle, and a meta row \
             with the presenter and the date"
        }
        "Agenda" => "a takeaway title plus 3-5 numbered agenda entries, one short line each",
        "Evidence" => {
            "a takeaway title plus the figures that support it — 3 KPI cards \
             (value + unit + label) or one chart placeholder with its insight \
             written out"
        }
        "Closing" => "a closing headline, one supporting line, and a contact / next-step block",
        _ => {
            "this slide's ONE takeaway as the title, plus the supporting \
             content that makes the point — short phrases, never paragraphs"
        }
    }
}

/// An explicitly requested slide count ("12 页", "10-slide", "8 slides").
///
/// Bounded to a plausible deck size so an unrelated number in the prompt (a
/// year, a pixel size, a price) cannot turn into a 2026-slide plan — the unit
/// word right after the digits is what qualifies it as a slide count at all.
fn explicit_slide_count(prompt: &str) -> Option<usize> {
    const UNITS: [&str; 5] = ["slides", "slide", "pages", "page", "页"];
    let lower = prompt.to_lowercase();
    let bytes = lower.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        // "10-slide" / "12 页" / "8slides" — only separators may intervene.
        let mut unit_start = i;
        while unit_start < bytes.len() && matches!(bytes[unit_start], b' ' | b'-' | b'_' | b'\t') {
            unit_start += 1;
        }
        if UNITS
            .iter()
            .any(|unit| lower[unit_start..].starts_with(unit))
        {
            if let Ok(count) = lower[start..i].parse::<usize>() {
                if (2..=30).contains(&count) {
                    return Some(count);
                }
            }
        }
    }
    None
}

fn explicit_mobile_size(prompt: &str) -> Option<(f64, f64)> {
    let normalized = prompt.replace('×', "x").to_lowercase();
    let bytes = normalized.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b != b'x' {
            continue;
        }

        let mut left_end = i;
        while left_end > 0 && bytes[left_end - 1].is_ascii_whitespace() {
            left_end -= 1;
        }
        let mut left_start = left_end;
        while left_start > 0 && bytes[left_start - 1].is_ascii_digit() {
            left_start -= 1;
        }

        let mut right_start = i + 1;
        while right_start < bytes.len() && bytes[right_start].is_ascii_whitespace() {
            right_start += 1;
        }
        let mut right_end = right_start;
        while right_end < bytes.len() && bytes[right_end].is_ascii_digit() {
            right_end += 1;
        }

        if left_start == left_end || right_start == right_end {
            continue;
        }
        let Ok(width) = normalized[left_start..left_end].parse::<f64>() else {
            continue;
        };
        let Ok(height) = normalized[right_start..right_end].parse::<f64>() else {
            continue;
        };
        if (240.0..=520.0).contains(&width) && (480.0..=1200.0).contains(&height) {
            return Some((width, height));
        }
    }
    None
}

#[cfg(test)]
#[path = "plan_fallback_deck_tests.rs"]
mod fallback_deck_tests;

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
            continuation_context: None,
            append_context: None,
            validation_enabled: true,

            visual_ref_enabled: false,
            pinned_style_guide: None,
            reference_skeleton: None,
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

    #[test]
    fn fallback_plan_honors_mobile_type_and_explicit_size() {
        let mobile = build_fallback_plan(&req("Design a 390x844 mobile food delivery home screen"));
        assert_eq!(mobile.root_frame.width, 390.0);
        assert_eq!(mobile.root_frame.height, 844.0);
        assert_eq!(mobile.subtasks.len(), 2);
        assert_eq!(mobile.subtasks[0].label, "Top Summary");
        assert_eq!(mobile.subtasks[1].label, "Main Content");
    }

    #[test]
    fn fallback_continuation_keeps_promised_screens_and_existing_artboard() {
        let mut request = req("继续生成 星图、观测计划、我的3个界面");
        request.continuation_context = Some(crate::types::ContinuationContext {
            screen_width: 390.0,
            screen_height: 844.0,
            background_color: Some("#050508".into()),
            screen_names: vec!["星图".into(), "观测计划".into(), "我的".into()],
        });

        let plan = build_fallback_plan(&request);

        assert_eq!(
            (plan.root_frame.width, plan.root_frame.height),
            (390.0, 844.0)
        );
        assert_eq!(
            plan.root_frame.first_solid_hex().as_deref(),
            Some("#050508")
        );
        assert_eq!(
            plan.subtasks
                .iter()
                .filter_map(|subtask| subtask.screen.as_deref())
                .collect::<Vec<_>>(),
            ["星图", "观测计划", "我的"]
        );
        assert!(plan
            .subtasks
            .iter()
            .all(|subtask| subtask.label != "Section 1"));
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
            insert_after_sibling_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
            retry_feedback: None,
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
            insert_after_sibling_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: Some(vec!["Hero".into(), "About".into()]),
            retry_feedback: None,
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
            insert_after_sibling_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
            retry_feedback: None,
        };
        let json = serde_json::to_string(&st).expect("serialize");
        assert!(
            !json.contains("existingSectionLabels"),
            "existingSectionLabels should be omitted when None, got: {json}"
        );
    }
}
