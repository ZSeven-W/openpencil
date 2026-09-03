use super::*;

use op_editor_core::EditorState;
use op_orchestrator::AbortFlag;
use serde_json::{json, Value};

fn node(value: Value) -> jian_ops_schema::node::PenNode {
    serde_json::from_value(value).expect("valid PenNode fixture")
}

fn frame(
    id: &str,
    name: &str,
    width: f64,
    height: f64,
    layout: &str,
    children: Vec<Value>,
) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "name": name,
        "width": width,
        "height": height,
        "layout": layout,
        "children": children,
    })
}

fn text(id: &str, content: &str) -> Value {
    json!({"type":"text", "id":id, "name":"Acme copy", "content":content})
}

fn landing_tree() -> jian_ops_schema::node::PenNode {
    let nav = frame(
        "nav",
        "Navigation Bar",
        1200.0,
        72.0,
        "horizontal",
        vec![text("nav-label", "Acme")],
    );
    let hero = frame(
        "hero",
        "Hero Section",
        1200.0,
        560.0,
        "horizontal",
        vec![
            frame(
                "hero-copy",
                "Acme Story",
                600.0,
                560.0,
                "vertical",
                vec![text("hero-title", "Acme")],
            ),
            json!({
                "type":"image", "id":"hero-image", "name":"Acme image",
                "width":600, "height":560, "src":""
            }),
        ],
    );
    let features = frame(
        "features",
        "Features",
        1200.0,
        480.0,
        "vertical",
        vec![frame(
            "feature-row",
            "Feature Row",
            1200.0,
            240.0,
            "horizontal",
            (1..=3)
                .map(|index| {
                    frame(
                        &format!("card-{index}"),
                        &format!("Acme Card {index}"),
                        380.0,
                        240.0,
                        "vertical",
                        vec![text(&format!("card-text-{index}"), "Acme")],
                    )
                })
                .collect(),
        )],
    );
    node(frame(
        "root",
        "Acme Page",
        1200.0,
        1352.0,
        "vertical",
        vec![
            nav,
            hero,
            features,
            frame("footer", "Footer", 1200.0, 240.0, "vertical", vec![]),
        ],
    ))
}

#[tokio::test]
async fn imported_landing_tree_becomes_content_free_skeleton_and_design_md() {
    let context = reference_context_from_nodes(
        &crate::test_support::ScriptedLlm,
        vec![landing_tree()],
        "https://Example.com/page?ref=1",
        "参考这个页面做我们的官网",
        Some("model-a".into()),
        Some("provider-a".into()),
        &AbortFlag::new(),
    )
    .await
    .expect("reference context");

    assert_eq!(context.source_host, "example.com");
    assert_eq!(context.skeleton.sections.len(), 4);
    assert_eq!(context.design_md.project_name.as_deref(), Some("Food App"));
    assert!(context
        .design_md
        .color_palette
        .as_ref()
        .is_some_and(|colors| colors.iter().any(|color| color.hex == "#FF5A1F")));
    let rendered = context.skeleton.render();
    assert!(!rendered.contains("Acme"));
    assert!(!rendered.contains("Navigation Bar"));
    assert!(!rendered.contains("Hero Section"));
}

#[tokio::test]
async fn reference_extraction_does_not_mutate_the_user_state_or_history() {
    let mut state = EditorState::new();
    state
        .active_children_mut()
        .extend([landing_tree(), landing_tree()]);
    state.commit_history();
    let before_doc = state.doc.clone();
    let before_nodes = state.active_children().len();
    let before_past = state.history.past.len();
    let before_future = state.history.future.len();

    reference_context_from_nodes(
        &crate::test_support::ScriptedLlm,
        state.active_children().to_vec(),
        "example.com",
        "参考页面做另一页",
        None,
        None,
        &AbortFlag::new(),
    )
    .await
    .expect("reference context");

    assert_eq!(state.doc, before_doc);
    assert_eq!(state.active_children().len(), before_nodes);
    assert_eq!(state.history.past.len(), before_past);
    assert_eq!(state.history.future.len(), before_future);
}

#[tokio::test]
async fn a_bare_url_is_not_fetched_but_a_triggered_loopback_is_rejected() {
    let url = "http://127.0.0.1:1/";
    let no_intent = resolve_reference_context(
        &crate::test_support::ScriptedLlm,
        &format!("把 {url} 放进页脚链接"),
        None,
        None,
        &AbortFlag::new(),
    )
    .await;
    assert!(matches!(no_intent, Ok(None)));

    let rejected = resolve_reference_context(
        &crate::test_support::ScriptedLlm,
        &format!("参考 {url} 做首页"),
        None,
        None,
        &AbortFlag::new(),
    )
    .await;
    assert!(matches!(
        rejected,
        Err(ReferenceContextError::Import(
            crate::import_html_url_error::ImportHtmlUrlError::UrlNotAllowed
        ))
    ));
}
