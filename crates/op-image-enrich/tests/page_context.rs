use std::collections::HashSet;

use op_editor_core::EditorState;
use op_image_enrich::{collect_targets, compose_generation_prompt, ImageSearchTarget};
use serde_json::{json, Value};

fn slot() -> Value {
    json!({"type":"image", "id":"slot", "src":"", "width":320, "height":240,
        "imageSearchQuery":"Velra Type", "imagePrompt":"Velra Type"})
}

fn text(id: &str, content: Value, size: f64) -> Value {
    json!({"type":"text", "id":id, "content":content, "fontSize":size})
}

fn frame(id: &str, name: &str, children: Vec<Value>) -> Value {
    json!({"type":"frame", "id":id, "name":name, "children":children})
}

fn state(children: Vec<Value>, name: Option<&str>) -> EditorState {
    EditorState::from_document(
        serde_json::from_value(json!({
            "version":"1.0", "name":name, "children":children
        }))
        .unwrap(),
    )
}

fn target(state: &EditorState) -> ImageSearchTarget {
    collect_targets(state, &HashSet::new())
        .into_iter()
        .find(|target| target.node_id.as_str() == "slot")
        .unwrap()
}

fn portfolio(title: &str) -> EditorState {
    state(
        vec![frame(
            "screen",
            "Studio Landing",
            vec![
                frame(
                    "status",
                    "Status Bar",
                    vec![text("time", json!("09:41 NOT ART DIRECTION"), 120.0)],
                ),
                frame(
                    "section",
                    "Selected Work",
                    vec![frame(
                        "card",
                        "Frame 12",
                        vec![
                            frame("layout", "Group", vec![slot()]),
                            text("subtitle", json!("Typography project"), 14.0),
                            text("title", json!(title), 32.0),
                        ],
                    )],
                ),
            ],
        )],
        Some("Brand Design Portfolio"),
    )
}

#[test]
fn portfolio_context_has_screen_section_and_nearest_card_title() {
    let target = target(&portfolio("Velra Type"));
    assert_eq!(
        target.page_context.as_deref(),
        Some("Studio Landing > Selected Work > Velra Type — Typography project > Brand Design Portfolio")
    );
    assert_eq!(target.query, "Velra Type");
    assert_eq!(target.prompt.as_deref(), Some("Velra Type"));
    let prompt = compose_generation_prompt(&target);
    assert_eq!(prompt, "Velra Type. Context: this image appears in Studio Landing > Selected Work > Velra Type — Typography project > Brand Design Portfolio. Match the page's subject and art direction. Use this context only for subject and mood; do not write its words into the image.");
    assert!(!prompt.contains("Frame 12"));
    assert!(!prompt.contains("Group"));
    assert!(!prompt.contains("09:41"));
}

#[test]
fn two_reported_project_names_get_page_art_direction() {
    for title in ["Velra Type", "Harbour Editions"] {
        let mut state = portfolio(title);
        fn rename_slot(nodes: &mut [jian_ops_schema::node::PenNode], title: &str) {
            use op_editor_core::PenNodeExt as _;
            for node in nodes {
                if let jian_ops_schema::node::PenNode::Image(image) = node {
                    image.image_prompt = Some(title.into());
                    image.image_search_query = Some(title.into());
                }
                if let Some(children) = node.children_mut() {
                    rename_slot(children, title);
                }
            }
        }
        rename_slot(state.active_children_mut(), title);
        let prompt = compose_generation_prompt(&target(&state));
        println!("{title}: {prompt}");
        assert!(prompt.starts_with(&format!("{title}. Context:")));
        assert!(prompt.contains("Brand Design Portfolio"));
    }
}

#[test]
fn naked_slot_preserves_prompt_bytes_and_blank_prompt_query_fallback() {
    let mut slot = slot();
    slot["imagePrompt"] = json!("  a vase.\n warm light  ");
    let mut target = target(&state(vec![slot], None));
    assert_eq!(target.page_context, None);
    assert_eq!(
        compose_generation_prompt(&target),
        "  a vase.\n warm light  "
    );
    target.prompt = Some(" \t\n".into());
    target.query = "  unchanged query\n".into();
    assert_eq!(compose_generation_prompt(&target), target.query);
    target.prompt = None;
    assert_eq!(compose_generation_prompt(&target), target.query);
    target.generation_subject = Some("A title without context must not replace the query".into());
    assert_eq!(compose_generation_prompt(&target), target.query);
}

#[test]
fn default_document_and_ancestor_names_do_not_create_context() {
    for name in [
        "Frame 12", "Frame12", "Group", "Group 4", "Page 1", "Untitled", "Document", "",
    ] {
        let state = state(
            vec![frame("root", name, vec![slot()])],
            Some("Untitled Document"),
        );
        assert_eq!(target(&state).page_context, None, "{name}");
    }
}

#[test]
fn descriptive_container_names_survive_and_whitespace_is_deduplicated() {
    let state = state(
        vec![frame(
            "screen",
            " Studio   Landing ",
            vec![frame(
                "section",
                "Selected Work Section",
                vec![frame(
                    "card",
                    "Brand Design Card",
                    vec![slot(), text("title", json!("Studio Landing"), 24.0)],
                )],
            )],
        )],
        Some(" studio landing "),
    );
    assert_eq!(
        target(&state).page_context.as_deref(),
        Some("Studio Landing > Selected Work Section > Brand Design Card")
    );
}

#[test]
fn screen_forest_and_active_page_never_borrow_other_screens() {
    let first = frame(
        "first",
        "Bakery",
        vec![text("bakery-title", json!("Bread"), 100.0)],
    );
    let second = frame("second", "Studio Landing", vec![slot()]);
    let state = EditorState::from_document(
        serde_json::from_value(json!({
            "version":"1.0", "children":[], "pages":[
                {"id":"active", "name":"Page 1", "children":[first, second]},
                {"id":"inactive", "name":"Travel", "children":[frame("third", "Airport", vec![])]}
            ]
        }))
        .unwrap(),
    );
    assert_eq!(
        target(&state).page_context.as_deref(),
        Some("Studio Landing")
    );
}

#[test]
fn nearest_title_beats_larger_section_and_cousin_card_headings() {
    let state = state(
        vec![frame(
            "screen",
            "Studio Landing",
            vec![
                text("hero", json!("Unrelated giant hero"), 120.0),
                frame(
                    "section",
                    "Selected Work",
                    vec![
                        frame(
                            "other",
                            "Other Project Card",
                            vec![
                                json!({"type":"image", "id":"other-image", "src":"filled.png"}),
                                text("other-title", json!("Other project"), 100.0),
                            ],
                        ),
                        frame(
                            "card",
                            "Frame 12",
                            vec![
                                slot(),
                                frame(
                                    "meta",
                                    "Group",
                                    vec![
                                        text(
                                            "title",
                                            json!([{ "text":"Velra ", "fontSize":24 }, { "text":"Type" }]),
                                            24.0,
                                        ),
                                        text("later", json!("Later tie"), 24.0),
                                    ],
                                ),
                            ],
                        ),
                    ],
                ),
            ],
        )],
        None,
    );
    assert_eq!(
        target(&state).page_context.as_deref(),
        Some("Studio Landing > Selected Work > Velra Type")
    );
}

#[test]
fn missing_card_title_falls_back_to_section_not_cousin_card() {
    let state = state(
        vec![frame(
            "screen",
            "Studio Landing",
            vec![frame(
                "section",
                "Selected Work",
                vec![
                    text("heading", json!("Brand Projects"), 24.0),
                    frame(
                        "other",
                        "Other Project Card",
                        vec![
                            json!({"type":"image", "id":"other-image", "src":"filled.png"}),
                            text("other-title", json!("Other project"), 100.0),
                        ],
                    ),
                    frame("card", "Group", vec![slot()]),
                ],
            )],
        )],
        None,
    );
    assert_eq!(
        target(&state).page_context.as_deref(),
        Some("Studio Landing > Selected Work > Brand Projects")
    );
}

#[test]
fn hidden_and_status_bar_subtrees_never_supply_titles() {
    let mut hidden = text("hidden", json!("Hidden title"), 120.0);
    hidden["visible"] = json!(false);
    let mut status = frame("status", "Group", vec![text("time", json!("09:41"), 120.0)]);
    status["role"] = json!("status-bar");
    let state = state(
        vec![frame(
            "screen",
            "Studio Landing",
            vec![slot(), hidden, status],
        )],
        None,
    );
    assert_eq!(
        target(&state).page_context.as_deref(),
        Some("Studio Landing")
    );
}

#[test]
fn target_in_status_bar_does_not_inherit_art_direction() {
    let state = state(
        vec![frame(
            "screen",
            "Studio Landing",
            vec![frame("status", "StatusBar", vec![slot()])],
        )],
        Some("Brand Design Portfolio"),
    );
    assert_eq!(target(&state).page_context, None);
}

#[test]
fn unicode_titles_and_total_context_are_bounded_without_losing_local_title() {
    let title = "字".repeat(100);
    let state = state(
        vec![frame(
            "screen",
            &"根".repeat(100),
            vec![frame(
                "section",
                &"节".repeat(100),
                vec![frame(
                    "card",
                    &"卡".repeat(100),
                    vec![slot(), text("title", json!(title), 24.0)],
                )],
            )],
        )],
        Some(&"文".repeat(100)),
    );
    let context = target(&state).page_context.unwrap();
    assert!(context.chars().count() <= 240);
    assert!(context.contains(&"字".repeat(60)));
    assert!(!context.contains(&"字".repeat(61)));
}

#[test]
fn ancestry_is_bounded_and_only_nearest_semantic_levels_survive() {
    let state = state(
        vec![frame(
            "screen",
            "Studio Landing",
            vec![frame(
                "old",
                "Distant Level",
                vec![frame(
                    "section",
                    "Selected Work",
                    vec![frame("card", "Project Card", vec![slot()])],
                )],
            )],
        )],
        None,
    );
    assert_eq!(
        target(&state).page_context.as_deref(),
        Some("Studio Landing > Selected Work > Project Card")
    );
}

fn velra(
    slot_name: &str,
    prompt: Option<&str>,
    query: Option<&str>,
    mut metadata: Vec<Value>,
) -> EditorState {
    let image = json!({"type":"image", "id":"slot", "name":slot_name, "src":"",
        "width":320, "height":240, "imagePrompt":prompt, "imageSearchQuery":query});
    let mut children = vec![
        frame("image-frame", "velra-image-frame", vec![image]),
        text("title", json!("Velra Type"), 32.0),
    ];
    children.append(&mut metadata);
    state(
        vec![frame(
            "screen",
            "Studio Portfolio — Landing",
            vec![frame("card", "card-velra", children)],
        )],
        None,
    )
}

fn velra_metadata() -> Vec<Value> {
    vec![
        text("subtitle", json!("Typeface microsite"), 16.0),
        text("year", json!("2024"), 22.0),
        text("tag", json!("DIGITAL"), 20.0),
    ]
}

#[test]
fn velra_name_fallback_uses_title_and_auxiliary_as_generation_subject() {
    let target = target(&velra("velra-image", None, None, velra_metadata()));
    assert_eq!(target.query, "velra-image");
    assert_eq!(target.prompt, None);
    assert_eq!(
        target.generation_subject.as_deref(),
        Some("Velra Type — Typeface microsite")
    );
    let expected = "Velra Type — Typeface microsite. Context: this image appears in Studio Portfolio — Landing > card-velra > velra-image-frame > Velra Type — Typeface microsite. Match the page's subject and art direction. Use this context only for subject and mood; do not write its words into the image.";
    assert_eq!(compose_generation_prompt(&target), expected);
    println!("Velra 1021: {expected}");
    assert!(!expected.contains("2024"));
    assert!(!expected.contains("DIGITAL"));
    assert!(!expected.contains("no text or logos"));
}

#[test]
fn authored_prompt_and_explicit_slug_query_keep_their_subject() {
    for prompt in [
        Some("  A typeset poster.\n warm studio light  "),
        Some("velra-image"),
    ] {
        let target = target(&velra("velra-image", prompt, None, velra_metadata()));
        assert!(compose_generation_prompt(&target)
            .starts_with(&format!("{}. Context:", prompt.unwrap())));
    }
    let target = target(&velra(
        "velra-image",
        None,
        Some("velra-image"),
        velra_metadata(),
    ));
    assert_eq!(target.generation_subject, None);
    assert_eq!(target.query, "velra-image");
    assert!(compose_generation_prompt(&target).starts_with("velra-image. Context:"));
}

#[test]
fn node_name_provenance_replaces_slugs_defaults_and_descriptive_names_without_guessing() {
    for name in [
        "velra-image",
        "project-image-0",
        "Frame 12",
        "A type specimen",
    ] {
        let target = target(&velra(name, Some(" \n\t"), None, velra_metadata()));
        assert_eq!(target.query, name);
        assert!(compose_generation_prompt(&target)
            .starts_with("Velra Type — Typeface microsite. Context:"));
    }
}

#[test]
fn ancestor_label_and_sibling_query_sources_keep_their_subjects() {
    let mut state = velra("image", None, None, velra_metadata());
    use op_editor_core::PenNodeExt as _;
    let card = &mut state.active_children_mut()[0].children_mut().unwrap()[0];
    card.base_mut().name = Some("Kyoto street".into());
    let image_frame = &mut card.children_mut().unwrap()[0];
    image_frame.base_mut().name = Some("Frame 12".into());
    let derived = target(&state);
    assert_eq!(derived.query, "Kyoto street");
    assert_eq!(derived.generation_subject, None);
    assert!(compose_generation_prompt(&derived).starts_with("Kyoto street. Context:"));

    for image in [
        json!({"type":"frame", "id":"slot", "name":"Frame 12", "role":"image-placeholder",
            "width":240, "height":240, "children":[{"type":"text", "id":"label",
                "role":"image-placeholder-label", "content":"velra-image"}]}),
        json!({"type":"rectangle", "id":"slot", "width":240, "height":240,
            "cornerRadius":12, "fill":[{"type":"solid", "color":"#cccccc"}]}),
    ] {
        let state = query_label_state(image);
        let target = target(&state);
        assert_eq!(target.query, "velra-image");
        assert_eq!(target.generation_subject, None);
    }
}

fn query_label_state(image: Value) -> EditorState {
    state(
        vec![frame(
            "root",
            "Frame 1",
            vec![image, text("caption", json!("velra-image"), 24.0)],
        )],
        None,
    )
}

#[test]
fn auxiliary_rejects_prices_dates_numbers_tags_duplicate_heading_and_buttons() {
    for rejected in [
        "2024",
        "2024-10-21",
        "June 2024",
        "2024年10月21日",
        "$99 USD",
        "99元",
        "DIGITAL",
        "Velra Type",
        "Read more",
        "View project →",
        "了解更多",
    ] {
        let target = target(&velra(
            "velra-image",
            None,
            None,
            vec![
                text("rejected", json!(rejected), 24.0),
                text("subtitle", json!("Typeface microsite"), 16.0),
            ],
        ));
        assert_eq!(
            target.generation_subject.as_deref(),
            Some("Velra Type — Typeface microsite"),
            "{rejected}"
        );
    }
    let mut button = frame(
        "button",
        "Button",
        vec![text("button-label", json!("Open the case"), 24.0)],
    );
    button["role"] = json!("button");
    let target = target(&velra(
        "velra-image",
        None,
        None,
        vec![
            button,
            frame(
                "tag",
                "Tag",
                vec![text("tag-label", json!("Digital design"), 24.0)],
            ),
            text("subtitle", json!("Typeface microsite"), 16.0),
        ],
    ));
    assert_eq!(
        target.generation_subject.as_deref(),
        Some("Velra Type — Typeface microsite")
    );
}

#[test]
fn auxiliary_prefers_next_largest_size_then_nearest_heading_neighbor() {
    let target = target(&velra(
        "velra-image",
        None,
        None,
        vec![
            text("small", json!("Small print"), 12.0),
            text("subtitle", json!("Typeface microsite"), 16.0),
            text("distant", json!("Other description"), 16.0),
        ],
    ));
    assert_eq!(
        target.generation_subject.as_deref(),
        Some("Velra Type — Typeface microsite")
    );
}

#[test]
fn missing_auxiliary_uses_only_title_and_missing_title_keeps_query() {
    let derived = target(&velra("velra-image", None, None, vec![]));
    assert_eq!(derived.generation_subject.as_deref(), Some("Velra Type"));
    assert!(compose_generation_prompt(&derived).starts_with("Velra Type. Context:"));
    let target = target(&state(
        vec![frame(
            "root",
            "Studio Landing",
            vec![json!({"type":"image", "id":"slot", "name":"velra-image", "src":""})],
        )],
        None,
    ));
    assert_eq!(target.generation_subject, None);
    assert!(compose_generation_prompt(&target).starts_with("velra-image. Context:"));
}

#[test]
fn auxiliary_is_short_and_combined_context_remains_bounded() {
    let short = "型".repeat(60);
    let target = target(&velra(
        "velra-image",
        None,
        None,
        vec![
            text("too-long", json!("字".repeat(61)), 24.0),
            text("subtitle", json!(short), 16.0),
        ],
    ));
    assert_eq!(
        target.generation_subject.as_deref(),
        Some(format!("Velra Type — {short}").as_str())
    );
    assert!(target.page_context.as_ref().unwrap().chars().count() <= 240);
    assert!(!target.page_context.as_ref().unwrap().contains('字'));
}

fn dream_state(prompt: &str) -> EditorState {
    state(
        vec![frame(
            "screen",
            "梦境详情",
            vec![frame(
                "hero",
                "月光云海",
                vec![
                    json!({"type":"image", "id":"slot", "name":"dream-stairway-hero", "src":"",
                        "width":375, "height":240, "imagePrompt":prompt}),
                    text("title", json!("沿着月光， 走进云海"), 28.0),
                ],
            )],
        )],
        None,
    )
}

#[test]
fn described_scenes_are_sent_without_page_words_the_model_would_paint() {
    // Prompts whose generated photos showed "梦境详情 / 月光云海 / 沿着月光，
    // 走进云海" and "1.4–1.8m 良好" painted in after the context was appended.
    for prompt in [
        "A surreal landscape of suspended stone steps without railings extending into a vast sea of clouds under midnight moonlight.",
        "一名冲浪者正在蓝绿色浪管内部冲浪，弧形海浪包围人物，浪花与水壁清晰可见。",
        "芦苇荡与木栈道晨雾实景，远处水面反光",
    ] {
        let target = target(&dream_state(prompt));
        assert!(target.page_context.as_deref().unwrap().contains("月光云海"));
        assert_eq!(compose_generation_prompt(&target), prompt);
    }
}

#[test]
fn bare_subjects_below_the_scene_weight_still_get_page_context() {
    for prompt in [
        "Velra Type",
        "Harbour Editions",
        "a quiet stone stairway into clouds",
        "云海阶梯",
    ] {
        let target = target(&dream_state(prompt));
        assert_eq!(
            compose_generation_prompt(&target),
            format!("{prompt}. Context: this image appears in {}. Match the page's subject and art direction. Use this context only for subject and mood; do not write its words into the image.", target.page_context.as_deref().unwrap()),
        );
    }
    let derived = target(&velra("velra-image", None, None, velra_metadata()));
    assert!(compose_generation_prompt(&derived).contains(". Context: "));
}
