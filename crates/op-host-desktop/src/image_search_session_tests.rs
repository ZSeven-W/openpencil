use super::*;
use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::{
    ContainerProps, FrameNode, ImageNode, PenNode, RectangleNode, TextContent, TextNode,
};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{ImageFillMode, PenFill, SolidFillBody};

fn image_node(id: &str, src: &str, query: Option<&str>) -> PenNode {
    PenNode::Image(ImageNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some("Menu photo".into()),
            ..Default::default()
        },
        src: src.to_string(),
        object_fit: None,
        width: Some(SizingBehavior::Number(240.0)),
        height: Some(SizingBehavior::Number(160.0)),
        corner_radius: None,
        effects: None,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
        image_prompt: None,
        image_search_query: query.map(str::to_string),
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn text_label(id: &str, role: Option<&str>, content: &str) -> PenNode {
    PenNode::Text(TextNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some("Label".into()),
            role: role.map(str::to_string),
            ..Default::default()
        },
        width: Some(SizingBehavior::Number(160.0)),
        height: Some(SizingBehavior::Number(24.0)),
        content: TextContent::Plain(content.to_string()),
        font_family: None,
        font_size: None,
        font_weight: None,
        font_style: None,
        letter_spacing: None,
        line_height: None,
        text_align: None,
        text_align_vertical: None,
        text_growth: None,
        underline: None,
        strikethrough: None,
        fill: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn frame_node(
    id: &str,
    name: &str,
    role: Option<&str>,
    fill: Option<Vec<PenFill>>,
    children: Vec<PenNode>,
) -> PenNode {
    PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.into()),
            role: role.map(str::to_string),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(240.0)),
            height: Some(SizingBehavior::Number(160.0)),
            fill,
            ..Default::default()
        },
        children: Some(children),
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn rectangle_node(id: &str, name: &str, fill: Option<Vec<PenFill>>) -> PenNode {
    PenNode::Rectangle(RectangleNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.into()),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(240.0)),
            height: Some(SizingBehavior::Number(160.0)),
            fill,
            ..Default::default()
        },
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn solid_fill() -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: "#E5E7EB".into(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

#[test]
fn collect_targets_prefers_query_on_empty_image_nodes() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "img1");
    assert_eq!(targets[0].query, "burger fries");
}

#[test]
fn apply_result_sets_empty_image_src() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    assert!(apply_result(
        &mut state,
        &NodeId::new("img1"),
        "https://example.com/photo.jpg"
    ));
    let PenNode::Image(image) = &state.active_children()[0] else {
        panic!("expected image");
    };
    assert_eq!(image.src, "https://example.com/photo.jpg");
}

#[test]
fn collect_targets_includes_unfilled_placeholder_frames() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame_node(
        "photo",
        "Image",
        Some("image-placeholder"),
        Some(vec![solid_fill()]),
        vec![text_label(
            "label",
            Some("image-placeholder-label"),
            "pizza hero",
        )],
    ));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "pizza hero");
}

#[test]
fn collect_targets_prefers_frame_image_search_query() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    let frame: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "photo",
        "name": "Image",
        "role": "image-placeholder",
        "width": 240,
        "height": 160,
        "fill": [{ "type": "solid", "color": "#E5E7EB" }],
        "imageSearchQuery": "burger fries"
    }))
    .unwrap();
    state.active_children_mut().push(frame);

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "burger fries");
}

#[test]
fn collect_targets_uses_parent_semantic_name_for_generic_heuristic_frame() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    let photo = frame_node("photo", "Image", None, Some(vec![solid_fill()]), Vec::new());
    state
        .active_children_mut()
        .push(frame_node("card", "Bella Italia", None, None, vec![photo]));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "Bella Italia");
}

#[test]
fn collect_targets_includes_solid_rectangle_image_areas() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(rectangle_node(
        "photo",
        "Latte Image",
        Some(vec![solid_fill()]),
    ));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "Latte Image");
}

#[test]
fn apply_result_repaints_placeholder_frame_with_image_fill_and_clears_children() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame_node(
        "photo",
        "Image",
        Some("image-placeholder"),
        Some(vec![solid_fill()]),
        vec![text_label(
            "label",
            Some("image-placeholder-label"),
            "pizza hero",
        )],
    ));

    assert!(apply_result(
        &mut state,
        &NodeId::new("photo"),
        "https://example.com/photo.jpg"
    ));
    let PenNode::Frame(frame) = &state.active_children()[0] else {
        panic!("expected frame");
    };
    let Some([PenFill::Image(image_fill)]) = frame.container.fill.as_deref() else {
        panic!("expected single image fill");
    };
    assert_eq!(image_fill.url, "https://example.com/photo.jpg");
    assert_eq!(image_fill.mode, Some(ImageFillMode::Crop));
    assert_eq!(frame.children.as_deref(), Some(&[][..]));
}

#[test]
fn apply_result_repaints_placeholder_rectangle_with_image_fill() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(rectangle_node(
        "photo",
        "Latte Image",
        Some(vec![solid_fill()]),
    ));

    assert!(apply_result(
        &mut state,
        &NodeId::new("photo"),
        "https://example.com/photo.jpg"
    ));
    let PenNode::Rectangle(rect) = &state.active_children()[0] else {
        panic!("expected rectangle");
    };
    let Some([PenFill::Image(image_fill)]) = rect.container.fill.as_deref() else {
        panic!("expected single image fill");
    };
    assert_eq!(image_fill.url, "https://example.com/photo.jpg");
    assert_eq!(image_fill.mode, Some(ImageFillMode::Crop));
}

#[test]
fn poll_into_applies_finished_job_to_placeholder_frame() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame_node(
        "photo",
        "Image",
        Some("image-placeholder"),
        Some(vec![solid_fill()]),
        vec![text_label(
            "label",
            Some("image-placeholder-label"),
            "pizza hero",
        )],
    ));

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Some("https://example.com/photo.jpg".to_string()))
        .unwrap();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["photo".to_string()]),
        completed: HashSet::new(),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("photo"),
            rx,
        }],
    };

    assert!(session.poll_into(&mut state));
    assert!(!session.is_pending());
    assert!(session.in_flight.is_empty());
    assert!(session.completed.contains("photo"));

    let PenNode::Frame(frame) = &state.active_children()[0] else {
        panic!("expected frame");
    };
    let Some([PenFill::Image(image_fill)]) = frame.container.fill.as_deref() else {
        panic!("expected single image fill");
    };
    assert_eq!(image_fill.url, "https://example.com/photo.jpg");
    assert_eq!(frame.children.as_deref(), Some(&[][..]));
}

#[test]
fn openverse_credentials_require_both_fields() {
    let mut state = EditorState::default();
    assert!(OpenverseCredentials::from_state(&state).is_none());

    state.editor_ui.agent_settings.openverse_client_id = " client ".into();
    assert!(OpenverseCredentials::from_state(&state).is_none());

    state.editor_ui.agent_settings.openverse_client_secret = " secret ".into();
    let credentials = OpenverseCredentials::from_state(&state).expect("complete credentials");
    assert_eq!(credentials.client_id, "client");
    assert_eq!(credentials.client_secret, "secret");
}

#[tokio::test]
#[ignore = "network smoke test for Openverse/Wikimedia"]
async fn fetch_first_image_url_smoke() {
    let url = fetch_first_image_url("burger fries", None)
        .await
        .expect("common query should return an image URL");
    assert!(url.starts_with("http"), "got {url}");
}
