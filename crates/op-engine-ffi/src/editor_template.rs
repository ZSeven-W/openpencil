//! Native-editor drain for scene-template card selections.
//!
//! Template widgets only enqueue an id. This module resolves and preflights the
//! shipped document, then commits the prepared state at an ABI-safe boundary.

use crate::error::{FfiError, FfiResult};
use crate::lifecycle::Session;
use crate::OpStatus;
use op_editor_core::scene_template_append::template_boards;
use op_editor_core::scene_template_catalog::{scene_template_by_id, scene_template_document};

/// Consume one queued template selection and adopt it into the live document.
///
/// Every fallible content step runs on a clone. The host is touched only after
/// the resulting state has a renderable active page, so a bad id or corrupt
/// shipped asset cannot replace or partially edit the user's document.
pub(crate) fn drain_pending_scene_template(session: &mut Session) -> FfiResult<bool> {
    let template_id = {
        let Some(host) = session.editor.as_mut() else {
            return Ok(false);
        };
        host.editor_state_mut()
            .editor_ui
            .scene_template_center
            .take_pending_open()
    };
    let Some(template_id) = template_id else {
        return Ok(false);
    };

    let definition = scene_template_by_id(&template_id).ok_or_else(|| {
        FfiError::new(
            OpStatus::BadDocument,
            format!("unknown scene template `{template_id}`"),
        )
    })?;
    let source = scene_template_document(&template_id).ok_or_else(|| {
        FfiError::new(
            OpStatus::BadDocument,
            format!("scene template `{template_id}` has no shipped document"),
        )
    })?;
    let boards = template_boards(source, &template_id).ok_or_else(|| {
        FfiError::new(
            OpStatus::BadDocument,
            format!("scene template `{template_id}` document is invalid"),
        )
    })?;

    let (mut next_state, replaces_starter) = {
        let host = session.editor_mut()?;
        (
            host.editor_state().clone(),
            op_editor_core::blank_starter::active_page_is_blank_starter(host.editor_state()),
        )
    };
    if !next_state.adopt_template_boards(boards) {
        return Err(FfiError::new(
            OpStatus::BadDocument,
            format!("scene template `{template_id}` contains no adoptable boards"),
        ));
    }
    if next_state.editor_ui.scenario.is_none() {
        next_state.editor_ui.scenario = Some(definition.scene);
    }
    let next_scene = op_pen_loader::editor_state_to_active_page_layout_scene(&next_state);
    if next_scene.active_page().is_none() {
        return Err(FfiError::new(
            OpStatus::LayoutError,
            format!("scene template `{template_id}` has no renderable page"),
        ));
    }

    if session
        .editor_mut()?
        .install_scene_template_state(next_state, replaces_starter)
        .is_err()
    {
        return Err(FfiError::new(
            OpStatus::Busy,
            "scene template adoption is blocked by the collaboration session",
        ));
    }

    session.scene = next_scene;
    session.selected = None;
    session.gesture.reset();
    session.user_interacted = false;
    session.fit_content_to_viewports();
    // Fitting mutates the host-owned viewport, so clone only afterwards. Page
    // APIs and any viewer-side state reader now observe the exact live state.
    session.state = session
        .editor()
        .ok_or_else(|| FfiError::new(OpStatus::NotReady, "engine is not in editor mode"))?
        .editor_state()
        .clone();
    session.request_redraw();
    Ok(true)
}

#[cfg(test)]
mod tests;
