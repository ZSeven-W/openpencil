//! Saved-template registry tests.
//!
//! The registry is process-global by design — one editor, one set of saved
//! templates — so every case here takes [`exclusive`] and starts from an
//! empty store. Without that, cargo's parallel threads would each see
//! whatever the others had just saved.

use super::exclusive_registry_for_tests as exclusive;
use super::*;

fn entry(id: &str, name: &str) -> UserSceneTemplate {
    UserSceneTemplate {
        id: id.to_string(),
        name: name.to_string(),
        frames: 2,
        frame_width: 1920,
        frame_height: 1080,
        document: "{\"version\":\"1.0.0\",\"children\":[]}".to_string(),
        preview_jpeg: Vec::new(),
    }
}

#[test]
fn a_saved_template_is_queryable_and_carries_its_fields() {
    let _guard = exclusive();
    load_user_scene_template(entry("user:my-deck", "My Deck")).expect("saves");

    let loaded = user_scene_templates();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "user:my-deck");
    assert_eq!(loaded[0].name, "My Deck");
    assert_eq!(loaded[0].slug(), "my-deck");
    assert_eq!(loaded[0].frames, 2);
    assert_eq!(loaded[0].frame_width, 1920);
    assert_eq!(loaded[0].frame_height, 1080);
}

#[test]
fn set_replaces_the_whole_store_like_a_boot_scan() {
    let _guard = exclusive();
    load_user_scene_template(entry("user:first", "First")).expect("saves");
    set_user_scene_templates(vec![entry("user:second", "Second")]);
    let loaded = user_scene_templates();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "user:second");
}

#[test]
fn removing_returns_the_entry_so_the_host_can_delete_its_directory() {
    let _guard = exclusive();
    load_user_scene_template(entry("user:my-deck", "My Deck")).expect("saves");

    let removed = remove_user_scene_template("user:my-deck").expect("removed");
    assert_eq!(removed.slug(), "my-deck");
    assert!(user_scene_templates().is_empty());
    assert!(remove_user_scene_template("user:my-deck").is_none());
}

#[test]
fn re_loading_the_same_id_replaces_it_so_a_rescan_is_idempotent() {
    let _guard = exclusive();
    load_user_scene_template(entry("user:disk", "First")).expect("saves");
    load_user_scene_template(entry("user:disk", "Second")).expect("saves");
    assert_eq!(user_scene_templates().len(), 1);
    assert_eq!(user_scene_templates()[0].name, "Second");
}

#[test]
fn same_slug_saves_are_numbered_rather_than_overwritten() {
    let _guard = exclusive();
    assert_eq!(allocate_template_id("my-deck"), "user:my-deck");
    load_user_scene_template(entry("user:my-deck", "A")).expect("saves");
    assert_eq!(allocate_template_id("my-deck"), "user:my-deck-2");
    load_user_scene_template(entry("user:my-deck-2", "B")).expect("saves");
    assert_eq!(allocate_template_id("my-deck"), "user:my-deck-3");
    assert_eq!(user_scene_templates().len(), 2);
}

#[test]
fn a_full_quota_refuses_a_new_id_without_touching_what_is_there() {
    let _guard = exclusive();
    for index in 0..USER_TEMPLATE_QUOTA {
        load_user_scene_template(entry(&format!("user:filled-{index}"), "Filled"))
            .expect("fits under the quota");
    }

    let refused = load_user_scene_template(entry("user:one-too-many", "Extra"));
    assert_eq!(refused, Err(UserSceneTemplateError::QuotaExceeded));
    assert_eq!(user_scene_templates().len(), USER_TEMPLATE_QUOTA);
}

#[test]
fn replacing_an_existing_id_is_allowed_at_the_quota_cap() {
    let _guard = exclusive();
    for index in 0..USER_TEMPLATE_QUOTA {
        load_user_scene_template(entry(&format!("user:filled-{index}"), "Filled"))
            .expect("fits under the quota");
    }
    // Re-registering an existing id is an idempotent rescan, not a new save —
    // it must not be refused even at the cap.
    load_user_scene_template(entry("user:filled-0", "Replacement")).expect("replaces in place");
    assert_eq!(user_scene_templates().len(), USER_TEMPLATE_QUOTA);
}

#[test]
fn a_poisoned_store_is_recovered_before_a_save_is_acknowledged() {
    let _guard = exclusive();
    let poisoned = std::panic::catch_unwind(|| {
        let _held = write_store();
        panic!("poison the saved-template store");
    });
    assert!(poisoned.is_err());

    let saved = load_user_scene_template(entry("user:after-panic", "Recovered"))
        .expect("a recovered store still accepts saves");
    let loaded = user_scene_templates();
    assert_eq!(loaded.len(), 1);
    assert!(
        Arc::ptr_eq(&saved, &loaded[0]),
        "Ok must name the entry that was actually registered"
    );
}
