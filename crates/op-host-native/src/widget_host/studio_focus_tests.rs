//! Studio keyboard navigation (Tab order, activation, focus-visible),
//! driven through the host.

use super::super::WidgetHostNative;
use op_editor_core::{HomeFamily, HomeHit, Tool, WorkspaceHit, WorkspacePhase};
use op_editor_ui::widgets::{HomeSurface, WorkspaceSurface};

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn home_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.set_now_ms(2_000);
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    host.editor_state_mut().editor_ui.locale = op_i18n::Locale::EnUs;
    host.editor_state_mut().editor_ui.home.visible = true;
    host
}

fn workspace_host(viewport_w: f32) -> WidgetHostNative {
    let source = r#"{ "version": "1.0.0", "children": [
        { "type": "frame", "id": "b0", "x": 0, "y": 0, "width": 1920, "height": 1080, "children": [] },
        { "type": "frame", "id": "b1", "x": 2000, "y": 0, "width": 1920, "height": 1080, "children": [] }
    ] }"#;
    let document = jian_ops_schema::load_str(source).expect("fixture").value;
    let mut host = WidgetHostNative::new();
    host.install_imported_state(op_editor_core::EditorState::from_document(document));
    host.set_now_ms(2_000);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = H;
    let editor = host.editor_state_mut();
    editor.tool = Tool::Hand;
    editor.editor_ui.open_workspace_for_generation(
        HomeFamily::Presentation,
        "deck",
        op_editor_core::TaskDraft::default(),
        0,
        1_000,
        Some(Tool::Select),
    );
    editor.editor_ui.workspace.phase = WorkspacePhase::Done;
    host
}

fn home_order(host: &WidgetHostNative) -> Vec<HomeHit> {
    HomeSurface::for_editor_at(host.editor_state(), 2_000)
        .expect("home")
        .focus_order(W, H)
}

fn workspace_order(host: &WidgetHostNative, w: f32) -> Vec<WorkspaceHit> {
    let surface = WorkspaceSurface::for_editor_at(host.editor_state(), 2_000).expect("workspace");
    let layout = surface.layout(w, H);
    surface
        .focus_order(&layout)
        .into_iter()
        .map(|(hit, _)| hit)
        .collect()
}

#[test]
fn home_tab_order_starts_at_the_composer_and_reaches_every_section() {
    let host = home_host();
    let order = home_order(&host);
    assert_eq!(order.first(), Some(&HomeHit::Sheet));
    for expected in [
        HomeHit::Send,
        HomeHit::Tab(HomeFamily::Web),
        HomeHit::UseExample,
        HomeHit::ExploreCard(op_editor_ui::widgets::home_surface::EXPLORE_FAMILIES[0]),
        HomeHit::NewCanvas,
        HomeHit::Professional,
    ] {
        assert!(order.contains(&expected), "{expected:?} missing: {order:?}");
    }
    let sheet = order.iter().position(|hit| *hit == HomeHit::Sheet).unwrap();
    let chip = order
        .iter()
        .position(|hit| matches!(hit, HomeHit::Tab(_)))
        .unwrap();
    let card = order
        .iter()
        .position(|hit| matches!(hit, HomeHit::ExploreCard(_)))
        .unwrap();
    assert!(
        sheet < chip && chip < card,
        "composer, chips, then examples"
    );
}

#[test]
fn tab_and_shift_tab_walk_the_order_and_wrap() {
    let mut host = home_host();
    let order = home_order(&host);
    assert!(host.studio_focus_available());
    assert!(host.apply_studio_focus_step(false));
    assert_eq!(host.editor_state().editor_ui.home.key_focus, Some(order[0]));
    assert!(host.apply_studio_focus_step(false));
    assert_eq!(host.editor_state().editor_ui.home.key_focus, Some(order[1]));
    assert!(host.apply_studio_focus_step(true));
    assert!(host.apply_studio_focus_step(true));
    assert_eq!(
        host.editor_state().editor_ui.home.key_focus,
        order.last().copied(),
        "Shift+Tab from the first target wraps to the last"
    );
}

#[test]
fn enter_on_a_task_chip_switches_the_task_and_keeps_the_ring() {
    let mut host = home_host();
    host.editor_state_mut().editor_ui.home.key_focus = Some(HomeHit::Tab(HomeFamily::Web));
    assert!(host.studio_key_focus_activatable());
    assert!(host.activate_studio_key_focus());
    let home = &host.editor_state().editor_ui.home;
    assert_eq!(home.task, HomeFamily::Web);
    assert_eq!(home.key_focus, Some(HomeHit::Tab(HomeFamily::Web)));
    assert_eq!(home.pressed, None);
}

#[test]
fn the_composer_keeps_enter_and_space_for_text() {
    let mut host = home_host();
    host.editor_state_mut().editor_ui.home.key_focus = Some(HomeHit::Sheet);
    assert!(!host.studio_key_focus_activatable());
}

#[test]
fn the_open_connect_card_traps_focus_in_its_rows() {
    let mut host = home_host();
    host.editor_state_mut().editor_ui.home.connect_card_open = true;
    let order = home_order(&host);
    assert!(!order.is_empty());
    assert!(order.iter().all(|hit| matches!(
        hit,
        HomeHit::ConnectFreeTier | HomeHit::ConnectApiKey | HomeHit::ConnectCli
    )));
}

#[test]
fn a_pointer_press_or_escape_drops_the_ring() {
    let mut host = home_host();
    host.apply_studio_focus_step(false);
    host.press_home(5.0, H - 5.0, W, H);
    assert_eq!(host.editor_state().editor_ui.home.key_focus, None);
    host.apply_studio_focus_step(false);
    assert!(host.apply_escape());
    assert_eq!(host.editor_state().editor_ui.home.key_focus, None);
}

#[test]
fn a_modal_over_home_takes_tab_away() {
    let mut host = home_host();
    host.editor_state_mut().editor_ui.agent_settings_open = true;
    assert!(!host.studio_focus_available());
    assert!(!host.apply_studio_focus_step(false));
}

#[test]
fn workspace_order_covers_header_toolbar_and_strip() {
    let host = workspace_host(W);
    let order = workspace_order(&host, W);
    assert_eq!(order.first(), Some(&WorkspaceHit::Back));
    for expected in [
        WorkspaceHit::Export,
        WorkspaceHit::Professional,
        WorkspaceHit::ToggleDock,
        WorkspaceHit::ZoomFit,
        WorkspaceHit::Thumb(0),
        WorkspaceHit::Play,
    ] {
        assert!(order.contains(&expected), "{expected:?} missing: {order:?}");
    }
    // A run still generating cannot present: Play drops out of the order.
    let mut generating = workspace_host(W);
    generating.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Generating;
    assert!(!workspace_order(&generating, W).contains(&WorkspaceHit::Play));
}

#[test]
fn enter_on_the_chat_toggle_collapses_the_dock() {
    let mut host = workspace_host(W);
    assert!(host.editor_state().editor_ui.sidebar_open);
    host.editor_state_mut().editor_ui.workspace.key_focus = Some(WorkspaceHit::ToggleDock);
    assert!(host.activate_studio_key_focus());
    assert!(!host.editor_state().editor_ui.sidebar_open);
    assert_eq!(
        host.editor_state().editor_ui.workspace.key_focus,
        Some(WorkspaceHit::ToggleDock)
    );
}
