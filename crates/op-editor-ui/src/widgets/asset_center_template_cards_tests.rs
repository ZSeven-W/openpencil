//! Saved-template card tests.

use super::template_test_support::{exclusive_user_templates, save_template};
use super::*;

#[test]
fn every_saved_template_becomes_a_card_in_registry_order() {
    let _guard = exclusive_user_templates();
    save_template("my-deck");
    save_template("poster");

    let cards = user_template_cards("");
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].id, "user:my-deck");
    assert_eq!(cards[0].name, "my-deck");
    assert_eq!(cards[1].id, "user:poster");
    assert_eq!(cards[1].frames, 2);
    assert_eq!(cards[1].frame_width, 1920);
    assert_eq!(cards[1].frame_height, 1080);
    assert_eq!(cards[1].preview_jpeg, [0xFF, 0xD8, 0xFF, 0xD9]);
}

#[test]
fn search_matches_the_name_only() {
    let _guard = exclusive_user_templates();
    save_template("warm-spring");
    save_template("cool-winter");

    assert_eq!(user_template_cards("spring").len(), 1);
    assert_eq!(user_template_cards("SPRING").len(), 1);
    assert_eq!(user_template_cards("").len(), 2);
    assert_eq!(user_template_cards("   ").len(), 2);
    assert!(user_template_cards("autumn").is_empty());
    assert_eq!(user_template_card_count("spring"), 1);
    assert_eq!(user_template_card_count(""), 2);
    assert_eq!(user_template_card_count("autumn"), 0);
}

/// Search creates a new card snapshot, but the immutable registry allocation
/// keeps its renderer identity.
#[test]
fn cache_ids_are_stable_across_searches() {
    let _guard = exclusive_user_templates();
    save_template("first");
    save_template("second");

    let full = user_template_cards("");
    let narrowed = user_template_cards("first");
    assert_eq!(narrowed[0].image_id, full[0].image_id);
    assert!(narrowed[0].image_id >= USER_PREVIEW_CACHE_ID_BASE);
    assert_ne!(full[0].image_id, full[1].image_id);
}

/// Removing an earlier registry entry must not move the following preview
/// into its cache slot. Reusing that id would make the renderer keep drawing
/// the deleted card's resident raster for the survivor.
#[test]
fn cache_identity_survives_predecessor_delete_and_is_not_reused() {
    let _guard = exclusive_user_templates();
    let first_id = save_template("first");
    let second_id = save_template("second");

    let before = user_template_cards("");
    let first_image_id = before[0].image_id;
    let second_image_id = before[1].image_id;
    drop(before);

    op_editor_core::user_scene_templates::remove_user_scene_template(&first_id)
        .expect("the first template exists");
    let after_delete = user_template_cards("");
    assert_eq!(after_delete[0].id, second_id);
    assert_eq!(after_delete[0].image_id, second_image_id);
    assert_ne!(after_delete[0].image_id, first_image_id);
    drop(after_delete);

    // The same logical slug may be saved again after deletion. Its new Arc is
    // a new immutable preview generation and must not resurrect the old id.
    assert_eq!(save_template("first"), first_id);
    let after_resave = user_template_cards("");
    let resaved = after_resave
        .iter()
        .find(|card| card.id == first_id)
        .expect("the slug was saved again");
    assert_ne!(resaved.image_id, first_image_id);
    assert_ne!(resaved.image_id, second_image_id);
}

/// Replacing an entry is how the registry publishes changed preview bytes.
/// The new immutable allocation needs a new cache identity even though its
/// user-facing template id stays the same.
#[test]
fn changed_preview_for_the_same_template_gets_a_fresh_cache_identity() {
    let _guard = exclusive_user_templates();
    let id = save_template("mutable");
    let old_image_id = user_template_cards("")[0].image_id;
    let held = op_editor_core::user_scene_templates::user_scene_templates()
        .into_iter()
        .find(|template| template.id == id)
        .expect("saved template");
    let mut replacement = held.as_ref().clone();
    replacement.preview_jpeg = vec![0xFF, 0xD8, 0x01, 0xFF, 0xD9];
    op_editor_core::user_scene_templates::load_user_scene_template(replacement)
        .expect("replacement stays within quota");

    let replaced = user_template_cards("");
    assert_eq!(replaced[0].id, id);
    assert_ne!(replaced[0].image_id, old_image_id);
}

/// Card snapshots keep the registry's Arc rather than cloning its JPEG. The
/// separate count path does not need to build cards at all.
#[test]
fn card_snapshots_share_preview_storage_and_count_is_lightweight() {
    let _guard = exclusive_user_templates();
    save_template("shared-preview");
    let registry = op_editor_core::user_scene_templates::user_scene_templates();

    assert_eq!(user_template_card_count("shared"), 1);
    let cards = user_template_cards("shared");
    assert!(std::sync::Arc::ptr_eq(&cards[0].template, &registry[0]));
    assert_eq!(
        cards[0].preview_jpeg.as_ptr(),
        registry[0].preview_jpeg.as_ptr(),
        "the card must borrow the registry JPEG allocation"
    );
}
