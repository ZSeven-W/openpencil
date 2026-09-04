//! Tests for the design-variable resolver.
//!
//! Split out of the `variables_resolve` spine (800-line file ceiling).

use super::*;

use jian_ops_schema::variable::{ThemedValue, VariableKind};

fn color_var(value: VariableValue) -> VariableDefinition {
    VariableDefinition {
        kind: VariableKind::Color,
        value,
    }
}

fn vars_with(name: &str, def: VariableDefinition) -> Vars {
    let mut vars = Vars::new();
    vars.insert(name.to_string(), def);
    vars
}

fn doc_from(json: &str) -> PenDocument {
    jian_ops_schema::load_str(json)
        .expect("fixture parses")
        .value
}

#[test]
fn scalar_ref_resolves_and_non_ref_passes_through() {
    let vars = vars_with(
        "brand",
        color_var(VariableValue::Scalar(VariableScalar::Str("#ff8800".into()))),
    );
    let theme = Theme::new();
    assert_eq!(
        resolve_color_ref("$brand", Some(&vars), &theme),
        Some("#ff8800".to_string())
    );
    assert_eq!(
        resolve_color_ref("#123456", Some(&vars), &theme),
        Some("#123456".to_string())
    );
}

#[test]
fn themed_ref_matches_active_then_falls_back_to_first_entry() {
    let entries = vec![
        ThemedValue {
            theme: Some(BTreeMap::from([("Mode".to_string(), "Light".to_string())])),
            value: VariableScalar::Str("#ffffff".into()),
        },
        ThemedValue {
            theme: Some(BTreeMap::from([("Mode".to_string(), "Dark".to_string())])),
            value: VariableScalar::Str("#000000".into()),
        },
    ];
    let vars = vars_with("surface", color_var(VariableValue::Themed(entries)));
    let dark = Theme::from([("Mode".to_string(), "Dark".to_string())]);
    assert_eq!(
        resolve_color_ref("$surface", Some(&vars), &dark),
        Some("#000000".to_string())
    );
    // Empty active theme: fully-themed list still resolves to
    // the FIRST entry (the post-load state before any axis pick).
    assert_eq!(
        resolve_color_ref("$surface", Some(&vars), &Theme::new()),
        Some("#ffffff".to_string())
    );
}

#[test]
fn unknown_token_falls_back_to_semantic_palette_per_mode() {
    let theme_light = Theme::new();
    assert_eq!(
        resolve_color_ref("$color-surface", None, &theme_light),
        Some("#FFFFFF".to_string())
    );
    let theme_dark = Theme::from([("Mode".to_string(), "Dark".to_string())]);
    assert_eq!(
        resolve_color_ref("$color-surface", None, &theme_dark),
        Some("#1E293B".to_string())
    );
    assert_eq!(
        resolve_numeric_ref("$spacing-2", None, &theme_light),
        Some(8.0)
    );
    assert_eq!(resolve_color_ref("$not-a-token", None, &theme_light), None);
}

#[test]
fn nested_namespaced_ref_resolves_against_literal_keyed_var() {
    // Pencil emits namespaced tokens like `$surface/surface` and
    // keys its variable table with the same literal nested string.
    // `strip_prefix('$')` + a direct `vars.get("surface/surface")`
    // hit already resolves them — no slash special-casing needed.
    // This is the exact fill the converted Pencil cards carry, so
    // it guards the resolver against a future nested-token regress.
    let vars = vars_with(
        "surface/surface",
        color_var(VariableValue::Scalar(VariableScalar::Str("#f5f5f5".into()))),
    );
    let theme = Theme::new();
    assert_eq!(
        resolve_color_ref("$surface/surface", Some(&vars), &theme),
        Some("#f5f5f5".to_string())
    );
    // A flat token sharing no slash still resolves unchanged.
    let flat = vars_with(
        "accent",
        color_var(VariableValue::Scalar(VariableScalar::Str("#2563eb".into()))),
    );
    assert_eq!(
        resolve_color_ref("$accent", Some(&flat), &theme),
        Some("#2563eb".to_string())
    );
}

#[test]
fn circular_ref_is_guarded() {
    let vars = vars_with(
        "loop",
        color_var(VariableValue::Scalar(VariableScalar::Str("$loop".into()))),
    );
    assert_eq!(resolve_color_ref("$loop", Some(&vars), &Theme::new()), None);
}

#[test]
fn document_pass_resolves_loaded_fill_and_gap_refs() {
    // The exact P0 repro: a TS-authored doc whose fill is a
    // `$ref` string and whose gap is a `$spacing` token must
    // render concrete values with NO transient editor cache.
    let mut doc = doc_from(
        r##"{"version":"1.0.0","variables":{"brand":{"type":"color","value":"#ff8800"}},
            "children":[{"type":"frame","id":"f1","name":"f1","x":0,"y":0,"width":100,"height":50,
              "fill":[{"type":"solid","color":"$brand"}],"layout":"vertical","gap":"$spacing-2",
              "children":[{"type":"text","id":"t1","name":"t1","content":"$brand"}]}]}"##,
    );
    doc = resolve_document_for_canvas(&doc, &Theme::new());
    let PenNode::Frame(frame) = &doc.children[0] else {
        panic!("frame survives");
    };
    let Some(PenFill::Solid(body)) = frame.container.fill.as_ref().and_then(|f| f.first()) else {
        panic!("solid fill survives");
    };
    assert_eq!(body.color, "#ff8800");
    assert_eq!(
        frame.container.gap,
        Some(NumberOrExpression::Number(8.0)),
        "palette spacing token resolves for the flex solver"
    );
    let Some(PenNode::Text(text)) = frame.children.as_ref().and_then(|c| c.first()) else {
        panic!("text child survives");
    };
    assert_eq!(text.content, TextContent::Plain("#ff8800".to_string()));
}

#[test]
fn replace_refs_renames_tokens_and_freezes_values_on_delete() {
    let mut doc = doc_from(
        r##"{"version":"1.0.0","variables":{"brand":{"type":"color","value":"#ff8800"}},
            "children":[{"type":"rectangle","id":"r1","name":"r1","x":0,"y":0,"width":10,"height":10,
              "fill":[{"type":"solid","color":"$brand"}]}]}"##,
    );
    let vars = doc.variables.clone();
    // Rename: `$brand` → `$primary`.
    replace_variable_refs_in_tree(
        &mut doc.children,
        "brand",
        Some("primary"),
        vars.as_ref(),
        &Theme::new(),
    );
    let fill_color = |doc: &PenDocument| -> String {
        let PenNode::Rectangle(r) = &doc.children[0] else {
            panic!("rect survives");
        };
        let Some(PenFill::Solid(body)) = r.container.fill.as_ref().and_then(|f| f.first()) else {
            panic!("solid fill survives");
        };
        body.color.clone()
    };
    assert_eq!(fill_color(&doc), "$primary");
    // Delete (`new = None`): freeze the resolved concrete value.
    let mut renamed_vars = Vars::new();
    renamed_vars.insert(
        "primary".to_string(),
        color_var(VariableValue::Scalar(VariableScalar::Str("#ff8800".into()))),
    );
    replace_variable_refs_in_tree(
        &mut doc.children,
        "primary",
        None,
        Some(&renamed_vars),
        &Theme::new(),
    );
    assert_eq!(fill_color(&doc), "#ff8800");
}

#[test]
fn effective_theme_layers_active_over_axis_defaults() {
    let doc = doc_from(
        r#"{"version":"1.0.0","themes":{"Mode":["Light","Dark"],"Density":["Comfort","Compact"]},"children":[]}"#,
    );
    let active = Theme::from([("Mode".to_string(), "Dark".to_string())]);
    let theme = effective_theme(&doc, &active);
    assert_eq!(theme.get("Mode"), Some(&"Dark".to_string()));
    assert_eq!(theme.get("Density"), Some(&"Comfort".to_string()));
}

/// B1: the seeded palette speaks shadcn vocabulary — `--`-prefixed
/// tokens resolve from the built-in fallback with Light/Dark values.
#[test]
fn shadcn_dictionary_fallback_resolves() {
    let light = Theme::from([("Mode".to_string(), "Light".to_string())]);
    let dark = Theme::from([("Mode".to_string(), "Dark".to_string())]);
    assert_eq!(
        resolve_color_ref("$--primary", None, &light),
        Some("#2563EB".to_string())
    );
    assert_eq!(
        resolve_color_ref("$--primary", None, &dark),
        Some("#60A5FA".to_string())
    );
    assert_eq!(
        resolve_color_ref("$--muted-foreground", None, &light),
        Some("#64748B".to_string())
    );
    assert_eq!(
        resolve_color_ref("$--scrim", None, &light),
        Some("#00000080".to_string())
    );
    assert_eq!(
        resolve_numeric_ref("$--radius-pill", None, &light),
        Some(999.0)
    );
    assert_eq!(resolve_numeric_ref("$--radius-xs", None, &light), Some(4.0));
    assert!(has_palette_fallback("--background"));
    assert!(has_palette_fallback("--sidebar-ring"));
    assert!(has_palette_fallback("--chart-1"));
}

/// B1 compat: legacy `color-*` refs keep resolving through the compat
/// fallback so pre-shadcn documents render without visual drift.
#[test]
fn legacy_color_refs_resolve_via_compat_fallback() {
    let light = Theme::from([("Mode".to_string(), "Light".to_string())]);
    let dark = Theme::from([("Mode".to_string(), "Dark".to_string())]);
    assert_eq!(
        resolve_color_ref("$color-surface", None, &light),
        Some("#FFFFFF".to_string())
    );
    assert_eq!(
        resolve_color_ref("$color-surface", None, &dark),
        Some("#1E293B".to_string())
    );
    assert_eq!(
        resolve_color_ref("$color-bg-deep", None, &dark),
        Some("#0F172A".to_string())
    );
    assert_eq!(
        resolve_color_ref("$color-danger-bg", None, &light),
        Some("#FEE2E2".to_string())
    );
    assert_eq!(
        resolve_color_ref("$color-border-strong", None, &light),
        Some("#CBD5E1".to_string())
    );
    assert_eq!(resolve_numeric_ref("$radius-md", None, &light), Some(8.0));
    assert!(has_palette_fallback("color-surface"));
}
