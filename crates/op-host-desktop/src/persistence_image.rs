//! Image / SVG import handlers for [`super::persistence::run_action`].
//!
//! Split out of `persistence.rs` so that file stays under the 800-line
//! cap. Three entry points, all embed-not-link (every one ends in a
//! `data:` URL — see the shared-.op portability audit, 2026-07-18):
//!
//! - [`handle_import_image_or_svg`] — toolbar shape-picker action:
//!   pops `rfd::FileDialog`, decodes the file, and inserts a new
//!   Image node centred on the viewport. SVG files land as a raster
//!   placeholder for now (a proper SVG-to-path parser is a follow-up).
//! - [`handle_pick_fill_image`] — Fill section "图片" body row:
//!   pops the same dialog and writes the chosen image into the
//!   selected node's primary fill as
//!   `PenFill::Image { url: <data-url> }`.
//! - [`handle_relink_image`] — image-section warning row's Relink
//!   button: pops the same dialog and rewrites the selected
//!   `ImageNode.src` with the picked file's `data:` URL.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use op_host_native::widget_host::WidgetHostNative;
use std::path::Path;

/// File extensions the import dialog accepts.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg"];

pub(crate) struct EmbeddedImage {
    pub(crate) url: String,
    pub(crate) original_size: Option<[f32; 2]>,
}

/// Pop a file dialog scoped to image / SVG extensions, returning the
/// chosen path. `None` when the user cancelled.
fn pick_image_path(host: &WidgetHostNative) -> Option<std::path::PathBuf> {
    let title = op_i18n::translate(
        host.editor_state().editor_ui.locale,
        "dialog.pickerOpenTitle",
    );
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("Images / SVG", IMAGE_EXTENSIONS)
        .pick_file()
}

/// Read `path` and encode it as a `data:` URL so the canvas painter +
/// renderer can resolve the image without re-reading from disk. SVG
/// files get the `image/svg+xml` MIME, everything else picks from a
/// small extension table — falling back to `application/octet-stream`
/// so an unknown extension still round-trips.
pub(crate) fn read_as_data_url(path: &Path) -> std::io::Result<EmbeddedImage> {
    let bytes = std::fs::read(path)?;
    // Shrink an oversized raster source before it lands in the document
    // (a multi-MB `src` lags every later scene rebuild + canvas decode).
    // SVG / undecodable / already-small sources fall through unchanged.
    let (mime, payload) = match crate::image_downscale::maybe_downscale(&bytes) {
        Some((mime, scaled)) => (mime, scaled),
        None => (mime_for(path), bytes),
    };
    let original_size = op_editor_ui::image_runtime::encoded_image_dimensions(&payload)
        .map(|(width, height)| [width as f32, height as f32]);
    Ok(EmbeddedImage {
        url: format!("data:{};base64,{}", mime, B64.encode(&payload)),
        original_size,
    })
}

fn mime_for(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// Handle the toolbar shape-picker's `Import image or SVG…` action.
///
/// SVG files are parsed into editable path / shape nodes via
/// `EditorState::import_svg` (TS parity with `parseSvgToNodes`);
/// raster formats land as a single `ImageNode` carrying the file as
/// a `data:` URL. On cancel: silent no-op. On read error: log.
pub fn handle_import_image_or_svg(host: &mut WidgetHostNative) {
    if !allow_external_asset_import(host) {
        return;
    }
    let Some(path) = pick_image_path(host) else {
        return;
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if ext.as_deref() == Some("svg") {
        let svg = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[import-svg] {}: {e}", path.display());
                return;
            }
        };
        // Centre roughly at viewport — the SVG's authored coords
        // shift by the offset.
        let pan_x = host.editor_state().viewport.pan_x as f64;
        let pan_y = host.editor_state().viewport.pan_y as f64;
        let zoom = (host.editor_state().viewport.zoom as f64).max(0.001);
        let centre_x = -pan_x / zoom;
        let centre_y = -pan_y / zoom;
        let mut next_id = 0u64;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string);
        let count = host.editor_state_mut().import_svg_named(
            &mut next_id,
            &svg,
            (centre_x - 200.0, centre_y - 150.0),
            stem.as_deref(),
        );
        if count == 0 {
            eprintln!("[import-svg] {} yielded no nodes", path.display());
        }
        host.mark_editor_state_dirty();
        return;
    }
    let embedded = match read_as_data_url(&path) {
        Ok(embedded) => embedded,
        Err(e) => {
            eprintln!("[import-image] {}: {e}", path.display());
            return;
        }
    };
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Image")
        .to_string();
    let _ = host
        .editor_state_mut()
        .insert_image_node_at_viewport(&name, &embedded.url);
    host.mark_editor_state_dirty();
}

/// Handle the Fill section's `图片` body click. Writes the picked
/// image into the selected node's first `PenFill` as `Image { url }`.
pub fn handle_pick_fill_image(host: &mut WidgetHostNative) {
    if !allow_external_asset_import(host) {
        return;
    }
    let Some(path) = pick_image_path(host) else {
        return;
    };
    apply_pick_fill_image(host, &path);
}

fn apply_pick_fill_image(host: &mut WidgetHostNative, path: &Path) {
    if !allow_external_asset_import(host) {
        return;
    }
    let embedded = match read_as_data_url(path) {
        Ok(embedded) => embedded,
        Err(e) => {
            eprintln!("[fill-image] {}: {e}", path.display());
            return;
        }
    };
    if host
        .editor_state_mut()
        .set_selected_fill_image_url_with_original_size(&embedded.url, embedded.original_size)
    {
        // `set_selected_fill_image_url` writes fill content without touching
        // the command/history path, so bump the revision (layer-panel cache +
        // save-dirty tracking key on `document_revision()`). The sibling
        // relink handler above bumps via `commit_history()`.
        host.editor_state_mut().mark_document_changed();
    }
    host.mark_editor_state_dirty();
}

/// Handle the image-section warning row's Relink button: pop the
/// image file dialog, read the picked file, and rewrite the selected
/// `ImageNode.src` as a `data:` URL — the SAME embed-not-link contract
/// every other image-entry path already uses
/// (`handle_import_image_or_svg`, `handle_pick_fill_image`).
///
/// This used to write a document-relative or absolute filesystem PATH
/// (TS `onUpdate({ src: toStoredAssetPath(result.filePath, documentPath) })`).
/// That "fixed" the warning locally — the relinked file exists on the
/// relinking machine — but produced a document that is STILL not
/// portable: share it, and the recipient hits the exact same missing-
/// image report the Relink button was supposed to resolve, because
/// their machine doesn't have that path either. Embedding closes that
/// gap (shared-.op portability audit, 2026-07-18): the fixed reference
/// is real content, not another local pointer.
pub fn handle_relink_image(host: &mut WidgetHostNative) {
    if !allow_external_asset_import(host) {
        return;
    }
    let Some(path) = pick_image_path(host) else {
        return;
    };
    apply_relink(host, &path);
}

/// Core of [`handle_relink_image`], factored out so the embed-then-write
/// behavior is testable without a real file-picker dialog.
fn apply_relink(host: &mut WidgetHostNative, path: &Path) {
    if !allow_external_asset_import(host) {
        return;
    }
    let embedded = match read_as_data_url(path) {
        Ok(embedded) => embedded,
        Err(e) => {
            eprintln!("[relink-image] {}: {e}", path.display());
            return;
        }
    };
    let state = host.editor_state_mut();
    let id = state.selection.anchor.clone();
    if !id.is_real() {
        return;
    }
    state.commit_history();
    if let Some(jian_ops_schema::node::PenNode::Image(image)) =
        op_editor_core::walkers::find_node_mut(state.active_children_mut(), &id)
    {
        image.src = embedded.url.into();
    }
    // Drop the stale asset check so the warning row clears on the
    // next pump (it re-probes the new src).
    state.editor_ui.image_panel.asset_check = None;
    host.mark_editor_state_dirty();
}

fn allow_external_asset_import(host: &mut WidgetHostNative) -> bool {
    host.gate_collaboration_action(
        op_editor_core::CollabGateAction::Document(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
        ),
        op_editor_core::CollabEditSource::Import,
    )
}

#[cfg(test)]
mod relink_tests {
    use super::{apply_pick_fill_image, apply_relink};
    use op_editor_core::{EditorState, ImageFillMode};
    use op_host_native::widget_host::WidgetHostNative;

    /// The Relink button must embed the picked file, not point at its
    /// path — a path is only ever valid on the machine that picked it,
    /// so a "fixed" reference that's still a path reproduces the exact
    /// missing-image report for anyone the document is shared with.
    #[test]
    fn relink_embeds_the_picked_file_as_a_data_url_instead_of_a_path() {
        let mut host = WidgetHostNative::new();
        let id = host
            .editor_state_mut()
            .insert_image_node_at_viewport("Photo", "assets/broken.png")
            .expect("image node inserted and selected");

        let tmp_path =
            std::env::temp_dir().join(format!("op-relink-test-{}.png", std::process::id()));
        std::fs::write(&tmp_path, b"not a real png, just bytes to embed").expect("write temp file");
        apply_relink(&mut host, &tmp_path);
        let _ = std::fs::remove_file(&tmp_path);

        let node = op_editor_core::walkers::find_node(host.editor_state().active_children(), &id)
            .expect("image node still present");
        let jian_ops_schema::node::PenNode::Image(image) = node else {
            panic!("expected an image node, got {node:?}");
        };
        assert!(
            image.src.starts_with("data:image/png;base64,"),
            "relink must embed as a data: URL, not store the picked path: {}",
            image.src
        );
    }

    /// A read failure (file vanished between pick and read, unreadable
    /// permissions, …) must leave the node's `src` untouched — no
    /// fallback to writing the unreadable path either.
    #[test]
    fn relink_leaves_src_untouched_when_the_file_cannot_be_read() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut()
            .insert_image_node_at_viewport("Photo", "assets/broken.png")
            .expect("image node inserted and selected");

        let missing_path = std::env::temp_dir().join("op-relink-test-does-not-exist.png");
        apply_relink(&mut host, &missing_path);

        let node = host.editor_state().selected_node().expect("still selected");
        let jian_ops_schema::node::PenNode::Image(image) = node else {
            panic!("expected an image node, got {node:?}");
        };
        assert_eq!(
            image.src.as_str(),
            "assets/broken.png",
            "a read failure must not overwrite the existing src"
        );
    }

    #[test]
    fn picked_fill_persists_final_bitmap_dimensions_and_exits_crop_edit() {
        let mut surface = skia_safe::surfaces::raster_n32_premul((7, 5)).expect("surface");
        surface.canvas().clear(skia_safe::Color::BLUE);
        let png = surface
            .image_snapshot()
            .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
            .expect("encode png");
        let path =
            std::env::temp_dir().join(format!("op-fill-upload-test-{}.png", std::process::id()));
        std::fs::write(&path, png.as_bytes()).expect("write png");

        let mut host = WidgetHostNative::new();
        *host.editor_state_mut() = EditorState::sample();
        let selected = host.editor_state().selection.anchor.clone();
        assert!(host
            .editor_state_mut()
            .set_selected_fill_image_url_with_original_size(
                "data:image/png;base64,old",
                Some([100.0, 100.0]),
            ));
        assert!(host
            .editor_state_mut()
            .set_selected_image_fill_mode(ImageFillMode::Crop));
        host.editor_state_mut().editor_ui.image_crop_editing = Some(selected.clone());

        apply_pick_fill_image(&mut host, &path);
        let _ = std::fs::remove_file(&path);

        let summary = op_editor_core::fills::first_image_fill_summary(
            host.editor_state().selected_node().expect("selected node"),
        )
        .expect("image fill");
        assert_eq!(summary.original_size, Some([7.0, 5.0]));
        assert_eq!(summary.mode, ImageFillMode::Fill);
        assert_eq!(host.editor_state().editor_ui.image_crop_editing, None);
        assert_eq!(host.editor_state().selection.anchor, selected);
    }
}
