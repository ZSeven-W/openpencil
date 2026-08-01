//! Compile-time embedded Scene Template card previews.
//!
//! Baked from the full-resolution renders by
//! `templates/step0/_generators/scene_preview_cards.py`; see that script for
//! why a deck is tiled into a grid rather than fitted as a strip.

/// Cache ids are hand-assigned and must stay stable: the renderer keys its
/// decoded-raster cache on them, so reusing an id for different bytes would
/// serve the wrong image. They start above the Prompt Center's range so the
/// two catalogues can never collide in that shared cache.
const CACHE_ID_BASE: u64 = 10_000;

macro_rules! preview {
    ($offset:expr, $name:literal) => {
        Some((
            CACHE_ID_BASE + $offset,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/scene_template_previews/",
                $name,
                ".jpg"
            ))
            .as_slice(),
        ))
    };
}

/// Return the stable cache id and embedded JPEG bytes for a template.
///
/// Every shipped template has one — `scene_template_catalog` rejects a
/// catalogue entry without a document, and the preview baker is driven by the
/// same id list — so `None` means an unknown id, not a missing asset.
pub(crate) fn scene_template_preview(template_id: &str) -> Option<(u64, &'static [u8])> {
    match template_id {
        "screenshot-tutorial" => preview!(1, "screenshot-tutorial"),
        "knowledge-carousel" => preview!(2, "knowledge-carousel"),
        "before-after" => preview!(3, "before-after"),
        "slide-deck" => preview!(4, "slide-deck"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::scene_template_catalog::scene_template_catalogue;
    use std::collections::HashSet;

    #[test]
    fn every_shipped_template_has_a_preview_with_a_unique_cache_id() {
        let mut ids = HashSet::new();
        for template in scene_template_catalogue() {
            let (cache_id, bytes) = scene_template_preview(&template.id)
                .unwrap_or_else(|| panic!("{} has no card preview", template.id));
            assert!(!bytes.is_empty(), "{} preview is empty", template.id);
            assert!(
                ids.insert(cache_id),
                "{} reuses cache id {cache_id}",
                template.id
            );
        }
        assert!(scene_template_preview("no-such-template").is_none());
    }

    #[test]
    fn previews_are_jpeg_so_the_raster_decoder_accepts_them() {
        for template in scene_template_catalogue() {
            let (_, bytes) = scene_template_preview(&template.id).expect("preview");
            assert_eq!(&bytes[..2], &[0xFF, 0xD8], "{} is not a JPEG", template.id);
        }
    }
}
