//! gl-host tests for Home's one-click start: the empty-box Send opens the
//! example as an instant template draft, then refines it in place (with a
//! model) or offers the connect path (without one). Run with
//! `--features gl-host`.

use super::WidgetHostNative;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::{
    BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, ChatRole, HomeFamily, LaunchRoute,
    WorkspaceHit, WorkspacePhase,
};
use op_editor_ui::widgets::{HomeSurface, WorkspaceSurface};
use op_editor_ui::{Point2D, Rect};

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn home_on(family: HomeFamily) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    host.editor_state_mut().editor_ui.home.visible = true;
    host.editor_state_mut().editor_ui.home.set_task(family, 1);
    host
}

fn connect_model(host: &mut WidgetHostNative) {
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .builtin_agents
        .push(BuiltinAgentConfig {
            id: "builtin-1".into(),
            preset: BuiltinAgentPresetKey::Custom,
            display_name: "MiniMax".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: "sk-test".into(),
            models: vec!["MiniMax-M3".into()],
            base_url: "http://localhost:9".into(),
            enabled: true,
        });
}

fn example(host: &WidgetHostNative) -> String {
    HomeSurface::for_editor(host.editor_state())
        .expect("home visible")
        .example_prompt()
        .to_string()
}

fn press_send(host: &mut WidgetHostNative) {
    let send = {
        let home = HomeSurface::for_editor(host.editor_state()).expect("home visible");
        center(home.layout(W, H).send)
    };
    assert!(host.apply_press(send.x, send.y, W, H));
}

fn first_board_width(host: &WidgetHostNative) -> Option<f64> {
    use op_editor_core::PenNodeExt;
    host.editor_state()
        .active_children()
        .first()
        .and_then(|node| node.width_px())
}

#[test]
fn an_empty_send_without_a_model_opens_the_template_draft_and_the_connect_banner() {
    let mut host = home_on(HomeFamily::Web);
    let example = example(&host);
    press_send(&mut host);

    let state = host.editor_state();
    assert!(!host.home_visible(), "the draft replaces Home");
    assert_eq!(
        state.editor_ui.home.draft, example,
        "the example is written into the box — what ran is visible"
    );
    let workspace = &state.editor_ui.workspace;
    assert!(workspace.active && workspace.visible);
    assert_eq!(workspace.draft_template, Some("saas-landing-orange"));
    assert_eq!(
        workspace.phase,
        WorkspacePhase::Done,
        "no model: the draft IS the result"
    );
    assert!(
        workspace.draft_banner_visible(),
        "and the banner offers the connect path"
    );
    assert!(state.chat.pending_send.is_none(), "no dead turn is queued");
    assert!(!op_editor_core::blank_starter::active_page_is_blank_starter(state));
    assert_eq!(
        first_board_width(&host),
        Some(1200.0),
        "the web template landed"
    );
    // The conversation records the example as the user turn.
    let messages = &host.editor_state().chat.messages;
    assert_eq!(messages[0].role, ChatRole::User);
    assert_eq!(messages[0].content, example.trim());
    assert_eq!(messages[1].role, ChatRole::Assistant);
    assert!(!messages[1].content.is_empty());
}

#[test]
fn an_empty_send_with_a_model_loads_the_draft_and_queues_an_in_place_refine() {
    let mut host = home_on(HomeFamily::Presentation);
    connect_model(&mut host);
    let example = example(&host);
    press_send(&mut host);

    let state = host.editor_state();
    let workspace = &state.editor_ui.workspace;
    assert_eq!(workspace.draft_template, Some("slide-deck"));
    assert_eq!(workspace.phase, WorkspacePhase::Generating);
    assert!(!workspace.draft_banner_visible());
    let boards = active_page_boards(state);
    assert_eq!(boards.len(), 6, "the six-slide deck is on the page at once");
    assert_eq!(first_board_width(&host), Some(1920.0));

    let state = host.editor_state();
    assert_eq!(state.chat.launch_route, LaunchRoute::Refine);
    let sent = state.chat.pending_send.as_deref().expect("a refine turn");
    assert!(
        sent.contains("不要重新开始"),
        "the brief says refine, not restart"
    );
    assert!(
        sent.ends_with(example.trim()),
        "the example is the user's need"
    );
    assert_eq!(state.chat.messages[0].content, sent);
    // The modify route edits exactly the selected frames: every board.
    let selected: Vec<String> = state
        .selection
        .set
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();
    assert_eq!(selected, boards);
}

#[test]
fn choosing_the_example_then_send_takes_the_same_instant_path() {
    let mut host = home_on(HomeFamily::Infographic);
    connect_model(&mut host);
    let example = example(&host);
    host.editor_state_mut().editor_ui.home.use_example(&example);
    assert_eq!(host.editor_state().editor_ui.home.draft, example);
    press_send(&mut host);
    assert_eq!(
        host.editor_state().editor_ui.workspace.draft_template,
        Some("data-report-infographic")
    );
    assert_eq!(host.editor_state().chat.launch_route, LaunchRoute::Refine);
}

#[test]
fn an_edited_example_is_the_users_own_brief_and_generates_as_before() {
    let mut host = home_on(HomeFamily::Web);
    connect_model(&mut host);
    host.editor_state_mut()
        .editor_ui
        .home
        .set_draft("我的面包店官网，暖黄配色");
    press_send(&mut host);
    let state = host.editor_state();
    assert_eq!(state.editor_ui.workspace.draft_template, None);
    assert_eq!(state.chat.launch_route, LaunchRoute::Orchestrator);
    assert!(state
        .chat
        .pending_send
        .as_deref()
        .is_some_and(|sent| sent.contains("我的面包店官网")));
}

#[test]
fn a_task_without_a_template_falls_back_to_generating_the_example() {
    // App 界面 ships no app-screen template: a fast draft of the wrong
    // type would be worse than today's generation.
    let mut host = home_on(HomeFamily::AppUi);
    connect_model(&mut host);
    let example = example(&host);
    press_send(&mut host);
    let state = host.editor_state();
    assert_eq!(state.editor_ui.workspace.draft_template, None);
    assert_eq!(state.chat.launch_route, LaunchRoute::Orchestrator);
    let expected = HomeFamily::AppUi
        .generation_prompt(&op_editor_core::TaskDraft {
            text: example,
            ..Default::default()
        })
        .expect("wrapped");
    assert_eq!(state.chat.pending_send.as_deref(), Some(expected.as_str()));

    // No model and no template: today's connect card, nothing queued.
    let mut offline = home_on(HomeFamily::AppUi);
    press_send(&mut offline);
    assert!(offline.editor_state().editor_ui.home.connect_card_open);
    assert!(offline.editor_state().chat.pending_send.is_none());
    assert!(!offline.editor_state().editor_ui.workspace.active);
}

#[test]
fn a_page_with_real_work_is_parked_before_the_draft_replaces_it() {
    let mut host = home_on(HomeFamily::EventPoster);
    // The user's previous work: a board that is not the blank starter.
    let state = host.editor_state_mut();
    let work: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame", "id": "mine", "name": "My poster",
        "width": 600, "height": 800, "children": []
    }))
    .expect("fixture");
    state.active_children_mut().clear();
    state.active_children_mut().push(work);
    press_send(&mut host);

    let replaced = host
        .take_replaced_home_document()
        .expect("the previous document is handed to the shell, never dropped");
    assert!(replaced
        .state
        .active_children()
        .iter()
        .any(|node| op_editor_core::PenNodeExt::id_str(node) == "mine"));
    assert_eq!(
        host.editor_state().editor_ui.workspace.draft_template,
        Some("music-fest-poster-card")
    );
    assert!(host
        .editor_state()
        .active_children()
        .iter()
        .all(|node| op_editor_core::PenNodeExt::id_str(node) != "mine"));
}

fn press_workspace_hit(host: &mut WidgetHostNative, want: WorkspaceHit) {
    let point = {
        let surface = WorkspaceSurface::for_editor(host.editor_state()).expect("workspace");
        let layout = surface.layout(W, H);
        let rect = match want {
            WorkspaceHit::DraftAction => surface.draft_banner_button(&layout),
            WorkspaceHit::Retry => surface.banner_buttons(&layout).map(|(retry, _)| retry),
            _ => None,
        }
        .expect("the banner button is up");
        assert_eq!(surface.hit_test_layout(&layout, center(rect)), Some(want));
        center(rect)
    };
    assert!(host.apply_press(point.x, point.y, W, H));
}

#[test]
fn the_draft_banner_connects_first_and_refines_once_a_model_is_there() {
    let mut host = home_on(HomeFamily::KnowledgeCards);
    press_send(&mut host);
    assert!(host
        .editor_state()
        .editor_ui
        .workspace
        .draft_banner_visible());

    press_workspace_hit(&mut host, WorkspaceHit::DraftAction);
    assert!(
        host.editor_state().editor_ui.agent_settings_open,
        "no model: the banner opens the connect path"
    );
    assert!(host.editor_state().chat.pending_send.is_none());

    host.editor_state_mut().editor_ui.agent_settings_open = false;
    connect_model(&mut host);
    let boards_before = active_page_boards(host.editor_state());
    press_workspace_hit(&mut host, WorkspaceHit::DraftAction);
    let state = host.editor_state();
    assert_eq!(state.editor_ui.workspace.phase, WorkspacePhase::Generating);
    assert_eq!(state.chat.launch_route, LaunchRoute::Refine);
    assert!(state.chat.pending_send.is_some());
    assert_eq!(
        active_page_boards(state),
        boards_before,
        "the refine works on the same boards — no new document"
    );
}

#[test]
fn retry_after_a_stopped_refine_refines_the_same_draft_again() {
    let mut host = home_on(HomeFamily::ScreenshotTutorial);
    connect_model(&mut host);
    press_send(&mut host);
    // The shell drains whatever the first press replaced.
    let _ = host.take_replaced_home_document();
    let boards = active_page_boards(host.editor_state());
    // The launcher drained the turn, then the user pressed Stop.
    {
        let state = host.editor_state_mut();
        state.chat.pending_send = None;
        state.chat.launch_route = LaunchRoute::Auto;
        state.editor_ui.workspace.run_epoch = 7;
        assert!(state.editor_ui.workspace.mark_stopped(7));
    }
    // A late idle edge from the stopped run must not flip it to Done.
    assert!(!host.settle_workspace_idle_edge(7, boards.len(), false, W, H));
    assert_eq!(
        host.editor_state().editor_ui.workspace.phase,
        WorkspacePhase::Stopped
    );

    press_workspace_hit(&mut host, WorkspaceHit::Retry);
    let state = host.editor_state();
    assert_eq!(state.editor_ui.workspace.phase, WorkspacePhase::Generating);
    assert_eq!(
        state.editor_ui.workspace.run_epoch, 0,
        "a new run, unstamped"
    );
    assert_eq!(state.chat.launch_route, LaunchRoute::Refine);
    assert!(state.chat.pending_send.is_some());
    assert_eq!(
        active_page_boards(state),
        boards,
        "retry refines the draft in place instead of starting a fresh page"
    );
    assert!(
        host.take_replaced_home_document().is_none(),
        "nothing was swapped out"
    );
}
