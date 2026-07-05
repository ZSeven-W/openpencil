//! Phase 1 end-to-end test for the user-font-import invalidation chain.
//!
//! Proves that registering an imported font at runtime (`jian_skia::
//! register_imported_font`) flows all the way through the process-global
//! font registry → generation counter → `FontResolver` cache invalidation →
//! `NativeBackend` measure/resolve paths, so an already-constructed backend
//! re-measures a family with the newly-imported face instead of a stale
//! fallback. This is the "measure-changes-after-register" gate from the
//! font-import design spec (§B, BLOCKER 1).
//!
//! Kept in one `#[test]` because every assertion mutates the process-global
//! font registry; a single owner of the `DM Serif Display` family avoids
//! races with a sibling test running in the same binary. Assertions on the
//! generation counter use strict `>` (robust to concurrent bumps from other
//! tests that register bundled fonts) and never assert an exact global value.

use jian_core::layout::measure::FontStyleKind;

use super::NativeBackend;

/// "DM Serif Display" is a Google display serif almost never installed
/// system-wide, so on CI the pre-import lookup falls back to the default
/// face and the post-import lookup resolves the real imported metrics —
/// making the invalidation observable. The bytes ship with op-host-desktop
/// (the native font owner); the sibling `tests.rs` already includes fonts
/// from there the same way.
const FAMILY: &str = "DM Serif Display";
const DM_SERIF_DISPLAY: &[u8] =
    include_bytes!("../../../../op-host-desktop/assets/fonts/DMSerifDisplay-Regular.ttf");

fn imported_face_count(family: &str) -> usize {
    jian_skia::list_families()
        .into_iter()
        .find(|m| m.family == family)
        .map(|m| m.face_count)
        .unwrap_or(0)
}

#[test]
fn measure_changes_after_registering_imported_font() {
    let mut be = NativeBackend::with_dpi(1.0);

    // Resolve + measure the family BEFORE the import. This also populates
    // the resolver's per-char cache, so a broken invalidation chain would
    // hand back this stale entry after the import and fail the assertions
    // below.
    let before_face = be
        .typeface_for_family_char('A', FAMILY, 400)
        .map(|tf| tf.family_name());
    let before_width = be.measure_text_family_styled("Imported", 40.0, FAMILY, 400, false);

    // --- register the imported font ------------------------------------
    let generation_before = jian_skia::font_generation();
    let blob = jian_skia::register_imported_font(DM_SERIF_DISPLAY.to_vec())
        .expect("DMSerifDisplay-Regular.ttf must parse as a font");

    assert_eq!(blob.family, FAMILY, "parsed family name");
    assert_eq!(blob.style, FontStyleKind::Normal, "regular face is upright");
    assert!(
        (300..=500).contains(&blob.weight),
        "DM Serif Display Regular weight should be ~400, got {}",
        blob.weight
    );
    assert!(
        jian_skia::font_generation() > generation_before,
        "register_imported_font must advance the font generation"
    );
    assert_eq!(
        imported_face_count(FAMILY),
        1,
        "one imported face registered"
    );

    // --- the SAME backend must now see the imported face ---------------
    let after_face = be
        .typeface_for_family_char('A', FAMILY, 400)
        .map(|tf| tf.family_name());
    assert_eq!(
        after_face.as_deref(),
        Some(FAMILY),
        "imported family must resolve after register (before was {before_face:?}) — \
         the resolver cache did not invalidate on the generation bump"
    );

    let after_width = be.measure_text_family_styled("Imported", 40.0, FAMILY, 400, false);
    assert!(
        after_width > 0.0 && after_width.is_finite(),
        "post-import measure must be a real width, got {after_width}"
    );

    // On any host lacking DM Serif Display the fallback width differs from
    // the imported face's, so the measure genuinely reflows. Guarded so the
    // test still passes on a dev machine that happens to have it installed.
    if before_face.as_deref() != Some(FAMILY) {
        assert_ne!(
            before_width, after_width,
            "measure must change once the imported face resolves"
        );
    }

    // --- re-importing byte-identical bytes is idempotent ---------------
    // Returns the existing blob and adds no second face (dedup on
    // (family, style, weight)); it must not duplicate the family.
    let again = jian_skia::register_imported_font(DM_SERIF_DISPLAY.to_vec())
        .expect("identical re-import parses");
    assert_eq!(again.hash, blob.hash, "identical bytes → same content hash");
    assert_eq!(
        imported_face_count(FAMILY),
        1,
        "identical re-import must not add a duplicate face"
    );

    // --- removal also invalidates --------------------------------------
    let generation_before_remove = jian_skia::font_generation();
    assert!(
        jian_skia::remove_imported_font(FAMILY),
        "removing the imported family must report a change"
    );
    assert!(
        jian_skia::font_generation() > generation_before_remove,
        "remove_imported_font must advance the generation again"
    );
    assert_eq!(imported_face_count(FAMILY), 0, "family gone after removal");
    if before_face.as_deref() != Some(FAMILY) {
        let removed_face = be
            .typeface_for_family_char('A', FAMILY, 400)
            .map(|tf| tf.family_name());
        assert_ne!(
            removed_face.as_deref(),
            Some(FAMILY),
            "the imported face must stop resolving after removal"
        );
    }
}
