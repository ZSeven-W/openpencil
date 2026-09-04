//! Scene Template card previews.
//!
//! Baked from the full-resolution renders by
//! `templates/step0/_generators/scene_preview_cards.py`; see that script for
//! why a deck is tiled into a grid rather than fitted as a strip.
//!
//! ~2.7 MiB of JPEG. Embedded on desktop, fetched from the daemon on `wasm32`
//! for the same reason the Prompt Center's are — see
//! `prompt_center_previews` for the full rationale and
//! `op_editor_core::web_assets` for the loader.

/// Cache ids are hand-assigned and must stay stable: the renderer keys its
/// decoded-raster cache on them, so reusing an id for different bytes would
/// serve the wrong image. They start above the Prompt Center's range so the
/// two catalogues can never collide in that shared cache.
const CACHE_ID_BASE: u64 = 10_000;

/// Cache-id namespace for the user's SAVED templates' previews (`UTPL`).
///
/// [`super::asset_center_template_cards`] assigns a monotonic process-local
/// offset to each immutable registry allocation. A high mnemonic namespace
/// keeps those ids disjoint from the shipped cards' small integer range and
/// from the other fixed UI image namespaces.
pub(crate) const USER_PREVIEW_CACHE_ID_BASE: u64 = 0x5554_504c_0000_0000;

// Test-only: `concat!` cannot interpolate a const, so the route literals in
// the macro below spell the directory out. This is the value the route
// tests rebuild the expected path from, which is what keeps the spelled-out
// literal and the staged bundle layout (`tools/stage-web-assets.sh`) from
// drifting into a silent per-asset 404.
#[cfg(test)]
const PREVIEW_DIR: &str = "scene_template_previews";

/// One card's preview, resolved for the current platform.
pub(crate) struct TemplatePreview {
    /// Stable renderer cache id, known before any bytes are.
    pub image_id: u64,
    /// `None` on wasm until the fetch lands; always `Some` on native.
    pub bytes: Option<&'static [u8]>,
    /// Daemon route to fetch. Unused on native.
    pub route: &'static str,
}

macro_rules! preview {
    ($offset:expr, $name:literal) => {{
        const ROUTE: &str = concat!(
            "/pkg/assets/",
            "scene_template_previews",
            "/",
            $name,
            ".jpg"
        );
        #[cfg(not(target_arch = "wasm32"))]
        let bytes = Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/scene_template_previews/",
                $name,
                ".jpg"
            ))
            .as_slice(),
        );
        #[cfg(target_arch = "wasm32")]
        let bytes = op_editor_core::web_assets::installed_bytes(ROUTE);
        Some(TemplatePreview {
            image_id: CACHE_ID_BASE + $offset,
            bytes,
            route: ROUTE,
        })
    }};
}

/// Return the preview for a template id.
///
/// Every shipped template has one — `scene_template_catalog` rejects a
/// catalogue entry without a document, and the preview baker is driven by the
/// same id list — so `None` means an unknown id, not a missing asset. On web a
/// `Some` with `bytes: None` means "not fetched yet", which is not the same
/// answer.
pub(crate) fn scene_template_preview(template_id: &str) -> Option<TemplatePreview> {
    match template_id {
        "screenshot-tutorial" => preview!(1, "screenshot-tutorial"),
        "knowledge-carousel" => preview!(2, "knowledge-carousel"),
        "before-after" => preview!(3, "before-after"),
        "slide-deck" => preview!(4, "slide-deck"),
        "knowledge-card-vertical" => preview!(5, "knowledge-card-vertical"),
        "knowledge-card-square" => preview!(6, "knowledge-card-square"),
        "pitch-deck-dark" => preview!(7, "pitch-deck-dark"),
        "lecture-deck-light" => preview!(8, "lecture-deck-light"),
        "minimal-keynote" => preview!(9, "minimal-keynote"),
        "gradient-tech" => preview!(10, "gradient-tech"),
        "saas-landing-orange" => preview!(11, "saas-landing-orange"),
        "product-landing-light" => preview!(12, "product-landing-light"),
        "punch-quote-card" => preview!(13, "punch-quote-card"),
        "journal-checklist-card" => preview!(14, "journal-checklist-card"),
        "data-report-infographic" => preview!(15, "data-report-infographic"),
        "steps-flow-infographic" => preview!(16, "steps-flow-infographic"),
        "event-poster-deck" => preview!(17, "event-poster-deck"),
        "pitfall-list-infographic" => preview!(18, "pitfall-list-infographic"),
        "spine-culture-card" => preview!(19, "spine-culture-card"),
        "metric-single-card" => preview!(20, "metric-single-card"),
        "quote-frame-card" => preview!(21, "quote-frame-card"),
        "daily-sign-card" => preview!(22, "daily-sign-card"),
        "price-tier-card" => preview!(23, "price-tier-card"),
        "notice-board-card" => preview!(24, "notice-board-card"),
        "milestone-timeline-infographic" => preview!(25, "milestone-timeline-infographic"),
        "concept-contrast-infographic" => preview!(26, "concept-contrast-infographic"),
        "ranking-board-infographic" => preview!(27, "ranking-board-infographic"),
        "faq-thread-infographic" => preview!(28, "faq-thread-infographic"),
        "data-story-infographic" => preview!(29, "data-story-infographic"),
        "challenge-tracker-infographic" => preview!(30, "challenge-tracker-infographic"),
        "ecosystem-map-infographic" => preview!(31, "ecosystem-map-infographic"),
        "do-dont-comparison" => preview!(32, "do-dont-comparison"),
        "myth-truth-comparison" => preview!(33, "myth-truth-comparison"),
        "pricing-tiers-comparison" => preview!(34, "pricing-tiers-comparison"),
        "scenario-guide-comparison" => preview!(35, "scenario-guide-comparison"),
        "spec-table-comparison" => preview!(36, "spec-table-comparison"),
        "three-way-comparison" => preview!(37, "three-way-comparison"),
        "time-shift-comparison" => preview!(38, "time-shift-comparison"),
        "tradeoff-scale-comparison" => preview!(39, "tradeoff-scale-comparison"),
        "version-diff-comparison" => preview!(40, "version-diff-comparison"),
        "app-onboarding-triptych" => preview!(41, "app-onboarding-triptych"),
        "diy-blueprint-guide" => preview!(42, "diy-blueprint-guide"),
        "photo-composition-tutorial" => preview!(43, "photo-composition-tutorial"),
        "recipe-four-step" => preview!(44, "recipe-four-step"),
        "skincare-routine-cards" => preview!(45, "skincare-routine-cards"),
        "software-step-tutorial" => preview!(46, "software-step-tutorial"),
        "storage-makeover-steps" => preview!(47, "storage-makeover-steps"),
        "weekly-report-lesson" => preview!(48, "weekly-report-lesson"),
        "workout-breakdown-guide" => preview!(49, "workout-breakdown-guide"),
        "bookreview-silk-carousel" => preview!(50, "bookreview-silk-carousel"),
        "cityguide-film-carousel" => preview!(51, "cityguide-film-carousel"),
        "datareport-grid-carousel" => preview!(52, "datareport-grid-carousel"),
        "opinion-longform-carousel" => preview!(53, "opinion-longform-carousel"),
        "qa-chalkboard-carousel" => preview!(54, "qa-chalkboard-carousel"),
        "story-night-carousel" => preview!(55, "story-night-carousel"),
        "toolkit-notebook-carousel" => preview!(56, "toolkit-notebook-carousel"),
        "tutorial-journal-carousel" => preview!(57, "tutorial-journal-carousel"),
        "yearreview-mineral-carousel" => preview!(58, "yearreview-mineral-carousel"),
        "sounding-navy-deck" => preview!(59, "sounding-navy-deck"),
        "tidemark-slate-deck" => preview!(60, "tidemark-slate-deck"),
        "banxin-rule-deck" => preview!(61, "banxin-rule-deck"),
        "gridpaper-graphite-deck" => preview!(62, "gridpaper-graphite-deck"),
        "dossier-linen-deck" => preview!(63, "dossier-linen-deck"),
        "ledger-tick-deck" => preview!(64, "ledger-tick-deck"),
        "brand-concept-sheet" => preview!(65, "brand-concept-sheet"),
        "logo-qa-board" => preview!(66, "logo-qa-board"),
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
            let preview = scene_template_preview(&template.id)
                .unwrap_or_else(|| panic!("{} has no card preview", template.id));
            let bytes = preview.bytes.expect("native embeds every preview");
            assert!(!bytes.is_empty(), "{} preview is empty", template.id);
            assert!(
                ids.insert(preview.image_id),
                "{} reuses cache id {}",
                template.id,
                preview.image_id
            );
        }
        assert!(scene_template_preview("no-such-template").is_none());
    }

    #[test]
    fn saved_preview_ids_never_overlap_the_shipped_range() {
        // The renderer keys its decoded-raster cache on the id, so an overlap
        // would serve a saved template's bytes under a shipped template's id
        // (or vice versa). The assertion derives the shipped range from the
        // catalogue, so it stays valid as either side grows.
        let shipped_first = CACHE_ID_BASE;
        let shipped_last = CACHE_ID_BASE + scene_template_catalogue().len() as u64 + 1; // the preview! offsets start at 1
        assert!(
            USER_PREVIEW_CACHE_ID_BASE > shipped_last,
            "saved range {USER_PREVIEW_CACHE_ID_BASE} must sit above shipped range \
             {shipped_first}..{shipped_last}"
        );
    }

    #[test]
    fn previews_are_jpeg_so_the_raster_decoder_accepts_them() {
        for template in scene_template_catalogue() {
            let bytes = scene_template_preview(&template.id)
                .expect("preview")
                .bytes
                .expect("native embeds every preview");
            assert_eq!(&bytes[..2], &[0xFF, 0xD8], "{} is not a JPEG", template.id);
        }
    }

    #[test]
    fn every_route_is_distinct_and_uses_the_shared_prefix() {
        // The route is the web host's identity for the asset, exactly as the
        // cache id is the renderer's. `concat!` cannot interpolate the prefix
        // constant, so this is what keeps the spelled-out literal honest.
        let mut routes = HashSet::new();
        for template in scene_template_catalogue() {
            let preview = scene_template_preview(&template.id).expect("preview");
            assert_eq!(
                preview.route,
                format!(
                    "{}{}/{}.jpg",
                    op_editor_core::web_assets::WEB_ASSET_ROUTE_PREFIX,
                    PREVIEW_DIR,
                    template.id
                )
            );
            assert!(routes.insert(preview.route), "duplicate {}", preview.route);
        }
    }
}
