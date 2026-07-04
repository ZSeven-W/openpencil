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
