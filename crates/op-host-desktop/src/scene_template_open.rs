//! Desktop arm for opening a scene template as a new document.
//!
//! The panel raises `scene_template_center.pending_open` rather than loading
//! anything itself (see `scene_template_press_flow`); this drains it.
//!
//! A template becomes an UNSAVED document with no bound path, like File > New
//! rather than File > Open: the shipped `.op` is read-only content, and
//! binding it as the save target would let the first Cmd+S overwrite the
//! template for every future use.

use op_editor_core::scene_template_catalog::scene_template_document;
use op_host_native::widget_host::WidgetHostNative;
use std::path::PathBuf;

use op_host_services::doc_io::preserve_app_preferences;

use crate::persistence::{fit_loaded_document, refresh_title};

/// Drain a pending template-open request. Returns true when the document was
/// replaced.
pub(crate) fn drain_pending_scene_template(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    let Some(template_id) = host
        .editor_state_mut()
        .editor_ui
        .scene_template_center
        .take_pending_open()
    else {
        return false;
    };
    // Replacing the document runs the same collaboration gate as File > New;
    // the panel's own press only opened a panel, which needed no gate.
    if !host.gate_collaboration_action(
        op_editor_core::CollabGateAction::ReplaceDocument,
        op_editor_core::CollabEditSource::User,
    ) {
        return false;
    }
    let Some(source) = scene_template_document(&template_id) else {
        // The catalogue verifies this at load, so reaching here means the
        // embedded set and the catalogue disagree — report it rather than
        // silently doing nothing under the user's click.
        eprintln!("[template] no embedded document for {template_id}");
        return false;
    };
    let locale = host.editor_state().editor_ui.locale;
    let mut state = match op_host_services::doc_io::load_editor_state_from_source(source, locale) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("[template] {template_id}: {error}");
            return false;
        }
    };
    preserve_app_preferences(host.editor_state(), &mut state);
    state.clear_selection();
    if !host.replace_editor_state(state) {
        return false;
    }
    fit_loaded_document(host, window);
    // No bound path: this is a new unsaved document, so Save must prompt for
    // a destination instead of writing back over the shipped template.
    *current_path = None;
    refresh_title(current_path, window);
    host.force_rotate_layer_panel_owner();
    host.mark_editor_state_dirty();
    true
}

/// Whether any template document fails to load, used by the smoke test below
/// and worth keeping cheap: it parses every shipped template.
#[cfg(test)]
fn all_templates_parse(locale: op_editor_core::Locale) -> Result<(), String> {
    use op_editor_core::EditorState;
    for template in op_editor_core::scene_template_catalog::scene_template_catalogue() {
        let source = scene_template_document(&template.id)
            .ok_or_else(|| format!("{} has no document", template.id))?;
        let state: EditorState =
            op_host_services::doc_io::load_editor_state_from_source(source, locale)
                .map_err(|error| format!("{}: {error}", template.id))?;
        if state.active_children().is_empty() {
            return Err(format!("{} loaded with no top-level nodes", template.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shipped template must survive the real loader.
    ///
    /// The catalogue test only checks the JSON shape; this one runs the same
    /// path a click takes, so a template that parses as JSON but fails schema
    /// conversion cannot reach a user as a click that does nothing.
    #[test]
    fn every_shipped_template_loads_through_the_real_loader() {
        all_templates_parse(op_editor_core::Locale::ZhCn).expect("templates load");
        all_templates_parse(op_editor_core::Locale::EnUs).expect("templates load");
    }
}
