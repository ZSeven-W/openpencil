//! gl-host tests for Home's 加链接 → brand kit flow. Run with
//! `--features gl-host`.

use std::collections::BTreeMap;

use super::WidgetHostNative;
use jian_ops_schema::variable::{
    ThemedValue, VariableDefinition, VariableKind, VariableScalar, VariableValue,
};
use op_editor_core::{
    BrandKitPayload, BrandSourceRequest, BuiltinAgentConfig, BuiltinAgentKind,
    BuiltinAgentPresetKey, HomeBrandStatus,
};
use op_editor_ui::widgets::HomeSurface;
use op_editor_ui::{Point2D, Rect};

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn desktop_home() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.home.visible = true;
    ui.home.brand.available = true;
    ui.agent_settings.builtin_agents.push(BuiltinAgentConfig {
        id: "builtin-1".into(),
        preset: BuiltinAgentPresetKey::Custom,
        display_name: "Test".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key: "sk-test".into(),
        models: vec!["test-model".into()],
        base_url: "http://localhost:9".into(),
        enabled: true,
    });
    host
}

fn kit(primary: &str) -> BrandKitPayload {
    let themed = |light: &str| VariableDefinition {
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![
            ThemedValue {
                value: VariableScalar::Str(light.into()),
                theme: Some(BTreeMap::from([("Mode".into(), "Light".into())])),
            },
            ThemedValue {
                value: VariableScalar::Str("#34D399".into()),
                theme: Some(BTreeMap::from([("Mode".into(), "Dark".into())])),
            },
        ]),
    };
    BrandKitPayload {
        label: "Verdant Loop".into(),
        swatches: vec![primary.into(), "#FFFFFF".into(), "#111111".into()],
        variables: BTreeMap::from([("--primary".to_string(), themed(primary))]),
        themes: BTreeMap::from([("Mode".into(), vec!["Light".into(), "Dark".into()])]),
        design_md: None,
    }
}

fn press_link_tool(host: &mut WidgetHostNative) {
    let layout = HomeSurface::for_editor(host.editor_state())
        .expect("home visible")
        .layout(W, H);
    let at = center(layout.reference_link);
    assert!(host.apply_press(at.x, at.y, W, H));
}

fn type_text(host: &mut WidgetHostNative, text: &str) {
    for c in text.chars() {
        assert!(host.apply_text(c));
    }
}

#[test]
fn the_link_tool_reads_the_brief_link_and_the_shell_finishes_it() {
    let mut host = desktop_home();
    type_text(&mut host, "a tea shop app, brand https://leafline.example");
    press_link_tool(&mut host);
    let (generation, request) = host.take_home_brand_request().expect("request queued");
    assert_eq!(
        request,
        BrandSourceRequest::Url("https://leafline.example".into())
    );
    assert!(host.finish_home_brand(generation, Ok(kit("#0D775A"))));
    let surface = HomeSurface::for_editor(host.editor_state()).unwrap();
    let (_, close) = surface.brand_chip_rects(W, H).expect("chip painted");
    let at = center(close);
    assert!(host.apply_press(at.x, at.y, W, H));
    assert_eq!(
        host.editor_state().editor_ui.home.brand.status,
        HomeBrandStatus::Idle,
        "× removes the staged kit"
    );
}

#[test]
fn without_a_runner_the_tool_stays_inert() {
    let mut host = desktop_home();
    host.editor_state_mut().editor_ui.home.brand.available = false;
    type_text(&mut host, "https://leafline.example");
    press_link_tool(&mut host);
    assert!(host.take_home_brand_request().is_none());
}

#[test]
fn a_home_send_applies_the_staged_kit_to_the_document_it_runs_on() {
    let mut host = desktop_home();
    type_text(&mut host, "a tea shop app https://leafline.example");
    press_link_tool(&mut host);
    let (generation, _) = host.take_home_brand_request().unwrap();
    assert!(host.finish_home_brand(generation, Ok(kit("#0D775A"))));
    assert!(host.apply_send());
    let vars = host
        .editor_state()
        .doc
        .variables
        .as_ref()
        .expect("kit applied");
    let VariableValue::Themed(values) = &vars["--primary"].value else {
        panic!("themed");
    };
    assert_eq!(values[0].value, VariableScalar::Str("#0D775A".into()));
    assert!(host.editor_state().chat.pending_send.is_some());
    assert!(
        host.editor_state().editor_ui.home.brand.staged().is_some(),
        "the kit stays staged for the next brief"
    );
}
