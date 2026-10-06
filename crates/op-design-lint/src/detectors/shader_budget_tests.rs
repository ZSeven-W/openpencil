use super::*;
use jian_ops_schema::PenDocument;
use std::collections::BTreeMap;

/// A root of `width x height` whose children are described by `fills`: each
/// entry is `(id, w, h, sksl_len, uniforms_json)`. `uniforms_json` of `null`
/// means the fill declares none.
fn doc_with_shaders(
    root_w: f32,
    root_h: f32,
    fills: &[(&str, f32, f32, usize, serde_json::Value)],
) -> PenNode {
    let children: Vec<String> = fills
        .iter()
        .map(|(id, w, h, len, uniforms)| {
            let sksl = "x".repeat(*len);
            let uniforms = if uniforms.is_null() {
                String::new()
            } else {
                format!(r#","uniforms":{uniforms}"#)
            };
            format!(
                r##"{{ "type": "frame", "id": "{id}", "width": {w}, "height": {h},
                       "fill": [{{ "type": "shader", "sksl": "{sksl}"{uniforms} }}] }}"##
            )
        })
        .collect();
    let src = format!(
        r##"{{ "version": "1.0", "children": [
            {{ "type": "frame", "id": "root", "width": {root_w}, "height": {root_h},
               "layout": "vertical", "children": [{}] }}
        ] }}"##,
        children.join(",")
    );
    let doc: PenDocument = serde_json::from_str(&src).expect("fixture doc");
    doc.children.into_iter().next().expect("root")
}

fn ids(issues: &[Issue]) -> Vec<&str> {
    issues.iter().map(|i| i.node_id.as_str()).collect()
}

fn shader_body(
    preset: Option<&str>,
    sksl: Option<String>,
    uniforms: BTreeMap<String, ShaderUniformValue>,
) -> ShaderFillBody {
    ShaderFillBody {
        preset: preset.map(str::to_string),
        sksl,
        uniforms: (!uniforms.is_empty()).then_some(uniforms),
        explain: None,
        opacity: None,
        blend_mode: None,
    }
}

fn turbulence_uniform(name: &str, value: ShaderUniformValue) -> ShaderFillBody {
    shader_body(
        Some("turbulence"),
        None,
        BTreeMap::from([(name.to_string(), value)]),
    )
}

fn doc_with_shader_bodies(
    root_w: f32,
    root_h: f32,
    fills: &[(&str, f32, f32, serde_json::Value)],
) -> PenNode {
    let children = fills
        .iter()
        .map(|(id, width, height, body)| {
            let mut body = body.as_object().expect("shader body object").clone();
            body.insert("type".to_string(), serde_json::json!("shader"));
            format!(
                r#"{{"type":"frame","id":"{id}","width":{width},"height":{height},"fill":[{}]}}"#,
                serde_json::Value::Object(body)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        r#"{{"version":"1.0","children":[{{"type":"frame","id":"root","width":{root_w},"height":{root_h},"layout":"vertical","children":[{children}]}}]}}"#
    );
    let document: PenDocument = serde_json::from_str(&source).expect("preset fixture document");
    document.children.into_iter().next().expect("root node")
}

#[test]
fn turbulence_preset_wins_over_non_empty_sksl_without_source_size_warning() {
    let body = shader_body(Some("turbulence"), Some("x".repeat(9_000)), BTreeMap::new());
    let issues = shader_issues("hero", &body);
    assert_eq!(issues.len(), 1, "ignored source must not also trip 8192");
    assert_eq!(issues[0].severity, IssueSeverity::Warning);
    assert_eq!(
        issues[0].reason,
        "shader preset `turbulence` takes precedence, so the non-empty authored SkSL is ignored"
    );
}

#[test]
fn invalid_num_octaves_warns_with_the_loader_resolution() {
    for (value, effective) in [(0.0, 1), (99.0, 6)] {
        let body = turbulence_uniform("numOctaves", ShaderUniformValue::Float(value));
        let issues = shader_issues("hero", &body);
        assert_eq!(issues.len(), 1, "{value} must warn");
        assert_eq!(
            issues[0].reason,
            format!(
                "numOctaves {value} is outside the supported range 1..=6; the turbulence loader clamps it to {effective}"
            )
        );
    }

    for value in [2.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let body = turbulence_uniform("numOctaves", ShaderUniformValue::Float(value));
        let issues = shader_issues("hero", &body);
        assert_eq!(issues.len(), 1, "{value} must warn");
        assert_eq!(
            issues[0].reason,
            format!(
                "numOctaves {value} must be a finite integer; the turbulence loader falls back to the default 3"
            )
        );
    }

    for value in [1.0, 6.0] {
        let body = turbulence_uniform("numOctaves", ShaderUniformValue::Float(value));
        assert!(shader_issues("hero", &body).is_empty());
    }

    for value in [
        ShaderUniformValue::Vec(vec![2.0, 3.0]),
        ShaderUniformValue::Color("#ffffff".to_string()),
    ] {
        let body = turbulence_uniform("numOctaves", value);
        let issues = shader_issues("hero", &body);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].reason,
            "numOctaves must be a number; the turbulence loader falls back to the default 3"
        );
    }
}

#[test]
fn non_positive_turbulence_frequency_warns_about_constant_noise() {
    for value in [
        ShaderUniformValue::Float(0.0),
        ShaderUniformValue::Float(-0.1),
        ShaderUniformValue::Vec(vec![0.08, 0.0]),
        ShaderUniformValue::Vec(vec![-0.02, 0.08]),
    ] {
        let body = turbulence_uniform("baseFrequency", value);
        let issues = shader_issues("hero", &body);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].reason,
            "baseFrequency must stay positive on every axis; zero or negative values make turbulence degenerate to a constant"
        );
    }

    for value in [
        ShaderUniformValue::Float(0.08),
        ShaderUniformValue::Vec(vec![0.08, 0.11]),
    ] {
        let body = turbulence_uniform("baseFrequency", value);
        assert!(shader_issues("hero", &body).is_empty());
    }
}

#[test]
fn turbulence_base_frequency_reuses_uniform_arity_warning() {
    let root = doc_with_shader_bodies(
        390.0,
        844.0,
        &[(
            "hero",
            100.0,
            100.0,
            serde_json::json!({
                "preset":"turbulence",
                "uniforms":{"baseFrequency":[0.08,0.08,0.08,0.08,0.08]}
            }),
        )],
    );
    let issues = detect_shader_budget(&root, DesignForm::MobileScreen);
    assert_eq!(issues.len(), 1);
    assert!(
        issues[0]
            .reason
            .contains("uniform `baseFrequency` has 5 components"),
        "{:?}",
        issues[0]
    );
}

#[test]
fn unknown_preset_warns_for_authored_and_empty_fallbacks() {
    let authored = shader_body(
        Some("future_noise"),
        Some("half4 main(float2 p){ return half4(1.0); }".to_string()),
        BTreeMap::new(),
    );
    let issues = shader_issues("authored", &authored);
    assert_eq!(issues.len(), 1);
    assert_eq!(
        issues[0].reason,
        "unknown shader preset `future_noise`; the loader falls back to the non-empty authored SkSL"
    );

    for sksl in [None, Some("   ".to_string())] {
        let empty = shader_body(Some("future_noise"), sksl, BTreeMap::new());
        let issues = shader_issues("empty", &empty);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].reason,
            "unknown shader preset `future_noise`; without non-empty authored SkSL this fill does not render"
        );
    }
}

#[test]
fn turbulence_presets_share_the_mobile_full_bleed_budget() {
    let fills = ["a", "b", "c", "d"]
        .map(|id| (id, 390.0, 600.0, serde_json::json!({"preset":"turbulence"})));
    let root = doc_with_shader_bodies(390.0, 844.0, &fills);
    let issues = detect_shader_budget(&root, DesignForm::MobileScreen);
    assert_eq!(ids(&issues), vec!["c", "d"]);
    assert!(issues.iter().all(|issue| {
        issue.category == IssueCategory::ShaderBudget && issue.severity == IssueSeverity::Info
    }));
}

#[test]
fn a_390_by_600_turbulence_hero_is_valid_on_mobile() {
    let root = doc_with_shader_bodies(
        390.0,
        844.0,
        &[(
            "hero",
            390.0,
            600.0,
            serde_json::json!({"preset":"turbulence"}),
        )],
    );
    assert!(detect_shader_budget(&root, DesignForm::MobileScreen).is_empty());
}

#[test]
fn an_ordinary_shader_hero_is_not_flagged() {
    // One full-bleed shader is exactly what a hero section is for.
    let root = doc_with_shaders(
        390.0,
        844.0,
        &[("hero", 390.0, 600.0, 400, serde_json::json!({"t": 1.0}))],
    );
    assert!(
        detect_shader_budget(&root, DesignForm::MobileScreen).is_empty(),
        "a single authored hero shader must pass"
    );
}

#[test]
fn a_phone_tiled_with_full_bleed_passes_is_flagged_past_its_budget() {
    let fills: Vec<_> = (0..5)
        .map(|i| {
            (
                ["a", "b", "c", "d", "e"][i],
                390.0,
                700.0,
                200,
                serde_json::Value::Null,
            )
        })
        .collect();
    let root = doc_with_shaders(390.0, 844.0, &fills);
    let issues = detect_shader_budget(&root, DesignForm::MobileScreen);

    // Budget is 2 on mobile, so passes 3..5 are reported — not all five.
    assert_eq!(ids(&issues), vec!["c", "d", "e"]);
    assert!(issues.iter().all(|i| i.severity == IssueSeverity::Info));
    assert!(
        issues.iter().all(|i| i.suggested_value.is_null()),
        "dropping a visual effect is a design decision, never an auto-fix"
    );
}

#[test]
fn the_same_document_passes_on_a_desktop_page() {
    // Identical content, looser form: the budget is about GPU headroom, and
    // a desktop page has it. Guards against the budget being a blanket rule.
    let fills: Vec<_> = (0..4)
        .map(|i| {
            (
                ["a", "b", "c", "d"][i],
                1440.0,
                700.0,
                200,
                serde_json::Value::Null,
            )
        })
        .collect();
    let root = doc_with_shaders(1440.0, 900.0, &fills);
    assert!(detect_shader_budget(&root, DesignForm::Page).is_empty());
}

#[test]
fn small_shader_accents_do_not_count_against_the_full_bleed_budget() {
    // Eight small shader chips are cheap; the budget is about fragment passes
    // over the whole surface, not about the word "shader" appearing often.
    let fills: Vec<_> = (0..8)
        .map(|i| {
            (
                ["a", "b", "c", "d", "e", "f", "g", "h"][i],
                40.0,
                40.0,
                120,
                serde_json::Value::Null,
            )
        })
        .collect();
    let root = doc_with_shaders(390.0, 844.0, &fills);
    assert!(detect_shader_budget(&root, DesignForm::MobileScreen).is_empty());
}

#[test]
fn a_pasted_shader_toy_is_flagged_on_source_size() {
    let root = doc_with_shaders(
        390.0,
        844.0,
        &[("hero", 100.0, 100.0, 9_000, serde_json::Value::Null)],
    );
    let issues = detect_shader_budget(&root, DesignForm::MobileScreen);
    assert_eq!(issues.len(), 1);
    assert!(
        issues[0].reason.contains("9000 characters"),
        "{:?}",
        issues[0]
    );
}

#[test]
fn a_bad_vec_arity_is_diagnosed_instead_of_silently_degrading() {
    // RuntimeShaderBuilder rejects this at paint time and the fill falls back
    // to a solid colour — which looks like a design choice unless something
    // says otherwise.
    let root = doc_with_shaders(
        390.0,
        844.0,
        &[(
            "hero",
            100.0,
            100.0,
            120,
            serde_json::json!({"tint": [1.0, 0.0, 0.0, 1.0, 0.5]}),
        )],
    );
    let issues = detect_shader_budget(&root, DesignForm::MobileScreen);
    assert_eq!(issues.len(), 1);
    assert!(
        issues[0].reason.contains("`tint` has 5 components"),
        "{:?}",
        issues[0]
    );
    assert!(issues[0].reason.contains("degrade"), "{:?}", issues[0]);
}

#[test]
fn valid_vec_arities_pass() {
    for arity in [2usize, 3, 4] {
        let components: Vec<f32> = vec![0.5; arity];
        let root = doc_with_shaders(
            390.0,
            844.0,
            &[(
                "hero",
                100.0,
                100.0,
                120,
                serde_json::json!({ "v": components }),
            )],
        );
        assert!(
            detect_shader_budget(&root, DesignForm::MobileScreen).is_empty(),
            "vec{arity} is valid SkSL"
        );
    }
}

#[test]
fn an_over_parameterised_shader_is_flagged_on_uniform_count() {
    let mut uniforms = serde_json::Map::new();
    for i in 0..20 {
        uniforms.insert(format!("u{i}"), serde_json::json!(1.0));
    }
    let root = doc_with_shaders(
        390.0,
        844.0,
        &[(
            "hero",
            100.0,
            100.0,
            120,
            serde_json::Value::Object(uniforms),
        )],
    );
    let issues = detect_shader_budget(&root, DesignForm::MobileScreen);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].reason.contains("20 uniforms"), "{:?}", issues[0]);
}

#[test]
fn a_document_with_no_shaders_costs_nothing() {
    let doc: PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "root", "width": 390, "height": 844,
              "fill": [{ "type": "solid", "color": "#101010" }] }
        ] }"##,
    )
    .expect("doc");
    let root = doc.children.into_iter().next().expect("root");
    assert!(detect_shader_budget(&root, DesignForm::MobileScreen).is_empty());
}

/// The split between "cannot render as authored" and "renders but costs a lot"
/// is the whole point of this detector having two categories, and callers gate
/// on severity — `detect_and_plan` drops `Info` outright. Without this test the
/// distinction is invisible to the suite and would be lost in a refactor.
#[test]
fn a_fault_is_a_warning_while_a_cost_stays_advisory() {
    // Bad uniform arity: the fill degrades to a flat colour, so what ships is
    // not the authored design.
    let faulty = doc_with_shaders(
        390.0,
        844.0,
        &[(
            "hero",
            100.0,
            100.0,
            120,
            serde_json::json!({"tint": [1.0, 0.0, 0.0, 1.0, 0.5]}),
        )],
    );
    let issues = detect_shader_budget(&faulty, DesignForm::MobileScreen);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].category, IssueCategory::ShaderInvalid);
    assert_eq!(
        issues[0].severity,
        IssueSeverity::Warning,
        "a fill that cannot render as authored is a defect, not a suggestion"
    );

    // Too many full-bleed passes: expensive, but it renders exactly as asked.
    let costly: Vec<_> = (0..5)
        .map(|i| {
            (
                ["a", "b", "c", "d", "e"][i],
                390.0,
                700.0,
                200,
                serde_json::Value::Null,
            )
        })
        .collect();
    let costly = doc_with_shaders(390.0, 844.0, &costly);
    let issues = detect_shader_budget(&costly, DesignForm::MobileScreen);
    assert!(!issues.is_empty());
    assert!(
        issues
            .iter()
            .all(|i| i.category == IssueCategory::ShaderBudget
                && i.severity == IssueSeverity::Info),
        "dropping a visual effect is a design decision — cost stays advisory"
    );
}

/// Neither kind offers an auto-fix. `apply_fixes` would otherwise try to write
/// `suggested_value` onto the node, and there is no safe machine edit for
/// either "too expensive" or "wrong uniform shape".
#[test]
fn neither_kind_proposes_an_automatic_fix() {
    let faulty = doc_with_shaders(
        390.0,
        844.0,
        &[("hero", 100.0, 100.0, 9_000, serde_json::Value::Null)],
    );
    for issue in detect_shader_budget(&faulty, DesignForm::MobileScreen) {
        assert!(issue.suggested_value.is_null(), "{issue:?}");
    }
}
