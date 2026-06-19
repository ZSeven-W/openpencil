//! Theme-preset `.optheme` import/export drain on the web host (#20) — the
//! browser-side counterpart to op-host-desktop `theme_preset_host.rs`.
//!
//! `dispatch_variables_preset_press` raises `pending_theme_preset_io` when the
//! preset dropdown's Import / Export row is clicked. The RAF pump drains it
//! once per frame here: Export serializes the preset payload and hands it to
//! the browser as a download; Import pops a hidden `.optheme` file input,
//! parses it, and merges the themes + variables into the document under one
//! undo step (TS `theme-preset-io.ts` + `variable-theme-manager.tsx:153-158`).

use std::cell::RefCell;
use std::rc::Rc;

use op_editor_core::editor_ui_state::ThemePresetIo;
use op_editor_core::theme_presets::{
    parse_preset_file, preset_file_to_json, sanitize_preset_file_stem, THEME_PRESET_EXPORT_NAME,
};

use crate::dom_io::{open_file_picker, read_file, ReadMode};
use crate::repaint_ctx::RepaintContext;

type InnerRc<C> = Rc<RefCell<C>>;

/// Drain a pending `.optheme` import/export raised by the preset dropdown.
/// Called once per frame from the canvaskit RAF pump alongside the other
/// `drain_pending_*` IO hooks. Without this drain the Import/Export rows would
/// set a flag nothing consumes — the action would silently never run.
pub(crate) fn drain_pending_theme_preset_io<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let action = inner
        .borrow_mut()
        .host_mut()
        .editor_state_mut()
        .editor_ui
        .pending_theme_preset_io
        .take();
    match action {
        Some(ThemePresetIo::Export) => export_theme_preset_file(inner),
        Some(ThemePresetIo::Import) => import_theme_preset_file(inner),
        None => {}
    }
}

/// TS `exportThemePreset` — fixed `theme-preset` name, sanitized `.optheme`
/// file name, pretty JSON, handed to the browser as a download.
fn export_theme_preset_file<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let b = inner.borrow();
    let state = b.host().editor_state();
    let json = preset_file_to_json(
        THEME_PRESET_EXPORT_NAME,
        &state.doc.themes.clone().unwrap_or_default(),
        &state.doc.variables.clone().unwrap_or_default(),
    );
    let file_name = format!(
        "{}.optheme",
        sanitize_preset_file_stem(THEME_PRESET_EXPORT_NAME)
    );
    if let Err(e) =
        crate::web_clipboard::download_bytes(&file_name, "application/json", json.as_bytes())
    {
        web_sys::console::error_1(&e);
    }
}

/// TS `importThemePreset` + `handleLoadPreset` — pick a `.optheme`, validate,
/// merge themes + variables into the document. An invalid file is logged and
/// skipped (TS resolves `null`); one undo step per import.
fn import_theme_preset_file<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let inner = inner.clone();
    open_file_picker(
        ".optheme,.json",
        Box::new(move |file| {
            let inner2 = inner.clone();
            read_file(
                file,
                ReadMode::Text,
                Box::new(move |value| match value.as_string() {
                    Some(raw) => apply_imported_theme_preset(&inner2, &raw),
                    None => web_sys::console::error_1(
                        &"[theme-preset-import] file read produced no text".into(),
                    ),
                }),
            );
        }),
    );
}

fn apply_imported_theme_preset<C: RepaintContext + 'static>(inner: &InnerRc<C>, raw: &str) {
    let Some((_name, themes, variables)) = parse_preset_file(raw) else {
        web_sys::console::error_1(&"[theme-preset-import] invalid .optheme file".into());
        return;
    };
    let mut b = inner.borrow_mut();
    {
        let state = b.host_mut().editor_state_mut();
        let snap = state.snapshot_for_history();
        state.merge_theme_preset_payload(themes, variables);
        state.history_push_past(snap);
    }
    b.host_mut().mark_editor_state_dirty();
    let _ = b.repaint();
}
