//! Missing-font prompt flow shared by the native and web widget hosts.
//!
//! The one-shot modal's scroll + press dispatch and the whole
//! detection/refresh bookkeeping were duplicated between
//! `op-host-native/.../missing_fonts_dispatch.rs` and
//! `op-host-web/.../missing_fonts_press.rs`. Everything here is
//! `EditorState` mutation plus widget-layer hit-tests (wasm32-clean);
//! only the *font enumeration* differs per platform — native scans
//! synchronously through FontMgr, the web host waits for CanvasKit's
//! `queryLocalFonts` drain — so the hosts keep their own arming
//! wrappers around [`arm_pending_detection`] and `mark_dirty()` tails.

use op_editor_core::missing_fonts::detect_missing_fonts;
use op_editor_core::EditorState;

use crate::widgets::{MissingFontsHit, MissingFontsPanel};
use crate::{Point2D, Rect};

/// Wheel scroll routed to the open missing-fonts modal.
///
/// Returns `None` when the modal is closed (the host reports the wheel
/// as unconsumed), otherwise `Some(changed)` — `changed` is `true` when
/// a scroll offset actually moved, which is the host's `mark_dirty`
/// trigger.
pub fn scroll_picker(
    state: &mut EditorState,
    x: f32,
    y: f32,
    delta_y: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<bool> {
    if !state.editor_ui.missing_fonts_modal_open {
        return None;
    }
    let viewport = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(viewport_width, viewport_height),
    };
    let point = Point2D::new(x, y);
    let resolved = MissingFontsPanel::for_editor(state).map(|panel| {
        let panel_rect = panel.rect(viewport_width, viewport_height);
        let picker = panel.picker_layout(panel_rect, viewport);
        let picker_scroll = picker
            .as_ref()
            .filter(|layout| layout.popup.contains(point))
            .map(|layout| layout.max_scroll);
        let rows_scroll = if picker.is_none() && panel.rows_rect(panel_rect).contains(point) {
            Some(panel.max_rows_scroll(panel_rect))
        } else {
            None
        };
        (picker_scroll, rows_scroll)
    });
    let mut changed = false;
    if let Some((Some(max_scroll), _)) = resolved {
        let ui = &mut state.editor_ui;
        let next = (ui.font_picker.scroll.offset - delta_y).clamp(0.0, max_scroll);
        if next != ui.font_picker.scroll.offset {
            ui.font_picker.scroll.offset = next;
            ui.font_picker.hover = None;
            ui.font_picker_import_hover = false;
            changed = true;
        }
    } else if let Some((_, Some(max_scroll))) = resolved {
        let ui = &mut state.editor_ui;
        let next = (ui.missing_fonts_scroll.offset - delta_y).clamp(0.0, max_scroll);
        if next != ui.missing_fonts_scroll.offset {
            ui.missing_fonts_scroll.offset = next;
            ui.missing_fonts_hover = None;
            changed = true;
        }
    }
    Some(changed)
}

/// Route a press to the top-most missing-font modal. Returns `true` when
/// the press was consumed (the host then marks dirty).
pub fn press(
    state: &mut EditorState,
    panel_rect: Rect,
    viewport_rect: Rect,
    point: Point2D,
) -> bool {
    let Some(hit) = MissingFontsPanel::for_editor(state)
        .map(|panel| panel.hit_test(panel_rect, viewport_rect, point))
    else {
        return false;
    };
    match hit {
        MissingFontsHit::ChooseFont(row) => {
            state
                .editor_ui
                .open_missing_font_picker(row, op_editor_core::MissingFontSurface::Prompt);
            state.editor_ui.missing_fonts_hover = None;
        }
        MissingFontsHit::SelectFont(index) => {
            let replacement = MissingFontsPanel::for_editor(state)
                .and_then(|panel| panel.picker_entries().get(index).cloned())
                .map(|entry| entry.family);
            let expected = match state.editor_ui.font_picker_purpose {
                Some(op_editor_core::FontPickerPurpose::MissingFont { row, .. }) => state
                    .editor_ui
                    .missing_fonts_prompt
                    .as_ref()
                    .and_then(|prompt| prompt.entries.get(row))
                    .map(|entry| entry.family.clone()),
                _ => None,
            };
            if let (Some(from), Some(to)) = (expected, replacement) {
                let _ = state.apply(op_editor_core::EditorCommand::ReplaceFontFamily { from, to });
            }
            state.editor_ui.close_font_picker();
            replace_data(state, true);
        }
        MissingFontsHit::ImportFont(row) => {
            state.editor_ui.missing_fonts_import_row = Some(row);
        }
        MissingFontsHit::ClosePicker => {
            state.editor_ui.close_font_picker();
        }
        MissingFontsHit::Dismiss => {
            state.editor_ui.missing_fonts_modal_open = false;
            state.editor_ui.missing_fonts_hover = None;
            state.editor_ui.close_font_picker();
        }
        MissingFontsHit::PickerInside | MissingFontsHit::Inside | MissingFontsHit::Outside => {}
    }
    true
}

/// Record that a font scan is owed, along with whether its result may
/// open the one-shot modal. Hosts call this on the platform path that
/// cannot answer synchronously, then kick their own enumeration.
pub fn arm_pending_detection(state: &mut EditorState, open_modal: bool) {
    let ui = &mut state.editor_ui;
    ui.missing_fonts_pending_detect = true;
    ui.missing_fonts_pending_open_modal = open_modal;
}

/// Clear a pending scan request without running one — the caller already
/// has a fresh font snapshot and only wants existing rows reconciled.
pub fn clear_pending_detection(state: &mut EditorState) {
    let ui = &mut state.editor_ui;
    ui.missing_fonts_pending_detect = false;
    ui.missing_fonts_pending_open_modal = false;
}

/// Finish a deferred font scan using the open/silent intent captured
/// when it was scheduled. No-op (returns `false`, so the host skips its
/// repaint) until both the request and a font snapshot are in place.
pub fn complete_pending_detection(state: &mut EditorState) -> bool {
    if !state.editor_ui.missing_fonts_pending_detect || !state.editor_ui.system_fonts_loaded {
        return false;
    }
    let open_modal =
        state.editor_ui.missing_fonts_pending_open_modal && !settings_fonts_open(state);
    replace_data(state, open_modal);
    true
}

/// Whether the settings modal is parked on its Fonts tab — that surface
/// owns the rows itself, so detection must not raise the one-shot modal
/// on top of it.
pub fn settings_fonts_open(state: &EditorState) -> bool {
    state.editor_ui.agent_settings_open
        && matches!(
            state.editor_ui.agent_settings.tab,
            op_editor_core::AgentSettingsTab::Fonts
        )
}

/// Re-detect from scratch and publish the result, optionally raising the
/// one-shot modal (only when something is actually missing).
pub fn replace_data(state: &mut EditorState, open_modal: bool) {
    let prompt = detect_missing_fonts(state);
    let ui = &mut state.editor_ui;
    ui.missing_fonts_pending_detect = false;
    ui.missing_fonts_pending_open_modal = false;
    ui.missing_fonts_modal_open = open_modal && prompt.is_some();
    if open_modal {
        ui.missing_fonts_scroll.offset = 0.0;
    }
    ui.missing_fonts_prompt = prompt;
}

/// Reconcile existing rows against the latest system/imported snapshots,
/// preserving the per-row mismatch notes an import already produced.
pub fn refresh_prompt(state: &mut EditorState) {
    let previous = state.editor_ui.missing_fonts_prompt.take();
    let mut next = detect_missing_fonts(state);
    if let (Some(previous), Some(next)) = (previous.as_ref(), next.as_mut()) {
        for entry in &mut next.entries {
            if let Some(old) = previous
                .entries
                .iter()
                .find(|old| old.family.eq_ignore_ascii_case(&entry.family))
            {
                entry.mismatch_note = old.mismatch_note.clone();
            }
        }
    }
    state.editor_ui.missing_fonts_prompt = next;
    if state.editor_ui.missing_fonts_prompt.is_none() {
        let ui = &mut state.editor_ui;
        ui.missing_fonts_modal_open = false;
        ui.missing_fonts_hover = None;
        ui.close_font_picker();
    }
}

/// Record whether the supplied file declared the row's expected family,
/// then refresh resolution from the live imported-font snapshot.
pub fn note_font_supplied(state: &mut EditorState, row: usize, actual_family: Option<&str>) {
    let mismatch_template =
        crate::widgets::editor_state_ext::translate(&state.editor_ui, "missingFonts.mismatch");
    if let Some(entry) = state
        .editor_ui
        .missing_fonts_prompt
        .as_mut()
        .and_then(|prompt| prompt.entries.get_mut(row))
    {
        entry.mismatch_note = actual_family
            .filter(|actual| !actual.eq_ignore_ascii_case(&entry.family))
            .map(|actual| {
                mismatch_template
                    .replace("{actual}", actual)
                    .replace("{expected}", &entry.family)
            });
    }
    refresh_prompt(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_yahei_ui_import_stays_unresolved_and_reports_mismatch() {
        let doc = serde_json::from_value(serde_json::json!({
            "version": "1.0.0",
            "children": [{
                "type": "text",
                "id": "yahei",
                "content": "字",
                "fontFamily": "Microsoft YaHei"
            }]
        }))
        .expect("document");
        let mut state = EditorState::from_document(doc);
        state.editor_ui.system_fonts_loaded = true;
        state.editor_ui.imported_font_families =
            std::sync::Arc::new(vec!["Microsoft YaHei UI".into()]);
        replace_data(&mut state, true);

        note_font_supplied(&mut state, 0, Some("Microsoft YaHei UI"));

        let entry = &state
            .editor_ui
            .missing_fonts_prompt
            .as_ref()
            .expect("plain YaHei remains unresolved")
            .entries[0];
        let note = entry.mismatch_note.as_deref().expect("mismatch note");
        assert_eq!(entry.family, "Microsoft YaHei");
        assert!(!entry.resolved);
        assert!(note.contains("Microsoft YaHei UI"));
        assert!(note.contains("Microsoft YaHei"));
    }
}
