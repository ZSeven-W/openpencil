use base64::Engine as _;
use op_editor_core::EditorState;

/// Extract the intrinsic raster size from a browser-produced data URL.
pub fn image_data_url_dimensions(src: &str) -> Option<[f32; 2]> {
    let after_scheme = src.strip_prefix("data:")?;
    let comma = after_scheme.find(',')?;
    let metadata = &after_scheme[..comma];
    if !metadata.split(';').any(|part| part == "base64") {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&after_scheme.as_bytes()[comma + 1..])
        .ok()?;
    op_editor_ui::image_runtime::encoded_image_dimensions(&bytes)
        .map(|(width, height)| [width as f32, height as f32])
}

/// Apply one browser-picked fill image together with its intrinsic size.
pub fn apply_fill_image_data_url(state: &mut EditorState, src: &str) -> bool {
    state.set_selected_fill_image_url_with_original_size(src, image_data_url_dimensions(src))
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::ImageFillMode;

    fn png_data_url(width: u32, height: u32) -> String {
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[8..12].copy_from_slice(&13_u32.to_be_bytes());
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&width.to_be_bytes());
        png[20..24].copy_from_slice(&height.to_be_bytes());
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )
    }

    #[test]
    fn web_fill_upload_persists_dimensions_and_exits_crop_edit() {
        let mut state = EditorState::sample();
        let selected = state.selection.anchor.clone();
        assert!(state.set_selected_fill_image_url_with_original_size(
            "data:image/png;base64,old",
            Some([100.0, 100.0]),
        ));
        assert!(state.set_selected_image_fill_mode(ImageFillMode::Crop));
        state.editor_ui.image_crop_editing = Some(selected.clone());
        let url = png_data_url(1179, 2556);

        assert!(apply_fill_image_data_url(&mut state, &url));

        let summary = op_editor_core::fills::first_image_fill_summary(
            state.selected_node().expect("selected node"),
        )
        .expect("image fill");
        assert_eq!(summary.original_size, Some([1179.0, 2556.0]));
        assert_eq!(summary.mode, ImageFillMode::Fill);
        assert_eq!(state.editor_ui.image_crop_editing, None);
        assert_eq!(state.selection.anchor, selected);
    }
}
