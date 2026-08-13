use super::{LayerDragState, WidgetHostNative};
use jian_ops_schema::PenDocument;
use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
use op_editor_core::{EditorState, LeftPanelTab, NodeId, PenNodeExt};
use op_editor_ui::widgets::{LayerPanelHit, PropertyPanel, PropertyPanelAction};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 390.0;
const VIEWPORT_H: f32 = 844.0;

fn document_with_rects(count: usize) -> PenDocument {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": (0..count).map(|index| serde_json::json!({
            "type": "rectangle",
            "id": format!("n{index}"),
            "name": format!("Rect {index}"),
            "x": 0,
            "y": index * 12,
            "width": 10,
            "height": 10
        })).collect::<Vec<_>>()
    }))
    .expect("valid touch-panel fixture")
}

fn document_with_slides(count: usize) -> PenDocument {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": (0..count).map(|index| serde_json::json!({
            "type": "frame",
            "id": format!("slide-{index}"),
            "name": format!("Slide {index}"),
            "x": index * 1200,
            "y": 0,
            "width": 1000,
            "height": 562,
            "children": []
        })).collect::<Vec<_>>()
    }))
    .expect("valid touch slides fixture")
}

fn touch_host(count: usize, sheet: MobileSheetKind) -> WidgetHostNative {
    let mut state = EditorState::from_document(document_with_rects(count));
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.mobile_sheet = Some(sheet);
    state.editor_ui.sidebar_open = true;
    state.set_single_selection(NodeId::new("n0"));
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host
}

fn touch_slides_host(count: usize) -> WidgetHostNative {
    let mut state = EditorState::from_document(document_with_slides(count));
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.mobile_sheet = Some(MobileSheetKind::Layers);
    state.editor_ui.sidebar_open = false;
    state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host
}

fn touch_font_picker_host(system_font_count: usize) -> WidgetHostNative {
    let mut state = EditorState::sample();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.mobile_sheet = Some(MobileSheetKind::Properties);
    state.editor_ui.system_fonts_loaded = true;
    state.editor_ui.font_import_supported = true;
    state.editor_ui.imported_font_families = std::sync::Arc::new(Vec::new());
    state.editor_ui.bundled_font_families = std::sync::Arc::new(Vec::new());
    state.editor_ui.system_font_families = std::sync::Arc::new(
        (0..system_font_count)
            .map(|index| format!("Touch Font {index:03}"))
            .collect(),
    );
    state.set_single_selection(NodeId::new("n11"));
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host.apply_property_action(PropertyPanelAction::ToggleFontFamilyPicker);
    assert!(host.editor_state().editor_ui.font_picker.open);
    host
}

fn find_font_picker_action(
    host: &WidgetHostNative,
    predicate: impl Fn(&PropertyPanelAction) -> bool,
) -> (Point2D, PropertyPanelAction) {
    let rect = op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
        host.editor_state(),
        VIEWPORT_W,
        VIEWPORT_H,
    );
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let mut y = 1.0;
    while y < VIEWPORT_H - 1.0 {
        let mut x = 1.0;
        while x < VIEWPORT_W - 1.0 {
            let point = Point2D::new(x, y);
            if let Some(action) = panel.hit_test_action(rect, point) {
                if predicate(&action) {
                    return (point, action);
                }
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("font picker action point");
}

fn selected_text_family(host: &WidgetHostNative) -> Option<String> {
    let node = host.editor_state().selected_node().expect("selected text");
    let jian_ops_schema::node::PenNode::Text(text) = node else {
        panic!("n11 must be text");
    };
    text.font_family.clone()
}

fn slide_row_point(host: &mut WidgetHostNative, index: usize) -> Point2D {
    let slides = host
        .slides_panel_frame(VIEWPORT_W, VIEWPORT_H)
        .expect("open touch sheet with boards shows slides");
    let row = slides.layout.row_rect(index);
    Point2D::new(
        row.origin.x + row.size.x / 2.0,
        row.origin.y + row.size.y / 2.0,
    )
}

fn find_layer_row_point(host: &WidgetHostNative) -> Point2D {
    let rect = host.layers_content_rect(VIEWPORT_W, VIEWPORT_H);
    let panel = host.layer_panel();
    let regions = panel.regions(rect);
    let y0 = regions.layers_rows_top;
    let y1 = y0 + regions.layers_view_h.min(80.0);
    let mut y = y0 + 1.0;
    while y < y1 {
        let mut x = rect.origin.x + 1.0;
        while x < rect.origin.x + rect.size.x - 1.0 {
            let point = Point2D::new(x, y);
            if matches!(panel.hit_test(rect, point), Some(LayerPanelHit::Layer(_))) {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("layer row point");
}

fn find_property_action(
    host: &WidgetHostNative,
) -> (Point2D, op_editor_ui::widgets::PropertyPanelAction) {
    let rect = op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
        host.editor_state(),
        VIEWPORT_W,
        VIEWPORT_H,
    );
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let mut y = rect.origin.y + 45.0;
    while y < rect.origin.y + rect.size.y - 1.0 {
        let mut x = rect.origin.x + 1.0;
        while x < rect.origin.x + rect.size.x - 1.0 {
            let point = Point2D::new(x, y);
            if let Some(action) = panel.hit_test_action(rect, point) {
                if action == op_editor_ui::widgets::PropertyPanelAction::ToggleCornerExpand {
                    return (point, action);
                }
            }
            x += 3.0;
        }
        y += 3.0;
    }
    panic!("property action point");
}

fn touch_code_host() -> WidgetHostNative {
    let mut host = touch_host(1, MobileSheetKind::Properties);
    host.editor_state_mut().editor_ui.property_tab = op_editor_core::PropertyTab::Code;
    host
}

fn code_framework_strip_point(host: &WidgetHostNative) -> Point2D {
    let rect = op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
        host.editor_state(),
        VIEWPORT_W,
        VIEWPORT_H,
    );
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("Code property panel");
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let point = Point2D::new(rect.origin.x + rect.size.x / 2.0, y);
        if panel.code_framework_strip_contains(rect, point) {
            return point;
        }
        y += 1.0;
    }
    panic!("framework strip point");
}

#[test]
fn property_body_tap_is_delayed_then_dispatched_once() {
    let mut host = touch_host(1, MobileSheetKind::Properties);
    let (point, action) = find_property_action(&host);
    assert!(!host.editor_state().editor_ui.corner_expand_open);

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_some());
    assert!(!host.apply_cursor_move(point.x + 4.0, point.y + 4.0));
    assert!(host.touch_panel_gesture.is_some());
    assert_eq!(
        PropertyPanel::for_selection(host.editor_state()).and_then(|panel| panel.hit_test_action(
            op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
                host.editor_state(),
                VIEWPORT_W,
                VIEWPORT_H,
            ),
            point,
        )),
        Some(action.clone())
    );
    assert!(
        !host.editor_state().editor_ui.corner_expand_open,
        "down must not dispatch the property action"
    );
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_none());
    assert!(host.editor_state().editor_ui.corner_expand_open);
    let _ = host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H);
    assert!(
        host.editor_state().editor_ui.corner_expand_open,
        "release cannot replay the toggle twice"
    );
}

#[test]
fn property_scroll_never_dispatches_the_pending_action() {
    let mut host = touch_host(1, MobileSheetKind::Properties);
    let (point, _) = find_property_action(&host);

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(point.x, point.y - 12.0));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert!(!host.editor_state().editor_ui.corner_expand_open);
}

#[test]
fn selected_frame_property_panel_scrolls_in_every_touch_layout() {
    for (viewport_w, viewport_h, size_class, mobile_sheet) in [
        (
            390.0,
            844.0,
            EditorSizeClass::Compact,
            Some(MobileSheetKind::Properties),
        ),
        (
            834.0,
            1112.0,
            EditorSizeClass::Medium,
            Some(MobileSheetKind::Properties),
        ),
        (1194.0, 834.0, EditorSizeClass::Expanded, None),
    ] {
        let mut state = EditorState::sample();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = size_class;
        state.editor_ui.mobile_sheet = mobile_sheet;
        state.set_single_selection(NodeId::new("n10"));
        let mut host = WidgetHostNative::new();
        assert!(host.replace_editor_state(state));

        let rect = op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
            host.editor_state(),
            viewport_w,
            viewport_h,
        );
        let start = Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y * 0.65,
        );
        assert!(host.apply_press(start.x, start.y, viewport_w, viewport_h));
        assert!(host.touch_panel_gesture.is_some());
        assert!(host.apply_cursor_move(start.x, start.y - 120.0));
        assert!(
            host.editor_state().editor_ui.property_panel_scroll.offset > 0.0,
            "{size_class:?} inspector must advance its scroll offset"
        );
        assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    }
}

#[test]
fn code_framework_strip_horizontal_drag_scrolls_without_selecting_a_chip() {
    let mut host = touch_code_host();
    let point = code_framework_strip_point(&host);
    let selected = host.editor_state().codegen.framework;

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_some());
    assert!(host.apply_cursor_move(point.x - 36.0, point.y + 2.0));
    assert!(host.editor_state().codegen.framework_scroll.offset > 0.0);
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().codegen.framework, selected);
    assert!(host.touch_panel_gesture.is_none());
}

#[test]
fn code_framework_drag_cancel_path_never_replays_the_pending_chip() {
    let mut host = touch_code_host();
    let point = code_framework_strip_point(&host);
    let selected = host.editor_state().codegen.framework;

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(point.x - 24.0, point.y));
    assert!(host.cancel_touch_panel_gesture());
    assert!(!host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().codegen.framework, selected);
    assert!(host.touch_panel_gesture.is_none());
}

#[test]
fn code_body_horizontal_drag_keeps_vertical_property_scroll_routing() {
    let mut host = touch_code_host();
    let rect = op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
        host.editor_state(),
        VIEWPORT_W,
        VIEWPORT_H,
    );
    let strip = code_framework_strip_point(&host);
    let body = Point2D::new(rect.origin.x + rect.size.x / 2.0, strip.y + 120.0);
    assert!(rect.contains(body));

    assert!(host.apply_press(body.x, body.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(body.x - 36.0, body.y + 2.0));
    assert_eq!(host.editor_state().codegen.framework_scroll.offset, 0.0);
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
}

#[test]
fn property_font_picker_tap_is_delayed_then_selects_once() {
    let mut host = touch_font_picker_host(80);
    let (point, action) = find_font_picker_action(&host, |action| {
        matches!(action, PropertyPanelAction::SetFontFamilyIndex(_))
    });
    let PropertyPanelAction::SetFontFamilyIndex(index) = action else {
        unreachable!();
    };
    let expected = PropertyPanel::for_selection(host.editor_state())
        .expect("property panel")
        .font_picker_entries()[index]
        .family
        .to_string();
    let before = selected_text_family(&host);

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_some());
    assert!(host.editor_state().editor_ui.font_picker.open);
    assert_eq!(selected_text_family(&host), before, "down cannot select");

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_none());
    assert!(!host.editor_state().editor_ui.font_picker.open);
    assert_eq!(
        selected_text_family(&host).as_deref(),
        Some(expected.as_str())
    );
    let _ = host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H);
    assert_eq!(
        selected_text_family(&host).as_deref(),
        Some(expected.as_str())
    );
}

#[test]
fn property_font_picker_drag_scrolls_without_selecting() {
    let mut host = touch_font_picker_host(80);
    let (point, _) = find_font_picker_action(&host, |action| {
        matches!(action, PropertyPanelAction::SetFontFamilyIndex(_))
    });
    let before = selected_text_family(&host);

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(point.x, point.y - 24.0));
    assert!(host.editor_state().editor_ui.font_picker.scroll.offset > 0.0);
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    assert!(host.editor_state().editor_ui.font_picker.open);
    assert_eq!(selected_text_family(&host), before);
}

#[test]
fn property_font_picker_import_and_remove_taps_wait_for_release() {
    let mut import_host = touch_font_picker_host(0);
    import_host.editor_state_mut().editor_ui.font_picker_search = "no-such-touch-font".into();
    let import_panel = PropertyPanel::for_selection(import_host.editor_state()).unwrap();
    assert!(import_panel.font_import_supported);
    assert!(import_panel.font_picker_entries().is_empty());
    let (import_point, _) = find_font_picker_action(&import_host, |action| {
        matches!(action, PropertyPanelAction::ImportFont)
    });
    assert!(import_host.apply_press(import_point.x, import_point.y, VIEWPORT_W, VIEWPORT_H,));
    assert!(!import_host.editor_state().editor_ui.pending_font_import);
    assert!(import_host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert!(import_host.editor_state().editor_ui.pending_font_import);

    let mut remove_host = touch_font_picker_host(0);
    remove_host
        .editor_state_mut()
        .editor_ui
        .imported_font_families = std::sync::Arc::new(vec!["Touch Imported".into()]);
    let (remove_point, _) = find_font_picker_action(&remove_host, |action| {
        matches!(action, PropertyPanelAction::RemoveImportedFont(_))
    });
    assert!(remove_host.apply_press(remove_point.x, remove_point.y, VIEWPORT_W, VIEWPORT_H,));
    assert!(remove_host
        .editor_state()
        .editor_ui
        .pending_font_remove
        .is_none());
    assert!(remove_host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        remove_host
            .editor_state()
            .editor_ui
            .pending_font_remove
            .as_deref(),
        Some("Touch Imported")
    );
}

#[test]
fn desktop_font_picker_keeps_immediate_mouse_selection() {
    let mut host = touch_font_picker_host(8);
    host.editor_state_mut().editor_ui.touch = false;
    host.editor_state_mut().editor_ui.size_class = EditorSizeClass::Expanded;
    host.editor_state_mut().editor_ui.mobile_sheet = None;
    let (point, _) = find_font_picker_action(&host, |action| {
        matches!(action, PropertyPanelAction::SetFontFamilyIndex(_))
    });

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_none());
    assert!(!host.editor_state().editor_ui.font_picker.open);
}

#[test]
fn native_gesture_cancellation_drops_pending_font_picker_tap() {
    for cancellation in 0..5 {
        let mut host = touch_font_picker_host(20);
        let (point, _) = find_font_picker_action(&host, |action| {
            matches!(action, PropertyPanelAction::SetFontFamilyIndex(_))
        });
        let before = selected_text_family(&host);
        assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
        assert!(host.touch_panel_gesture.is_some());

        match cancellation {
            0 => {
                let _ = host.apply_right_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H);
            }
            1 => {
                let _ =
                    host.apply_pan_gesture(point.x, point.y, 0.0, -20.0, VIEWPORT_W, VIEWPORT_H);
            }
            2 => {
                let _ = host.apply_pinch_gesture(point.x, point.y, -20.0, VIEWPORT_W, VIEWPORT_H);
            }
            3 => assert!(host.apply_escape()),
            _ => host.toggle_mobile_sheet(MobileSheetKind::Properties),
        }
        assert!(host.touch_panel_gesture.is_none());
        let _ = host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H);
        assert_eq!(selected_text_family(&host), before);
    }
}

#[test]
fn layer_drag_promotes_to_scroll_without_reorder_candidate() {
    let mut host = touch_host(40, MobileSheetKind::Layers);
    let point = find_layer_row_point(&host);
    let before: Vec<String> = host
        .editor_state()
        .doc
        .children
        .iter()
        .map(|node| node.base().id.clone())
        .collect();

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_some());
    assert!(
        host.layer_drag.is_none(),
        "touch down must not seed reorder"
    );
    assert!(host.apply_cursor_move(point.x, point.y - 24.0));
    assert!(
        host.layer_drag.is_none(),
        "touch scroll must never start reorder"
    );
    assert!(host.editor_state().editor_ui.layer_layers_scroll.offset > 0.0);
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    let after: Vec<String> = host
        .editor_state()
        .doc
        .children
        .iter()
        .map(|node| node.base().id.clone())
        .collect();
    assert_eq!(after, before);
}

#[test]
fn compact_slides_sheet_delays_a_tap_and_activates_it_once() {
    let mut host = touch_slides_host(12);
    assert!(!host.editor_state().editor_ui.sidebar_open);
    assert_eq!(
        host.editor_state().editor_ui.mobile_sheet,
        Some(MobileSheetKind::Layers)
    );
    let point = slide_row_point(&mut host, 1);
    let before = host.editor_state().viewport;

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_some());
    assert_eq!(host.editor_state().editor_ui.slides_panel.pressed, None);
    assert_eq!(host.editor_state().editor_ui.slides_panel.drag, None);
    assert!(!host.apply_cursor_move(point.x + 4.0, point.y + 4.0));
    assert_eq!(
        host.editor_state().viewport,
        before,
        "a deferred down must not frame the board"
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_none());
    assert_eq!(host.editor_state().editor_ui.slides_panel.pressed, None);
    assert_eq!(host.editor_state().editor_ui.slides_panel.drag, None);
    let after = host.editor_state().viewport;
    assert_ne!(after, before, "the stationary release frames slide 2");

    let _ = host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H);
    assert_eq!(
        host.editor_state().viewport,
        after,
        "the replayed slide tap cannot activate a second time"
    );
}

#[test]
fn compact_slides_drag_scrolls_without_seeding_or_committing_reorder() {
    let mut host = touch_slides_host(12);
    let point = slide_row_point(&mut host, 0);
    let order_before = op_editor_core::preview_slideshow::active_page_boards(host.editor_state());
    let viewport_before = host.editor_state().viewport;

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.slides_panel.pressed, None);
    assert_eq!(host.editor_state().editor_ui.slides_panel.drag, None);
    assert!(host.apply_cursor_move(point.x, point.y - 24.0));
    assert!(host.editor_state().editor_ui.slides_panel.scroll.offset > 0.0);
    assert_eq!(
        host.editor_state().editor_ui.slides_panel.drag,
        None,
        "one-finger scrolling never creates a SlidesDrag"
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        op_editor_core::preview_slideshow::active_page_boards(host.editor_state()),
        order_before
    );
    assert_eq!(
        host.editor_state().viewport,
        viewport_before,
        "a scroll release must not frame the pending row"
    );
    assert_eq!(host.editor_state().editor_ui.slides_panel.pressed, None);
    assert_eq!(host.editor_state().editor_ui.slides_panel.drag, None);
}

#[test]
fn right_pan_pinch_escape_and_sheet_close_cancel_pending_slides_tap() {
    for cancellation in 0..5 {
        let mut host = touch_slides_host(12);
        let point = slide_row_point(&mut host, 0);
        assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
        assert!(host.touch_panel_gesture.is_some());

        match cancellation {
            0 => {
                let _ = host.apply_right_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H);
            }
            1 => {
                let _ =
                    host.apply_pan_gesture(point.x, point.y, 0.0, -20.0, VIEWPORT_W, VIEWPORT_H);
            }
            2 => {
                let _ = host.apply_pinch_gesture(point.x, point.y, -20.0, VIEWPORT_W, VIEWPORT_H);
            }
            3 => assert!(host.apply_escape()),
            _ => host.toggle_mobile_sheet(MobileSheetKind::Layers),
        }
        assert!(host.touch_panel_gesture.is_none());
        assert_eq!(host.editor_state().editor_ui.slides_panel.pressed, None);
        assert_eq!(host.editor_state().editor_ui.slides_panel.drag, None);
    }
}

#[test]
fn right_pan_pinch_escape_and_sheet_close_cancel_pending_panel_tap() {
    for cancellation in 0..5 {
        let mut host = touch_host(10, MobileSheetKind::Layers);
        let point = find_layer_row_point(&host);
        assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
        host.layer_drag = Some(LayerDragState {
            source: NodeId::new("n0"),
            start_y: point.y,
            current_x: point.x,
            current_y: point.y,
            active: false,
        });

        match cancellation {
            0 => {
                let _ = host.apply_right_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H);
            }
            1 => {
                let _ =
                    host.apply_pan_gesture(point.x, point.y, 0.0, -20.0, VIEWPORT_W, VIEWPORT_H);
            }
            2 => {
                let _ = host.apply_pinch_gesture(point.x, point.y, -20.0, VIEWPORT_W, VIEWPORT_H);
            }
            3 => assert!(host.apply_escape()),
            _ => host.toggle_mobile_sheet(MobileSheetKind::Layers),
        }
        assert!(host.touch_panel_gesture.is_none());
        assert!(host.layer_drag.is_none());
    }
}

#[test]
fn defensive_touch_reorder_candidate_requires_more_than_twelve_points() {
    let mut host = touch_host(10, MobileSheetKind::Layers);
    let point = find_layer_row_point(&host);
    host.layer_drag = Some(LayerDragState {
        source: NodeId::new("n0"),
        start_y: point.y,
        current_x: point.x,
        current_y: point.y,
        active: false,
    });

    assert!(host.apply_cursor_move(point.x, point.y + 10.0));
    assert!(!host.layer_drag.as_ref().expect("candidate").active);
    assert!(host.apply_cursor_move(point.x, point.y + 13.0));
    assert!(host.layer_drag.as_ref().expect("active drag").active);
}
