//! Placement + overlay-cursor tests for the floating Git panel.
//!
//! Locks two behaviours the desktop relies on: the panel hangs
//! centred under the TopBar Git button (TS popover parity), and the
//! cursor stays neutral over it — no canvas Move / Crosshair bleeding
//! through a node sitting underneath the popover.

use super::helpers::GIT_PANEL_CARET_GAP;
use super::{CursorHint, WidgetHostNative};
use op_editor_core::{GitFileEntry, GitPanelAction};
use op_editor_ui::widgets::{GitPanel, GitPanelHit, TopBar, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

/// A host with the Git panel open in its no-repo onboarding state
/// (no saved file → the disabled-Init empty state).
fn host_with_git_panel_open() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let panel = &mut host.editor_state_mut().editor_ui.git_panel;
    panel.open = true;
    panel.loading = false;
    panel.in_repo = false;
    panel.has_saved_file = false;
    host
}

fn find_git_hit(panel: &GitPanel<'_>, body: Rect, target: GitPanelHit) -> Point2D {
    let mut y = body.origin.y;
    let max_y = body.origin.y + body.size.y + 140.0;
    while y <= max_y {
        let mut x = body.origin.x;
        let max_x = body.origin.x + body.size.x;
        while x <= max_x {
            let point = Point2D::new(x, y);
            if panel.hit_test(body, point) == Some(target) {
                return point;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    panic!("could not find git hit target {target:?}");
}

#[test]
fn open_git_popover_is_modal_and_dismisses_on_any_outside_press() {
    let mut host = WidgetHostNative::new();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.open = true;
        panel.loading = false;
        panel.in_repo = true; // clean bound repo → ready view
        panel.branch = Some("main".to_string());
        panel.branch_picker_open = true;
    }
    let (vw, vh) = (1440.0, 900.0);
    // A press far from the panel (top-left of the canvas) must close the
    // open branch-picker popover AND be consumed (modal).
    let consumed = host.apply_press(8.0, 220.0, vw, vh);
    assert!(consumed, "a press while a Git popover is open is consumed");
    assert!(
        !host.editor_state().editor_ui.git_panel.branch_picker_open,
        "an outside press must dismiss the open branch-picker popover"
    );
}

#[test]
fn git_commit_input_uses_text_input_state_for_editing() {
    let mut host = host_with_git_panel_open();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.in_repo = true;
        panel.branch = Some("main".to_string());
        panel.commit_focused = true;
        panel.commit_input.set_text("设计");
        panel.changed_files = vec![GitFileEntry {
            path: "design.op".into(),
            staged: true,
            status: 'M',
        }];
    }

    assert!(host.apply_select_all());
    {
        let panel = &host.editor_state().editor_ui.git_panel;
        assert!(panel.commit_input.is_select_all());
    }

    assert!(host.apply_text('改'));
    assert_eq!(
        host.editor_state().editor_ui.git_panel.commit_input.text(),
        "改"
    );

    assert!(host.apply_text('进'));
    assert_eq!(
        host.editor_state().editor_ui.git_panel.commit_input.text(),
        "改进"
    );

    assert!(host.apply_backspace());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.commit_input.text(),
        "改"
    );

    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.pending_action,
        Some(GitPanelAction::Commit)
    );
}

#[test]
fn git_remote_inputs_use_text_input_state_for_editing() {
    let mut host = host_with_git_panel_open();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.in_repo = true;
        panel.branch = Some("main".to_string());
        panel.remote_focused = true;
        panel.remote_input.set_text("https://old.example/repo.git");
    }

    assert!(host.apply_select_all());
    assert!(host.apply_text('新'));
    assert_eq!(
        host.editor_state().editor_ui.git_panel.remote_input.text(),
        "新"
    );
    assert!(host.apply_text('址'));
    assert_eq!(
        host.editor_state().editor_ui.git_panel.remote_input.text(),
        "新址"
    );
    assert!(host.apply_backspace());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.remote_input.text(),
        "新"
    );
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.pending_action,
        Some(GitPanelAction::SetRemote("新".to_string()))
    );

    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.pending_action = None;
        panel.remote_focused = false;
        panel.https_focused = true;
        panel.https_input.set_text("user:old-token");
    }
    assert!(host.apply_select_all());
    for c in "user:new-token".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.pending_action,
        Some(GitPanelAction::SetHttpsAuth("user:new-token".to_string()))
    );
}

#[test]
fn git_branch_create_input_uses_text_input_state_for_editing() {
    let mut host = host_with_git_panel_open();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.in_repo = true;
        panel.branch = Some("main".to_string());
        panel.branch_picker_open = true;
        panel.branch_create_focused = true;
        panel.branch_create_input.set_text("old");
    }

    assert!(host.apply_select_all());
    for c in "feature/new".chars() {
        assert!(host.apply_text(c));
    }
    assert_eq!(
        host.editor_state()
            .editor_ui
            .git_panel
            .branch_create_input
            .text(),
        "feature/new"
    );
    assert!(host.apply_send());
    let panel = &host.editor_state().editor_ui.git_panel;
    assert_eq!(
        panel.pending_action,
        Some(GitPanelAction::CreateBranch("feature/new".to_string()))
    );
    assert!(panel.branch_create_input.text().is_empty());
    assert!(!panel.branch_create_focused);
}

#[test]
fn stale_git_popover_flag_does_not_dead_end_input() {
    // A popover flag left `true` while the panel is NOT in the ready
    // view (here: a dirty working tree) must not swallow + drop every
    // press: the modal guard only consumes a press the Git panel
    // actually handled, so an outside click still reaches the canvas.
    let mut host = WidgetHostNative::new();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.open = true;
        panel.loading = false;
        panel.in_repo = true;
        panel.branch = Some("main".to_string());
        // Dirty tree → NOT the ready view (so `hit_test` won't capture
        // the popover), yet the flag is stale-true.
        panel.changed_files = vec![op_editor_core::GitFileEntry {
            path: "x.op".into(),
            staged: false,
            status: 'M',
        }];
        panel.branch_picker_open = true;
    }
    let (vw, vh) = (1440.0, 900.0);
    // A press on the empty canvas (centre, well below the panel) must
    // reach the empty-canvas handler (which consumes it) rather than
    // being dead-ended to `false` by the modal guard.
    assert!(
        host.apply_press(700.0, 600.0, vw, vh),
        "a stale popover flag must not dead-end the press"
    );
}

#[test]
fn git_panel_hangs_centred_under_the_git_button() {
    let host = host_with_git_panel_open();
    let (vw, vh) = (1400.0, 900.0);
    let r = host.git_panel_rect(vw, vh).expect("panel open => Some");

    // Centre-x aligns with the Git button centre (a wide viewport
    // leaves the centred panel un-clamped).
    let top_bar_rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(vw, TOP_BAR_HEIGHT),
    };
    let btn_cx = TopBar::for_editor_ui(&host.editor_state().editor_ui)
        .git_button_center_x(top_bar_rect)
        .expect("Git button is shown on desktop");
    let panel_cx = r.origin.x + r.size.x / 2.0;
    assert!(
        (panel_cx - btn_cx).abs() < 0.5,
        "panel centre {panel_cx} should align with button centre {btn_cx}",
    );

    // Hangs just below the top bar, leaving room for the up-caret.
    assert_eq!(r.origin.y, TOP_BAR_HEIGHT + GIT_PANEL_CARET_GAP);
}

#[test]
fn git_panel_pins_to_the_inset_when_the_canvas_is_too_narrow() {
    let host = host_with_git_panel_open();
    // Both rails open on a small viewport leave little/no canvas — the
    // 380px panel can't fit at all. It must still pin to the left
    // inset (never slide off-screen-left or under the layer rail)
    // rather than centre off the edge.
    let (vw, vh) = (520.0, 700.0);
    let r = host.git_panel_rect(vw, vh).expect("panel open => Some");
    let (canvas_left, _t, _cw, _h) = host.canvas_region(vw, vh);
    assert!(
        r.origin.x >= canvas_left - 0.01,
        "panel left {} pins at/after the canvas inset {canvas_left}",
        r.origin.x
    );
}

#[test]
fn cursor_stays_neutral_over_the_git_panel() {
    let host = host_with_git_panel_open();
    let (vw, vh) = (1400.0, 900.0);
    let r = host.git_panel_rect(vw, vh).unwrap();
    let mx = r.origin.x + r.size.x / 2.0;
    let my = r.origin.y + r.size.y / 2.0;

    assert!(
        host.over_floating_overlay(mx, my, vw, vh),
        "the Git panel must count as a floating overlay",
    );
    assert_eq!(
        host.cursor_hint(mx, my, vw, vh),
        CursorHint::Default,
        "no canvas action cursor bleeds through the panel",
    );

    // Coverage is not git-panel-only: the always-on Toolbar (a
    // floating widget over the canvas) suppresses the cursor too.
    let tb = host.toolbar_rect(vw, vh);
    assert!(
        host.over_floating_overlay(
            tb.origin.x + tb.size.x / 2.0,
            tb.origin.y + tb.size.y / 2.0,
            vw,
            vh,
        ),
        "the floating Toolbar must also count as an overlay",
    );

    // The bare centre of the canvas (below the top-anchored panel,
    // clear of the corner-anchored chat / status widgets) must NOT be
    // treated as an overlay — otherwise the fix would be too greedy.
    let (canvas_left, _t, canvas_w, canvas_h) = host.canvas_region(vw, vh);
    let bare_x = canvas_left + canvas_w / 2.0;
    let bare_y = TOP_BAR_HEIGHT + canvas_h / 2.0;
    assert!(
        !host.over_floating_overlay(bare_x, bare_y, vw, vh),
        "bare canvas must not be flagged as an overlay",
    );
}

#[test]
fn caret_bridge_catches_cursor_and_clicks() {
    let mut host = host_with_git_panel_open();
    let (vw, vh) = (1400.0, 900.0);
    let body = host.git_panel_rect(vw, vh).expect("panel open => Some");
    // A probe in the caret bridge: below the top bar, above the body.
    let caret_x = body.origin.x + body.size.x / 2.0;
    let caret_y = (TOP_BAR_HEIGHT + body.origin.y) / 2.0;
    assert!(
        caret_y > TOP_BAR_HEIGHT && caret_y < body.origin.y,
        "probe sits in the gap above the painted body",
    );

    // The caret is painted as part of the popover, so the cursor must
    // stay neutral over it (no canvas Move cursor bleeding through).
    assert!(
        host.over_floating_overlay(caret_x, caret_y, vw, vh),
        "the caret bridge counts as the Git overlay",
    );
    assert_eq!(
        host.cursor_hint(caret_x, caret_y, vw, vh),
        CursorHint::Default
    );

    // And a click on the caret is swallowed by the panel rather than
    // falling through to the canvas underneath.
    assert!(
        host.dispatch_git_panel_press(caret_x, caret_y, vw, vh),
        "a caret click is consumed by the Git panel",
    );
}

#[test]
fn init_card_hover_tracks_the_card_index_and_not_allowed_cursor() {
    // `host_with_git_panel_open` leaves `has_saved_file == false`, so
    // the Init card (index 0) is the disabled one.
    let mut host = host_with_git_panel_open();
    let (vw, vh) = (1400.0, 900.0);
    host.last_viewport_w = vw;
    host.last_viewport_h = vh;
    let body = host.git_panel_rect(vw, vh).expect("panel open => Some");
    let card = GitPanel::for_editor(host.editor_state())
        .and_then(|p| p.empty_init_card_rect(body))
        .expect("empty state => an Init card rect");
    let cx = card.origin.x + card.size.x / 2.0;
    let cy = card.origin.y + card.size.y / 2.0;

    // Nothing is flagged until the cursor is over a card.
    assert_eq!(
        host.editor_state().editor_ui.git_panel.empty_hovered_card,
        None
    );
    assert!(
        host.update_git_panel_empty_hover(cx, cy),
        "moving onto the Init card records its index (a repaint is due)",
    );
    assert_eq!(
        host.editor_state().editor_ui.git_panel.empty_hovered_card,
        Some(0)
    );
    // The disabled Init card shows the not-allowed cursor.
    assert_eq!(host.cursor_hint(cx, cy, vw, vh), CursorHint::NotAllowed);

    // Moving off the cards clears it again.
    assert!(
        host.update_git_panel_empty_hover(card.origin.x - 40.0, cy),
        "moving off the cards clears the hovered index",
    );
    assert_eq!(
        host.editor_state().editor_ui.git_panel.empty_hovered_card,
        None
    );
}

#[test]
fn git_popover_row_hover_uses_shared_menu_state() {
    let mut host = WidgetHostNative::new();
    let (vw, vh) = (1400.0, 900.0);
    host.last_viewport_w = vw;
    host.last_viewport_h = vh;
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.open = true;
        panel.loading = false;
        panel.in_repo = true;
        panel.branch = Some("main".to_string());
        panel.overflow_open = true;
    }
    let body = host.git_panel_rect(vw, vh).expect("panel open");
    let panel = GitPanel::for_editor(host.editor_state()).expect("panel widget");
    let point = find_git_hit(&panel, body, GitPanelHit::OverflowRemoteSettings);
    assert!(host.update_git_panel_ready_hover(point.x, point.y));
    assert_eq!(
        host.editor_state().editor_ui.git_panel.overflow_menu.hover,
        Some(2)
    );

    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.overflow_open = false;
        panel.overflow_menu.hover = None;
        panel.branch_picker_open = true;
        panel.branches = vec!["main".to_string(), "feature".to_string()];
    }
    let panel = GitPanel::for_editor(host.editor_state()).expect("panel widget");
    let point = find_git_hit(&panel, body, GitPanelHit::SwitchBranch(1));
    assert!(host.update_git_panel_ready_hover(point.x, point.y));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .git_panel
            .branch_picker_menu
            .hover,
        Some(1)
    );
}

#[test]
fn clone_wizard_owns_keyboard_and_enter() {
    use op_editor_core::{CloneField, CloneFormState, GitPanelAction};
    let mut host = host_with_git_panel_open();
    host.editor_state_mut().editor_ui.git_panel.clone_form = Some(CloneFormState {
        focus: Some(CloneField::Url),
        ..Default::default()
    });
    // The wizard captures the keyboard, so a tool-letter like `h` types
    // into the URL field instead of switching tools (the regression the
    // first clone-form review caught: `https://…` was untypeable).
    assert!(host.git_clone_input_active());
    assert!(host.input_active());
    for c in "http".chars() {
        assert!(host.apply_text(c), "char should be consumed by the wizard");
    }
    assert_eq!(
        host.editor_state()
            .editor_ui
            .git_panel
            .clone_form
            .as_ref()
            .unwrap()
            .url_input
            .text(),
        "http"
    );
    // Enter on a focused field requests the clone.
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.pending_action,
        Some(GitPanelAction::SubmitClone)
    );
    // Enter with no field focused is swallowed (returns true, no
    // fall-through to chat send) and does not re-queue a submit.
    host.editor_state_mut().editor_ui.git_panel.pending_action = None;
    host.editor_state_mut()
        .editor_ui
        .git_panel
        .clone_form
        .as_mut()
        .unwrap()
        .focus = None;
    assert!(host.apply_send());
    assert_eq!(host.editor_state().editor_ui.git_panel.pending_action, None);
}

#[test]
fn hidden_clone_form_does_not_capture_keyboard() {
    use op_editor_core::{CloneField, CloneFormState};
    let mut host = host_with_git_panel_open();
    host.editor_state_mut().editor_ui.git_panel.clone_form = Some(CloneFormState {
        focus: Some(CloneField::Url),
        ..Default::default()
    });
    // Close the panel: the wizard is now hidden state. It must NOT own
    // the keyboard, or keystrokes would silently edit an invisible form
    // (and Delete / arrows would leak to the document).
    host.editor_state_mut().editor_ui.git_panel.open = false;
    assert!(!host.git_clone_input_active());
    host.apply_text('h');
    assert_eq!(
        host.editor_state()
            .editor_ui
            .git_panel
            .clone_form
            .as_ref()
            .unwrap()
            .url_input
            .text(),
        "",
        "a hidden clone form must not capture keystrokes"
    );
}

#[test]
fn canvas_press_dismisses_the_open_git_panel() {
    // The Git panel is a popover — a press on the canvas (outside the
    // panel body + caret bridge) closes it, the way clicking off a popover
    // does, and is consumed so it doesn't also clear selection / pan.
    let mut host = host_with_git_panel_open();
    let (vw, vh) = (1400.0, 900.0);
    let body = host.git_panel_rect(vw, vh).expect("panel open => Some");
    // A point centred under the panel but well below its body — on the
    // empty canvas, clear of the panel, caret bridge, toolbar, and chat.
    let px = body.origin.x + body.size.x / 2.0;
    let py = body.origin.y + body.size.y + 120.0;
    assert!(py < vh - 80.0, "probe sits on-screen below the panel");
    assert!(
        host.apply_press(px, py, vw, vh),
        "a canvas press while the panel is open is consumed by the dismiss",
    );
    assert!(
        !host.editor_state().editor_ui.git_panel.open,
        "a canvas press dismisses the open Git-panel popover",
    );
}

#[test]
fn clone_wizard_accepts_pasted_url() {
    use op_editor_core::{CloneField, CloneFormState};
    let mut host = host_with_git_panel_open();
    host.editor_state_mut().editor_ui.git_panel.clone_form = Some(CloneFormState {
        focus: Some(CloneField::Url),
        ..Default::default()
    });
    // Pasting a URL (with a stray trailing newline) lands in the focused
    // field; control characters are dropped (single-line input).
    assert!(host.apply_input_paste("https://github.com/owner/repo.git\n"));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .git_panel
            .clone_form
            .as_ref()
            .unwrap()
            .url_input
            .text(),
        "https://github.com/owner/repo.git"
    );
}

#[test]
fn clone_wizard_select_all_replaces_only_the_focused_field() {
    use op_editor_core::{CloneField, CloneFormState};
    let mut host = host_with_git_panel_open();
    let mut form = CloneFormState {
        focus: Some(CloneField::Dest),
        ..Default::default()
    };
    form.url_input.set_text("https://github.com/owner/repo.git");
    form.dest_input.set_text("/tmp/repo");
    host.editor_state_mut().editor_ui.git_panel.clone_form = Some(form);

    assert!(host.apply_select_all());
    assert!(host.apply_text('x'));

    let form = host
        .editor_state()
        .editor_ui
        .git_panel
        .clone_form
        .as_ref()
        .unwrap();
    assert_eq!(form.url_input.text(), "https://github.com/owner/repo.git");
    assert_eq!(form.dest_input.text(), "x");
    assert_eq!(form.dest_input.caret(), 1);
}
