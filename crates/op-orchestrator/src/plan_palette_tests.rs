use super::*;
use crate::plan::{parse_plan, OrchestratorPlan};
use crate::plan_repair::{finalize_plan, parse_orchestrator_response, repair_plan_object};
use crate::test_support::VecDocSink;
use crate::types::{DesignRequest, DocSink};
use crate::variables::{seed_commands, snapshot_plan_vars};
use jian_ops_schema::node::PenNode;
use serde_json::json;

fn request() -> DesignRequest {
    DesignRequest {
        prompt: "A delivery detail screen in dark navy and warm orange".into(),
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

fn plan_json() -> Value {
    json!({
        "rootFrame": {"id":"page","name":"Delivery","width":375,"height":812,
            "fill":[{"type":"solid","color":"#0F1B2D"}]},
        "subtasks":[{"id":"map","label":"Map","region":{"width":375,"height":340}}],
        "styleGuideName":"music-dark-mobile",
        "palette":{"background":"#0f1b2d","primary":"#f97316","primaryForeground":"#0f1b2d"}
    })
}

fn delivery_plan() -> OrchestratorPlan {
    parse_plan(&plan_json().to_string()).expect("valid palette plan")
}

fn seed(sink: &mut VecDocSink, plan: &OrchestratorPlan) {
    let snap = snapshot_plan_vars(sink, plan);
    for command in seed_commands(plan, &snap) {
        sink.apply(command);
    }
}

#[test]
fn named_roles_are_normalized_and_invalid_roles_cannot_change_status_colors() {
    let palette = PlanPalette::from_value(&json!({
        "primary":" #f90 ","primaryForeground":"#102030","background":"navy",
        "surface":42,"text":"$--foreground","mutedText":"#00000080",
        "border":"#边框","success":"#F97316","--color-warning":"#F97316"
    }))
    .expect("the valid named roles survive");
    assert_eq!(
        palette.token_overrides(),
        vec![
            ("--primary", "#FF9900".into()),
            ("--primary-foreground", "#102030".into())
        ]
    );
    assert_eq!(PlanPalette::from_value(&json!({"primary":"orange"})), None);
    assert_eq!(PlanPalette::from_value(&json!(["#F97316"])), None);
}

#[test]
fn legacy_plans_stay_compatible_and_malformed_palette_does_not_break_plan() {
    let mut source = plan_json();
    source.as_object_mut().unwrap().remove("palette");
    let legacy = parse_plan(&source.to_string()).unwrap();
    assert!(legacy.palette.is_none());
    assert!(serde_json::to_value(&legacy)
        .unwrap()
        .get("palette")
        .is_none());

    source["palette"] = json!({"primary":"#F97316","surface":false});
    let parsed = parse_plan(&source.to_string()).unwrap();
    assert_eq!(parsed.palette.unwrap().primary.as_deref(), Some("#F97316"));
    source["palette"] = json!("orange");
    assert!(parse_plan(&source.to_string()).unwrap().palette.is_none());
}

#[test]
fn declared_palette_overrides_catalog_in_both_modes_without_touching_status_or_chart() {
    let mut baseline = VecDocSink::new();
    let mut plan = delivery_plan();
    let declared = plan.palette.take();
    seed(&mut baseline, &plan);
    let mut sink = VecDocSink::new();
    plan.palette = declared;
    seed(&mut sink, &plan);
    let vars = serde_json::to_value(sink.state.doc.variables.as_ref().unwrap()).unwrap();
    for entry in vars["--primary"]["value"].as_array().unwrap() {
        assert_eq!(entry["value"], "#F97316");
    }
    for entry in vars["--primary-foreground"]["value"].as_array().unwrap() {
        assert_eq!(entry["value"], "#0F1B2D");
    }
    for name in [
        "--color-success",
        "--color-warning",
        "--color-error",
        "--chart-6",
    ] {
        assert_eq!(
            sink.state.doc.variables.as_ref().unwrap()[name],
            baseline.state.doc.variables.as_ref().unwrap()[name],
            "{name} must retain its original semantic color"
        );
    }
}

#[test]
fn existing_brand_variables_keep_ownership_even_with_a_declared_plan_palette() {
    let mut sink = VecDocSink::new();
    let original = json!({"type":"color","value":[
        {"value":"#D946EF","theme":{"Mode":"Light"}},
        {"value":"#E879F9","theme":{"Mode":"Dark"}}
    ]});
    sink.state.doc.variables = Some(
        [(
            "--primary".into(),
            serde_json::from_value(original.clone()).unwrap(),
        )]
        .into(),
    );
    seed(&mut sink, &delivery_plan());
    let vars = serde_json::to_value(sink.state.doc.variables.as_ref().unwrap()).unwrap();
    assert_eq!(vars["--primary"], original);
}

#[test]
fn repaired_plans_keep_palette_and_canvas_uses_its_declared_background() {
    let mut source = plan_json();
    source["rootFrame"]["width"] = json!("375");
    source["rootFrame"]["fill"] = json!("#000000");
    let repaired = repair_plan_object(&source, &request()).unwrap();
    assert_eq!(
        repaired.palette.as_ref().unwrap().primary.as_deref(),
        Some("#F97316")
    );
    assert_eq!(
        repaired.root_frame.first_solid_hex().as_deref(),
        Some("#0F1B2D")
    );
}

#[test]
fn design_md_precedence_clears_model_palette() {
    let mut plan = delivery_plan();
    let spec: jian_ops_schema::DesignMdSpec = serde_json::from_value(json!({
        "raw":"", "visualTheme":"light",
        "colorPalette":[{"name":"Canvas","hex":"#F0F4F8","role":"page background"}]
    }))
    .unwrap();
    let finalized = finalize_plan(&mut plan, None, Some(&spec));
    assert!(finalized.palette.is_none());
    assert_eq!(
        finalized.style_guide_name.as_deref(),
        Some("design-md-custom")
    );
    assert_eq!(
        finalized.root_frame.first_solid_hex().as_deref(),
        Some("#F0F4F8")
    );
}

#[test]
fn shared_palette_keeps_generated_cta_and_owned_map_on_the_same_brand_token() {
    let mut sink = VecDocSink::new();
    seed(&mut sink, &delivery_plan());
    let mut nodes: Vec<PenNode> = serde_json::from_value(json!([{
        "type":"frame","id":"cta","name":"Call courier","width":44,"height":44,
        "fill":[{"type":"solid","color":"#F97316"}]
    }]))
    .unwrap();
    crate::variable_binding::bind_generated_color_variables(&mut nodes, &sink.state);
    assert_eq!(
        serde_json::to_value(&nodes[0]).unwrap()["fill"][0]["color"],
        "$--primary"
    );
    let map = crate::map_templates::children(
        "map",
        327.0,
        340.0,
        crate::role_defaults::Theme::Dark,
        crate::map_templates::MapTemplate::Delivery,
    );
    let route = map.iter().find(|node| node["name"] == "map-route").unwrap();
    assert_eq!(route["stroke"]["fill"][0]["color"], "$--primary");
    assert_eq!(
        sink.state
            .resolve_color_variable_hex("--primary")
            .as_deref(),
        Some("#F97316")
    );
    // The token survives a save/load round trip and a theme switch.
    let doc = serde_json::to_value(&sink.state.doc).unwrap();
    let mut restored = op_editor_core::EditorState::new();
    restored.doc = serde_json::from_value(doc).unwrap();
    restored
        .ui
        .variables
        .active_theme
        .insert("Mode".into(), "Dark".into());
    assert_eq!(
        restored.resolve_color_variable_hex("--primary").as_deref(),
        Some("#F97316")
    );
}

#[test]
fn primary_only_palette_has_readable_cta_and_map_marker_in_both_modes() {
    for (primary, expected_foreground) in [("#0E7C66", "#FFFFFF"), ("#F97316", "#000000")] {
        // Also cover a directly constructed palette: prompt generation must
        // agree with the normalized palette consumed by variable seeding.
        let palette = PlanPalette {
            primary: Some(primary.into()),
            ..Default::default()
        };
        assert_eq!(
            palette.token_overrides(),
            palette.normalized().unwrap().token_overrides()
        );
        let normalized = palette.normalized().unwrap();
        assert!(
            normalized.primary_foreground.is_none(),
            "keep derived provenance"
        );
        assert_eq!(
            normalized
                .token_overrides()
                .into_iter()
                .find(|(name, _)| *name == "--primary-foreground")
                .map(|(_, color)| color),
            Some(expected_foreground.into())
        );
        let mut plan = delivery_plan();
        plan.palette = Some(palette);
        let mut sink = VecDocSink::new();
        seed(&mut sink, &plan);
        for mode in ["Light", "Dark"] {
            sink.state
                .ui
                .variables
                .active_theme
                .insert("Mode".into(), mode.into());
            let primary = sink.state.resolve_color_variable_hex("--primary").unwrap();
            let foreground = sink
                .state
                .resolve_color_variable_hex("--primary-foreground")
                .unwrap();
            assert_eq!(foreground, expected_foreground);
            assert!(op_design_lint::color_contrast(&primary, &foreground) >= 4.5);
            let mut cta: PenNode = serde_json::from_value(json!({
                "type":"frame","id":"cta","width":120,"height":44,
                "fill":[{"type":"solid","color":"$--primary"}],
                "children":[{"type":"text","id":"label","content":"Continue",
                    "fill":[{"type":"solid","color":"$--primary-foreground"}]}]
            }))
            .unwrap();
            op_editor_core::variables_resolve::resolve_node_for_canvas(
                &mut cta,
                sink.state.doc.variables.as_ref(),
                &sink.state.ui.variables.active_theme,
            );
            let rendered = serde_json::to_value(cta).unwrap();
            assert_eq!(rendered["fill"][0]["color"], primary);
            assert_eq!(rendered["children"][0]["fill"][0]["color"], foreground);
            for scene in [
                crate::map_templates::MapTemplate::Delivery,
                crate::map_templates::MapTemplate::Location,
            ] {
                let map = crate::map_templates::children(
                    "map",
                    327.0,
                    240.0,
                    crate::role_defaults::Theme::Dark,
                    scene,
                );
                let marker = map.iter().find(|node| node["name"] == "map-pin").unwrap();
                assert_eq!(marker["fill"][0]["color"], "$--primary");
                assert_eq!(
                    marker["children"][0]["fill"][0]["color"],
                    "$--primary-foreground"
                );
                let mut marker: PenNode = serde_json::from_value(marker.clone()).unwrap();
                op_editor_core::variables_resolve::resolve_node_for_canvas(
                    &mut marker,
                    sink.state.doc.variables.as_ref(),
                    &sink.state.ui.variables.active_theme,
                );
                let marker = serde_json::to_value(marker).unwrap();
                assert_eq!(marker["fill"][0]["color"], primary);
                assert_eq!(marker["children"][0]["fill"][0]["color"], foreground);
            }
        }
    }
}

#[test]
fn derived_foreground_preserves_explicit_and_existing_user_on_colors() {
    let palette =
        PlanPalette::from_value(&json!({"primary":"#0E7C66","primaryForeground":"#FAF8EF"}))
            .unwrap();
    assert_eq!(palette.primary_foreground.as_deref(), Some("#FAF8EF"));
    let mut sink = VecDocSink::new();
    let original = json!({"type":"color","value":[
        {"value":"#FAF8EF","theme":{"Mode":"Light"}},
        {"value":"#E8F0ED","theme":{"Mode":"Dark"}}
    ]});
    sink.state.doc.variables = Some(
        [(
            "--primary-foreground".into(),
            serde_json::from_value(original.clone()).unwrap(),
        )]
        .into(),
    );
    let mut plan = delivery_plan();
    plan.palette = PlanPalette::from_value(&json!({"primary":"#0E7C66"}));
    seed(&mut sink, &plan);
    assert_eq!(
        serde_json::to_value(&sink.state.doc.variables.as_ref().unwrap()["--primary-foreground"])
            .unwrap(),
        original
    );
}

#[test]
fn existing_primary_does_not_receive_foreground_derived_from_a_different_accent() {
    let existing: std::collections::BTreeMap<
        String,
        jian_ops_schema::variable::VariableDefinition,
    > = serde_json::from_value(json!({"--primary":{"type":"color","value":"#0F1B2D"}})).unwrap();
    let mut plan = delivery_plan();
    plan.palette = None;
    let mut baseline = VecDocSink::new();
    baseline.state.doc.variables = Some(existing.clone());
    seed(&mut baseline, &plan);
    let mut sink = VecDocSink::new();
    sink.state.doc.variables = Some(existing.clone());
    plan.palette = PlanPalette::from_value(&json!({"primary":"#F97316"}));
    // Normalization and serialization must not relabel a derived value as
    // explicit, or the existing-accent protection would be bypassed.
    let serialized = serde_json::to_value(plan.palette.as_ref().unwrap()).unwrap();
    assert!(serialized.get("primaryForeground").is_none());
    seed(&mut sink, &plan);
    assert_eq!(
        sink.state.doc.variables.as_ref().unwrap()["--primary"],
        existing["--primary"]
    );
    assert_eq!(
        sink.state.doc.variables.as_ref().unwrap()["--primary-foreground"],
        baseline.state.doc.variables.as_ref().unwrap()["--primary-foreground"],
        "both stock theme slots remain unchanged"
    );
    let primary = sink.state.resolve_color_variable_hex("--primary").unwrap();
    let foreground = sink
        .state
        .resolve_color_variable_hex("--primary-foreground")
        .unwrap();
    assert_eq!(foreground, "#FFFFFF");
    assert!(op_design_lint::color_contrast(&primary, &foreground) >= 4.5);

    let mut explicit_sink = VecDocSink::new();
    explicit_sink.state.doc.variables = Some(existing);
    plan.palette.as_mut().unwrap().primary_foreground = Some("#FAF8EF".into());
    seed(&mut explicit_sink, &plan);
    assert_eq!(
        explicit_sink
            .state
            .resolve_color_variable_hex("--primary-foreground")
            .as_deref(),
        Some("#FAF8EF")
    );
}

#[test]
fn production_strict_parse_finalizes_declared_canvas_in_all_text_wrappers() {
    let mut source = plan_json();
    source["rootFrame"]["fill"][0]["color"] = json!("#000000");
    let raw = source.to_string();
    for text in [
        raw.clone(),
        format!("```json\n{raw}\n```"),
        format!("Plan:\n{raw}\nDone."),
    ] {
        let (plan, repaired) = parse_orchestrator_response(&text, &request()).unwrap();
        assert!(!repaired);
        assert_eq!(
            plan.root_frame.first_solid_hex().as_deref(),
            Some("#0F1B2D")
        );
        assert_eq!(
            plan.palette.as_ref().unwrap().primary.as_deref(),
            Some("#F97316")
        );
    }
}

#[test]
fn production_strict_parse_preserves_design_md_precedence() {
    let mut req = request();
    req.design_md = Some(
        serde_json::from_value(json!({
            "raw":"", "visualTheme":"light",
            "colorPalette":[{"name":"Canvas","hex":"#F0F4F8","role":"page background"}]
        }))
        .unwrap(),
    );
    let raw = plan_json().to_string();
    for text in [
        raw.clone(),
        format!("```json\n{raw}\n```"),
        format!("Plan:\n{raw}\nDone."),
    ] {
        let (plan, repaired) = parse_orchestrator_response(&text, &req).unwrap();
        assert!(!repaired);
        assert!(plan.palette.is_none());
        assert_eq!(plan.style_guide_name.as_deref(), Some("design-md-custom"));
        assert_eq!(
            plan.root_frame.first_solid_hex().as_deref(),
            Some("#F0F4F8")
        );
    }
}
