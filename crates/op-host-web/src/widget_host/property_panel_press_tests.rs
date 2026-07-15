use super::WidgetHost;
use jian_ops_schema::node::container::{AlignItems, JustifyContent, LayoutMode};
use jian_ops_schema::node::text::{TextAlign, TextGrowth};
use jian_ops_schema::node::BoolOrExpression;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::PenFill;
use op_editor_core::codegen::{CodegenHover, CodegenPhase};
use op_editor_core::image_panel_state::{ImageAssetCheck, ImageAssetStatus};
use op_editor_core::{ButtonPressTarget, NodeId, PropertyFocus};
use op_editor_core::{FlexLayout, PaddingEditMode, PropertyTab};
use op_editor_ui::widgets::property_panel_action::CodegenAction;
use op_editor_ui::widgets::property_panel_action::{
    LayoutAlignValue, LayoutJustifyValue, TextAlignValue, TextGrowthValue,
};
use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn seed(host: &mut WidgetHost, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn property_rect(host: &WidgetHost) -> Rect {
    let width = host.editor_state.editor_ui.property_panel_width;
    Rect {
        origin: Point2D::new(VIEWPORT_W - width, TOP_BAR_HEIGHT),
        size: Point2D::new(width, VIEWPORT_H - TOP_BAR_HEIGHT),
    }
}

fn point_for_action(host: &WidgetHost, want: impl Fn(&PropertyPanelAction) -> bool) -> Point2D {
    let panel = PropertyPanel::for_selection(&host.editor_state).expect("property panel");
    let rect = property_rect(host);
    let mut y = rect.origin.y + 2.0;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x + 2.0;
        while x < rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if panel
                .hit_test_action(rect, point)
                .as_ref()
                .is_some_and(&want)
            {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no property-panel action point maps to requested action");
}

fn point_for_focus(host: &WidgetHost, want: PropertyFocus) -> Point2D {
    let panel = PropertyPanel::for_selection(&host.editor_state).expect("property panel");
    let rect = property_rect(host);
    let mut y = rect.origin.y + 2.0;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x + 2.0;
        while x < rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if panel.hit_test(rect, point) == Some(want) {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no property-panel input point maps to {want:?}");
}

fn point_inside_property_panel_without_target(host: &WidgetHost) -> Point2D {
    let panel = PropertyPanel::for_selection(&host.editor_state).expect("property panel");
    let rect = property_rect(host);
    let mut y = rect.origin.y + rect.size.y - 12.0;
    while y > rect.origin.y {
        let mut x = rect.origin.x + 12.0;
        while x < rect.origin.x + rect.size.x - 12.0 {
            let point = Point2D::new(x, y);
            let no_action = panel.hit_test_action(rect, point).is_none();
            let no_input = panel.hit_test(rect, point).is_none();
            if no_action && no_input {
                return point;
            }
            x += 8.0;
        }
        y -= 8.0;
    }
    panic!("no empty property-panel point found");
}

fn selected_rectangle(host: &WidgetHost) -> &jian_ops_schema::node::RectangleNode {
    match host.editor_state.selected_node() {
        Some(PenNode::Rectangle(rect)) => rect,
        other => panic!("expected selected rectangle, got {other:?}"),
    }
}

fn selected_fills(host: &WidgetHost) -> &[PenFill] {
    let node = host.editor_state.selected_node().expect("selected node");
    op_editor_core::fills::node_fills(node)
        .map(Vec::as_slice)
        .expect("selected node carries fills")
}

fn selected_frame(host: &WidgetHost) -> &jian_ops_schema::node::FrameNode {
    match host.editor_state.selected_node() {
        Some(PenNode::Frame(frame)) => frame,
        other => panic!("expected selected frame, got {other:?}"),
    }
}

fn selected_text(host: &WidgetHost) -> &jian_ops_schema::node::TextNode {
    match host.editor_state.selected_node() {
        Some(PenNode::Text(text)) => text,
        other => panic!("expected selected text, got {other:?}"),
    }
}

fn selected_checkbox(host: &WidgetHost) -> &jian_ops_schema::node::CheckboxNode {
    match host.editor_state.selected_node() {
        Some(PenNode::Checkbox(checkbox)) => checkbox,
        other => panic!("expected selected checkbox, got {other:?}"),
    }
}

fn ref_node<'a>(host: &'a WidgetHost, id: &str) -> &'a jian_ops_schema::node::RefNode {
    match op_editor_core::walkers::find_node(host.editor_state.active_children(), &NodeId::new(id))
    {
        Some(PenNode::Ref(reference)) => reference,
        other => panic!("{id} must stay a Ref, got {other:?}"),
    }
}

fn selected_scene_size(host: &mut WidgetHost) -> (f32, f32) {
    let id = host.editor_state.selection.anchor.as_str().to_string();
    host.refresh_layout_scene();
    let node = host
        .layout_scene
        .active_page()
        .and_then(|page| page.find(&id))
        .expect("selected scene node present");
    (node.bounds.size.x, node.bounds.size.y)
}

#[test]
fn property_panel_action_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));

    let point = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::ToggleSizeFillWidth)
    });
    let panel = PropertyPanel::for_selection(&host.editor_state).expect("property panel");
    let rect = property_rect(&host);
    let expected_index = panel
        .action_hover_index(rect, point)
        .expect("action maps to hover index");

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::PropertyPanel(expected_index))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn web_disabling_fill_height_freezes_resolved_height_then_numeric_input_resizes_scene() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"frame","id":"screen","width":390,"height":710,
               "layout":"vertical","gap":0,"children":[
                 {"type":"frame","id":"content","name":"Content Wrapper",
                  "width":"fill_container","height":"fill_container",
                  "layout":"vertical","children":[
                    {"type":"rectangle","id":"body","width":"fill_container","height":100}
                  ]},
                 {"type":"frame","id":"nav","width":"fill_container","height":94}
               ]}
        ]}"##,
    );
    host.editor_state
        .set_single_selection(NodeId::new("content"));

    assert_eq!(selected_scene_size(&mut host), (390.0, 616.0));
    host.apply_property_action(PropertyPanelAction::ToggleSizeFillHeight);

    let content = selected_frame(&host);
    assert_eq!(
        content.container.height,
        Some(SizingBehavior::Number(616.0)),
        "turning Fill Height off must freeze the current resolved height"
    );
    assert_eq!(selected_scene_size(&mut host), (390.0, 616.0));

    let point = point_for_focus(&host, PropertyFocus::SizeH);
    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.ui.property_input.text(), "616");
    host.editor_state.ui.property_input.set_text("200");
    assert!(host.apply_send());

    assert_eq!(selected_scene_size(&mut host), (390.0, 200.0));
}

#[test]
fn web_property_panel_component_button_creates_and_detaches_component() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"frame","id":"card","name":"Card",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#FFFFFF"}],
               "children":[]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("card"));

    let create = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::CreateComponent)
    });
    assert!(host.apply_press(create.x, create.y, VIEWPORT_W, VIEWPORT_H));
    assert!(selected_frame(&host).reusable.unwrap_or(false));
    assert!(host
        .editor_state
        .components
        .find_by_id(&NodeId::new("card"))
        .is_some());

    let detach = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::DetachComponent)
    });
    assert!(host.apply_press(detach.x, detach.y, VIEWPORT_W, VIEWPORT_H));
    assert!(!selected_frame(&host).reusable.unwrap_or(false));
    assert!(host
        .editor_state
        .components
        .find_by_id(&NodeId::new("card"))
        .is_none());
}

#[test]
fn web_property_panel_group_component_button_switches_to_detach() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"group","id":"text_group","name":"Text Group",
               "children":[
                 {"type":"text","id":"label","name":"Label","content":"Hello"}
               ]}
        ]}"##,
    );
    host.editor_state
        .set_single_selection(NodeId::new("text_group"));

    let create = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::CreateComponent)
    });
    assert!(host.apply_press(create.x, create.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host
        .editor_state
        .components
        .find_by_id(&NodeId::new("text_group"))
        .is_some());

    let detach = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::DetachComponent)
    });
    assert!(host.apply_press(detach.x, detach.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host
        .editor_state
        .components
        .find_by_id(&NodeId::new("text_group"))
        .is_none());
}

#[test]
fn web_property_panel_background_consumes_clicks() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"group","id":"text_group","name":"Text Group",
               "children":[
                 {"type":"text","id":"label","name":"Label","content":"Hello"}
               ]}
        ]}"##,
    );
    host.editor_state
        .set_single_selection(NodeId::new("text_group"));

    let point = point_inside_property_panel_without_target(&host);
    assert!(
        host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H),
        "right inspector should own clicks inside its bounds even when no control is hit"
    );
    assert_eq!(
        host.editor_state.selection.anchor,
        NodeId::new("text_group")
    );
}

#[test]
fn web_property_panel_instance_buttons_go_to_master_and_detach_instance() {
    const COMPONENT_DOC: &str = r##"{ "version":"1.0.0", "children": [
              {"type":"frame","id":"card","name":"Card","reusable":true,
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#FFFFFF"}],
               "children":[{"type":"text","id":"title","name":"Title","content":"Hello"}]},
              {"type":"ref","id":"inst1","ref":"card","x":300,"y":40,
               "descendants":{"card":{"fill":[{"type":"solid","color":"#FF8800"}]}}}
        ]}"##;

    let mut host = WidgetHost::new();
    seed(&mut host, COMPONENT_DOC);
    host.editor_state.set_single_selection(NodeId::new("inst1"));
    let go_to = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::GoToComponent)
    });
    assert!(host.apply_press(go_to.x, go_to.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("card"));

    let mut host = WidgetHost::new();
    seed(&mut host, COMPONENT_DOC);
    host.editor_state.set_single_selection(NodeId::new("inst1"));
    let detach = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::DetachInstance)
    });
    assert!(host.apply_press(detach.x, detach.y, VIEWPORT_W, VIEWPORT_H));
    assert_ne!(host.editor_state.selection.anchor, NodeId::new("inst1"));
    assert!(matches!(
        host.editor_state.selected_node(),
        Some(PenNode::Frame(_))
    ));
}

#[test]
fn web_property_panel_size_action_on_instance_routes_to_descendants_override() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version":"1.0.0", "children": [
              {"type":"frame","id":"card","name":"Card","reusable":true,
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#FFFFFF"}],
               "children":[]},
              {"type":"ref","id":"inst1","ref":"card","x":300,"y":40}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("inst1"));

    let fill_width = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::ToggleSizeFillWidth)
    });
    assert!(host.apply_press(fill_width.x, fill_width.y, VIEWPORT_W, VIEWPORT_H));

    let over = ref_node(&host, "inst1")
        .descendants
        .as_ref()
        .and_then(|d| d.get("card"))
        .expect("width override routed under descendants[card]");
    assert_eq!(
        over.pointer("/width").and_then(|v| v.as_str()),
        Some("fill_container")
    );

    let resolved = op_editor_core::ref_resolve::resolve_refs_for_canvas(&host.editor_state.doc);
    let Some(PenNode::Frame(expanded)) = resolved.children.get(1) else {
        panic!("instance resolves to the component frame");
    };
    assert_eq!(
        expanded.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    );
}

#[test]
fn web_property_panel_layout_actions_write_selected_node() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));

    let vertical = point_for_action(&host, |action| {
        matches!(
            action,
            PropertyPanelAction::SetFlexLayout(FlexLayout::Vertical)
        )
    });
    assert!(host.apply_press(vertical.x, vertical.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        selected_rectangle(&host).container.layout,
        Some(LayoutMode::Vertical)
    );

    let fill_width = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::ToggleSizeFillWidth)
    });
    assert!(host.apply_press(fill_width.x, fill_width.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        selected_rectangle(&host).container.width.as_ref(),
        Some(&SizingBehavior::Keyword(SizingKeyword::FillContainer))
    );

    let align_center = point_for_action(&host, |action| {
        matches!(
            action,
            PropertyPanelAction::SetLayoutAlignment {
                justify: LayoutJustifyValue::Center,
                align: LayoutAlignValue::Center
            }
        )
    });
    assert!(host.apply_press(align_center.x, align_center.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        selected_rectangle(&host).container.justify_content,
        Some(JustifyContent::Center)
    );
    assert_eq!(
        selected_rectangle(&host).container.align_items,
        Some(AlignItems::Center)
    );
}

#[test]
fn web_property_panel_padding_mode_gear_opens_and_selects_mode() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"group","id":"text_group","name":"Text Group",
               "layout":"vertical","gap":8,
               "children":[
                 {"type":"text","id":"label","name":"Label","content":"Hello"}
               ]}
        ]}"##,
    );
    host.editor_state
        .set_single_selection(NodeId::new("text_group"));

    let gear = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::TogglePaddingModePopover)
    });
    assert!(host.apply_press(gear.x, gear.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state.editor_ui.padding_mode_popover_open);

    let axis = point_for_action(&host, |action| {
        matches!(
            action,
            PropertyPanelAction::SetPaddingMode(PaddingEditMode::Axis)
        )
    });
    assert!(host.apply_press(axis.x, axis.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state.editor_ui.padding_edit_mode,
        Some(PaddingEditMode::Axis)
    );
    assert!(!host.editor_state.editor_ui.padding_mode_popover_open);
}

#[test]
fn web_property_panel_fill_add_and_remove_match_native() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"rectangle","id":"n62","name":"No fill",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));
    assert_eq!(selected_fills(&host).len(), 0);

    let add = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::AddFill)
    });
    assert!(host.apply_press(add.x, add.y, VIEWPORT_W, VIEWPORT_H));
    assert!(matches!(
        selected_fills(&host).first(),
        Some(PenFill::Solid(_))
    ));

    let remove = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::RemoveFill(0))
    });
    assert!(host.apply_press(remove.x, remove.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(selected_fills(&host).len(), 0);
}

#[test]
fn web_property_panel_text_layout_actions_match_native() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"text","id":"label","name":"Label","content":"Hello",
               "x":40,"y":40,"width":180,"height":40,
               "fontSize":24}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("label"));

    let center = point_for_action(&host, |action| {
        matches!(
            action,
            PropertyPanelAction::SetTextAlign(TextAlignValue::Center)
        )
    });
    assert!(host.apply_press(center.x, center.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(selected_text(&host).text_align, Some(TextAlign::Center));

    let fixed = point_for_action(&host, |action| {
        matches!(
            action,
            PropertyPanelAction::SetTextGrowth(TextGrowthValue::FixedWidthHeight)
        )
    });
    assert!(host.apply_press(fixed.x, fixed.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        selected_text(&host).text_growth,
        Some(TextGrowth::FixedWidthHeight)
    );
}

#[test]
fn web_property_panel_widget_checked_toggle_matches_native() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"checkbox","id":"cb","name":"Agree",
               "x":40,"y":40,"width":18,"height":18,
               "label":"Accept","checked":false}
        ]}"##,
    );
    host.editor_state
        .editor_ui
        .agent_settings
        .experimental_features_enabled = true;
    host.editor_state.set_single_selection(NodeId::new("cb"));
    assert_eq!(
        selected_checkbox(&host).checked,
        Some(BoolOrExpression::Bool(false))
    );

    let toggle = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::ToggleWidgetChecked(true))
    });
    assert!(host.apply_press(toggle.x, toggle.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        selected_checkbox(&host).checked,
        Some(BoolOrExpression::Bool(true))
    );
}

#[test]
fn web_property_panel_image_actions_match_native_state_intents() {
    let mut host = WidgetHost::new();
    let _ = host
        .editor_state
        .insert_image_node_at_viewport("Hero photo", "./assets/hero.png");
    let selected = host.editor_state.selection.anchor.clone();

    let search = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::ToggleImageSearchPopover)
    });
    assert!(host.apply_press(search.x, search.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state.editor_ui.image_panel.search_open);
    assert_eq!(
        host.editor_state.editor_ui.image_panel.search_query,
        "Hero photo"
    );
    assert!(!host.editor_state.editor_ui.image_panel.generate_open);

    let generate = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::ToggleImageGeneratePopover)
    });
    assert!(host.apply_press(generate.x, generate.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state.editor_ui.image_panel.generate_open);
    assert_eq!(
        host.editor_state.editor_ui.image_panel.generate_prompt,
        "Hero photo"
    );
    assert!(!host.editor_state.editor_ui.image_panel.search_open);

    host.editor_state.editor_ui.image_panel.close_popovers();
    host.editor_state.editor_ui.image_panel.asset_check = Some(ImageAssetCheck {
        node_id: selected.as_str().to_string(),
        src: "./assets/hero.png".into(),
        status: ImageAssetStatus::Missing,
    });
    let relink = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::RelinkImage)
    });
    assert!(host.apply_press(relink.x, relink.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state.editor_ui.pending_file_action,
        Some(op_editor_core::editor_ui_state::FileAction::RelinkImage)
    );
}

#[test]
fn web_image_popover_keyboard_and_outside_press_match_native() {
    let mut host = WidgetHost::new();
    let _ = host
        .editor_state
        .insert_image_node_at_viewport("Hero photo", "https://x/y.png");

    let search = point_for_action(&host, |action| {
        matches!(action, PropertyPanelAction::ToggleImageSearchPopover)
    });
    assert!(host.apply_press(search.x, search.y, VIEWPORT_W, VIEWPORT_H));
    host.editor_state.editor_ui.image_panel.search_query.clear();

    assert!(host.apply_text('c'));
    assert!(host.apply_text('a'));
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state.editor_ui.image_panel.search_query, "c");

    assert!(host.apply_send());
    assert!(host.editor_state.editor_ui.image_panel.search_loading);
    assert!(host.editor_state.editor_ui.image_panel.search_has_searched);

    host.editor_state.editor_ui.image_panel.search_loading = false;
    assert!(host.apply_press(360.0, 120.0, VIEWPORT_W, VIEWPORT_H));
    assert!(!host.editor_state.editor_ui.image_panel.search_open);
}

#[test]
fn codegen_action_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.property_tab = PropertyTab::Code;
    host.editor_state.codegen.phase = CodegenPhase::Complete;
    host.editor_state.codegen.code = "fn main() {\n    println!(\"hi\");\n}\n".into();

    let point = point_for_action(&host, |action| {
        matches!(
            action,
            PropertyPanelAction::Codegen(CodegenAction::Regenerate)
        )
    });

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::Codegen(CodegenHover::Regenerate))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn pick_fill_image_queues_web_file_picker_like_native() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.image_fill_popover_open = true;

    host.apply_property_action(PropertyPanelAction::PickFillImage);

    assert_eq!(
        host.editor_state.editor_ui.pending_file_action,
        Some(op_editor_core::editor_ui_state::FileAction::PickFillImage),
    );
    assert!(
        host.editor_state.editor_ui.image_fill_popover_open,
        "the image popover must stay open so Fill/Fit/Crop/Tile remain selectable",
    );
}
