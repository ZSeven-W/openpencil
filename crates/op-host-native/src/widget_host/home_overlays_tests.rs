//! gl-host tests for Home's model-access surface: the model chip, the
//! connect card, the Home-anchored picker, and the orchestrator launch
//! route. Run with `--features gl-host`.

use super::WidgetHostNative;
use op_editor_core::{
    BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, HomeHit, LaunchRoute,
};
use op_editor_ui::widgets::ai_chat_model_picker::{MODEL_GROUP_H, MODEL_ROW_H, MODEL_SEARCH_H};
use op_editor_ui::widgets::HomeSurface;
use op_editor_ui::{Point2D, Rect};

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn host_with_home() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.home.visible = true;
    host
}

fn host_with_usable_agent() -> WidgetHostNative {
    let mut host = host_with_home();
    // A ready built-in agent: opening the picker rebuilds the catalog
    // (`rebuild_chat_models`), and built-in entries survive that
    // rebuild without CLI discovery or verified-connection state.
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
            models: vec!["MiniMax-M2.7".into(), "MiniMax-M3".into()],
            base_url: "http://localhost:9".into(),
            enabled: true,
        });
    host
}

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn chip_center(host: &WidgetHostNative) -> Point2D {
    let home = HomeSurface::for_editor(host.editor_state()).expect("home visible");
    center(home.layout(W, H).model_chip)
}

#[test]
fn pressing_the_model_chip_opens_the_chat_model_picker() {
    let mut host = host_with_usable_agent();
    let point = chip_center(&host);
    assert!(host.apply_press(point.x, point.y, W, H));
    assert!(
        host.editor_state().editor_ui.chat_model_picker.open,
        "the chip must open the same picker the chat panel uses"
    );
}

#[test]
fn pressing_the_model_chip_without_a_usable_agent_opens_the_connect_card() {
    let mut host = host_with_home();
    let point = chip_center(&host);
    assert!(host.apply_press(point.x, point.y, W, H));
    assert!(host.editor_state().editor_ui.home.connect_card_open);
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
}

#[test]
fn pressing_send_without_a_usable_agent_opens_the_connect_card_not_a_turn() {
    let mut host = host_with_home();
    host.editor_state_mut().editor_ui.home.set_draft("取餐预约");
    let home = HomeSurface::for_editor(host.editor_state()).expect("home visible");
    let send = center(home.layout(W, H).send);
    assert!(host.apply_press(send.x, send.y, W, H));
    assert!(host.editor_state().editor_ui.home.connect_card_open);
    assert!(
        host.editor_state().chat.pending_send.is_none(),
        "no turn may be queued while nothing can answer it"
    );
    assert!(host.home_visible(), "Home stays up under the card");
}

#[test]
fn connect_card_rows_open_their_modals_and_escape_closes_the_card() {
    let mut host = host_with_home();
    let chip = chip_center(&host);
    assert!(host.apply_press(chip.x, chip.y, W, H));
    let home = HomeSurface::for_editor(host.editor_state()).expect("home visible");
    let layout = home.layout(W, H);

    let api_key = center(layout.connect_rows[1]);
    assert!(host.apply_press(api_key.x, api_key.y, W, H));
    assert!(!host.editor_state().editor_ui.home.connect_card_open);
    assert!(host.editor_state().editor_ui.agent_settings_open);
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.tab,
        op_editor_core::AgentSettingsTab::Agents
    );

    // Re-open via Send this time, then Escape peels the card off. The
    // settings modal must be closed first — while it is open it owns
    // every press above Home.
    host.editor_state_mut().editor_ui.agent_settings_open = false;
    let send = center(layout.send);
    assert!(host.apply_press(send.x, send.y, W, H));
    assert!(host.editor_state().editor_ui.home.connect_card_open);
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.home.connect_card_open);
}

#[test]
fn home_send_marks_the_turn_for_the_orchestrator_route() {
    let mut host = host_with_usable_agent();
    for character in "取餐预约".chars() {
        assert!(host.apply_text(character));
    }
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().chat.launch_route,
        LaunchRoute::Orchestrator,
        "a Home-launched brief must take the orchestrator pipeline"
    );
    assert!(host.editor_state().chat.pending_send.is_some());
}

#[test]
fn picking_a_row_from_the_home_picker_selects_that_model() {
    let mut host = host_with_usable_agent();
    let chip = chip_center(&host);
    assert!(host.apply_press(chip.x, chip.y, W, H));
    let card = host
        .home_model_picker_geometry(W, H)
        .expect("picker geometry resolves above the chip");
    // Second row of the single Claude Code group: search strip + pad +
    // group header + one row, centred in the second 28 px row.
    let row_center = Point2D::new(
        card.origin.x + 24.0,
        card.origin.y + MODEL_SEARCH_H + 6.0 + MODEL_GROUP_H + MODEL_ROW_H + MODEL_ROW_H / 2.0,
    );
    assert!(host.apply_press(row_center.x, row_center.y, W, H));
    assert_eq!(
        host.editor_state()
            .chat
            .selected_model_entry()
            .map(|entry| entry.display_name.as_str()),
        Some("MiniMax-M3")
    );
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
}

#[test]
fn pressing_outside_the_home_picker_closes_it_without_touching_home() {
    let mut host = host_with_usable_agent();
    let chip = chip_center(&host);
    assert!(host.apply_press(chip.x, chip.y, W, H));
    assert!(host.editor_state().editor_ui.chat_model_picker.open);
    // A press far from the card closes the picker and is still consumed.
    assert!(host.apply_press(120.0, 120.0, W, H));
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
    assert!(host.home_visible(), "Home itself must survive the close");
}

#[test]
fn the_home_picker_connect_row_lives_inside_the_card_now() {
    // The action used to be an external row BELOW the card, which meant
    // Home showed 接入更多模型 twice (the picker carries its own footer)
    // and the cursor had to leave the popover to reach it.
    let mut host = host_with_usable_agent();
    let chip = chip_center(&host);
    assert!(host.apply_press(chip.x, chip.y, W, H));
    let card = host
        .home_model_picker_geometry(W, H)
        .expect("picker geometry resolves");
    // Nothing hangs below the card any more: a press just under it is
    // an outside press and closes the picker.
    let below = Point2D::new(card.origin.x + 24.0, card.origin.y + card.size.y + 20.0);
    assert!(host.apply_press(below.x, below.y, W, H));
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
    assert!(!host.editor_state().editor_ui.agent_settings_open);
}

/// Hover must never dismiss a click-opened popover: moving the cursor
/// off the card used to close it, so its own footer row was unreachable.
#[test]
fn moving_the_cursor_off_the_home_picker_keeps_it_open() {
    let mut host = host_with_usable_agent();
    let chip = chip_center(&host);
    assert!(host.apply_press(chip.x, chip.y, W, H));
    let card = host.home_model_picker_geometry(W, H).expect("picker open");
    assert!(host.editor_state().editor_ui.chat_model_picker.open);
    for point in [
        Point2D::new(card.origin.x - 40.0, card.origin.y + 20.0),
        Point2D::new(card.origin.x + 24.0, card.origin.y + card.size.y + 30.0),
        Point2D::new(W - 20.0, H - 20.0),
    ] {
        host.apply_cursor_move(point.x, point.y);
        assert!(
            host.editor_state().editor_ui.chat_model_picker.open,
            "hover at {point:?} must not dismiss"
        );
    }
}

#[test]
fn home_hit_test_still_routes_the_sheet_under_the_new_chip() {
    // The chip must not have swallowed the reference row or the sheet.
    let host = host_with_usable_agent();
    let home = HomeSurface::for_editor(host.editor_state()).expect("home visible");
    let layout = home.layout(W, H);
    let sheet = center(layout.input_box);
    assert_eq!(
        home.hit_test(W, H, sheet),
        Some(HomeHit::Sheet),
        "the input area keeps its caret-press hit"
    );
}

/// Both avatars — Home's and the professional TopBar's — must open the
/// same thing, or a first-run user finds a way in on one screen and a
/// different one on the other.
#[test]
fn home_and_top_bar_avatars_open_the_same_account_entry() {
    let mut host = WidgetHostNative::new();
    // Without the host's account gate neither entry does anything.
    assert!(!host.open_account_entry());
    assert!(!host.editor_state().editor_ui.login_modal_open);

    host.editor_state_mut().editor_ui.account_ui_available = true;
    assert!(host.open_account_entry());
    assert!(
        host.editor_state().editor_ui.login_modal_open,
        "signed out opens the login modal"
    );
    assert!(!host.editor_state().editor_ui.account_menu_open);
}

/// Home's avatar opens one of two things, and the takeover has to draw
/// and route BOTH — a signed-in user clicking it got an account menu
/// that existed in state and appeared nowhere.
#[test]
fn home_routes_the_signed_in_account_menu_as_well_as_the_sign_in_modal() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.home.visible = true;
    host.editor_state_mut().editor_ui.account_ui_available = true;
    // Signed out: the avatar opens the sign-in modal, and Home owns the press.
    assert!(host.open_account_entry());
    assert!(host.editor_state().editor_ui.login_modal_open);
    assert_eq!(
        host.press_home_overlays(400.0, 400.0, 1440.0, 900.0),
        Some(true),
        "the modal owns presses over the takeover"
    );

    // Signed in: the menu opens instead, and Home owns its presses too.
    host.editor_state_mut().editor_ui.login_modal_open = false;
    host.editor_state_mut().editor_ui.account_menu_open = true;
    assert_eq!(
        host.press_home_overlays(400.0, 400.0, 1440.0, 900.0),
        Some(true),
        "an open account menu must not let presses fall through Home"
    );
}

/// ...and the menu hangs off HOME's avatar, not the professional top
/// bar's, which is a different place on the same window.
#[test]
fn the_account_menu_anchors_to_homes_own_avatar() {
    let mut state = op_editor_core::EditorState::new();
    state.editor_ui.account_ui_available = true;
    let pro = op_editor_ui::widgets::touch_overlay_geometry::account_anchor(&state, 1440.0);
    state.editor_ui.home.visible = true;
    let home = op_editor_ui::widgets::touch_overlay_geometry::account_anchor(&state, 1440.0);
    assert_ne!(
        pro.origin.x, home.origin.x,
        "different avatars, different anchors"
    );
    // The anchor sits in Home's top bar, left of its 打开文件 button.
    assert!(home.size.x > 0.0 && home.size.y > 0.0);
    assert!(home.origin.y < 60.0, "in the top bar: {home:?}");
    assert!(home.origin.x < pro.origin.x, "left of the pro avatar");
}
