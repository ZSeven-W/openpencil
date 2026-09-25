//! Side-by-side directions on the web host: the Home toggle the daemon's
//! `variants` capability unlocks, the route it pins on the wire, the SSE
//! `variant` frames folded into the workspace, the retry that keeps the
//! count, and the "use this" pick.

use super::WidgetHost;
use op_editor_core::variant_wire::VariantEventWire;
use op_editor_core::{
    HomeHit, LaunchRoute, PenNodeExt, WorkspaceHit, WorkspacePhase, WorkspaceVariant,
    DEFAULT_VARIANT_COUNT,
};
use op_editor_ui::widgets::{HomeSurface, WorkspaceSurface};
use op_editor_ui::{Point2D, Rect};

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn centre(rect: Rect) -> (f32, f32) {
    (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// A browser on Home whose daemon serves models; `variants` is the
/// daemon's capability answer.
fn home_host(variants: bool) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    host.editor_state.editor_ui.agent_settings.web_served_models = true;
    host.editor_state.editor_ui.home.visible = true;
    host.editor_state.editor_ui.home.variants_unavailable = !variants;
    host
}

fn toggle_rect(host: &WidgetHost) -> Rect {
    HomeSurface::for_editor(&host.editor_state)
        .expect("home visible")
        .layout(W, H)
        .variants
}

fn send_brief(host: &mut WidgetHost) {
    for c in "记账 app".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());
}

fn wire_body(host: &mut WidgetHost) -> serde_json::Value {
    let prepared = crate::web_chat::prepare_turn(&mut host.editor_state).expect("turn");
    serde_json::from_str(&prepared.body_json).expect("json")
}

#[test]
fn without_the_capability_home_offers_no_toggle() {
    let mut host = home_host(false);
    assert_eq!(toggle_rect(&host), Rect::ZERO);
    send_brief(&mut host);
    assert!(!host.editor_state.editor_ui.workspace.is_variants_run());
    let body = wire_body(&mut host);
    assert_eq!(body["launchRoute"], "orchestrator");
    assert!(body["variantCount"].is_null());
}

#[test]
fn with_the_capability_the_toggle_turns_a_send_into_a_variants_turn() {
    let mut host = home_host(true);
    let toggle = toggle_rect(&host);
    assert!(toggle.size.x > 0.0, "a variants daemon gets the toggle");
    let (x, y) = centre(toggle);
    assert_eq!(
        HomeSurface::for_editor(&host.editor_state)
            .expect("home")
            .hit_test(W, H, Point2D::new(x, y)),
        Some(HomeHit::Variants)
    );
    assert!(host.apply_press(x, y, W, H));
    assert!(host.editor_state.editor_ui.home.variants_on);

    send_brief(&mut host);
    assert_eq!(
        host.editor_state.editor_ui.workspace.variant_count,
        DEFAULT_VARIANT_COUNT
    );
    host.editor_state.editor_ui.locale = op_editor_core::Locale::ZhCn;
    let body = wire_body(&mut host);
    assert_eq!(body["launchRoute"], "variants");
    assert_eq!(body["variantCount"], DEFAULT_VARIANT_COUNT);
    assert_eq!(body["locale"], "zh-CN");
    assert_eq!(host.editor_state.chat.launch_route, LaunchRoute::Auto);
}

fn landed(index: usize, root: &str) -> WorkspaceVariant {
    let letter = op_editor_core::variant_letter(index);
    WorkspaceVariant {
        index,
        name: format!("方案 {letter}"),
        style_guide: format!("guide-{letter}"),
        style_label: format!("Guide {letter}"),
        name_prefix: format!("方案 {letter} · Guide {letter} · "),
        root_ids: vec![root.into()],
        variables: None,
        themes: None,
    }
}

/// A variants turn launched as `generation`, as the web chat drain leaves it.
fn launched_variants_host(generation: u64) -> WidgetHost {
    let mut host = home_host(true);
    let (x, y) = centre(toggle_rect(&host));
    assert!(host.apply_press(x, y, W, H));
    send_brief(&mut host);
    host.editor_state.chat.pending_send.take().expect("queued");
    assert!(host.stamp_workspace_run(generation));
    host
}

#[test]
fn variant_frames_are_folded_into_this_run_workspace_and_narrated() {
    let mut host = launched_variants_host(7);
    let ready = VariantEventWire::ready(&landed(1, "b"));
    assert!(host.apply_run_variant(7, None, ready));
    let failed = VariantEventWire::failed(2, "方案 C", "boom");
    assert!(host.apply_run_variant(7, None, failed));
    // A frame from an older turn narrates nothing new and records nothing.
    assert!(!host.apply_run_variant(3, None, VariantEventWire::ready(&landed(1, "b"))));
    assert!(host.apply_run_variant(3, None, VariantEventWire::ready(&landed(0, "a"))));

    let workspace = &host.editor_state.editor_ui.workspace;
    let slots: Vec<usize> = workspace.variants.iter().map(|v| v.index).collect();
    assert_eq!(slots, vec![1], "only this run's landed direction");
    let reply = host
        .editor_state
        .chat
        .messages
        .iter()
        .rev()
        .find(|m| m.streaming)
        .expect("streaming reply");
    assert!(reply.content.contains("方案 B"), "{}", reply.content);
    assert!(reply.content.contains("方案 C"), "{}", reply.content);
}

#[test]
fn a_retry_of_a_variants_run_asks_for_the_same_directions() {
    let mut host = launched_variants_host(2);
    host.editor_state.editor_ui.workspace.variant_count = 4;
    assert!(host.stop_workspace_run());
    host.retry_workspace_brief();
    assert_eq!(
        host.editor_state.chat.launch_route,
        LaunchRoute::Variants(4)
    );
    let body = wire_body(&mut host);
    assert_eq!(body["launchRoute"], "variants");
    assert_eq!(body["variantCount"], 4);
}

#[test]
fn use_this_keeps_one_direction_once_the_run_settled() {
    let source = r#"{ "version": "1.0.0", "children": [
        { "type": "frame", "id": "a", "name": "方案 A · Guide A · Home", "x": 0, "y": 0,
          "width": 375, "height": 812, "children": [] },
        { "type": "frame", "id": "b", "name": "方案 B · Guide B · Home", "x": 615, "y": 0,
          "width": 375, "height": 812, "children": [] }
    ] }"#;
    let document = jian_ops_schema::load_str(source).expect("fixture").value;
    let mut host = home_host(true);
    host.install_ingested_state(op_editor_core::EditorState::from_document(document));
    host.editor_state.editor_ui.open_workspace_for_generation(
        op_editor_core::HomeFamily::AppUi,
        "记账 app",
        op_editor_core::TaskDraft::default(),
        4,
        1_000,
        None,
    );
    host.editor_state.editor_ui.workspace.begin_variants(2);
    for (index, root) in [(0, "a"), (1, "b")] {
        let event = VariantEventWire::ready(&landed(index, root));
        host.apply_run_variant(4, None, event);
    }
    host.editor_state.editor_ui.workspace.phase = WorkspacePhase::Done;

    let button = {
        let surface = WorkspaceSurface::for_editor(&host.editor_state).expect("workspace");
        let layout = surface.layout(W, H);
        let bar = surface.variant_bar(&layout);
        assert_eq!(bar.len(), 2);
        let (x, y) = centre(bar[1].button);
        assert_eq!(
            surface.hit_test(W, H, Point2D::new(x, y)),
            Some(WorkspaceHit::UseVariant(1))
        );
        bar[1].button
    };
    let (x, y) = centre(button);
    assert!(host.apply_press(x, y, W, H));
    let names: Vec<String> = host
        .editor_state
        .active_children()
        .iter()
        .map(|node| node.base().name.clone().unwrap_or_default())
        .collect();
    assert_eq!(names, vec!["Home".to_string()], "B stays, prefix dropped");
    assert!(host.editor_state.editor_ui.workspace.variants.is_empty());
}
