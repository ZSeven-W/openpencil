//! Dispatch and stale-result checks must resolve the same request mode.

use super::super::*;
use super::*;
use op_editor_core::agent_settings::{ImageGenProfile, ImageGenProvider, ImageTestStatus};

fn state_with_auto_slot_and_disabled_generator() -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.image_gen_enabled = false;
    state.editor_ui.agent_settings.image_gen_profiles = vec![ImageGenProfile {
        id: "saved-generator".into(),
        name: "Saved generator".into(),
        provider: ImageGenProvider::Workbench,
        api_key: "test-key".into(),
        model: "test-model".into(),
        base_url: Some("http://127.0.0.1:1".into()),
        test_status: ImageTestStatus::Idle,
    }];
    state.editor_ui.agent_settings.active_image_gen_profile_id = Some("saved-generator".into());
    state.active_children_mut().clear();
    let mut node = image_node("photo", "", Some("ceramic vase"));
    let PenNode::Image(image) = &mut node else {
        panic!("image")
    };
    image.image_prompt = Some("Warm studio photograph on an ivory background".into());
    state.active_children_mut().push(node);
    state
}

#[test]
fn auto_search_result_applies_with_a_saved_but_disabled_generator() {
    let mut state = state_with_auto_slot_and_disabled_generator();
    assert!(!state.editor_ui.agent_settings.image_gen_active());
    assert!(crate::image_panel_host::active_image_gen_profile(&state).is_some());
    let target = collect_targets(&state, &HashSet::new()).remove(0);
    assert_eq!(target.mode, ImageRequestMode::Auto);
    let expected = intent_fingerprint(&target, None);
    let mut session = ImageSearchSession::new();
    session.search_memo.lock().unwrap().insert(
        search_intent_key(&target.query, target.aspect_ratio, target.prompt.as_deref()),
        SearchMemoEntry::Ready("https://example.test/vase.jpg".into()),
    );

    assert!(session.enqueue_missing(&state));
    assert_eq!(session.jobs[0].intent, Some(expected));
    assert!(
        session.poll_into(&mut state),
        "the search result must not be discarded as generation"
    );
    let PenNode::Image(image) = &state.active_children()[0] else {
        panic!("image")
    };
    assert_eq!(image.src, "https://example.test/vase.jpg");
    assert!(!session.is_pending());
}

#[test]
fn current_intents_follow_auto_toggle_but_preserve_explicit_request_modes() {
    for (mode, enabled, prefix) in [
        (ImageRequestMode::Auto, false, "search|"),
        (
            ImageRequestMode::Auto,
            true,
            "generate|saved-generator|test-model|",
        ),
        (ImageRequestMode::Search, false, "search|"),
        (ImageRequestMode::Search, true, "search|"),
        (
            ImageRequestMode::Generate,
            false,
            "generate|saved-generator|test-model|",
        ),
        (
            ImageRequestMode::Generate,
            true,
            "generate|saved-generator|test-model|",
        ),
    ] {
        let mut state = state_with_auto_slot_and_disabled_generator();
        state.editor_ui.agent_settings.image_gen_enabled = enabled;
        let PenNode::Image(image) = &mut state.active_children_mut()[0] else {
            panic!("image")
        };
        match mode {
            ImageRequestMode::Search => image.image_prompt = None,
            ImageRequestMode::Generate => image.image_search_query = None,
            ImageRequestMode::Auto => {}
        }
        assert_eq!(collect_targets(&state, &HashSet::new())[0].mode, mode);
        let intents = current_intent_fingerprints(&state, None);
        assert!(
            intents["photo"].starts_with(prefix),
            "{mode:?}, toggle={enabled}: {:?}",
            intents["photo"]
        );
    }
}
