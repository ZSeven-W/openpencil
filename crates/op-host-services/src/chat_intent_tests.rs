//! Tests for the CLI standard-mode intent router (GAP #33).

use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use op_ai::chat_provider::{ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest, StopReason};
use op_editor_core::EditorState;
use op_orchestrator::DesignRequest;

use super::*;
use crate::chat_canvas_tools::{apply_design_modification, chat_tool_channel};

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
    // Unknown / empty → new.
    assert_eq!(parse_classified("gibberish"), DesignIntent::New);
    assert_eq!(parse_classified(""), DesignIntent::New);
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
fn llm_classifier_falls_back_to_new_on_error() {
    // TS: classify failure → { intent: 'new' }.
    let provider = Scripted::error("boom");
    assert_eq!(
        classify_intent_llm(&provider, "anything", None),
        DesignIntent::New
    );
}

#[test]
fn llm_classifier_falls_back_to_new_on_timeout() {
    let provider = Scripted::slow("CHAT", Duration::from_millis(300));
    let got =
        classify_intent_llm_with_timeout(&provider, "anything", None, Duration::from_millis(30));
    assert_eq!(got, DesignIntent::New, "timeout mirrors the TS abort → new");
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
    let state = state_with_page();
    assert!(detect_append_intent(&state, "make a dashboard").is_none());
    assert!(detect_append_intent(&state, "").is_none());
}

#[test]
fn append_intent_detects_and_filters_status_bar() {
    let state = state_with_page();
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
    let state = state_with_page();
    assert!(
        detect_append_intent(&state, "continue, but on a new page").is_none(),
        "new-screen phrasing suppresses append mode"
    );
    assert!(detect_append_intent(&state, "继续，做一个全新设计").is_none());
}

#[test]
fn append_intent_vetoed_by_named_follow_on_pages() {
    let state = state_with_page();
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
    let state = state_with_page();
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
    let state = state_with_page();
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
    assert!(detect_append_intent(&state, "continue the design").is_none());
    // No page frame at all.
    let mut empty = EditorState::new();
    empty.active_children_mut().clear();
    assert!(detect_append_intent(&empty, "continue the design").is_none());
}

#[test]
fn append_intent_matches_cjk_phrases() {
    let state = state_with_page();
    assert!(detect_append_intent(&state, "再加一个定价区块").is_some());
    assert!(detect_append_intent(&state, "接着补充内容").is_some());
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

fn seed_variables(state: &mut EditorState) {
    use jian_ops_schema::variable::{
        ThemedValue, VariableDefinition, VariableKind, VariableScalar, VariableValue,
    };
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(
        "color-1".to_string(),
        VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#112233".into()),
                    theme: None,
                },
                ThemedValue {
                    value: VariableScalar::Str("#aabbcc".into()),
                    theme: None,
                },
            ]),
        },
    );
    vars.insert(
        "spacing-1".to_string(),
        VariableDefinition {
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(8.0)),
        },
    );
    state.doc.variables = Some(vars);
    let mut themes = std::collections::BTreeMap::new();
    themes.insert("Theme-1".to_string(), vec!["Light".into(), "Dark".into()]);
    state.doc.themes = Some(themes);
}

#[test]
fn variable_context_matches_ts_format() {
    let mut state = EditorState::new();
    assert!(build_variable_context(&state).is_none());
    seed_variables(&mut state);
    let ctx = build_variable_context(&state).expect("variables present");
    assert!(ctx.starts_with(
        "DOCUMENT VARIABLES (use \"$name\" to reference, e.g. fill color \"$color-1\"):"
    ));
    assert!(ctx.contains("  - color-1 (color): #112233 [themed]"));
    assert!(ctx.contains("  - spacing-1 (number): 8"));
    assert!(ctx.contains("Themes: Theme-1: [Light, Dark]"));
}

#[test]
fn modify_plan_targets_selection_when_present() {
    let mut state = state_with_page();
    state.selection.set = vec![op_editor_core::NodeId::new("page-1")];
    state.selection.anchor = op_editor_core::NodeId::new("page-1");
    let plan = build_modify_plan(&state, "make it red").expect("plan");
    assert!(plan.user_message.starts_with("CONTEXT NODES:\n"));
    assert!(plan.user_message.contains("\"id\":\"page-1\""));
    assert!(plan.user_message.contains("\n\nINSTRUCTION:\nmake it red"));
}

#[test]
fn modify_plan_strips_base64_data_uris_from_context_nodes() {
    let image_data_uri = "data:image/png;base64,AAAABBBBCCCC";
    let fill_data_uri = "data:image/jpeg;base64,DDDDEEEEFFFF";
    let mut image_fill_rect: PenNode = serde_json::from_value(serde_json::json!({
        "type": "rectangle",
        "id": "fill-card",
        "name": "Image Fill Card",
        "x": 0.0,
        "y": 100.0,
        "width": 120.0,
        "height": 80.0,
        "fill": [
            { "type": "image", "url": fill_data_uri, "mode": "crop" },
            { "type": "solid", "color": "$color-1" }
        ],
    }))
    .expect("valid rectangle with image fill json");
    image_fill_rect.base_mut().explain = Some("keep metadata".into());

    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "page-1",
        "Home",
        375.0,
        vec![
            image("hero-photo", "Hero Photo", image_data_uri),
            image_fill_rect,
        ],
    ));

    let plan = build_modify_plan(&state, "make it warmer").expect("plan");
    let context_json = plan
        .user_message
        .strip_prefix("CONTEXT NODES:\n")
        .and_then(|rest| rest.split_once("\n\nINSTRUCTION:\n").map(|(ctx, _)| ctx))
        .expect("context section");
    let context: serde_json::Value =
        serde_json::from_str(context_json).expect("valid context json");

    assert_eq!(
        context
            .pointer("/0/children/0/src")
            .and_then(|v| v.as_str()),
        Some("<image>")
    );
    assert_eq!(
        context
            .pointer("/0/children/1/fill/0/url")
            .and_then(|v| v.as_str()),
        Some("<image>")
    );
    assert_eq!(
        context
            .pointer("/0/children/1/fill/1/color")
            .and_then(|v| v.as_str()),
        Some("$color-1")
    );
    assert_eq!(
        context
            .pointer("/0/children/1/explain")
            .and_then(|v| v.as_str()),
        Some("keep metadata")
    );
    assert!(
        !plan.user_message.contains("AAAABBBBCCCC") && !plan.user_message.contains("DDDDEEEEFFFF"),
        "base64 blobs must not be sent to the model: {}",
        plan.user_message
    );
}

#[test]
fn modify_plan_falls_back_to_last_frame_then_last_child() {
    // No selection → last top-level frame.
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(frame("f1", "One", 375.0, vec![]));
    state
        .active_children_mut()
        .push(frame("f2", "Two", 375.0, vec![]));
    state.active_children_mut().push(rect("r1", "Loose"));
    let plan = build_modify_plan(&state, "tweak").expect("plan");
    assert!(plan.user_message.contains("\"id\":\"f2\""));
    assert!(!plan.user_message.contains("\"id\":\"f1\""));

    // No frames at all → last child.
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(rect("r1", "Only"));
    let plan = build_modify_plan(&state, "tweak").expect("plan");
    assert!(plan.user_message.contains("\"id\":\"r1\""));
}

#[test]
fn modify_plan_appends_variable_context() {
    let mut state = state_with_page();
    seed_variables(&mut state);
    let plan = build_modify_plan(&state, "recolor with variables").expect("plan");
    assert!(plan.user_message.contains("DOCUMENT VARIABLES"));
    assert!(
        !plan.system_prompt.is_empty(),
        "maintenance skills resolve into the system prompt"
    );
}

#[test]
fn modify_plan_is_none_for_an_empty_page() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    assert!(build_modify_plan(&state, "tweak").is_none());
}

// ---------------------------------------------------------------------------
// apply_design_modification (extractAndApplyDesignModification port)
// ---------------------------------------------------------------------------

#[test]
fn apply_modification_replaces_existing_and_inserts_unknown_top_level() {
    let mut state = state_with_page();
    let nodes = vec![
        // Existing id -> whole-node replacement.
        modify_op(
            "null",
            serde_json::json!({
                "id": "hero",
                "type": "frame",
                "name": "Hero Updated",
                "width": 375.0,
                "height": 200.0,
                "children": []
            }),
        ),
        // Unknown id → insert under the primary frame (canonical
        // TextNode carries `content`).
        modify_op(
            "null",
            serde_json::json!({
                "id": "fresh-1",
                "type": "text",
                "name": "New Caption",
                "content": "Hello",
            }),
        ),
    ];
    let (count, mutated) = apply_design_modification(&mut state, &nodes);
    assert_eq!(count, 2, "replace existing plus insert unknown top-level");
    assert!(mutated);
    let doc = serde_json::to_string(&state.doc).unwrap();
    assert!(doc.contains("Hero Updated"), "existing node is replaced");
    assert!(doc.contains("New Caption"), "new node inserted");
    assert_eq!(count_node_id(state.active_children(), "hero"), 1);
    // The insert landed inside the page frame, not at the page root.
    let page = state
        .active_children()
        .iter()
        .find(|n| n.id_str() == "page-1")
        .unwrap();
    let kids = page.children().unwrap();
    assert!(
        kids.iter().any(|k| k.id_str() == "hero"
            && k.base()
                .name
                .as_deref()
                .is_some_and(|n| n == "Hero Updated")),
        "existing node remains in the primary frame"
    );
    assert!(
        kids.iter()
            .any(|k| k.base().name.as_deref().is_some_and(|n| n == "New Caption")),
        "implied-new node parents to the active page's primary frame"
    );
}

#[test]
fn apply_modification_adds_under_declared_existing_parent_without_touching_siblings() {
    use op_editor_core::{walkers::find_node, NodeId};

    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "n217",
        "Player",
        320.0,
        vec![rect("n218", "Track Info"), rect("n220", "Actions")],
    ));
    let before_parent = find_node(state.active_children(), &NodeId::new("n217")).unwrap();
    let before_children = before_parent.children().unwrap();
    let before_n218 = serde_json::to_value(&before_children[0]).unwrap();
    let before_n220 = serde_json::to_value(&before_children[1]).unwrap();

    let nodes = vec![modify_op(
        "n217",
        serde_json::json!({
            "type": "frame",
            "name": "Progress Bar",
            "width": 220.0,
            "height": 8.0,
            "children": []
        }),
    )];

    let (count, mutated) = apply_design_modification(&mut state, &nodes);

    assert_eq!(count, 1);
    assert!(mutated);
    let parent = find_node(state.active_children(), &NodeId::new("n217")).unwrap();
    assert_eq!(parent.base().name.as_deref(), Some("Player"));
    let children = parent.children().unwrap();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].id_str(), "n218");
    assert_eq!(children[1].id_str(), "n220");
    assert_eq!(children[2].base().name.as_deref(), Some("Progress Bar"));
    assert_eq!(serde_json::to_value(&children[0]).unwrap(), before_n218);
    assert_eq!(serde_json::to_value(&children[1]).unwrap(), before_n220);
}

#[test]
fn apply_modification_inserts_idless_null_parent_under_primary_frame() {
    let mut state = state_with_page();
    let nodes = vec![modify_op(
        "null",
        serde_json::json!({
            "type": "text",
            "name": "Loose Label",
            "content": "Hello"
        }),
    )];

    let (count, mutated) = apply_design_modification(&mut state, &nodes);

    assert_eq!(count, 1);
    assert!(mutated);
    let page = state
        .active_children()
        .iter()
        .find(|n| n.id_str() == "page-1")
        .unwrap();
    let kids = page.children().unwrap();
    assert!(
        kids.iter()
            .any(|k| k.base().name.as_deref().is_some_and(|n| n == "Loose Label")),
        "idless null-parent node inserts under the active page primary frame"
    );
}

// ---------------------------------------------------------------------------
// run_cli_turn routing
// ---------------------------------------------------------------------------

fn test_design_request() -> DesignRequest {
    DesignRequest {
        prompt: "p".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
    }
}

fn drain_chat(rx: &mpsc::Receiver<ChatDelta>) -> Vec<ChatDelta> {
    let mut out = Vec::new();
    while let Ok(delta) = rx.recv_timeout(Duration::from_secs(10)) {
        let done = matches!(delta, ChatDelta::Done { .. });
        out.push(delta);
        if done {
            break;
        }
    }
    out
}

fn run_modify_turn_with_apply(
    response: &str,
) -> (
    Vec<ChatDelta>,
    EditorState,
    Vec<(String, serde_json::Value)>,
) {
    let provider = Scripted::text(response);
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, tool_rx) = chat_tool_channel();
    let worker = std::thread::spawn(move || {
        run_modify_turn(&provider, ChatRequest::default(), &chat_tx, &executor);
    });

    let req = tool_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("modify route forwards the apply op");
    assert_eq!(req.name, APPLY_MODIFICATION_OP);
    let nodes = modification_pairs_from_args(&req.args_json);
    let mut state = state_with_page();
    let (count, mutated) = apply_design_modification(&mut state, &nodes);
    assert_eq!(count, 1);
    assert!(mutated);
    req.ack
        .send(op_ai::chat_provider::ChatToolResult {
            content: serde_json::json!({ "success": true, "count": count }).to_string(),
            is_error: false,
        })
        .unwrap();

    let deltas = drain_chat(&chat_rx);
    worker.join().unwrap();
    (deltas, state, nodes)
}

fn run_modify_turn_with_sequence_apply(
    provider: Arc<ScriptedSequence>,
    request: ChatRequest,
) -> (
    Vec<ChatDelta>,
    EditorState,
    Vec<(String, serde_json::Value)>,
) {
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, tool_rx) = chat_tool_channel();
    let worker_provider = Arc::clone(&provider);
    let worker = std::thread::spawn(move || {
        run_modify_turn(worker_provider.as_ref(), request, &chat_tx, &executor);
    });

    let req = tool_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("modify route forwards the apply op");
    assert_eq!(req.name, APPLY_MODIFICATION_OP);
    let nodes = modification_pairs_from_args(&req.args_json);
    let mut state = state_with_page();
    let (count, mutated) = apply_design_modification(&mut state, &nodes);
    assert_eq!(count, 1);
    assert!(mutated);
    req.ack
        .send(op_ai::chat_provider::ChatToolResult {
            content: serde_json::json!({ "success": true, "count": count }).to_string(),
            is_error: false,
        })
        .unwrap();

    let deltas = drain_chat(&chat_rx);
    worker.join().unwrap();
    (deltas, state, nodes)
}

fn retry_test_request() -> ChatRequest {
    ChatRequest {
        system_prompt: "base modify system prompt".into(),
        user_message: "CONTEXT NODES: []\n\nINSTRUCTION: change the hero".into(),
        history: vec![
            (ChatHistoryRole::User, "previous user turn".into()),
            (ChatHistoryRole::Assistant, "previous assistant turn".into()),
        ],
        max_output_tokens: 1234,
        model: Some("glm-test-model".into()),
        ..ChatRequest::default()
    }
}

fn expected_retry_request(mut request: ChatRequest) -> ChatRequest {
    request.system_prompt.push_str(
        "\n\nCRITICAL: Respond with ONLY I(...) JavaScript statements -- never prose, explanations, or numbered/bulleted lists. If you truly cannot make the change, return an empty program.",
    );
    request
}

fn text_delta_count(deltas: &[ChatDelta], needle: &str) -> usize {
    deltas
        .iter()
        .filter(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains(needle)))
        .count()
}

fn expected_applied_json_delta(nodes: &[(String, serde_json::Value)]) -> ChatDelta {
    let node_values = nodes
        .iter()
        .map(|(_, node)| node.clone())
        .collect::<Vec<_>>();
    let json = serde_json::to_string_pretty(&node_values).unwrap();
    ChatDelta::TextDelta(format!("\n```json\n{json}\n```"))
}

#[test]
fn run_modify_turn_script_response_applies_nodes_and_marks_applied() {
    let response = r##"
        I(null, {
            id:"hero",
            type:"frame",
            name:"Hero Rewritten",
            children:[{type:"text", name:"Progress Label", content:"0:42"}]
        });
    "##;

    let (deltas, state, nodes) = run_modify_turn_with_apply(response);

    assert_eq!(nodes[0].0, "null");
    assert_eq!(nodes[0].1["id"], serde_json::json!("hero"));
    assert_eq!(
        deltas[0],
        ChatDelta::TextDelta(
            r#"<step title="Checking guidelines">Analyzing modification request...</step>"#.into()
        )
    );
    assert_eq!(deltas[1], expected_applied_json_delta(&nodes));
    assert_eq!(
        deltas[2],
        ChatDelta::TextDelta("\n\n<!-- APPLIED -->".into())
    );
    assert!(matches!(deltas[3], ChatDelta::Done { .. }));
    let transcript_text = deltas
        .iter()
        .filter_map(|delta| match delta {
            ChatDelta::TextDelta(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert!(!transcript_text.contains("I(null"));
    let doc = serde_json::to_string(&state.doc).unwrap();
    assert!(doc.contains("Progress Label"));
    assert!(doc.contains("Hero Rewritten"));
    assert_eq!(count_node_id(state.active_children(), "hero"), 1);
}

#[test]
fn run_modify_turn_retries_prose_once_then_applies_script() {
    let prose = "I can change the hero by making it clearer and more direct.";
    let response = r##"
        I(null, {
            id:"hero",
            type:"frame",
            name:"Hero Retry Applied",
            children:[{type:"text", name:"Retry Label", content:"Applied"}]
        });
    "##;
    let provider = Arc::new(ScriptedSequence::text(&[prose, response]));
    let request = retry_test_request();

    let (deltas, state, nodes) =
        run_modify_turn_with_sequence_apply(Arc::clone(&provider), request.clone());

    let requests = provider.requests();
    assert_eq!(
        requests.len(),
        2,
        "empty first parse gets exactly one retry"
    );
    assert_eq!(requests[0], request);
    assert_eq!(requests[1], expected_retry_request(request));
    assert_eq!(nodes[0].0, "null");
    assert_eq!(nodes[0].1["id"], serde_json::json!("hero"));
    assert_eq!(
        text_delta_count(&deltas, MODIFY_STEP),
        1,
        "retry must not stack a second modify progress step"
    );
    assert_eq!(
        text_delta_count(&deltas, prose),
        0,
        "discarded first prose attempt must stay out of the transcript"
    );
    assert!(
        deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("<!-- APPLIED -->"))),
        "successful retry must use the normal applied marker"
    );
    assert!(
        !deltas.iter().any(|d| matches!(d, ChatDelta::Error(_))),
        "successful retry must not emit the friendly recovery error"
    );
    let doc = serde_json::to_string(&state.doc).unwrap();
    assert!(doc.contains("Hero Retry Applied"));
    assert!(doc.contains("Retry Label"));
}

#[test]
fn run_modify_turn_retries_prose_once_then_surfaces_friendly_recovery_error() {
    let prose_1 = "I would make the selected card red.";
    let prose_2 = "Here are the changes I would make: use a stronger accent color.";
    let provider = Arc::new(ScriptedSequence::text(&[prose_1, prose_2]));
    let request = retry_test_request();
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, tool_rx) = chat_tool_channel();

    run_modify_turn(provider.as_ref(), request.clone(), &chat_tx, &executor);

    assert!(
        tool_rx.try_recv().is_err(),
        "double-prose responses must not dispatch an apply op"
    );
    let deltas = drain_chat(&chat_rx);
    let requests = provider.requests();
    assert_eq!(requests.len(), 2, "retry is capped at one extra call");
    assert_eq!(requests[0], request);
    assert_eq!(requests[1], expected_retry_request(request));
    assert_eq!(
        text_delta_count(&deltas, MODIFY_STEP),
        1,
        "retry must not stack a second modify progress step"
    );
    assert_eq!(text_delta_count(&deltas, prose_1), 0);
    assert_eq!(text_delta_count(&deltas, prose_2), 0);
    let errors: Vec<_> = deltas
        .iter()
        .filter_map(|d| match d {
            ChatDelta::Error(msg) => Some(msg.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        errors,
        vec![
            "The model returned a description instead of an applyable edit. Try rephrasing (e.g. name the element to add) or run it again."
        ]
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::Aborted
        })
    ));
}

#[test]
fn run_modify_turn_script_response_does_not_retry() {
    let response = r##"
        I(null, {
            id:"hero",
            type:"frame",
            name:"Hero First Attempt",
            children:[{type:"text", name:"First Attempt Label", content:"Applied"}]
        });
    "##;
    let provider = Arc::new(ScriptedSequence::text(&[
        response,
        "this second response must never be requested",
    ]));
    let request = retry_test_request();

    let (deltas, state, nodes) =
        run_modify_turn_with_sequence_apply(Arc::clone(&provider), request.clone());

    let requests = provider.requests();
    assert_eq!(requests.len(), 1, "valid first script must not retry");
    assert_eq!(requests[0], request);
    assert_eq!(nodes[0].1["id"], serde_json::json!("hero"));
    assert_eq!(text_delta_count(&deltas, MODIFY_STEP), 1);
    assert!(deltas
        .iter()
        .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("<!-- APPLIED -->"))));
    let doc = serde_json::to_string(&state.doc).unwrap();
    assert!(doc.contains("Hero First Attempt"));
    assert!(doc.contains("First Attempt Label"));
}

#[test]
fn run_modify_turn_flat_json_response_still_applies_via_fallback() {
    let response = r##"[{"id":"flat-new","type":"text","name":"Flat Caption","content":"Hello"}]"##;

    let (deltas, state, nodes) = run_modify_turn_with_apply(response);

    assert_eq!(nodes[0].0, "null");
    assert_eq!(deltas[1], expected_applied_json_delta(&nodes));
    assert!(
        deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("<!-- APPLIED -->"))),
        "modify route must emit the applied marker"
    );
    let doc = serde_json::to_string(&state.doc).unwrap();
    assert!(doc.contains("Flat Caption"));
}

#[test]
fn run_modify_turn_prose_response_surfaces_friendly_recovery_error() {
    let provider = Scripted::text("sorry, I cannot help with that");
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, tool_rx) = chat_tool_channel();

    run_modify_turn(&provider, ChatRequest::default(), &chat_tx, &executor);

    assert!(
        tool_rx.try_recv().is_err(),
        "prose responses must not dispatch an apply op"
    );
    let error = drain_chat(&chat_rx)
        .into_iter()
        .find_map(|d| match d {
            ChatDelta::Error(msg) => Some(msg),
            _ => None,
        })
        .expect("parse failure surfaces an error");
    assert_eq!(
        error,
        "The model returned a description instead of an applyable edit. Try rephrasing (e.g. name the element to add) or run it again."
    );
}

#[test]
fn cli_turn_chat_route_streams_provider_deltas() {
    let plan = CliTurnPlan {
        user_text: "what is a frame?".into(),
        page_children_empty: false,
        classify_provider: Box::new(Scripted::text("CHAT")),
        chat_provider: Box::new(Scripted::text("a frame is a container")),
        design_provider: Box::new(Scripted::text("unused")),
        chat_request: ChatRequest::default(),
        modify_request: Some(ChatRequest::default()),
        design_request: test_design_request(),
        initial_state: EditorState::new(),
        indicator_epoch: 0,
        model: None,
    };
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, _tool_rx) = chat_tool_channel();
    let (delta_tx, delta_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let worker =
        std::thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));
    let deltas = drain_chat(&chat_rx);
    worker.join().unwrap();
    assert_eq!(
        deltas,
        vec![
            ChatDelta::TextDelta("a frame is a container".into()),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            },
        ]
    );
    // Design channels were dropped — the design pump would retire.
    assert!(delta_rx.recv().is_err());
    assert!(cmd_rx.recv().is_err());
}

#[test]
fn cli_turn_modify_route_applies_nodes_and_marks_applied() {
    let response = r##"
        I("hero", {type:"text", name:"CLI Caption", content:"Added"});
    "##;
    let plan = CliTurnPlan {
        user_text: "add a caption".into(),
        page_children_empty: false,
        classify_provider: Box::new(Scripted::text("DESIGN_MODIFY")),
        chat_provider: Box::new(Scripted::text("unused")),
        design_provider: Box::new(Scripted::text(response)),
        chat_request: ChatRequest::default(),
        modify_request: Some(ChatRequest::default()),
        design_request: test_design_request(),
        initial_state: EditorState::new(),
        indicator_epoch: 0,
        model: None,
    };
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, tool_rx) = chat_tool_channel();
    let (delta_tx, delta_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let worker =
        std::thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));

    // Act as the UI pump: execute the internal apply op.
    let req = tool_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("modify route forwards the apply op");
    assert_eq!(req.name, APPLY_MODIFICATION_OP);
    let nodes = modification_pairs_from_args(&req.args_json);
    let mut state = state_with_page();
    let (count, mutated) = apply_design_modification(&mut state, &nodes);
    assert_eq!(count, 1);
    assert!(mutated);
    req.ack
        .send(op_ai::chat_provider::ChatToolResult {
            content: serde_json::json!({ "success": true, "count": count }).to_string(),
            is_error: false,
        })
        .unwrap();

    let deltas = drain_chat(&chat_rx);
    worker.join().unwrap();
    // Step → fenced design JSON → APPLIED marker → Done.
    assert_eq!(
        deltas[0],
        ChatDelta::TextDelta(
            r#"<step title="Checking guidelines">Analyzing modification request...</step>"#.into()
        )
    );
    assert_eq!(deltas[1], expected_applied_json_delta(&nodes));
    assert_eq!(
        deltas[2],
        ChatDelta::TextDelta("\n\n<!-- APPLIED -->".into())
    );
    assert!(matches!(deltas[3], ChatDelta::Done { .. }));
    assert_eq!(nodes[0].0, "hero");
    // The new node was inserted under the existing hero through the apply path.
    let doc = serde_json::to_string(&state.doc).unwrap();
    assert!(doc.contains("CLI Caption"));
    assert_eq!(count_node_id(state.active_children(), "hero"), 1);
    // Design channels dropped.
    assert!(delta_rx.recv().is_err());
    assert!(cmd_rx.recv().is_err());
}

#[test]
fn cli_turn_modify_keyword_overrides_new_classifier_reply() {
    let response = r##"[{"id":"hero","type":"frame","name":"Hero Dumpling"}]"##;
    let plan = CliTurnPlan {
        user_text: "修改成饺子".into(),
        page_children_empty: false,
        classify_provider: Box::new(Scripted::text("DESIGN_NEW")),
        chat_provider: Box::new(Scripted::text("unused")),
        design_provider: Box::new(Scripted::text(response)),
        chat_request: ChatRequest::default(),
        modify_request: Some(ChatRequest::default()),
        design_request: test_design_request(),
        initial_state: EditorState::new(),
        indicator_epoch: 0,
        model: None,
    };
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, tool_rx) = chat_tool_channel();
    let (delta_tx, delta_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let worker =
        std::thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));

    let req = tool_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("keyword modify should use the modify route even if the classifier says new");
    assert_eq!(req.name, APPLY_MODIFICATION_OP);
    req.ack
        .send(op_ai::chat_provider::ChatToolResult {
            content: serde_json::json!({ "success": true, "count": 1 }).to_string(),
            is_error: false,
        })
        .unwrap();

    let deltas = drain_chat(&chat_rx);
    worker.join().unwrap();
    assert!(
        deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("<!-- APPLIED -->"))),
        "modify route must emit the applied marker"
    );
    assert!(delta_rx.recv().is_err());
    assert!(cmd_rx.recv().is_err());
}

#[test]
fn cli_turn_modify_parse_failure_surfaces_friendly_recovery_error() {
    let plan = CliTurnPlan {
        user_text: "make the hero red".into(),
        page_children_empty: false,
        classify_provider: Box::new(Scripted::text("DESIGN_MODIFY")),
        chat_provider: Box::new(Scripted::text("unused")),
        design_provider: Box::new(Scripted::text("sorry, I cannot help with that")),
        chat_request: ChatRequest::default(),
        modify_request: Some(ChatRequest::default()),
        design_request: test_design_request(),
        initial_state: EditorState::new(),
        indicator_epoch: 0,
        model: None,
    };
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, _tool_rx) = chat_tool_channel();
    let (delta_tx, cmd_tx) = (mpsc::channel().0, mpsc::channel().0);
    let worker =
        std::thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));
    let deltas = drain_chat(&chat_rx);
    worker.join().unwrap();
    let error = deltas
        .iter()
        .find_map(|d| match d {
            ChatDelta::Error(msg) => Some(msg.clone()),
            _ => None,
        })
        .expect("parse failure surfaces an error");
    assert_eq!(
        error,
        "The model returned a description instead of an applyable edit. Try rephrasing (e.g. name the element to add) or run it again."
    );
}

#[test]
fn route_resolution_degrades_modify_like_ts() {
    use DesignIntent::*;
    // TS: modify on an empty page → new.
    assert_eq!(resolve_route(Modify, true, true), New);
    // Belt-and-braces: modify without a usable target plan → new.
    assert_eq!(resolve_route(Modify, false, false), New);
    // Healthy modify survives.
    assert_eq!(resolve_route(Modify, false, true), Modify);
    // Chat / new pass through untouched.
    assert_eq!(resolve_route(Chat, true, false), Chat);
    assert_eq!(resolve_route(New, false, true), New);
}
