//! Image / SVG import handlers for [`super::persistence::run_action`].
//!
//! Split out of `persistence.rs` so that file stays under the 800-line
//! cap. Two entry points:
//!
//! - [`handle_import_image_or_svg`] — toolbar shape-picker action:
//!   pops `rfd::FileDialog`, decodes the file, and inserts a new
//!   Image node centred on the viewport. SVG files land as a raster
//!   placeholder for now (a proper SVG-to-path parser is a follow-up).
//! - [`handle_pick_fill_image`] — Fill section "图片" body row:
//!   pops the same dialog and writes the chosen image into the
//!   selected node's primary fill as
//!   `PenFill::Image { url: <data-url> }`.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use op_host_native::widget_host::WidgetHostNative;
use std::path::Path;

/// File extensions the import dialog accepts.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg"];

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
fn read_as_data_url(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    // Shrink an oversized raster source before it lands in the document
    // (a multi-MB `src` lags every later scene rebuild + canvas decode).
    // SVG / undecodable / already-small sources fall through unchanged.
    if let Some((mime, scaled)) = crate::image_downscale::maybe_downscale(&bytes) {
        return Ok(format!("data:{};base64,{}", mime, B64.encode(&scaled)));
    }
    let mime = mime_for(path);
    let encoded = B64.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
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
    let url = match read_as_data_url(&path) {
        Ok(u) => u,
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
        .insert_image_node_at_viewport(&name, &url);
    host.mark_editor_state_dirty();
}

/// Handle the Fill section's `图片` body click. Writes the picked
/// image into the selected node's first `PenFill` as `Image { url }`.
pub fn handle_pick_fill_image(host: &mut WidgetHostNative) {
    let Some(path) = pick_image_path(host) else {
        return;
    };
    let url = match read_as_data_url(&path) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("[fill-image] {}: {e}", path.display());
            return;
        }
    };
    if host.editor_state_mut().set_selected_fill_image_url(&url) {
        // `set_selected_fill_image_url` writes fill content without touching
        // the command/history path, so bump the revision (layer-panel cache +
        // save-dirty tracking key on `document_revision()`). The sibling
        // relink handler above bumps via `commit_history()`.
        host.editor_state_mut().mark_document_changed();
    }
    host.mark_editor_state_dirty();
}

/// Handle the image-section warning row's Relink button: pop the
/// image file dialog and rewrite the selected `ImageNode.src` —
/// stored relative to the document when both share a root (TS
/// `onUpdate({ src: toStoredAssetPath(result.filePath, documentPath) })`).
pub fn handle_relink_image(host: &mut WidgetHostNative, current_path: Option<&Path>) {
    let Some(path) = pick_image_path(host) else {
        return;
    };
    let stored = to_stored_asset_path(&path, current_path);
    let state = host.editor_state_mut();
    let id = state.selection.anchor.clone();
    if !id.is_real() {
        return;
    }
    state.commit_history();
    if let Some(jian_ops_schema::node::PenNode::Image(image)) =
        op_editor_core::walkers::find_node_mut(state.active_children_mut(), &id)
    {
        image.src = stored.into();
    }
    // Drop the stale asset check so the warning row clears on the
    // next pump (it re-probes the new src).
    state.editor_ui.image_panel.asset_check = None;
    host.mark_editor_state_dirty();
}

/// Port of TS `toStoredAssetPath` (document-assets.ts:87-104): an
/// absolute picked path becomes document-relative when the document
/// path shares its prefix (drive / root), else stays absolute.
/// Separators normalize to `/` either way.
fn to_stored_asset_path(asset: &Path, document: Option<&Path>) -> String {
    let normalized = asset.display().to_string().replace('\\', "/");
    let Some(doc_dir) = document.and_then(Path::parent) else {
        return normalized;
    };
    let base = doc_dir.display().to_string().replace('\\', "/");
    let base_parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    let target_parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    // Windows drive prefixes must match case-insensitively (TS
    // extractPathPrefix comparison); on POSIX both prefixes are `/`.
    let drive = |parts: &[&str]| -> String {
        parts
            .first()
            .filter(|p| p.len() == 2 && p.as_bytes()[1] == b':')
            .map(|p| p.to_lowercase())
            .unwrap_or_default()
    };
    if drive(&base_parts) != drive(&target_parts) {
        return normalized;
    }
    let mut shared = 0;
    while shared < base_parts.len()
        && shared < target_parts.len()
        && base_parts[shared].eq_ignore_ascii_case(target_parts[shared])
    {
        shared += 1;
    }
    let ups = base_parts.len() - shared;
    let mut segments: Vec<String> = std::iter::repeat_n("..".to_string(), ups).collect();
    segments.extend(target_parts[shared..].iter().map(|s| s.to_string()));
    if segments.is_empty() {
        ".".to_string()
    } else {
        segments.join("/")
    }
}

#[cfg(test)]
mod relink_tests {
    use super::to_stored_asset_path;
    use std::path::Path;

    #[test]
    fn stored_path_relativizes_against_the_document_dir() {
        let doc = Path::new("/projects/site/design.op");
        assert_eq!(
            to_stored_asset_path(Path::new("/projects/site/assets/a.png"), Some(doc)),
            "assets/a.png"
        );
        assert_eq!(
            to_stored_asset_path(Path::new("/projects/shared/b.png"), Some(doc)),
            "../shared/b.png"
        );
        // No document → absolute, normalized.
        assert_eq!(
            to_stored_asset_path(Path::new("/projects/site/assets/a.png"), None),
            "/projects/site/assets/a.png"
        );
    }
}
