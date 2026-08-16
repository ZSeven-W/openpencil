//! The File-menu "Save As Template" flow: turn the open document into a
//! `user:<slug>` scene template.
//!
//! Memory first, then disk. The registry entry makes the card appear in the
//! Asset Center's "My templates" section the instant the menu item is
//! pressed; [`crate::user_template_store::persist_user_scene_template`] is
//! what makes it survive a restart. A disk failure rolls the registry entry
//! back — a template that is not on disk must not masquerade as one.
//!
//! The document is serialized through the same document-only canonical
//! writer `op_host_services::doc_io::save_document_to_path` wraps, so the
//! registry string and the on-disk `document.op` are the same bytes by
//! construction. The preview is a single JPEG of the active page's first
//! top-level node — deliberately not the shipped templates' collage
//! contract, which is the factory bake pipeline's own.

use op_editor_core::editor_toast::EditorToastLevel;
use op_host_native::widget_host::WidgetHostNative;

use crate::DesktopApp;

/// Why a template save failed. Private to this flow: the failure surfaces as
/// a toast key plus a log line, so the type exists to keep the arms typed
/// rather than to carry user-facing wording.
#[derive(Debug)]
enum SaveTemplateError {
    /// The canonical document could not be serialized.
    Document(op_host_services::doc_io::DocIoError),
    /// The canonical serializer produced non-UTF-8 output.
    Utf8(std::string::FromUtf8Error),
    /// The saved-template quota is full and the id is new.
    Quota,
    /// The template is in memory but could not be written down.
    Persist(std::io::Error),
}

impl std::fmt::Display for SaveTemplateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveTemplateError::Document(error) => write!(formatter, "{error}"),
            SaveTemplateError::Utf8(error) => write!(formatter, "{error}"),
            SaveTemplateError::Quota => write!(
                formatter,
                "template quota ({}) reached",
                op_editor_core::user_scene_templates::USER_TEMPLATE_QUOTA
            ),
            SaveTemplateError::Persist(error) => write!(formatter, "{error}"),
        }
    }
}

impl DesktopApp {
    /// Drain the in-canvas File menu's host request. The native app menu calls
    /// `save_current_as_template` directly; the widget menu cannot own disk IO,
    /// so it raises this one-shot request instead.
    pub(crate) fn drain_save_current_template_request(&mut self) -> bool {
        if !self
            .host
            .editor_state_mut()
            .editor_ui
            .scene_template_center
            .take_pending_save_current()
        {
            return false;
        }
        self.save_current_as_template()
    }

    /// Save the open document as a `user:` scene template and report the
    /// outcome with a toast — the menu action's whole body.
    pub(crate) fn save_current_as_template(&mut self) -> bool {
        // Snapshot only after every focused draft has reached the document.
        // This shared path covers both property fields and variable rows.
        self.host.commit_pending_input_pub();
        let now_ms = self.clock_start.elapsed().as_millis() as u64;
        // The document's file stem is the template's name; an unsaved
        // document still names its template, just generically.
        let name = self
            .current_path
            .as_ref()
            .and_then(|path| path.file_stem())
            .and_then(|stem| stem.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| "untitled-template".to_string());
        match save_template_from_state(&mut self.host, &name) {
            Ok(_) => self.host.editor_state_mut().editor_ui.show_toast(
                "menu.saveAsTemplate.saved",
                Vec::new(),
                EditorToastLevel::Info,
                now_ms,
            ),
            Err(error) => {
                eprintln!("[templates] could not save {name:?} as a template: {error}");
                self.host.editor_state_mut().editor_ui.show_toast(
                    "menu.saveAsTemplate.failed",
                    Vec::new(),
                    EditorToastLevel::Warn,
                    now_ms,
                );
            }
        }
        self.host.mark_editor_state_dirty();
        true
    }
}

/// Serialize, render, register, and persist the current document as a
/// template. Returns the allocated id on success.
fn save_template_from_state(
    host: &mut WidgetHostNative,
    name: &str,
) -> Result<String, SaveTemplateError> {
    let state = host.editor_state();
    let active_page_index = state.ui.active_page_index;

    // Canonical `.op` JSON once, through the same document-only serializer
    // `save_document_to_path` wraps — the registry entry and the file the
    // store writes are the same bytes by construction.
    let mut buffer: Vec<u8> = Vec::new();
    op_host_services::doc_io::write_canonical_document(&mut buffer, &state.doc, active_page_index)
        .map_err(SaveTemplateError::Document)?;
    let document = String::from_utf8(buffer).map_err(SaveTemplateError::Utf8)?;

    // The preview and the frame size both come from the same resolved scene,
    // so the card's numbers and its picture can never disagree.
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let (frame_width, frame_height, preview_jpeg) = first_node_preview(&scene);

    let id = op_editor_core::user_scene_templates::allocate_template_id(
        &op_ai_skills::style_guide::slugify(name),
    );
    let frames = state.active_children().len().min(u16::MAX as usize) as u16;
    let entry = op_editor_core::user_scene_templates::UserSceneTemplate {
        id: id.clone(),
        name: name.to_string(),
        frames,
        frame_width,
        frame_height,
        document,
        preview_jpeg,
    };
    // The quota gate sits at registration: a refused save must not leave a
    // stray directory behind that the next boot scan would resurrect.
    op_editor_core::user_scene_templates::load_user_scene_template(entry)
        .map_err(|_| SaveTemplateError::Quota)?;
    if let Err(error) = crate::user_template_store::persist_user_scene_template(&id) {
        op_editor_core::user_scene_templates::remove_user_scene_template(&id);
        return Err(SaveTemplateError::Persist(error));
    }
    Ok(id)
}

/// The first top-level node of the active page, rendered as a JPEG preview
/// at `1024 / width` (capped at 1.0).
///
/// A document with no top-level node — or a first node that cannot be
/// rendered (hidden, paints nothing) — yields an empty preview rather than
/// failing the save; the card then paints a plain block, exactly like a
/// template that was saved without one.
fn first_node_preview(scene: &op_editor_ui::layout_scene::LayoutScene) -> (u32, u32, Vec<u8>) {
    let Some(first) = scene.active_page().and_then(|page| page.children.first()) else {
        return (0, 0, Vec::new());
    };
    let width = first.bounds.size.x.max(0.0).round() as u32;
    let height = first.bounds.size.y.max(0.0).round() as u32;
    let scale = if width > 0 {
        (1024.0 / width as f32).min(1.0)
    } else {
        1.0
    };
    let bytes = op_host_services::export::render_node_raster_bytes(
        scene,
        &first.id,
        op_host_services::export::RasterFormat::Jpeg,
        scale,
    )
    .unwrap_or_default();
    (width, height, bytes)
}
