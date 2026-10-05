//! Selection-biased standard-route intent tests.

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, StopReason};
use op_editor_core::EditorState;
use std::time::Duration;

use super::*;

struct Scripted;

impl ChatProvider for Scripted {
    fn provider_label(&self) -> &str {
        "scripted"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(
            vec![
                ChatDelta::TextDelta("DESIGN_NEW".to_string()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]
            .into_iter()
            .inspect(|_| std::thread::sleep(Duration::ZERO)),
        )
    }
}

fn frame(id: &str, name: &str, children: Vec<PenNode>) -> PenNode {
    let mut node: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": id,
        "name": name,
        "x": 0.0,
        "y": 0.0,
        "width": 390.0,
        "height": 800.0,
        "children": [],
    }))
    .expect("valid frame json");
    if let Some(kids) = node.children_mut() {
        *kids = children;
    }
    node
}

fn state_with_selected_card() -> EditorState {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "screen",
        "Home",
        vec![frame("card", "Selected Card", Vec::new())],
    ));
    state.set_single_selection(op_editor_core::NodeId::new("card"));
    state
}

#[test]
fn selection_bias_routes_keywordless_instruction_to_modify() {
    let provider = Scripted;
    let state = state_with_selected_card();

    assert_eq!(
        classify_intent_for_standard_route(&provider, &state, "给它加一个边框", None),
        DesignIntent::Modify
    );
}

#[test]
fn selection_bias_does_not_hijack_whole_new_screen_or_chat() {
    let provider = Scripted;
    let state = state_with_selected_card();

    assert_eq!(
        classify_intent_for_standard_route(&provider, &state, "重新画一个首页", None),
        DesignIntent::New
    );
    assert_eq!(
        classify_intent_for_standard_route(&provider, &state, "这是什么字体", None),
        DesignIntent::Chat
    );
}

#[test]
fn selection_does_not_hijack_english_section_heavy_new_design() {
    // Regression: "Design a … page" whose spec mentions "section" three times
    // trips `is_section_add_request`, so `requests_new_whole_screen` is false;
    // a stray selection then dragged the whole new-design prompt into modify
    // (measured: M3 flat-JSONL → "Could not parse design nodes"). The
    // creation-signal veto keeps it New.
    let provider = Scripted;
    let state = state_with_selected_card();
    let prompt = "Design a travel booking mobile app explore page. Include a search section with \"Where to?\" input, date picker chips, and guest count. \"Deals of the Week\" section with 2 featured deal cards. Recently viewed section with 2 compact cards. Bottom tab bar. Warm, inviting design with orange accents.";
    assert!(
        crate::chat_intent::has_new_screen_creation_signal(prompt),
        "creation signal must fire on a design-a-page prompt"
    );
    assert_ne!(
        classify_intent_for_standard_route(&provider, &state, prompt, None),
        DesignIntent::Modify,
        "a section-heavy new-design prompt must not be hijacked to modify by a selection"
    );
}

#[test]
fn selection_does_not_hijack_listed_follow_on_screens() {
    let provider = Scripted;
    let state = state_with_selected_card();
    for prompt in [
        "继续完成 explore/profile界面",
        "Continue generating the explore/profile interface",
    ] {
        assert!(
            requests_new_whole_screen(prompt),
            "an explicit list of sibling interfaces is a whole-screen request: {prompt}"
        );
        assert!(
            detect_append_intent(&state, prompt).is_none(),
            "listed sibling interfaces must not append into the first existing frame: {prompt}"
        );
        assert_eq!(
            classify_intent_for_standard_route(&provider, &state, prompt, None),
            DesignIntent::New,
            "a stale selection must not turn a multi-screen continuation into modify: {prompt}"
        );
        assert!(
            should_auto_generate_design_md(&state, prompt, None),
            "follow-on screens should inherit the existing canvas design system: {prompt}"
        );
    }
}

#[test]
fn ambiguous_current_interface_completion_is_not_a_new_screen() {
    for prompt in [
        "继续完成这个界面",
        "继续完成当前界面的推荐区块",
        "继续完成 profile 界面的 header",
        "继续完成 explore/profile 界面之间的跳转",
    ] {
        assert!(
            !requests_new_whole_screen(prompt),
            "current-screen or subobject work must stay in-place: {prompt}"
        );
    }
}

#[test]
fn ordinary_homepage_followup_captures_and_enforces_only_its_real_board() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    for (id, name) in [
        ("coffee-home", "01 咖啡首页"),
        ("menu", "02 菜单"),
        ("profile", "03 我的"),
    ] {
        state.active_children_mut().push(frame(
            id,
            name,
            vec![serde_json::from_value(serde_json::json!({
                "type": "text", "id": format!("{id}-title"), "content": "原标题",
                "x": 20, "y": 40, "width": 320, "height": 40
            }))
            .unwrap()],
        ));
    }
    state
        .editor_ui
        .workspace
        .open_for_reading(op_editor_core::HomeFamily::AppUi, 1);
    let prompt = "把首页标题改成“周末，来杯好咖啡”，保留其他页面和布局。";
    assert_eq!(
        classify_intent_for_standard_route(&Scripted, &state, prompt, None),
        DesignIntent::Modify
    );
    let plan = build_modify_plan(&state, prompt).expect("named existing homepage");
    assert_eq!(plan.target_frame_ids, ["coffee-home"]);
    assert!(!plan.user_message.contains("menu-title"));
    let before = state.active_children().to_vec();
    let mut changed = serde_json::to_value(&before[0].children().unwrap()[0]).unwrap();
    changed["content"] = serde_json::json!("周末，来杯好咖啡");
    let mut unrelated = serde_json::to_value(&before[1].children().unwrap()[0]).unwrap();
    unrelated["content"] = serde_json::json!("Must not change");
    let (count, applied) = crate::chat_canvas_tools::apply_design_modification(
        &mut state,
        &[("null".into(), changed), ("null".into(), unrelated)],
        &plan.target_frame_ids,
    );
    assert!(applied);
    assert_eq!(count, 1);
    assert_eq!(state.active_children()[1..], before[1..]);
    assert_eq!(
        serde_json::to_value(&state.active_children()[0].children().unwrap()[0]).unwrap()
            ["content"],
        "周末，来杯好咖啡"
    );
}

#[test]
fn normal_mode_replacement_copy_cannot_override_the_scoped_route() {
    let mut state = state_with_selected_card();
    state.clear_selection();
    state
        .editor_ui
        .workspace
        .open_for_reading(op_editor_core::HomeFamily::AppUi, 1);
    for prompt in [
        "把首页标题改成“如何做咖啡”",
        "Change Home title to \"Create a new page\"",
        "把首页标题改成如何做咖啡",
        "Change Home title to How to create a new page",
        "Change Home title to Orders",
        "Set Home title to Create a new page",
    ] {
        assert_eq!(
            classify_intent_for_standard_route(&Scripted, &state, prompt, None),
            DesignIntent::Modify,
            "{prompt}"
        );
        let plan = build_modify_plan(&state, prompt).expect("literal copy is a scoped edit");
        assert!(
            plan.user_message.ends_with(prompt),
            "routing must not rewrite model copy"
        );
        assert_eq!(plan.target_frame_ids.len(), 1);
    }
}
