use super::super::*;
use super::*;
use crate::image_generate_host::test_support::capture_provider;
use op_editor_core::PenNodeExt as _;

fn context_state() -> EditorState {
    EditorState::from_document(
        serde_json::from_value(serde_json::json!({
            "version":"1.0", "name":"Brand Design Portfolio", "children":[{
                "type":"frame", "id":"screen", "name":"Studio Landing", "children":[{
                    "type":"frame", "id":"section", "name":"Selected Work", "children":[
                        {"type":"image", "id":"slot", "src":"", "width":320, "height":240,
                         "imagePrompt":"Velra Type"},
                        {"type":"text", "id":"title", "content":"Velra Type", "fontSize":24}
                    ]
                }]
            }]
        }))
        .unwrap(),
    )
}

fn velra_state() -> EditorState {
    EditorState::from_document(
        serde_json::from_value(serde_json::json!({
            "version":"1.0", "children":[{
                "type":"frame", "id":"screen", "name":"Studio Portfolio — Landing", "children":[{
                    "type":"frame", "id":"card", "name":"card-velra", "children":[
                        {"type":"frame", "id":"image-frame", "name":"velra-image-frame", "children":[
                            {"type":"image", "id":"slot", "name":"velra-image", "src":"",
                             "width":320, "height":240}
                        ]},
                        {"type":"text", "id":"title", "content":"Velra Type", "fontSize":32},
                        {"type":"text", "id":"subtitle", "content":"Typeface microsite", "fontSize":16},
                        {"type":"text", "id":"year", "content":"2024", "fontSize":22},
                        {"type":"text", "id":"tag", "content":"DIGITAL", "fontSize":20}
                    ]
                }]
            }]
        })).unwrap(),
    )
}

fn test_profile() -> ImageGenProfile {
    use op_editor_core::agent_settings::{ImageGenProvider, ImageTestStatus};
    ImageGenProfile {
        id: "capture".into(),
        name: "Test".into(),
        provider: ImageGenProvider::Custom,
        api_key: "test-key".into(),
        model: "test-model".into(),
        base_url: None,
        test_status: ImageTestStatus::Idle,
    }
}

#[test]
fn automatic_generation_sends_composed_prompt_to_provider() {
    let target = collect_targets(&context_state(), &HashSet::new()).remove(0);
    let expected = op_image_enrich::compose_generation_prompt(&target);
    let (profile, request) = capture_provider();
    let expected_intent = intent_fingerprint(&target, Some(&profile));
    let job = spawn_gen_job(target, profile, None);
    let body = request.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(body["prompt"], expected);
    assert!(body["prompt"].as_str().unwrap().contains("Selected Work"));
    assert_eq!(job.intent.as_deref(), Some(expected_intent.as_str()));
    assert!(matches!(
        job.rx.recv_timeout(Duration::from_secs(10)).unwrap(),
        JobOutcome::Generate(Ok(_))
    ));
}

#[test]
fn velra_generation_sends_derived_subject_but_keeps_authored_prompt() {
    for authored in [None, Some("  A typeset poster.\n warm light  ")] {
        let mut target = collect_targets(&velra_state(), &HashSet::new()).remove(0);
        assert_eq!(target.query, "velra-image");
        target.prompt = authored.map(str::to_string);
        let subject = authored.unwrap_or("Velra Type — Typeface microsite");
        let expected = format!("{subject}. Context: this image appears in Studio Portfolio — Landing > card-velra > velra-image-frame > Velra Type — Typeface microsite. Match the page's subject and art direction. Use this context only for subject and mood; do not write its words into the image.");
        let (profile, request) = capture_provider();
        let job = spawn_gen_job(target, profile, None);
        let body = request.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(body["prompt"], expected);
        assert!(matches!(
            job.rx.recv_timeout(Duration::from_secs(10)).unwrap(),
            JobOutcome::Generate(Ok(_))
        ));
    }
}

#[test]
fn context_changes_do_not_change_generation_or_search_identity() {
    let mut target = collect_targets(&velra_state(), &HashSet::new()).remove(0);
    let profile = test_profile();
    let generation_before = intent_fingerprint(&target, Some(&profile));
    target.mode = ImageRequestMode::Search;
    let search_before = intent_fingerprint(&target, None);
    let prompt_before = op_image_enrich::compose_generation_prompt(&target);
    target.page_context = Some("Renamed page > A title that arrived later".into());
    target.generation_subject = Some("Renamed title — Updated subtitle".into());
    assert_ne!(
        op_image_enrich::compose_generation_prompt(&target),
        prompt_before
    );
    assert_eq!(intent_fingerprint(&target, None), search_before);
    target.mode = ImageRequestMode::Generate;
    assert_eq!(
        intent_fingerprint(&target, Some(&profile)),
        generation_before
    );
}

#[test]
fn late_context_change_keeps_inflight_image_and_filled_slot_is_not_reenqueued() {
    let mut state = velra_state();
    let target = collect_targets(&state, &HashSet::new()).remove(0);
    let profile = test_profile();
    state.editor_ui.agent_settings.image_gen_profiles = vec![profile.clone()];
    state.editor_ui.agent_settings.active_image_gen_profile_id = Some(profile.id.clone());
    state.editor_ui.agent_settings.image_gen_enabled = true;
    let intent = intent_fingerprint(&target, Some(&profile));
    let (tx, rx) = mpsc::channel();
    tx.send(JobOutcome::Generate(
        Ok("data:image/png;base64,AA==".into()),
    ))
    .unwrap();
    let mut session = ImageSearchSession::new();
    session.jobs.push(ImageSearchJob {
        node_id: target.node_id.clone(),
        deadline: None,
        intent: Some(intent),
        stock_view: None,
        fallback_target: Some(target),
        rx,
    });
    state.active_children_mut()[0].base_mut().name = Some("Renamed Studio Landing".into());
    let card = &mut state.active_children_mut()[0].children_mut().unwrap()[0];
    let PenNode::Text(subtitle) = &mut card.children_mut().unwrap()[2] else {
        panic!("subtitle")
    };
    subtitle.content = TextContent::Plain("Updated typography project".into());
    assert!(session.poll_into(&mut state));
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
    assert!(!session.enqueue_missing(&state));
    assert!(!session.is_pending());
}
