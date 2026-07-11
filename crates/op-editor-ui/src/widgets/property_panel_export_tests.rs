use super::property_panel::{PropertyPanel, PropertyPanelAction};
use super::property_panel_sections as sections;
use super::property_panel_test_support::visible_for;
use crate::Rect;
use op_editor_core::{EditorState, NodeId};

#[test]
fn export_format_picker_lists_svg_after_webp() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n10"));
    state.editor_ui.export_format_picker_open = true;
    let panel = PropertyPanel::for_selection(&state).expect("frame panel");
    let formats: Vec<_> = sections::action_button_rects_with_fill_picker(
        Rect::xywh(0.0, 0.0, 280.0, 1600.0),
        visible_for(&panel),
        &panel.snapshot.effects,
        &panel.snapshot.fills,
        false,
        0,
        false,
        false,
        false,
        true,
        false,
    )
    .into_iter()
    .filter_map(|(action, _)| match action {
        PropertyPanelAction::SetExportFormat(format) => Some(format),
        _ => None,
    })
    .collect();

    assert_eq!(formats, op_editor_core::ExportFormat::ALL[..4]);
}
