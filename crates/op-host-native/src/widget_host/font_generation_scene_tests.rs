//! Regression test for the font-import scene-rebuild gate (Codex Phase 1
//! BLOCKER 3).
//!
//! A runtime font import bumps the `jian_skia` font-registry generation
//! WITHOUT dirtying `editor_state`. `refresh_layout_scene` early-outs on
//! `!editor_state_dirty`, so without an explicit generation watch the open
//! document keeps its stale fallback-font layout until an unrelated dirty
//! event. This test pins the watch: an import between two refreshes must
//! advance the recorded scene generation (i.e. the rebuild branch ran).

use super::WidgetHostNative;

// A variable font almost never installed system-wide, distinct from the
// family used by `backend/skia/font_import_tests.rs` so the two tests don't
// contend on the same registry family when run in the same binary.
const SPACE_GROTESK: &[u8] =
    include_bytes!("../../../op-host-desktop/assets/fonts/SpaceGrotesk-VF.ttf");

#[test]
fn font_import_forces_layout_scene_rebuild_without_editor_dirty() {
    let _guard = crate::font_registry_test_support::lock();
    let mut host = WidgetHostNative::new();

    // Settle the initial scene so `editor_state_dirty` is cleared and the
    // font generation is recorded against the current layout.
    host.refresh_layout_scene();
    assert!(
        !host.editor_state_dirty,
        "scene should be settled (not dirty) after the initial refresh"
    );
    let generation_before = host.layout_scene_font_generation;

    // Import a font: bumps the global generation but does NOT dirty
    // editor_state — exactly the case the gate must catch.
    let blob = jian_skia::register_imported_font(SPACE_GROTESK.to_vec())
        .expect("SpaceGrotesk-VF.ttf must parse as a font");
    assert!(
        !host.editor_state_dirty,
        "a font import must not dirty editor state (that's the whole trap)"
    );

    // The next refresh must observe the generation change and rebuild,
    // advancing the recorded generation to the current global value.
    host.refresh_layout_scene();
    assert!(
        host.layout_scene_font_generation > generation_before,
        "refresh_layout_scene must pick up the font-generation bump \
         (before={generation_before}, after={})",
        host.layout_scene_font_generation
    );
    assert_eq!(
        host.layout_scene_font_generation,
        jian_skia::font_generation(),
        "recorded scene generation must match the current global generation"
    );

    // Clean up so the family doesn't leak into sibling tests in this binary.
    jian_skia::remove_imported_font(&blob.family);
}
