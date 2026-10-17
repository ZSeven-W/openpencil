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
    let text =
        r#"{ "rootFrame": { "id": "r", "name": "P", "width": 1, "height": 1 }, "subtasks": [] }"#;
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
        bleed_hero: false,
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
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
        bleed_hero: false,
        id_prefix: "features".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: Some(vec!["Hero".into(), "About".into()]),
        covers: None,
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
        bleed_hero: false,
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    };
    let json = serde_json::to_string(&st).expect("serialize");
    assert!(
        !json.contains("existingSectionLabels"),
        "existingSectionLabels should be omitted when None, got: {json}"
    );
}
