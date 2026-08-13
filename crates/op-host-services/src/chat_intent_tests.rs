//! Tests for the CLI standard-mode intent router (GAP #33).

use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use op_ai::chat_provider::{ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest, StopReason};
use op_editor_core::EditorState;
use op_orchestrator::DesignRequest;

use super::*;
use crate::chat_canvas_tools::{
    apply_design_modification, chat_tool_channel, DesignModificationOp,
};

// ---------------------------------------------------------------------------
// Keyword + tag classification
// ---------------------------------------------------------------------------

#[test]
fn keyword_classifier_matches_ts_rule_order() {
    // MODIFY keywords win unless a CHAT keyword fires without one.
    assert_eq!(
        classify_by_keywords("change the title color"),
        DesignIntent::Modify
    );
    assert_eq!(
        classify_by_keywords("make it smaller please"),
        DesignIntent::Modify
    );
    assert_eq!(
        classify_by_keywords("add to the header"),
        DesignIntent::Modify
    );
    // CHAT keyword without a modify keyword.
    assert_eq!(classify_by_keywords("what is a frame?"), DesignIntent::Chat);
    assert_eq!(
        classify_by_keywords("explain auto layout"),
        DesignIntent::Chat
    );
    // CHAT + MODIFY both present → modify (TS: chat requires !modify).
    assert_eq!(
        classify_by_keywords("can you fix the button"),
        DesignIntent::Modify
    );
    // Neither → new.
    assert_eq!(
        classify_by_keywords("design a login page"),
        DesignIntent::New
    );
    assert_eq!(classify_by_keywords("a pricing table"), DesignIntent::New);
}

#[test]
fn keyword_classifier_handles_cjk_modify_requests() {
    assert_eq!(classify_by_keywords("修改成饺子"), DesignIntent::Modify);
    assert_eq!(
        classify_by_keywords("把这个卡片改为饺子"),
        DesignIntent::Modify
    );
    assert_eq!(classify_by_keywords("替换成新的主色"), DesignIntent::Modify);
}

#[test]
fn keyword_phrases_respect_word_boundaries() {
    // "moved" must not match \bmove\b; "exchange" must not match change.
    assert_eq!(
        classify_by_keywords("the moved exchange"),
        DesignIntent::New
    );
    // Multi-word phrase across extra whitespace still matches.
    assert_eq!(
        classify_by_keywords("what   is a vector?"),
        DesignIntent::Chat
    );
}

#[test]
fn classification_tag_parsing_matches_ts() {
    assert_eq!(parse_classified("DESIGN_MODIFY"), DesignIntent::Modify);
    assert_eq!(parse_classified("  design_modify  "), DesignIntent::Modify);
    assert_eq!(parse_classified("DESIGN_NEW"), DesignIntent::New);
    // Bare DESIGN counts as new (TS `upper.includes('DESIGN')`).
    assert_eq!(parse_classified("This is a DESIGN task"), DesignIntent::New);
    assert_eq!(parse_classified("CHAT"), DesignIntent::Chat);
    // Unknown / empty → chat: classifier failures must not mutate the canvas.
    assert_eq!(parse_classified("gibberish"), DesignIntent::Chat);
    assert_eq!(parse_classified(""), DesignIntent::Chat);
}

#[test]
fn retry_instruction_replays_the_last_user_request() {
    let history = vec![
        (
            ChatHistoryRole::User,
            "invert the activity list and remove notifications".into(),
        ),
        (
            ChatHistoryRole::Assistant,
            "error: no applicable edit was returned".into(),
        ),
        (ChatHistoryRole::User, "retry".into()),
        (
            ChatHistoryRole::Assistant,
            "error: no applicable edit was returned".into(),
        ),
    ];

    for retry in [
        "vuelve a intentar",
        "Intenta de nuevo!",
        "retry",
        "try again",
    ] {
        assert_eq!(
            resolve_retry_instruction(retry, &history),
            "invert the activity list and remove notifications"
        );
    }
}

#[test]
fn retry_instruction_without_prior_user_request_stays_unchanged() {
    let history = vec![(ChatHistoryRole::Assistant, "How can I help?".into())];
    assert_eq!(
        resolve_retry_instruction("vuelve a intentar", &history),
        "vuelve a intentar"
    );
    assert_eq!(
        resolve_retry_instruction("make the button red", &history),
        "make the button red"
    );
}

#[test]
fn retry_instruction_after_successful_turn_stays_conversational() {
    for assistant_reply in [
        "Here is the explanation you asked for.",
        "\n```json\n[{\"type\":\"frame\"}]\n```\n\n<!-- APPLIED -->",
    ] {
        let history = vec![
            (ChatHistoryRole::User, "tell me about frames".into()),
            (ChatHistoryRole::Assistant, assistant_reply.into()),
        ];
        assert_eq!(
            resolve_retry_instruction("try again", &history),
            "try again",
            "a successful chat or edit must not silently replay the prior request"
        );
    }
}

// ---------------------------------------------------------------------------
// Scripted provider
// ---------------------------------------------------------------------------

struct Scripted {
    deltas: Vec<ChatDelta>,
    delay: Duration,
}

impl Scripted {
    fn text(s: &str) -> Self {
        Self {
            deltas: vec![
                ChatDelta::TextDelta(s.to_string()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
            delay: Duration::ZERO,
        }
    }

    fn error(msg: &str) -> Self {
        Self {
            deltas: vec![
                ChatDelta::Error(msg.to_string()),
                ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                },
            ],
            delay: Duration::ZERO,
        }
    }

    fn slow(s: &str, delay: Duration) -> Self {
        let mut p = Self::text(s);
        p.delay = delay;
        p
    }
}

impl ChatProvider for Scripted {
    fn provider_label(&self) -> &str {
        "scripted"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let deltas = self.deltas.clone();
        let delay = self.delay;
        Box::new(deltas.into_iter().inspect(move |_| {
            std::thread::sleep(delay);
        }))
    }
}

struct ScriptedSequence {
    responses: Mutex<VecDeque<Vec<ChatDelta>>>,
    requests: Mutex<Vec<ChatRequest>>,
}

impl ScriptedSequence {
    fn text(responses: &[&str]) -> Self {
        Self {
            responses: Mutex::new(responses.iter().map(|s| scripted_text_deltas(s)).collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn error(message: &str) -> Self {
        Self {
            responses: Mutex::new(VecDeque::from([vec![
                ChatDelta::Error(message.into()),
                ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                },
            ]])),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn aborted() -> Self {
        Self {
            responses: Mutex::new(VecDeque::from([vec![ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            }]])),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn requests(&self) -> Vec<ChatRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl ChatProvider for ScriptedSequence {
    fn provider_label(&self) -> &str {
        "scripted-sequence"
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.requests.lock().unwrap().push(request);
        let deltas = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("scripted response for provider call");
        Box::new(deltas.into_iter())
    }
}

fn scripted_text_deltas(s: &str) -> Vec<ChatDelta> {
    vec![
        ChatDelta::TextDelta(s.to_string()),
        ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        },
    ]
}

// ---------------------------------------------------------------------------
// LLM classification
// ---------------------------------------------------------------------------

#[test]
fn llm_classifier_parses_provider_reply() {
    let provider = Scripted::text("DESIGN_MODIFY");
    assert_eq!(
        classify_intent_llm(&provider, "make it red", None),
        DesignIntent::Modify
    );
    let provider = Scripted::text("CHAT");
    assert_eq!(
        classify_intent_llm(&provider, "what is a frame", None),
        DesignIntent::Chat
    );
}

#[test]
fn llm_classifier_falls_back_to_chat_on_error() {
    let provider = Scripted::error("boom");
    assert_eq!(
        classify_intent_llm(&provider, "anything", None),
        DesignIntent::Chat
    );
}

#[test]
fn llm_classifier_falls_back_to_chat_on_timeout() {
    let provider = Scripted::slow("CHAT", Duration::from_millis(300));
    let got =
        classify_intent_llm_with_timeout(&provider, "anything", None, Duration::from_millis(30));
    assert_eq!(got, DesignIntent::Chat);
}

#[test]
fn punctuation_only_stays_in_chat() {
    let provider = Scripted::text("DESIGN_NEW");
    assert_eq!(
        classify_intent_for_standard_route(&provider, &EditorState::new(), "？？？？", None,),
        DesignIntent::Chat
    );
}

// ---------------------------------------------------------------------------
// Fixture nodes
// ---------------------------------------------------------------------------

/// Build a canonical frame node from JSON — immune to schema field
/// growth, and exactly the shape `parse_nodes` consumes.
fn frame(id: &str, name: &str, width: f64, children: Vec<PenNode>) -> PenNode {
    let mut node: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": id,
        "name": name,
        "x": 0.0,
        "y": 0.0,
        "width": width,
        "height": 800.0,
        "children": [],
    }))
    .expect("valid frame json");
    if let Some(kids) = node.children_mut() {
        *kids = children;
    }
    node
}

fn rect(id: &str, name: &str) -> PenNode {
    serde_json::from_value(serde_json::json!({
        "type": "rectangle",
        "id": id,
        "name": name,
        "x": 0.0,
        "y": 0.0,
        "width": 120.0,
        "height": 40.0,
    }))
    .expect("valid rectangle json")
}

fn image(id: &str, name: &str, src: &str) -> PenNode {
    serde_json::from_value(serde_json::json!({
        "type": "image",
        "id": id,
        "name": name,
        "src": src,
        "x": 0.0,
        "y": 0.0,
        "width": 120.0,
        "height": 80.0,
    }))
    .expect("valid image json")
}

/// Page frame with a status bar + two content sections.
fn state_with_page() -> EditorState {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "page-1",
        "Home",
        375.0,
        vec![
            frame("sb", "Status Bar", 375.0, vec![]),
            frame("hero", "Hero", 375.0, vec![]),
            frame("features", "Features", 375.0, vec![]),
        ],
    ));
    state
}

fn state_with_selected_page() -> EditorState {
    let mut state = state_with_page();
    state.set_single_selection(op_editor_core::NodeId::new("page-1"));
    state
}

fn count_node_id(nodes: &[PenNode], id: &str) -> usize {
    nodes
        .iter()
        .map(|node| {
            usize::from(node.id_str() == id)
                + node
                    .children()
                    .map(|kids| count_node_id(kids, id))
                    .unwrap_or(0)
        })
        .sum()
}

fn modify_op(parent: &str, node: serde_json::Value) -> (String, serde_json::Value) {
    (parent.to_string(), node)
}

fn apply_modify_ops_to_frame(
    state: &mut EditorState,
    nodes: &[DesignModificationOp],
    frame_id: &str,
) -> (usize, bool) {
    apply_design_modification(state, nodes, &[frame_id.to_string()])
}

fn modification_pairs_from_args(args_json: &str) -> Vec<(String, serde_json::Value)> {
    let value = serde_json::from_str::<serde_json::Value>(args_json).expect("valid apply args");
    serde_json::from_value(value.get("nodes").cloned().expect("nodes array"))
        .expect("nodes are serialized parent/object pairs")
}

// ---------------------------------------------------------------------------
// Append-intent detection
// ---------------------------------------------------------------------------

#[test]
fn append_intent_requires_a_keyword() {
    let state = state_with_selected_page();
    assert!(detect_append_intent(&state, "make a dashboard").is_none());
    assert!(detect_append_intent(&state, "").is_none());
}

#[test]
fn append_intent_detects_and_filters_status_bar() {
    let state = state_with_selected_page();
    let ctx = detect_append_intent(&state, "continue with a pricing section").expect("append");
    assert_eq!(ctx.target_parent_id, "page-1");
    assert_eq!(ctx.target_width, 375.0);
    assert_eq!(
        ctx.existing_section_labels,
        vec!["Hero".to_string(), "Features".to_string()],
        "status-bar sections are filtered from the labels"
    );
    assert!(ctx.is_mobile, "375px page is mobile (≤480)");
}

#[test]
fn append_intent_prefers_a_content_named_root() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "page-1",
        "Home",
        1200.0,
        vec![
            frame("sb", "Status Bar", 1200.0, vec![]),
            frame(
                "content-1",
                "Main Content",
                1100.0,
                vec![
                    frame("hero", "Hero", 1100.0, vec![]),
                    frame("about", "About", 1100.0, vec![]),
                ],
            ),
        ],
    ));
    state.set_single_selection(op_editor_core::NodeId::new("page-1"));
    let ctx = detect_append_intent(&state, "also add a testimonials section").expect("append");
    assert_eq!(ctx.target_parent_id, "content-1");
    assert_eq!(ctx.target_width, 1100.0);
    assert_eq!(
        ctx.existing_section_labels,
        vec!["Hero".to_string(), "About".to_string()]
    );
    assert!(!ctx.is_mobile);
}

#[test]
fn append_intent_vetoed_by_new_screen_phrases() {
    let state = state_with_selected_page();
    assert!(
        detect_append_intent(&state, "continue, but on a new page").is_none(),
        "new-screen phrasing suppresses append mode"
    );
    assert!(detect_append_intent(&state, "继续，做一个全新设计").is_none());
}

#[test]
fn append_intent_vetoed_by_named_follow_on_pages() {
    let state = state_with_selected_page();
    assert!(
        detect_append_intent(&state, "继续画出发现页").is_none(),
        "a named app page is a new sibling screen, not an appended section"
    );
    assert!(
        detect_append_intent(&state, "继续做订单页").is_none(),
        "tab/detail pages should not be inserted below the current page"
    );
    assert!(
        detect_append_intent(&state, "continue with a discover page").is_none(),
        "English named pages also suppress append mode"
    );
}

#[test]
fn append_intent_vetoed_by_whole_screen_draw_requests() {
    let state = state_with_selected_page();
    // Mixed-language "search 页面" (English noun + Chinese 页面) is NOT in the
    // named-screen vocabulary, yet drawing a whole page must become a new
    // sibling frame to the right, not rows appended below (user report).
    assert!(
        detect_append_intent(&state, "继续画一下search 页面").is_none(),
        "drawing a whole page is a new sibling screen, not an appended section"
    );
    // Generic (un-listed) screen names also become new screens.
    assert!(
        detect_append_intent(&state, "继续画一个登录页面").is_none(),
        "any '画...页面' request opens a new frame"
    );
    assert!(
        detect_append_intent(&state, "continue and draw an onboarding screen").is_none(),
        "English '... screen' draw requests open a new frame too"
    );
}

#[test]
fn append_intent_keeps_section_add_even_with_page_context() {
    let state = state_with_selected_page();
    // The page word is mere context; the unit added is a section, so this
    // still appends into the current screen.
    assert!(
        detect_append_intent(&state, "这个页面再加一个区块").is_some(),
        "adding a section to an existing page must keep appending"
    );
}

#[test]
fn whole_screen_draw_overrides_modify_in_standard_route() {
    // A draw request that ALSO trips the modify classifier ("修改后重新画一个
    // search 页面" has 修改 + 画 + 页面) must route to New, not Modify — else
    // it edits the existing frame instead of opening a new one.
    let provider = Scripted::text("DESIGN_MODIFY");
    let state = state_with_page();
    assert_eq!(
        classify_intent_for_standard_route(&provider, &state, "修改后重新画一个search 页面", None),
        DesignIntent::New,
        "a whole-screen draw must win over the modify classifier"
    );
    // But a genuine edit of the current screen still routes to Modify.
    assert_eq!(
        classify_intent_for_standard_route(&provider, &state, "把这个页面改成深色", None),
        DesignIntent::Modify,
        "editing the current screen stays on the modify route"
    );
}

#[test]
fn english_page_edits_stay_on_modify_route() {
    // An English EDIT that merely mentions a page/screen must NOT be mistaken
    // for a new-screen draw — it needs a creation verb (draw/create/design/…),
    // which "change"/"resize"/"make" are not.
    let provider = Scripted::text("CHAT");
    let state = state_with_page();
    assert_eq!(
        classify_intent_for_standard_route(&provider, &state, "change the home page layout", None),
        DesignIntent::Modify,
        "editing an existing page is a modify, not a new design"
    );
    assert_eq!(
        classify_intent_for_standard_route(&provider, &state, "resize the screen header", None),
        DesignIntent::Modify,
    );
    // A genuine English page DRAW still routes to New.
    assert_eq!(
        classify_intent_for_standard_route(&provider, &state, "draw a checkout page", None),
        DesignIntent::New,
        "a creation verb + page noun is a new screen"
    );
    // And the bare-noun edit must not be flagged as a whole-screen request.
    assert!(!requests_new_whole_screen("change the home page layout"));
    assert!(requests_new_whole_screen("draw a checkout page"));
}

#[test]
fn english_determiner_decides_new_vs_existing_screen() {
    // "make a login page" — indefinite article ⇒ a NEW page, even though the
    // ambiguous verb "make" is not in the creation-verb list.
    assert!(requests_new_whole_screen("make a login page"));
    assert!(requests_new_whole_screen("create a settings screen"));
    assert!(requests_new_whole_screen("add another profile page"));
    // "create a button on the login page" — the page is the LOCATION (definite
    // article), the object made is a button ⇒ NOT a new screen.
    assert!(!requests_new_whole_screen(
        "create a button on the login page"
    ));
    assert!(!requests_new_whole_screen("build out the rest of the page"));
    assert!(!requests_new_whole_screen("tweak this screen"));
}

#[test]
fn whole_screen_draw_requests_llm_design_md_extraction() {
    let state = state_with_page();
    assert!(
        should_auto_generate_design_md(&state, "继续画一下search 页面", None),
        "a new whole screen should inherit the canvas design system for consistency"
    );
}

#[test]
fn named_follow_on_page_forces_new_route_before_llm_classifier() {
    let provider = Scripted::text("CHAT");
    let state = state_with_page();
    assert_eq!(
        classify_intent_for_standard_route(&provider, &state, "继续画出发现页", None),
        DesignIntent::New,
        "named app pages must not be classified as plain chat or append"
    );
}

#[test]
fn append_intent_needs_existing_content() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    // Page frame whose only child is a status bar — no real content.
    state.active_children_mut().push(frame(
        "page-1",
        "Home",
        375.0,
        vec![frame("sb", "status_bar", 375.0, vec![])],
    ));
    state.set_single_selection(op_editor_core::NodeId::new("page-1"));
    assert!(detect_append_intent(&state, "continue the design").is_none());
    // No page frame at all.
    let mut empty = EditorState::new();
    empty.active_children_mut().clear();
    assert!(detect_append_intent(&empty, "continue the design").is_none());
}

#[test]
fn append_intent_matches_cjk_phrases() {
    let state = state_with_selected_page();
    assert!(detect_append_intent(&state, "再加一个定价区块").is_some());
    assert!(detect_append_intent(&state, "接着补充内容").is_some());
}

#[test]
fn append_intent_requires_one_selected_frame_and_uses_that_frame() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "first",
        "First",
        375.0,
        vec![frame("first-hero", "Hero", 375.0, vec![])],
    ));
    state.active_children_mut().push(frame(
        "second",
        "Second",
        1200.0,
        vec![frame("second-section", "Second Section", 1200.0, vec![])],
    ));
    state
        .active_children_mut()
        .push(rect("loose", "Loose Rectangle"));

    assert!(
        detect_append_intent(&state, "continue the design").is_none(),
        "continue alone must not authorize writing into an existing frame"
    );

    state.set_single_selection(op_editor_core::NodeId::new("loose"));
    assert!(
        detect_append_intent(&state, "continue the design").is_none(),
        "a non-frame selection must not authorize append mode"
    );

    state.set_single_selection(op_editor_core::NodeId::new("second"));
    let ctx = detect_append_intent(&state, "continue the design").expect("selected frame");
    assert_eq!(ctx.target_parent_id, "second");
    assert_eq!(ctx.target_width, 1200.0);
    assert_eq!(
        ctx.existing_section_labels,
        vec!["Second Section".to_string()]
    );
    assert!(!ctx.is_mobile);
}

#[test]
fn status_bar_matcher_covers_separator_runs() {
    assert!(is_status_bar_like_text("Status Bar"));
    assert!(is_status_bar_like_text("status_bar"));
    assert!(is_status_bar_like_text("status--bar"));
    assert!(is_status_bar_like_text("System  Chrome"));
    assert!(is_status_bar_like_text("状态栏"));
    assert!(!is_status_bar_like_text("Status Mention"));
}

#[test]
fn follow_on_screen_requests_llm_design_md_extraction() {
    let state = state_with_page();
    assert!(
        should_auto_generate_design_md(&state, "继续画出发现页", None),
        "named follow-on pages should extract design.md from the current canvas via LLM"
    );
}

#[test]
fn append_prompt_does_not_request_llm_design_md_extraction() {
    let state = state_with_page();
    let ctx = AppendContext {
        target_parent_id: "page-1".into(),
        target_width: 375.0,
        existing_section_labels: vec!["Hero".into()],
        is_mobile: true,
    };
    assert!(
        !should_auto_generate_design_md(&state, "继续补充内容", Some(&ctx)),
        "append mode already has append context and should not be treated as a new screen"
    );
}

#[test]
fn existing_design_md_skips_llm_extraction() {
    let mut state = state_with_page();
    state.doc.design_md = Some(op_editor_core::parse_design_md(
        "# Design System: Existing\n\n## 1. Visual Theme & Atmosphere\nReuse me.",
    ));
    assert!(
        !should_auto_generate_design_md(&state, "继续画出发现页", None),
        "a document-bound design.md should be reused directly"
    );
}

// ---------------------------------------------------------------------------
// Variable context + modify plan
// ---------------------------------------------------------------------------

#[path = "chat_intent_modify_plan_tests.rs"]
mod modify_plan_tests;

#[path = "chat_intent_modify_turn_tests.rs"]
mod modify_turn_tests;
