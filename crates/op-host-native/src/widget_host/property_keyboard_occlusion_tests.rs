use super::WidgetHostNative;
use op_editor_core::editor_ui_state::EffectParamFocus;
use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
use op_editor_core::{
    EditorState, EffectField, FontPickerPurpose, MissingFontSurface, NodeId, PropertyFocus,
};
use op_editor_ui::widgets::{host_canvas_geometry, PropertyPanelAction};

#[derive(Clone, Copy)]
enum KeyboardOwner {
    Property,
    Effect,
    Image,
    Font,
}

fn touch_property_host(
    size_class: EditorSizeClass,
    mobile_sheet: Option<MobileSheetKind>,
) -> WidgetHostNative {
    let mut state = EditorState::sample();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = size_class;
    state.editor_ui.mobile_sheet = mobile_sheet;
    state.set_single_selection(NodeId::new("n10"));
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host
}

fn assert_property_keyboard_cap(
    mut host: WidgetHostNative,
    viewport_w: f32,
    viewport_h: f32,
    keyboard_height: f32,
) {
    let base =
        host_canvas_geometry::property_panel_rect(host.editor_state(), viewport_w, viewport_h);
    let canvas = host_canvas_geometry::canvas_rect(host.editor_state(), viewport_w, viewport_h);
    assert!(host.set_keyboard_occlusion(keyboard_height));
    assert_eq!(host.property_rect(viewport_w, viewport_h), base);

    for owner in [
        KeyboardOwner::Property,
        KeyboardOwner::Effect,
        KeyboardOwner::Image,
        KeyboardOwner::Font,
    ] {
        clear_keyboard_owners(&mut host);
        set_keyboard_owner(&mut host, owner);
        let capped = host.property_rect(viewport_w, viewport_h);
        let visible_bottom = host.keyboard_visible_bottom(viewport_h);
        assert_eq!(capped.origin, base.origin);
        assert_eq!(capped.size.x, base.size.x);
        assert_eq!(capped.origin.y + capped.size.y, visible_bottom);
        assert!(capped.size.y < base.size.y);
        assert_eq!(
            host.mobile_sheet_rect(viewport_w, viewport_h, MobileSheetKind::Properties),
            capped
        );
        assert_eq!(
            host_canvas_geometry::canvas_rect(host.editor_state(), viewport_w, viewport_h),
            canvas
        );
    }

    clear_keyboard_owners(&mut host);
    set_keyboard_owner(&mut host, KeyboardOwner::Property);
    let capped = host.property_rect(viewport_w, viewport_h);
    let visible_bottom = host.keyboard_visible_bottom(viewport_h);
    let x = capped.origin.x + capped.size.x / 2.0;
    assert!(host.try_scroll_property_panel(
        x,
        capped.origin.y + capped.size.y - 2.0,
        24.0,
        viewport_w,
        viewport_h,
    ));
    assert!(!host.try_scroll_property_panel(x, visible_bottom + 2.0, 24.0, viewport_w, viewport_h,));
}

fn clear_keyboard_owners(host: &mut WidgetHostNative) {
    let state = host.editor_state_mut();
    state.ui.property_focus = None;
    state.editor_ui.effect_param_focus = None;
    state.editor_ui.image_panel.search_open = false;
    state.editor_ui.image_panel.generate_open = false;
    state.editor_ui.font_picker.open = false;
}

fn set_keyboard_owner(host: &mut WidgetHostNative, owner: KeyboardOwner) {
    let state = host.editor_state_mut();
    match owner {
        KeyboardOwner::Property => state.ui.property_focus = Some(PropertyFocus::PositionX),
        KeyboardOwner::Effect => {
            state.editor_ui.effect_param_focus = Some(EffectParamFocus {
                effect: 0,
                field: EffectField::Radius,
            });
        }
        KeyboardOwner::Image => state.editor_ui.image_panel.search_open = true,
        KeyboardOwner::Font => {
            state.editor_ui.font_picker.open = true;
            state.editor_ui.font_picker_purpose = Some(FontPickerPurpose::PropertyText);
        }
    }
}

fn assert_focused_field_revealed(
    mut host: WidgetHostNative,
    viewport_w: f32,
    viewport_h: f32,
    keyboard_height: f32,
) {
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    assert!(host.set_keyboard_occlusion(keyboard_height));
    let initial_offset = host.editor_state().editor_ui.property_panel_scroll.offset;
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::SizeW);
    assert!(host.reveal_property_keyboard_owner());
    let offset = host.editor_state().editor_ui.property_panel_scroll.offset;
    assert!(
        offset > initial_offset,
        "switching fields while the keyboard is already open must reveal the lower row"
    );
    let rect = host.property_rect(viewport_w, viewport_h);
    let next = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .and_then(|panel| panel.keyboard_owner_scroll_offset(rect))
        .expect("focused property row");
    assert!(
        (next - offset).abs() < 0.01,
        "focused row reveal must settle in one pass"
    );
}

fn assert_action_reveals_keyboard_owner(
    mut host: WidgetHostNative,
    action: PropertyPanelAction,
    viewport_w: f32,
    viewport_h: f32,
    keyboard_height: f32,
) -> WidgetHostNative {
    let action_name = format!("{action:?}");
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    assert!(host.set_keyboard_occlusion(keyboard_height));
    host.apply_property_action(action);
    let offset = host.editor_state().editor_ui.property_panel_scroll.offset;
    let rect = host.property_rect(viewport_w, viewport_h);
    let panel = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .expect("Property panel");
    let owner = panel
        .keyboard_owner_rect(rect)
        .unwrap_or_else(|| panic!("{action_name} must expose a keyboard owner"));
    assert!(
        owner.origin.y >= rect.origin.y
            && owner.origin.y + owner.size.y <= rect.origin.y + rect.size.y,
        "{action_name} input must be inside the keyboard-safe Property viewport: {owner:?} vs {rect:?}"
    );
    let next = panel
        .keyboard_owner_scroll_offset(rect)
        .expect("open Property overlay keyboard owner");
    assert!(
        (next - offset).abs() < 0.01,
        "action-tail reveal must settle in one pass"
    );
    host
}

fn touch_image_host() -> WidgetHostNative {
    let mut state = EditorState::sample();
    let image = state
        .insert_image_node_at_viewport("Hero", "https://example.invalid/hero.png")
        .expect("inserted image node");
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.mobile_sheet = Some(MobileSheetKind::Properties);
    state.set_single_selection(image);
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host
}

#[test]
fn open_property_overlays_reveal_when_keyboard_height_is_already_known() {
    let viewport_w = 390.0;
    let viewport_h = 844.0;
    let keyboard_height = 300.0;

    let mut font_host =
        touch_property_host(EditorSizeClass::Compact, Some(MobileSheetKind::Properties));
    font_host
        .editor_state_mut()
        .set_single_selection(NodeId::new("n11"));
    let font = assert_action_reveals_keyboard_owner(
        font_host,
        PropertyPanelAction::ToggleFontFamilyPicker,
        viewport_w,
        viewport_h,
        keyboard_height,
    );
    assert!(font.editor_state().editor_ui.font_picker.open);
    assert_eq!(
        font.editor_state().editor_ui.font_picker_purpose,
        Some(FontPickerPurpose::PropertyText)
    );

    let search = assert_action_reveals_keyboard_owner(
        touch_image_host(),
        PropertyPanelAction::ToggleImageSearchPopover,
        viewport_w,
        viewport_h,
        keyboard_height,
    );
    assert!(search.editor_state().editor_ui.image_panel.search_open);

    let mut generate_host = touch_image_host();
    let profile = generate_host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    generate_host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .image_gen_profiles
        .iter_mut()
        .find(|candidate| candidate.id == profile)
        .expect("new image-generation profile")
        .api_key = "test-key".into();
    let generate = assert_action_reveals_keyboard_owner(
        generate_host,
        PropertyPanelAction::ToggleImageGeneratePopover,
        viewport_w,
        viewport_h,
        keyboard_height,
    );
    assert!(generate.editor_state().editor_ui.image_panel.generate_open);
}

#[test]
fn settings_font_picker_does_not_own_the_property_surface() {
    let mut host = touch_property_host(EditorSizeClass::Medium, Some(MobileSheetKind::Properties));
    let viewport_w = 834.0;
    let viewport_h = 1_112.0;
    let base =
        host_canvas_geometry::property_panel_rect(host.editor_state(), viewport_w, viewport_h);
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.font_picker.open = true;
        ui.font_picker_purpose = Some(FontPickerPurpose::MissingFont {
            row: 0,
            surface: MissingFontSurface::Settings,
        });
    }
    assert!(host.set_keyboard_occlusion(360.0));
    assert_eq!(host.property_rect(viewport_w, viewport_h), base);
}

#[test]
fn compact_property_panel_caps_and_reveals_the_focused_field() {
    let mut host = touch_property_host(EditorSizeClass::Compact, Some(MobileSheetKind::Properties));
    let viewport_w = 390.0;
    let viewport_h = 844.0;
    let keyboard_height = 300.0;
    let canvas = host_canvas_geometry::canvas_rect(host.editor_state(), viewport_w, viewport_h);
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    assert!(host.set_keyboard_occlusion(keyboard_height));
    let rect = host.property_rect(viewport_w, viewport_h);
    assert_eq!(rect.origin.x, 0.0);
    assert_eq!(rect.size.x, viewport_w);
    assert!((rect.origin.y - 54.48).abs() < 0.01);
    assert_eq!(rect.origin.y + rect.size.y, viewport_h - keyboard_height);
    assert_eq!(
        host_canvas_geometry::canvas_rect(host.editor_state(), viewport_w, viewport_h),
        canvas
    );
    assert_focused_field_revealed(
        touch_property_host(EditorSizeClass::Compact, Some(MobileSheetKind::Properties)),
        390.0,
        844.0,
        300.0,
    );
}

#[test]
fn compact_landscape_property_sheet_reanchors_above_keyboard() {
    let viewport_w = 844.0;
    let viewport_h = 390.0;
    let keyboard_height = 220.0;
    let mut host = touch_property_host(EditorSizeClass::Compact, Some(MobileSheetKind::Properties));
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let canvas = host_canvas_geometry::canvas_rect(host.editor_state(), viewport_w, viewport_h);
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    assert!(host.set_keyboard_occlusion(keyboard_height));

    let rect = host.property_rect(viewport_w, viewport_h);
    let app_bar_bottom = host_canvas_geometry::touch_app_bar_height(host.editor_state());
    assert_eq!(rect.origin.y, app_bar_bottom);
    assert_eq!(rect.origin.y + rect.size.y, viewport_h - keyboard_height);
    assert!(
        rect.size.y >= 100.0,
        "header and input body must remain usable"
    );
    let close = op_editor_ui::widgets::mobile_chrome::sheet_close_rect(rect);
    assert!(rect.contains(op_editor_ui::Point2D::new(
        close.origin.x + close.size.x / 2.0,
        close.origin.y + close.size.y / 2.0,
    )));
    assert_eq!(
        host.mobile_sheet_rect(viewport_w, viewport_h, MobileSheetKind::Properties),
        rect
    );
    assert_eq!(
        host_canvas_geometry::canvas_rect(host.editor_state(), viewport_w, viewport_h),
        canvas
    );
    let panel = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .expect("Property panel");
    let owner = panel
        .keyboard_owner_rect(rect)
        .expect("focused Property input");
    assert!(owner.origin.y >= rect.origin.y);
    assert!(owner.origin.y + owner.size.y <= rect.origin.y + rect.size.y);
}

#[test]
fn viewport_publish_reveals_property_owner_against_new_geometry() {
    let portrait = (390.0, 844.0);
    let landscape = (844.0, 390.0);
    let keyboard_height = 220.0;
    let mut host = touch_property_host(EditorSizeClass::Compact, Some(MobileSheetKind::Properties));
    host.publish_viewport_geometry(portrait.0, portrait.1);
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::StrokeWidth);
    assert!(host.set_keyboard_occlusion(keyboard_height));
    let portrait_offset = host.editor_state().editor_ui.property_panel_scroll.offset;

    let landscape_rect = host.property_rect(landscape.0, landscape.1);
    let expected = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .and_then(|panel| panel.keyboard_owner_scroll_offset(landscape_rect))
        .expect("focused stroke field");
    assert!(
        (expected - portrait_offset).abs() > 0.01,
        "the transition fixture must require a fresh keyboard reveal"
    );

    host.publish_viewport_geometry(landscape.0, landscape.1);
    let offset = host.editor_state().editor_ui.property_panel_scroll.offset;
    assert!(
        (expected - offset).abs() < 0.01,
        "viewport publication must settle the owner against the new geometry"
    );
    let panel = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .expect("Property panel");
    let owner = panel
        .keyboard_owner_rect(landscape_rect)
        .expect("focused stroke field");
    assert!(owner.origin.y >= landscape_rect.origin.y);
    assert!(owner.origin.y + owner.size.y <= landscape_rect.origin.y + landscape_rect.size.y);
}

#[test]
fn medium_property_panel_caps_at_keyboard_without_moving_its_top_or_width() {
    assert_property_keyboard_cap(
        touch_property_host(EditorSizeClass::Medium, Some(MobileSheetKind::Properties)),
        834.0,
        1_112.0,
        360.0,
    );
    assert_focused_field_revealed(
        touch_property_host(EditorSizeClass::Medium, Some(MobileSheetKind::Properties)),
        834.0,
        1_112.0,
        700.0,
    );
}

#[test]
fn expanded_property_panel_caps_at_keyboard_without_moving_its_top_or_width() {
    assert_property_keyboard_cap(
        touch_property_host(EditorSizeClass::Expanded, None),
        1_194.0,
        834.0,
        280.0,
    );
    assert_focused_field_revealed(
        touch_property_host(EditorSizeClass::Expanded, None),
        1_194.0,
        834.0,
        500.0,
    );
}
