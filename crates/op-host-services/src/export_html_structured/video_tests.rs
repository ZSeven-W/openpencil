//! Video placeholder tests for structured HTML export.

use super::{board_slide_markup, SlideMarkup};
use op_editor_ui::layout_scene::{NodeKind, SceneImageFit, SceneNode, ScenePage, SceneVideo};
use op_editor_ui::{Color, Rect};

fn board(children: Vec<SceneNode>) -> SceneNode {
    let mut node = SceneNode::leaf("board", NodeKind::Frame);
    node.bounds = Rect::xywh(0.0, 0.0, 200.0, 100.0);
    node.fill = Some(Color::WHITE);
    node.children = children;
    node
}

fn markup_of(board: SceneNode) -> SlideMarkup {
    let page = ScenePage {
        id: "p1".into(),
        name: "Page 1".into(),
        children: vec![board],
    };
    board_slide_markup(&page, "board", "Slide".into()).expect("board emits")
}

#[test]
fn an_embedded_video_keeps_the_poster_and_remote_video_src_structured() {
    const POSTER: &str = "data:image/png;base64,AA==";
    let mut node = SceneNode::leaf("video", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 80.0, 45.0);
    node.image_src = Some(POSTER.into());
    node.image_fit = SceneImageFit::Fill;
    node.video = Some(SceneVideo {
        src: "https://cdn.example.com/hero.mp4?token=a&b=c".into(),
        autoplay: true,
        r#loop: true,
        muted: false,
        hold_last_frame: false,
        click_to_replay: false,
    });

    let markup = markup_of(board(vec![node]));

    assert!(
        markup.fallback_reasons.is_empty(),
        "{:?}",
        markup.fallback_reasons
    );
    assert_eq!(
        markup.structured_nodes, 2,
        "board and video should be structured"
    );
    assert!(markup
        .body
        .contains("<video src=\"https://cdn.example.com/hero.mp4?token=a&amp;b=c\""));
    assert!(markup.body.contains("poster=\"data:image/png"));
    assert!(markup.body.contains(" autoplay"));
    assert!(markup.body.contains(" loop"));
    assert!(markup.body.contains(" muted"));
}

#[test]
fn a_video_without_a_poster_stays_structured_over_the_fill() {
    let mut node = SceneNode::leaf("video", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 80.0, 45.0);
    node.fill = Some(Color::BLACK);
    node.image_src = Some(String::new().into());
    node.video = Some(SceneVideo {
        src: "hero.mp4".into(),
        autoplay: false,
        r#loop: false,
        muted: false,
        hold_last_frame: false,
        click_to_replay: false,
    });

    let markup = markup_of(board(vec![node]));

    assert!(
        markup.fallback_reasons.is_empty(),
        "{:?}",
        markup.fallback_reasons
    );
    assert!(markup.body.contains("<video src=\"hero.mp4\""));
    assert!(!markup.body.contains("background-image:url(\"\")"));
}
