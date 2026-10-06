//! Property-panel coverage for image-node video metadata.

use super::property_panel::{PropertyPanel, PropertyPanelAction};
use super::property_panel_test_support::{state_from, CountingBackend};
use crate::widgets::{PaintCx, Widget};
use crate::{Point2D, Rect};
use op_editor_core::{NodeId, PropertyFocus};

fn image_state(video: bool) -> op_editor_core::EditorState {
    let video_json = if video {
        r#", "video":{"src":"https://cdn.example/hero.mp4","autoplay":true,"loop":true,"muted":true,"holdLastFrame":false,"clickToReplay":true}"#
    } else {
        ""
    };
    let mut state = state_from(&format!(
        r##"{{"version":"1.0.0","children":[{{"type":"image","id":"hero","name":"Hero","x":0,"y":0,"width":320,"height":180,"src":"poster.png"{video_json}}}]}}"##
    ));
    state.set_single_selection(NodeId::new("hero"));
    state
}

fn panel_rect() -> Rect {
    Rect::xywh(0.0, 0.0, 280.0, 1000.0)
}

#[test]
fn video_section_is_present_only_when_image_has_video() {
    let plain = PropertyPanel::for_selection(&image_state(false)).expect("image panel");
    assert!(plain.snapshot.video.is_none());
    assert!(!plain.visible_sections_for_test().video);

    let with_video = PropertyPanel::for_selection(&image_state(true)).expect("video image panel");
    assert!(with_video.snapshot.video.is_some());
    assert!(with_video.visible_sections_for_test().video);
    let video_title = op_i18n::translate(with_video.locale, "video.title");
    let video_url = op_i18n::translate(with_video.locale, "video.url");

    let mut plain_backend = CountingBackend::default();
    let mut plain_cx = PaintCx {
        backend: &mut plain_backend,
    };
    plain.paint(&mut plain_cx, panel_rect());
    assert!(!plain_backend.texts.iter().any(|text| text == video_title));

    let mut video_backend = CountingBackend::default();
    let mut video_cx = PaintCx {
        backend: &mut video_backend,
    };
    with_video.paint(&mut video_cx, panel_rect());
    assert!(video_backend.texts.iter().any(|text| text == video_title));
    assert!(video_backend.texts.iter().any(|text| text == video_url));
}

#[test]
fn video_url_input_and_toggle_actions_have_aligned_hit_rects() {
    let panel = PropertyPanel::for_selection(&image_state(true)).expect("video image panel");
    let rect = panel_rect();
    let url_rect = super::property_panel_sections::editable_input_rects(
        rect,
        panel.visible_sections_for_test(),
        &panel.snapshot.fills,
        &panel.snapshot.effects,
    )
    .into_iter()
    .find_map(|(focus, rect)| (focus == PropertyFocus::VideoSrc).then_some(rect))
    .expect("video URL input rect");
    let url_center = Point2D::new(
        url_rect.origin.x + url_rect.size.x / 2.0,
        url_rect.origin.y + url_rect.size.y / 2.0,
    );
    assert_eq!(
        panel.hit_test(rect, url_center),
        Some(PropertyFocus::VideoSrc)
    );

    let actions = super::property_panel_sections::action_button_rects_with_fill_picker(
        rect,
        panel.visible_sections_for_test(),
        &panel.snapshot.effects,
        &panel.snapshot.fills,
        &panel.snapshot.interactions,
        false,
        0,
        false,
        false,
        false,
        false,
        false,
    );
    for expected in [
        PropertyPanelAction::ToggleVideoAutoplay,
        PropertyPanelAction::ToggleVideoLoop,
        PropertyPanelAction::ToggleVideoMuted,
        PropertyPanelAction::ToggleVideoHoldLastFrame,
        PropertyPanelAction::ToggleVideoClickToReplay,
    ] {
        let (_, target) = actions
            .iter()
            .find(|(action, _)| *action == expected)
            .expect("video toggle action");
        let point = Point2D::new(
            target.origin.x + target.size.x / 2.0,
            target.origin.y + target.size.y / 2.0,
        );
        assert_eq!(panel.hit_test_action(rect, point), Some(expected));
    }
}
