use super::*;
use jian_ops_schema::variable::{ThemedValue, VariableKind, VariableScalar, VariableValue};

fn themed(light: &str, dark: &str) -> VariableDefinition {
    let entry = |mode: &str, hex: &str| ThemedValue {
        value: VariableScalar::Str(hex.to_string()),
        theme: Some(BTreeMap::from([("Mode".to_string(), mode.to_string())])),
    };
    VariableDefinition {
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![entry("Light", light), entry("Dark", dark)]),
    }
}

fn kit(primary: &str) -> BrandKitPayload {
    BrandKitPayload {
        label: "Verdant Loop".into(),
        swatches: vec![primary.into()],
        variables: BTreeMap::from([
            ("--primary".to_string(), themed(primary, "#34D399")),
            (
                "--radius-m".to_string(),
                VariableDefinition {
                    kind: VariableKind::Number,
                    value: VariableValue::Scalar(VariableScalar::Num(12.0)),
                },
            ),
        ]),
        themes: BTreeMap::from([(
            "Mode".to_string(),
            vec!["Light".to_string(), "Dark".to_string()],
        )]),
        design_md: Some(DesignMdSpec {
            raw: format!("{BRAND_KIT_MARKER}\n# Design System: Verdant Loop\n"),
            project_name: Some("Verdant Loop".into()),
            visual_theme: None,
            color_palette: None,
            typography: None,
            component_styles: None,
            layout_principles: None,
            generation_notes: None,
        }),
    }
}

fn primary_light(state: &EditorState) -> String {
    let VariableValue::Themed(values) = &state.doc.variables.as_ref().unwrap()["--primary"].value
    else {
        panic!("themed");
    };
    match &values[0].value {
        VariableScalar::Str(s) => s.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn apply_writes_variables_axis_and_design_md_as_one_undo_step() {
    let mut state = EditorState::new();
    let depth_before = state.history.past.len();
    assert!(state.apply_brand_kit(&kit("#0D775A")));
    assert_eq!(state.history.past.len(), depth_before + 1, "one undo entry");
    assert_eq!(primary_light(&state), "#0D775A");
    assert_eq!(
        state.doc.themes.as_ref().unwrap()["Mode"],
        vec!["Light", "Dark"]
    );
    assert!(design_md_is_brand_kit(
        state.doc.design_md.as_ref().unwrap()
    ));

    assert!(state.undo());
    assert!(state
        .doc
        .variables
        .as_ref()
        .is_none_or(|v| !v.contains_key("--primary")));
    assert!(state.doc.design_md.is_none());
}

#[test]
fn swapping_brands_replaces_values_and_the_kit_design_md() {
    let mut state = EditorState::new();
    assert!(state.apply_brand_kit(&kit("#0D775A")));
    let mut second = kit("#C2410C");
    second.design_md.as_mut().unwrap().project_name = Some("Cobalt Harbor".into());
    assert!(state.apply_brand_kit(&second));
    assert_eq!(primary_light(&state), "#C2410C");
    assert_eq!(
        state
            .doc
            .design_md
            .as_ref()
            .unwrap()
            .project_name
            .as_deref(),
        Some("Cobalt Harbor")
    );
}

#[test]
fn a_user_written_design_md_is_never_replaced() {
    let mut state = EditorState::new();
    state.doc.design_md = Some(DesignMdSpec {
        raw: "# Design System: Mine\n".into(),
        project_name: Some("Mine".into()),
        visual_theme: None,
        color_palette: None,
        typography: None,
        component_styles: None,
        layout_principles: None,
        generation_notes: None,
    });
    assert!(state.apply_brand_kit(&kit("#0D775A")));
    assert_eq!(primary_light(&state), "#0D775A", "variables still apply");
    assert_eq!(
        state
            .doc
            .design_md
            .as_ref()
            .unwrap()
            .project_name
            .as_deref(),
        Some("Mine")
    );
}

#[test]
fn command_is_a_single_batch() {
    let EditorCommand::Batch { commands } = kit("#0D775A").to_command(None) else {
        panic!("batch");
    };
    assert_eq!(commands.len(), 2);
    assert!(matches!(
        commands[0],
        EditorCommand::MergeThemePreset { .. }
    ));
    assert!(matches!(commands[1], EditorCommand::SetDesignMd { .. }));
}
