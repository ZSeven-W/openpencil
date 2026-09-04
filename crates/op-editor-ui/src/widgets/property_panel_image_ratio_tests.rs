//! Tests for the image popover's "match image ratio" row: where it
//! sits, when it is live, and where its intrinsic source size comes
//! from on both the frame-fill and the standalone-Image-node path.

use super::property_panel::{PropertyPanel, PropertyPanelAction};
use super::property_panel_image_ratio::image_source_size;
use super::property_panel_sections as sections;
use super::property_panel_test_support::{state_from, visible_for, CountingBackend};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::NodeId;

const PANEL_RECT: Rect = Rect {
    origin: Point2D::new(320.0, 24.0),
    size: Point2D::new(280.0, 900.0),
};

/// A minimal PNG carrying nothing but a readable IHDR — enough for the
/// header reader the fallback path uses, without shipping a fixture.
fn png_data_url(width: u32, height: u32) -> String {
    let mut png = vec![0u8; 24];
    png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    png[8..12].copy_from_slice(&13_u32.to_be_bytes());
    png[12..16].copy_from_slice(b"IHDR");
    png[16..20].copy_from_slice(&width.to_be_bytes());
    png[20..24].copy_from_slice(&height.to_be_bytes());
    use base64::Engine as _;
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )
}

/// A wide screenshot sitting in a tall placeholder — the scenario the
/// row exists for. `original_size` is authored, as the drop / upload
/// paths write it.
fn tall_frame_with_wide_fill() -> op_editor_core::EditorState {
    let mut state = state_from(&format!(
        r##"{{"version":"1.0.0","children":[{{
            "type":"rectangle","id":"shot","name":"Screenshot",
            "x":0,"y":0,"width":320,"height":640,
            "fill":[{{"type":"image","url":"{}","mode":"fit",
                "originalSize":{{"width":1600,"height":400}}}}]
        }}]}}"##,
        png_data_url(1600, 400)
    ));
    state.set_single_selection(NodeId::new("shot"));
    state.editor_ui.image_fill_popover_open = true;
    state
}

fn popover_rects(panel: &PropertyPanel) -> Vec<(PropertyPanelAction, Rect)> {
    sections::image_fill_popover_action_rects(PANEL_RECT, visible_for(panel), &panel.snapshot)
}

fn rect_for(panel: &PropertyPanel, wanted: PropertyPanelAction) -> Option<Rect> {
    popover_rects(panel)
        .into_iter()
        .find_map(|(action, rect)| (action == wanted).then_some(rect))
}

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.x
        && b.origin.x < a.origin.x + a.size.x
        && a.origin.y < b.origin.y + b.size.y
        && b.origin.y < a.origin.y + a.size.y
}

#[test]
fn match_ratio_row_sits_below_the_upload_well_and_clears_every_other_control() {
    let state = tall_frame_with_wide_fill();
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");
    let rects = popover_rects(&panel);

    let upload = rect_for(&panel, PropertyPanelAction::PickFillImage).expect("upload well");
    let row =
        rect_for(&panel, PropertyPanelAction::MatchImageAspectRatio).expect("match-ratio row");

    assert!(
        row.origin.y >= upload.origin.y + upload.size.y,
        "the match-ratio row belongs under the upload well, not over it",
    );
    assert_eq!(
        (row.origin.x, row.size.x),
        (upload.origin.x, upload.size.x),
        "the row spans the popover's inner width like the well above it",
    );
    // The row was inserted into a stack the adjustments sit below; a
    // missed offset there would show up as two hit rects claiming the
    // same pixels.
    for (action, other) in &rects {
        if *action == PropertyPanelAction::MatchImageAspectRatio {
            continue;
        }
        assert!(
            !overlaps(row, *other),
            "match-ratio row overlaps {action:?} — the popover stack did not shift",
        );
    }
    // Sliders are hit-tested outside the action-rect list, so prove the
    // row wins its own pixels there too.
    assert_eq!(
        panel.hit_test_action(PANEL_RECT, center(row)),
        Some(PropertyPanelAction::MatchImageAspectRatio),
    );
}

#[test]
fn every_popover_control_stays_inside_the_grown_popover() {
    let mut state = tall_frame_with_wide_fill();
    // Reset only appears once an adjustment is dirty; include it so the
    // lowest-reaching control is covered by the containment sweep.
    assert!(
        state.set_selected_image_adjustment(op_editor_core::ImageAdjustmentField::Shadows, 40.0)
    );
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");

    for (action, rect) in popover_rects(&panel) {
        for corner in [
            Point2D::new(rect.origin.x + 0.5, rect.origin.y + 0.5),
            Point2D::new(
                rect.origin.x + rect.size.x - 0.5,
                rect.origin.y + rect.size.y - 0.5,
            ),
        ] {
            assert!(
                panel.image_fill_popover_contains(PANEL_RECT, corner),
                "{action:?} escapes the popover at {corner:?} — panel_h is short",
            );
        }
    }
}

#[test]
fn match_ratio_row_paints_its_localized_label() {
    let mut state = tall_frame_with_wide_fill();
    state.editor_ui.locale = op_editor_core::Locale::ZhCn;
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");

    let mut backend = CountingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint_overlays(&mut cx, PANEL_RECT);
    }

    assert!(
        backend.texts.iter().any(|text| text == "匹配图片比例"),
        "the row must paint its own label, not fall through to the key",
    );
}

#[test]
fn an_unresolvable_image_size_leaves_the_row_inert_but_still_swallows_the_click() {
    let mut state = state_from(
        r##"{"version":"1.0.0","children":[{
            "type":"rectangle","id":"remote","name":"Remote",
            "x":0,"y":0,"width":320,"height":640,
            "fill":[{"type":"image","url":"https://example.test/never-fetched.png","mode":"fit"}]
        }]}"##,
    );
    state.set_single_selection(NodeId::new("remote"));
    state.editor_ui.image_fill_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");

    assert!(
        rect_for(&panel, PropertyPanelAction::MatchImageAspectRatio).is_none(),
        "a disabled row must not also be clickable",
    );
    // The row still paints (the popover height is unconditional), so the
    // pixels it occupies have to stay inside the popover and keep it open.
    let upload = rect_for(&panel, PropertyPanelAction::PickFillImage).expect("upload well");
    let row_area = Point2D::new(
        upload.origin.x + upload.size.x / 2.0,
        upload.origin.y + upload.size.y + 14.0,
    );
    assert_eq!(panel.hit_test_action(PANEL_RECT, row_area), None);
    assert!(panel.image_fill_popover_contains(PANEL_RECT, row_area));
}

#[test]
fn a_standalone_image_node_resolves_its_ratio_from_the_raster_header() {
    // `ImageNode` has no `originalSize` field, so this path only works
    // if the size is read back out of the encoded bytes.
    let mut state = state_from(&format!(
        r##"{{"version":"1.0.0","children":[{{
            "type":"image","id":"photo","name":"Photo",
            "x":0,"y":0,"width":300,"height":600,"src":"{}"
        }}]}}"##,
        png_data_url(1200, 300)
    ));
    state.set_single_selection(NodeId::new("photo"));
    state.editor_ui.image_fill_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("image node panel");

    let summary = panel
        .snapshot
        .image_fill
        .as_ref()
        .expect("image node reports an image summary");
    assert_eq!(summary.original_size, None, "the schema authors nothing");
    assert_eq!(image_source_size(summary), Some([1200.0, 300.0]));

    let row = rect_for(&panel, PropertyPanelAction::MatchImageAspectRatio)
        .expect("an Image node with a decodable src can match its ratio");
    assert_eq!(
        panel.hit_test_action(PANEL_RECT, center(row)),
        Some(PropertyPanelAction::MatchImageAspectRatio),
    );
}

#[test]
fn a_standalone_image_node_without_decodable_bytes_stays_inert() {
    let mut state = state_from(
        r##"{"version":"1.0.0","children":[{
            "type":"image","id":"photo","name":"Photo",
            "x":0,"y":0,"width":300,"height":600,"src":"assets/never-loaded.png"
        }]}"##,
    );
    state.set_single_selection(NodeId::new("photo"));
    state.editor_ui.image_fill_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("image node panel");

    assert!(rect_for(&panel, PropertyPanelAction::MatchImageAspectRatio).is_none());
}

#[test]
fn the_authored_original_size_wins_over_the_raster_header() {
    // A fill whose bitmap was re-encoded at a different size must still
    // report the authored source, so crop math and this row agree.
    let mut state = state_from(&format!(
        r##"{{"version":"1.0.0","children":[{{
            "type":"rectangle","id":"shot","name":"Screenshot",
            "x":0,"y":0,"width":320,"height":640,
            "fill":[{{"type":"image","url":"{}","mode":"fit",
                "originalSize":{{"width":1000,"height":250}}}}]
        }}]}}"##,
        png_data_url(64, 64)
    ));
    state.set_single_selection(NodeId::new("shot"));
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");

    assert_eq!(
        image_source_size(panel.snapshot.image_fill.as_ref().expect("image fill")),
        Some([1000.0, 250.0]),
    );
}

#[test]
fn a_degenerate_authored_size_falls_through_to_the_raster_header() {
    // `first_image_fill_summary` drops a non-positive `originalSize`,
    // so the fallback is what keeps such a fill actionable.
    let mut state = state_from(&format!(
        r##"{{"version":"1.0.0","children":[{{
            "type":"rectangle","id":"shot","name":"Screenshot",
            "x":0,"y":0,"width":320,"height":640,
            "fill":[{{"type":"image","url":"{}","mode":"fit",
                "originalSize":{{"width":0,"height":0}}}}]
        }}]}}"##,
        png_data_url(800, 200)
    ));
    state.set_single_selection(NodeId::new("shot"));
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");

    assert_eq!(
        image_source_size(panel.snapshot.image_fill.as_ref().expect("image fill")),
        Some([800.0, 200.0]),
    );
}
