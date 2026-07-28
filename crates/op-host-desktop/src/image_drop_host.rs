//! Dragging an image FILE onto the canvas.
//!
//! Two outcomes from one gesture, decided by what is under the pointer when
//! the file is released:
//!
//! - over a frame / rectangle / … → the image becomes that node's fill
//!   (cover), so a template's placeholder box is filled in one motion;
//! - over bare canvas → the image is inserted as a node at the drop point.
//!
//! The decision is structural (`op_editor_core::image_drop`), never based on
//! node names. Reading + embedding reuses the shared import path in
//! [`crate::persistence_image`], so a drop and a File ▸ Import produce
//! byte-identical `data:` URLs (downscale included).
//!
//! Position comes from [`crate::drag_cursor`], which is macOS-only today; on
//! other platforms every drop degrades to the viewport-centre insert.

use crate::persistence_image::read_as_data_url;
use crate::DesktopApp;
use op_editor_core::NodeId;
use op_host_native::widget_host::WidgetHostNative;
use std::path::Path;

/// Raster formats a canvas drop accepts.
///
/// SVG is deliberately absent: importing one produces editable vector nodes,
/// which is a different gesture from "become this box's picture" — a dropped
/// `.svg` keeps falling through to the existing routing.
const DROP_IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

/// Whether a dropped path is a raster image this module handles.
pub fn is_supported_image_drop(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|ext| DROP_IMAGE_EXTENSIONS.contains(&ext.as_str()))
}

/// What a drop did, for logging and redraw decisions.
#[derive(Debug, PartialEq, Eq)]
pub enum ImageDropOutcome {
    /// The image became `NodeId`'s fill.
    Filled(NodeId),
    /// The image was inserted as a new node.
    Inserted(NodeId),
    /// Nothing happened (unreadable file, or the document refused it).
    Ignored,
}

/// Apply a dropped image file.
///
/// `point` is the LOGICAL window position of the drop, when the platform can
/// report one. `None` — or a point outside the canvas — inserts at the
/// viewport centre, which is where the toolbar import already puts it.
pub fn apply_image_drop(
    host: &mut WidgetHostNative,
    path: &Path,
    point: Option<(f32, f32)>,
    viewport_w: f32,
    viewport_h: f32,
) -> ImageDropOutcome {
    if !host.gate_collaboration_action(
        op_editor_core::CollabGateAction::Document(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
        ),
        op_editor_core::CollabEditSource::Import,
    ) {
        return ImageDropOutcome::Ignored;
    }
    let embedded = match read_as_data_url(path) {
        Ok(embedded) => embedded,
        Err(e) => {
            eprintln!("[image-drop] {}: {e}", path.display());
            return ImageDropOutcome::Ignored;
        }
    };
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Image")
        .to_string();

    if let Some((x, y)) = point {
        if let Some(target) = host.image_drop_target_at(x, y, viewport_w, viewport_h) {
            if host.apply_image_drop(&target, &embedded.url, embedded.original_size) {
                return ImageDropOutcome::Filled(target);
            }
            return ImageDropOutcome::Ignored;
        }
        if let Some(centre) = host.canvas_doc_point(x, y, viewport_w, viewport_h) {
            return match host.insert_dropped_image(
                &name,
                &embedded.url,
                pixel_size(embedded.original_size),
                centre,
            ) {
                Some(id) => ImageDropOutcome::Inserted(id),
                None => ImageDropOutcome::Ignored,
            };
        }
    }

    let inserted = match pixel_size(embedded.original_size) {
        Some((width, height)) => host.editor_state_mut().insert_image_node_at_viewport_sized(
            &name,
            &embedded.url,
            width,
            height,
        ),
        None => host
            .editor_state_mut()
            .insert_image_node_at_viewport(&name, &embedded.url),
    };
    host.mark_editor_state_dirty();
    match inserted {
        Some(id) => ImageDropOutcome::Inserted(id),
        None => ImageDropOutcome::Ignored,
    }
}

/// Encoded bitmap dimensions as whole pixels, when they were decodable.
fn pixel_size(original_size: Option<[f32; 2]>) -> Option<(u32, u32)> {
    let [width, height] = original_size?;
    (width >= 1.0 && height >= 1.0).then_some((width as u32, height as u32))
}

impl DesktopApp {
    /// Track the pointer while an image file hovers the window and highlight
    /// the node it would fill.
    ///
    /// winit reports `HoveredFile` once, without a position, and the platform
    /// swallows cursor moves for the duration of the drag — so the position is
    /// polled from the window system instead (see [`crate::drag_cursor`]).
    /// Returns `true` when the highlight changed.
    pub(crate) fn refresh_image_drop_hover(&mut self) -> bool {
        if !self.hovered_image_drop {
            return false;
        }
        let Some(window) = self.window.as_ref() else {
            return false;
        };
        let point = crate::drag_cursor::window_cursor_position(window);
        self.drop_cursor = point;
        let target = point.and_then(|(x, y)| {
            self.host
                .image_drop_target_at(x, y, self.viewport_width, self.viewport_height)
        });
        self.host.set_file_drop_target(target)
    }

    /// Clear every drag-hover trace. Called on drop and on drag-cancel so a
    /// stale ring can never outlive the gesture.
    pub(crate) fn clear_image_drop_hover(&mut self) {
        self.hovered_image_drop = false;
        self.drop_cursor = None;
        self.host.set_file_drop_target(None);
    }
}

#[cfg(test)]
#[path = "image_drop_host_tests.rs"]
mod tests;
