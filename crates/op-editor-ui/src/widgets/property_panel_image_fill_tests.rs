//! Image-fill section tests for `widgets::property_panel`: the fill
//! body opening the editor popover, painting the selected image
//! preview / thumbnail, i18n labels, and the popover's upload + fit-
//! mode hit rects.

use super::property_panel::{PropertyPanel, PropertyPanelAction};
use super::property_panel_sections as sections;
use super::property_panel_test_support::{state_from, visible_for, CountingBackend};
use crate::widgets::{PaintCx, Widget};
use crate::{ImageDrawMode, Point2D, Rect};
use op_editor_core::NodeId;

fn image_fill_state_with_url(url: &str) -> op_editor_core::EditorState {
    let mut state = state_from(&format!(
        r##"{{ "version": "0.8.0", "children": [
              {{"type":"rectangle","id":"n60","name":"Photo fill",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{{"type":"image","url":"{}","mode":"fill",
                 "exposure":0,"contrast":0,"saturation":0,
                 "temperature":0,"tint":0,"highlights":0,"shadows":0}}]}}
        ]}}"##,
        url
    ));
    state.set_single_selection(NodeId::new("n60"));
    state
}

fn image_fill_state() -> op_editor_core::EditorState {
    image_fill_state_with_url("")
}

#[test]
fn image_fill_body_click_opens_the_image_popover() {
    let state = image_fill_state();
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");
    let rect = Rect {
        origin: Point2D::new(320.0, 24.0),
        size: Point2D::new(280.0, 900.0),
    };
    let rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&panel),
        &panel.snapshot.effects,
        false,
        false,
        false,
        false,
        false,
        false,
    );
    let body = rects
        .iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::ToggleImageFillPopover))
        .map(|(_, r)| *r)
        .expect("image fill body emits popover toggle action");
    let center = Point2D::new(
        body.origin.x + body.size.x / 2.0,
        body.origin.y + body.size.y / 2.0,
    );
    assert!(
        matches!(
            panel.hit_test_action(rect, center),
            Some(PropertyPanelAction::ToggleImageFillPopover)
        ),
        "image fill body click should open the image editor popover",
    );
}

#[test]
fn open_image_fill_popover_paints_selected_image_preview() {
    const PNG_DATA_URL: &str =
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";
    let mut state = image_fill_state_with_url(PNG_DATA_URL);
    state.editor_ui.image_fill_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");
    assert_eq!(
        panel
            .snapshot
            .image_fill
            .as_ref()
            .unwrap()
            .image_url
            .as_deref(),
        Some(PNG_DATA_URL),
    );

    let mut backend = CountingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let rect = Rect {
            origin: Point2D::new(320.0, 24.0),
            size: Point2D::new(280.0, 900.0),
        };
        panel.paint(&mut cx, rect);
        panel.paint_overlays(&mut cx, rect);
    }
    assert!(
        backend.images.iter().any(|(_, _, bytes)| *bytes > 0),
        "selected image data URL should be decoded and painted in the upload well",
    );
}

#[test]
fn image_fill_body_paints_selected_image_thumbnail_with_mode() {
    const PNG_DATA_URL: &str =
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";
    let mut state = image_fill_state_with_url(PNG_DATA_URL);
    assert!(state.set_selected_image_fill_mode(op_editor_core::ImageFillMode::Tile));
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");

    let mut backend = CountingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(
            &mut cx,
            Rect {
                origin: Point2D::new(320.0, 24.0),
                size: Point2D::new(280.0, 900.0),
            },
        );
    }

    assert!(
        backend.image_modes.contains(&ImageDrawMode::Tile),
        "fill body thumbnail should paint the selected image using the current image mode",
    );
}

#[test]
fn image_fill_adjustment_reset_label_uses_i18n() {
    let mut state = image_fill_state();
    state.editor_ui.image_fill_popover_open = true;
    state.editor_ui.locale = op_editor_core::Locale::ZhCn;
    assert!(
        state.set_selected_image_adjustment(op_editor_core::ImageAdjustmentField::Exposure, 36.0)
    );
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");

    let mut backend = CountingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint_overlays(
            &mut cx,
            Rect {
                origin: Point2D::new(320.0, 24.0),
                size: Point2D::new(280.0, 900.0),
            },
        );
    }
    assert!(backend.texts.iter().any(|s| s == "重置"));
    assert!(!backend.texts.iter().any(|s| s == "Reset"));
}

#[test]
fn open_image_fill_popover_routes_upload_and_mode_actions() {
    let mut state = image_fill_state();
    state.editor_ui.image_fill_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");
    let rect = Rect {
        origin: Point2D::new(320.0, 24.0),
        size: Point2D::new(280.0, 900.0),
    };
    let popup_rects =
        sections::image_fill_popover_action_rects(rect, visible_for(&panel), &panel.snapshot);
    let upload = popup_rects
        .iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::PickFillImage))
        .map(|(_, r)| *r)
        .expect("open image popover exposes an upload hit rect");
    let upload_center = Point2D::new(
        upload.origin.x + upload.size.x / 2.0,
        upload.origin.y + upload.size.y / 2.0,
    );
    assert!(
        matches!(
            panel.hit_test_action(rect, upload_center),
            Some(PropertyPanelAction::PickFillImage)
        ),
        "upload well should trigger the image file picker",
    );
    let crop = popup_rects
        .iter()
        .find(|(a, _)| {
            matches!(
                a,
                PropertyPanelAction::SetImageFillMode(op_editor_core::ImageFillMode::Crop)
            )
        })
        .map(|(_, r)| *r)
        .expect("open image popover exposes fit-mode hit rects");
    let crop_center = Point2D::new(
        crop.origin.x + crop.size.x / 2.0,
        crop.origin.y + crop.size.y / 2.0,
    );
    assert!(
        matches!(
            panel.hit_test_action(rect, crop_center),
            Some(PropertyPanelAction::SetImageFillMode(
                op_editor_core::ImageFillMode::Crop
            ))
        ),
        "fit-mode chips should dispatch mode updates",
    );
}

#[test]
fn image_fill_popover_internal_gap_is_consumed_without_action() {
    let mut state = image_fill_state();
    state.editor_ui.image_fill_popover_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("image fill panel");
    let rect = Rect {
        origin: Point2D::new(320.0, 24.0),
        size: Point2D::new(280.0, 900.0),
    };
    let popup_rects =
        sections::image_fill_popover_action_rects(rect, visible_for(&panel), &panel.snapshot);
    let upload = popup_rects
        .iter()
        .find(|(a, _)| matches!(a, PropertyPanelAction::PickFillImage))
        .map(|(_, r)| *r)
        .expect("upload rect exists");
    let gap = Point2D::new(upload.origin.x + 20.0, upload.origin.y - 5.0);

    assert_eq!(panel.hit_test_action(rect, gap), None);
    assert!(
        panel.image_fill_popover_contains(rect, gap),
        "clicks in non-interactive popover gaps must be consumed so the popover stays open",
    );
}
