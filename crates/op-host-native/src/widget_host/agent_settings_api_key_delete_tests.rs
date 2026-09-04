//! End-to-end reproduction of the "API key cannot be deleted" report:
//! open the settings modal, enter the built-in provider edit form the
//! way the product does (press-driven), type into the API-key field,
//! then delete every character and commit. Regression coverage for the
//! full press + keyboard ladder, not just the direct state transitions.

use super::WidgetHostNative;
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::PenNodeExt;
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 1000.0;

const TOUCH_VIEWPORT_W: f32 = 390.0;
const TOUCH_VIEWPORT_H: f32 = 844.0;

fn desktop_settings_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings_open = true;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Provider", "sk-old", "model-0");
    host
}

fn touch_settings_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = true;
    ui.size_class = EditorSizeClass::Compact;
    ui.agent_settings_open = true;
    ui.agent_settings
        .add_builtin_agent_with_defaults("Provider", "sk-old", "model-0");
    host
}

/// Screen point over the API-key input of the expanded card for agent 0.
/// The card must already be expanded (a `BuiltinAgent { index: 0, .. }`
/// focus) — the row rect is identical whichever field of the card holds
/// focus, so probing it through `focused_input_rect` with a temporary
/// ApiKey focus matches what paint lays out.
fn api_key_field_point(host: &mut WidgetHostNative, vw: f32, vh: f32) -> Point2D {
    let restore = host.editor_state().editor_ui.agent_settings.focus;
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    let point = {
        let (panel, panel_rect) = host.agent_settings_geometry(vw, vh);
        let mut input = panel
            .focused_input_rect(panel_rect)
            .expect("expanded card exposes the api-key input");
        input.origin.y -= panel.effective_scroll(panel_rect);
        // Press near the right edge: tap-to-caret honors the pressed x, so
        // this parks the caret at the end and the typing/backspace ladders
        // below operate on the tail like they used to.
        Point2D::new(
            input.origin.x + input.size.x - 10.0,
            input.origin.y + input.size.y / 2.0,
        )
    };
    host.editor_state_mut().editor_ui.agent_settings.focus = restore;
    point
}

fn press_api_key_field(host: &mut WidgetHostNative, vw: f32, vh: f32) {
    // Enter the edit form the way the product does: the card's edit
    // press focuses DisplayName and expands the card.
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::DisplayName,
    });
    op_editor_core::host_ui_transitions::set_settings_input_text(
        &mut host.editor_state_mut().editor_ui,
        "Provider".into(),
        0,
    );
    let point = api_key_field_point(host, vw, vh);
    assert!(host.apply_press(point.x, point.y, vw, vh));
    if host.editor_state().editor_ui.touch_chrome() {
        assert!(host.apply_release_with_viewport(vw, vh));
    }
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.focus,
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        }),
        "press must land on the api-key input"
    );
}

#[test]
fn desktop_api_key_backspace_deletes_typed_characters() {
    let mut host = desktop_settings_host();
    press_api_key_field(&mut host, VIEWPORT_W, VIEWPORT_H);
    assert_eq!(
        host.editor_state().editor_ui.settings_input.text(),
        "sk-old"
    );

    // Type two more characters through the full keyboard ladder.
    assert!(host.apply_text('x'));
    assert!(host.apply_text('y'));
    assert_eq!(
        host.editor_state().editor_ui.settings_input.text(),
        "sk-oldxy"
    );

    // Backspace through the full ladder must pop characters one by one.
    for expected in ["sk-oldx", "sk-old", "sk-ol", "sk-o", "sk-", "sk", "s", ""] {
        assert!(host.apply_backspace(), "backspace must be consumed");
        assert_eq!(
            host.editor_state().editor_ui.settings_input.text(),
            expected
        );
    }
    // Empty input: backspace still owned by the settings field (never
    // falls through to delete a canvas node).
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "");
}

#[test]
fn desktop_cleared_api_key_commits_empty_on_close() {
    let mut host = desktop_settings_host();
    press_api_key_field(&mut host, VIEWPORT_W, VIEWPORT_H);
    while !host
        .editor_state()
        .editor_ui
        .settings_input
        .text()
        .is_empty()
    {
        assert!(host.apply_backspace());
    }

    // Close the modal — the focused draft commits.
    assert!(host.apply_toggle_agent_settings());
    assert!(!host.editor_state().editor_ui.agent_settings_open);
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.builtin_agents[0].api_key,
        "",
        "an emptied api-key draft must commit as empty (the key is deletable)"
    );
}

#[test]
fn touch_api_key_backspace_deletes_typed_characters() {
    let mut host = touch_settings_host();
    press_api_key_field(&mut host, TOUCH_VIEWPORT_W, TOUCH_VIEWPORT_H);
    assert_eq!(
        host.editor_state().editor_ui.settings_input.text(),
        "sk-old"
    );

    assert!(host.apply_text('z'));
    assert_eq!(
        host.editor_state().editor_ui.settings_input.text(),
        "sk-oldz"
    );

    for expected in ["sk-old", "sk-ol", "sk-o", "sk-", "sk", "s", ""] {
        assert!(host.apply_backspace(), "backspace must be consumed");
        assert_eq!(
            host.editor_state().editor_ui.settings_input.text(),
            expected
        );
    }
}

/// Forward Delete while the API-key input owns the keyboard must edit
/// the draft — and must NEVER fall through to canvas node deletion
/// behind the modal. This is the reported "cannot delete the key" hole:
/// `apply_delete` had no settings arm and `delete_owned_by_chrome_input`
/// did not list the settings focus, so the keystroke destroyed the
/// selected node while the field stayed untouched.
#[test]
fn desktop_api_key_forward_delete_edits_field_and_never_deletes_nodes() {
    let mut host = desktop_settings_host();
    let node_count = host.editor_state().active_children().len();
    assert!(node_count > 0, "fixture document must have nodes");
    let first = host.editor_state().active_children()[0].base().id.clone();
    host.editor_state_mut()
        .set_single_selection(op_editor_core::NodeId::new(first));

    press_api_key_field(&mut host, VIEWPORT_W, VIEWPORT_H);
    assert_eq!(
        host.editor_state().editor_ui.settings_input.text(),
        "sk-old"
    );
    // Move the caret to the start, then forward-delete twice.
    for _ in 0.."sk-old".len() {
        assert!(host.apply_settings_caret(false));
    }
    assert!(
        host.apply_delete(),
        "Delete must be consumed by the focused settings input"
    );
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "k-old");
    assert!(host.apply_delete());
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "-old");
    // Forward delete at the buffer end is a no-op but still owned.
    for _ in 0..8 {
        let _ = host.apply_delete();
    }
    assert_eq!(
        host.editor_state().active_children().len(),
        node_count,
        "Delete in a settings field must never remove canvas nodes"
    );
}

/// Same guarantee on touch chrome (the iOS shell forwards mid-text
/// deletions as `KEY_DELETE`).
#[test]
fn touch_api_key_forward_delete_edits_field_and_never_deletes_nodes() {
    let mut host = touch_settings_host();
    let node_count = host.editor_state().active_children().len();
    let first = host.editor_state().active_children()[0].base().id.clone();
    host.editor_state_mut()
        .set_single_selection(op_editor_core::NodeId::new(first));

    press_api_key_field(&mut host, TOUCH_VIEWPORT_W, TOUCH_VIEWPORT_H);
    for _ in 0..2 {
        assert!(host.apply_settings_caret(false));
    }
    assert!(host.apply_delete());
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "sk-od");
    assert_eq!(
        host.editor_state().active_children().len(),
        node_count,
        "Delete in a settings field must never remove canvas nodes"
    );
}

/// The add-provider draft path: `+ 添加服务商` focuses the draft's
/// API-key input directly; typing then backspacing must edit the draft.
#[test]
fn touch_add_provider_draft_api_key_backspace_works() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.touch = true;
        ui.size_class = EditorSizeClass::Compact;
        ui.agent_settings_open = true;
    }
    let outcome = op_editor_ui::widgets::agent_settings_press_flow::apply_agent_settings_hit(
        host.editor_state_mut(),
        op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::AddProvider,
        op_editor_core::host_settings_commit::SettingsCommitScope::Operator,
        0,
    );
    assert_eq!(
        outcome.effect,
        op_editor_ui::widgets::agent_settings_press_flow::SettingsPress::Handled
    );
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.focus,
        Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey))
    );

    for c in "sk-typo".chars() {
        assert!(host.apply_text(c));
    }
    assert_eq!(
        host.editor_state().editor_ui.settings_input.text(),
        "sk-typo"
    );
    for expected in ["sk-typ", "sk-ty", "sk-t", "sk-", "sk", "s", ""] {
        assert!(host.apply_backspace(), "backspace must be consumed");
        assert_eq!(
            host.editor_state().editor_ui.settings_input.text(),
            expected
        );
    }
}

/// The editing card's `删除服务商` row (added in 17211f7b9) must remove
/// the provider — the report's "cannot delete the API key" also reads as
/// "cannot delete the provider entry that holds it".
#[test]
fn touch_delete_provider_row_removes_the_agent() {
    let mut host = touch_settings_host();
    // Expand the edit form.
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::DisplayName,
    });
    op_editor_core::host_ui_transitions::set_settings_input_text(
        &mut host.editor_state_mut().editor_ui,
        "Provider".into(),
        0,
    );
    // Locate the delete affordance inside the expanded card through the
    // real hit-test. The action row is Save-primary with a trailing
    // destructive square, so probe near the row's right edge for
    // RemoveBuiltinAgent — and assert the centre of the same row resolves
    // to the Save action, guarding the primary/secondary split.
    let point = {
        let (panel, panel_rect) = host.agent_settings_geometry(TOUCH_VIEWPORT_W, TOUCH_VIEWPORT_H);
        let content = panel.resolved_content_viewport(panel_rect);
        let scroll = panel.effective_scroll(panel_rect);
        let mut found = None;
        let x = content.origin.x + content.size.x - 40.0;
        let mut y = content.origin.y;
        while y < content.origin.y + content.size.y {
            let hit = panel.hit_test(panel_rect, op_editor_ui::Point2D::new(x, y));
            if hit
                == op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::RemoveBuiltinAgent(
                    0,
                )
            {
                let centre_hit = panel.hit_test(
                    panel_rect,
                    op_editor_ui::Point2D::new(content.origin.x + content.size.x / 2.0, y),
                );
                assert_eq!(
                    centre_hit,
                    op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit::SaveBuiltinAgentEditing(0),
                    "the action row's centre must be the primary Save button"
                );
                found = Some(op_editor_ui::Point2D::new(x, y));
                break;
            }
            y += 4.0;
        }
        let _ = scroll;
        found
    };
    let point = point.unwrap_or_else(|| {
        panic!("the expanded card must expose a hit-testable delete-provider row")
    });
    assert!(host.apply_press(point.x, point.y, TOUCH_VIEWPORT_W, TOUCH_VIEWPORT_H));
    assert!(host.apply_release_with_viewport(TOUCH_VIEWPORT_W, TOUCH_VIEWPORT_H));
    assert!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .builtin_agents
            .is_empty(),
        "pressing the delete-provider row must remove the agent"
    );
}

#[test]
fn touch_cleared_api_key_commits_empty_on_close() {
    let mut host = touch_settings_host();
    press_api_key_field(&mut host, TOUCH_VIEWPORT_W, TOUCH_VIEWPORT_H);
    while !host
        .editor_state()
        .editor_ui
        .settings_input
        .text()
        .is_empty()
    {
        assert!(host.apply_backspace());
    }

    assert!(host.apply_toggle_agent_settings());
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.builtin_agents[0].api_key,
        "",
        "an emptied api-key draft must commit as empty (the key is deletable)"
    );
}
