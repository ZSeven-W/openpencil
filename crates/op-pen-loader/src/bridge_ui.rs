//! `EditorUiState` + `UiDraftState` → shell-core `UiState`.
//!
//! `op-editor-core` deliberately splits the editor's UI state across
//! two structs to keep the wasm-clean state layer tidy:
//!
//!   - `EditorUiState` — the ~38 chrome / panel / menu fields.
//!   - `UiDraftState` — the transient edit subset: property focus +
//!     drafts, pen path, color picker, text-edit, the variable caches,
//!     `active_page_index`, `property_focus`.
//!
//! `openpencil-shell-core::document::UiState` collapses both into one
//! struct. This module merges them back, field-by-field.
//!
//! ### Fields with no `op-editor-core` source
//!
//! Three `UiState` fields are pre-edit *history snapshots*
//! (`pending_pen_history`, `pending_text_edit_history`, and
//! `ColorPickerState::pre_snap`). They carry a `DocumentSnapshot` —
//! shell-core's undo-stack entry type. `op-editor-core` models the
//! same snapshot as `EditorSnapshot`, a structurally different type;
//! there is no faithful `EditorSnapshot → DocumentSnapshot`
//! conversion, and a derived paint `Document` never runs undo anyway.
//! They are defaulted to `None` with a one-line note at each site.

use op_editor_core as ec;
use openpencil_shell_core as sc;

use crate::bridge_enums;

/// Merge `op-editor-core`'s two UI structs into shell-core's single
/// `UiState`. `editor_ui` supplies the chrome / panel / menu fields;
/// `draft` supplies the transient edit subset.
pub fn editor_ui_to_ui_state(
    editor_ui: &ec::EditorUiState,
    draft: &ec::UiDraftState,
) -> sc::document::UiState {
    sc::document::UiState {
        // --- Sidebar + panel metrics (EditorUiState) ---------------
        sidebar_open: editor_ui.sidebar_open,
        layer_panel_width: editor_ui.layer_panel_width,
        property_panel_width: editor_ui.property_panel_width,

        // --- Property-panel input editing (UiDraftState) -----------
        property_focus: draft.property_focus.map(bridge_enums::property_focus),
        property_input_draft: draft.property_input_draft.clone(),
        property_caret_anchor_ms: draft.property_caret_anchor_ms,
        property_draft_select_all: draft.property_draft_select_all,

        // --- Settings-modal input draft (EditorUiState) ------------
        settings_input_draft: editor_ui.settings_input_draft.clone(),

        // --- Theme + locale (EditorUiState) ------------------------
        theme_mode: bridge_enums::theme_mode(editor_ui.theme_mode),
        locale: bridge_enums::locale(editor_ui.locale),
        locale_picker_open: editor_ui.locale_picker_open,
        locale_picker_hover: editor_ui.locale_picker_hover.map(bridge_enums::locale),

        // --- AI chat model picker (EditorUiState) ------------------
        chat_model_picker_open: editor_ui.chat_model_picker_open,
        chat_selected_agent: editor_ui.chat_selected_agent,

        // --- File menu (EditorUiState) -----------------------------
        file_menu_open: editor_ui.file_menu_open,
        file_menu_hover: editor_ui
            .file_menu_hover
            .map(bridge_enums::file_menu_choice),
        pending_file_action: editor_ui
            .pending_file_action
            .map(file_action),
        recent_files: editor_ui
            .recent_files
            .iter()
            .map(|r| sc::document::RecentFile {
                path: r.path.clone(),
                modified_at: r.modified_at,
            })
            .collect(),
        file_name_display: editor_ui.file_name_display.clone(),

        // --- Toolbar shape slot (EditorUiState) --------------------
        shape_picker_open: editor_ui.shape_picker_open,
        shape_picker_hover: editor_ui
            .shape_picker_hover
            .map(bridge_enums::shape_choice),
        shape_tool: bridge_enums::tool(editor_ui.shape_tool),

        // --- Alignment toolbar (EditorUiState) ---------------------
        align_toolbar_hover: editor_ui
            .align_toolbar_hover
            .map(bridge_enums::align_action),

        // --- Modals (EditorUiState) --------------------------------
        export_scale: editor_ui.export_scale,
        export_dialog_open: editor_ui.export_dialog_open,
        export_format: bridge_enums::export_format(editor_ui.export_format),
        figma_import_open: editor_ui.figma_import_open,
        agent_settings_open: editor_ui.agent_settings_open,
        agent_settings: bridge_enums::agent_settings(editor_ui.agent_settings),
        // `AgentSettingsDrag` is the modal's drag state. shell-core's
        // variant is `Reserved` (the modal does not drag yet); the
        // editor-core side is identically a single `Reserved` variant.
        agent_settings_drag: editor_ui
            .agent_settings_drag
            .map(|d| match d {
                ec::AgentSettingsDrag::Reserved => {
                    sc::document::AgentSettingsDrag::Reserved
                }
            }),

        // --- Property panel: tabs + layout toggles (EditorUiState) -
        property_tab: bridge_enums::property_tab(editor_ui.property_tab),
        flex_layout: bridge_enums::flex_layout(editor_ui.flex_layout),
        size_fill_width: editor_ui.size_fill_width,
        size_fill_height: editor_ui.size_fill_height,
        size_hug_width: editor_ui.size_hug_width,
        size_hug_height: editor_ui.size_hug_height,
        size_clip_content: editor_ui.size_clip_content,
        fill_type_picker_open: editor_ui.fill_type_picker_open,
        axis_dropdown_open: editor_ui.axis_dropdown_open.clone(),
        variable_row_focus: editor_ui
            .variable_row_focus
            .map(bridge_enums::variable_row_focus),

        // --- Layer / page hover + context menu ---------------------
        hovered_layer_id: editor_ui
            .hovered_layer_id
            .as_ref()
            .map(node_id),
        hovered_page_index: editor_ui.hovered_page_index,
        layer_context_menu: editor_ui
            .layer_context_menu
            .as_ref()
            .map(layer_context_menu),
        // Inline rename lives on `UiDraftState` in op-editor-core.
        layer_rename: draft.layer_rename.as_ref().map(layer_rename),
        rename_caret_anchor_ms: editor_ui.rename_caret_anchor_ms,
        last_layer_click: editor_ui
            .last_layer_click
            .as_ref()
            .map(|(t, ms)| (layer_context_target(t), *ms)),

        // --- Canvas text edit --------------------------------------
        // `text_editing` is transient draft state; `last_canvas_click`
        // is a chrome field on `EditorUiState`.
        text_editing: draft.text_editing.as_ref().map(node_id),
        text_edit_caret_anchor_ms: draft.text_edit_caret_anchor_ms,
        last_canvas_click: editor_ui
            .last_canvas_click
            .as_ref()
            .map(|(id, ms)| (node_id(id), *ms)),
        // No `EditorSnapshot → DocumentSnapshot` conversion exists;
        // a derived paint `Document` never runs undo. Defaulted.
        pending_text_edit_history: None,
        text_edit_last_ms: draft.text_edit_last_ms,

        // --- Pen tool (UiDraftState) -------------------------------
        pen_in_progress: draft.pen_in_progress.as_ref().map(node_id),
        pen_cursor_doc: draft.pen_cursor_doc.map(point2d),
        // Pre-pen history snapshot — see the module note. Defaulted.
        pending_pen_history: None,

        // --- Color picker (UiDraftState) ---------------------------
        color_picker: draft.color_picker.as_ref().map(color_picker),
    }
}

/// `op_editor_core::FileAction` → `shell-core` `FileAction`.
fn file_action(a: ec::FileAction) -> sc::document::FileAction {
    match a {
        ec::FileAction::New => sc::document::FileAction::New,
        ec::FileAction::Open => sc::document::FileAction::Open,
        ec::FileAction::Save => sc::document::FileAction::Save,
        ec::FileAction::SaveAs => sc::document::FileAction::SaveAs,
        ec::FileAction::ExportImage => sc::document::FileAction::ExportImage,
        ec::FileAction::ExportImageConfirm => {
            sc::document::FileAction::ExportImageConfirm
        }
        ec::FileAction::ImportFigma => sc::document::FileAction::ImportFigma,
        ec::FileAction::OpenRecent(i) => sc::document::FileAction::OpenRecent(i),
        ec::FileAction::ClearRecent => sc::document::FileAction::ClearRecent,
    }
}

/// `op_editor_core::NodeId` → `shell-core` `NodeId`. Both are string
/// newtypes; a real (non-empty) id maps straight across. The empty
/// sentinel maps to `NodeId::NONE`.
pub(crate) fn node_id(id: &ec::NodeId) -> sc::document::NodeId {
    match sc::document::NodeId::new_opt(id.as_str()) {
        Some(real) => real,
        None => sc::document::NodeId::NONE,
    }
}

/// `glam::Vec2` point → shell-core `Point2D` (same `glam::Vec2`
/// alias, but the two crates name it through different paths).
fn point2d(p: ec::render_backend::Point2D) -> sc::Point2D {
    sc::Point2D::new(p.x, p.y)
}

/// `op_editor_core::LayerContextTarget` → shell-core's.
fn layer_context_target(
    t: &ec::LayerContextTarget,
) -> sc::document::LayerContextTarget {
    match t {
        ec::LayerContextTarget::Layer(id) => {
            sc::document::LayerContextTarget::Layer(node_id(id))
        }
        ec::LayerContextTarget::Page(i) => {
            sc::document::LayerContextTarget::Page(*i)
        }
    }
}

/// `op_editor_core::LayerContextMenuState` → shell-core's.
fn layer_context_menu(
    m: &ec::LayerContextMenuState,
) -> sc::document::LayerContextMenuState {
    sc::document::LayerContextMenuState {
        target: layer_context_target(&m.target),
        anchor_x: m.anchor_x,
        anchor_y: m.anchor_y,
        hovered_row: m.hovered_row,
    }
}

/// `op_editor_core::LayerRenameState` → shell-core's.
fn layer_rename(r: &ec::LayerRenameState) -> sc::document::LayerRenameState {
    sc::document::LayerRenameState {
        target: layer_context_target(&r.target),
        draft: r.draft.clone(),
    }
}

/// `op_editor_core::ColorPickerState` → shell-core's. The op-editor
/// side has no `pre_snap` field (it stashes the pre-edit
/// `EditorSnapshot` separately on `UiDraftState::pending_color_history`,
/// so the picker stays a plain value type); `pre_snap` is defaulted.
fn color_picker(c: &ec::ColorPickerState) -> sc::document::ColorPickerState {
    sc::document::ColorPickerState {
        target: match c.target {
            ec::ColorTarget::Fill => sc::document::ColorTarget::Fill,
            ec::ColorTarget::Stroke => sc::document::ColorTarget::Stroke,
        },
        hue: c.hue,
        sat: c.sat,
        val: c.val,
        // No `EditorSnapshot → DocumentSnapshot` conversion. Defaulted.
        pre_snap: None,
        drag: c.drag.map(|d| match d {
            ec::ColorPickerDrag::SvBox => sc::document::ColorPickerDrag::SvBox,
            ec::ColorPickerDrag::HueSlider => {
                sc::document::ColorPickerDrag::HueSlider
            }
        }),
        anchor_y: c.anchor_y,
        variable: c.variable.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_a_non_default_editor_ui_plus_draft() {
        // A non-default `EditorUiState`.
        let mut eui = ec::EditorUiState::new();
        eui.sidebar_open = false;
        eui.layer_panel_width = 333.0;
        eui.property_panel_width = 444.0;
        eui.theme_mode = ec::ThemeMode::Light;
        eui.locale = ec::Locale::Ja;
        eui.locale_picker_open = true;
        eui.locale_picker_hover = Some(ec::Locale::Ko);
        eui.file_menu_open = true;
        eui.file_menu_hover = Some(ec::FileMenuChoice::Save);
        eui.pending_file_action = Some(ec::FileAction::SaveAs);
        eui.recent_files.push(ec::RecentFile {
            path: "/tmp/a.op".into(),
            modified_at: 1234,
        });
        eui.file_name_display = Some("Design A".into());
        eui.export_scale = 3.0;
        eui.export_dialog_open = true;
        eui.export_format = ec::ExportFormat::Svg;
        eui.figma_import_open = true;
        eui.agent_settings_open = true;
        eui.settings_input_draft = "3200".into();
        eui.shape_picker_open = true;
        eui.shape_picker_hover = Some(ec::ShapeChoice::Tool(ec::Tool::Line));
        eui.shape_tool = ec::Tool::Ellipse;
        eui.chat_model_picker_open = true;
        eui.chat_selected_agent = 2;
        eui.align_toolbar_hover = Some(ec::AlignAction::CenterH);
        eui.property_tab = ec::PropertyTab::Code;
        eui.flex_layout = ec::FlexLayout::Vertical;
        eui.size_fill_width = true;
        eui.size_hug_height = true;
        eui.size_clip_content = true;
        eui.fill_type_picker_open = true;
        eui.axis_dropdown_open = Some("mode".into());
        eui.variable_row_focus = Some(ec::VariableRowFocus::Number(1));
        eui.hovered_layer_id = Some(ec::NodeId::new("n5"));
        eui.hovered_page_index = Some(2);
        eui.layer_context_menu = Some(ec::LayerContextMenuState {
            target: ec::LayerContextTarget::Layer(ec::NodeId::new("n5")),
            anchor_x: 12.0,
            anchor_y: 34.0,
            hovered_row: Some(1),
        });
        eui.rename_caret_anchor_ms = 999;
        eui.last_layer_click =
            Some((ec::LayerContextTarget::Page(0), 555));
        eui.last_canvas_click = Some((ec::NodeId::new("n9"), 777));

        // A non-default `UiDraftState`.
        let mut draft = ec::UiDraftState::new();
        draft.property_focus = Some(ec::PropertyFocus::SizeW);
        draft.property_input_draft = "120".into();
        draft.property_caret_anchor_ms = 88;
        draft.property_draft_select_all = true;
        draft.layer_rename = Some(ec::LayerRenameState {
            target: ec::LayerContextTarget::Layer(ec::NodeId::new("n5")),
            draft: "Renamed".into(),
        });
        draft.text_editing = Some(ec::NodeId::new("n9"));
        draft.text_edit_caret_anchor_ms = 66;
        draft.text_edit_last_ms = 44;
        draft.pen_in_progress = Some(ec::NodeId::new("n12"));
        draft.pen_cursor_doc =
            Some(ec::render_backend::Point2D::new(10.0, 20.0));
        draft.color_picker = Some(ec::ColorPickerState {
            target: ec::ColorTarget::Stroke,
            hue: 180.0,
            sat: 0.5,
            val: 0.75,
            drag: Some(ec::ColorPickerDrag::HueSlider),
            anchor_y: 200.0,
            variable: Some("brand".into()),
        });

        let ui = editor_ui_to_ui_state(&eui, &draft);

        // Chrome fields from `EditorUiState`.
        assert!(!ui.sidebar_open);
        assert_eq!(ui.layer_panel_width, 333.0);
        assert_eq!(ui.property_panel_width, 444.0);
        assert_eq!(ui.theme_mode, sc::document::ThemeMode::Light);
        assert_eq!(ui.locale, sc::document::Locale::Ja);
        assert!(ui.locale_picker_open);
        assert_eq!(ui.locale_picker_hover, Some(sc::document::Locale::Ko));
        assert!(ui.file_menu_open);
        assert_eq!(
            ui.file_menu_hover,
            Some(sc::widgets::file_menu::FileMenuChoice::Save)
        );
        assert_eq!(
            ui.pending_file_action,
            Some(sc::document::FileAction::SaveAs)
        );
        assert_eq!(ui.recent_files.len(), 1);
        assert_eq!(ui.recent_files[0].path, "/tmp/a.op");
        assert_eq!(ui.recent_files[0].modified_at, 1234);
        assert_eq!(ui.file_name_display.as_deref(), Some("Design A"));
        assert_eq!(ui.export_scale, 3.0);
        assert!(ui.export_dialog_open);
        assert_eq!(
            ui.export_format,
            sc::widgets::export_dialog::ExportFormat::Svg
        );
        assert!(ui.figma_import_open);
        assert!(ui.agent_settings_open);
        assert_eq!(ui.settings_input_draft, "3200");
        assert!(ui.shape_picker_open);
        assert_eq!(
            ui.shape_picker_hover,
            Some(sc::widgets::shape_picker::ShapeChoice::Tool(
                sc::document::Tool::Line
            ))
        );
        assert_eq!(ui.shape_tool, sc::document::Tool::Ellipse);
        assert!(ui.chat_model_picker_open);
        assert_eq!(ui.chat_selected_agent, 2);
        assert_eq!(
            ui.align_toolbar_hover,
            Some(sc::document::AlignAction::CenterH)
        );
        assert_eq!(ui.property_tab, sc::document::PropertyTab::Code);
        assert_eq!(ui.flex_layout, sc::document::FlexLayout::Vertical);
        assert!(ui.size_fill_width);
        assert!(!ui.size_fill_height);
        assert!(ui.size_hug_height);
        assert!(ui.size_clip_content);
        assert!(ui.fill_type_picker_open);
        assert_eq!(ui.axis_dropdown_open.as_deref(), Some("mode"));
        assert_eq!(
            ui.variable_row_focus,
            Some(sc::document::VariableRowFocus::Number(1))
        );
        assert_eq!(
            ui.hovered_layer_id,
            Some(sc::document::NodeId::new("n5"))
        );
        assert_eq!(ui.hovered_page_index, Some(2));
        let menu = ui.layer_context_menu.expect("menu present");
        assert_eq!(menu.anchor_x, 12.0);
        assert_eq!(menu.hovered_row, Some(1));
        assert_eq!(ui.rename_caret_anchor_ms, 999);
        assert_eq!(
            ui.last_layer_click,
            Some((sc::document::LayerContextTarget::Page(0), 555))
        );
        assert_eq!(
            ui.last_canvas_click,
            Some((sc::document::NodeId::new("n9"), 777))
        );

        // Transient edit subset from `UiDraftState`.
        assert_eq!(
            ui.property_focus,
            Some(sc::document::PropertyFocus::SizeW)
        );
        assert_eq!(ui.property_input_draft, "120");
        assert_eq!(ui.property_caret_anchor_ms, 88);
        assert!(ui.property_draft_select_all);
        let rename = ui.layer_rename.expect("rename present");
        assert_eq!(rename.draft, "Renamed");
        assert_eq!(
            ui.text_editing,
            Some(sc::document::NodeId::new("n9"))
        );
        assert_eq!(ui.text_edit_caret_anchor_ms, 66);
        assert_eq!(ui.text_edit_last_ms, 44);
        assert_eq!(
            ui.pen_in_progress,
            Some(sc::document::NodeId::new("n12"))
        );
        let cursor = ui.pen_cursor_doc.expect("pen cursor present");
        assert_eq!(cursor.x, 10.0);
        assert_eq!(cursor.y, 20.0);
        let cp = ui.color_picker.expect("color picker present");
        assert_eq!(cp.target, sc::document::ColorTarget::Stroke);
        assert_eq!(cp.hue, 180.0);
        assert_eq!(cp.drag, Some(sc::document::ColorPickerDrag::HueSlider));
        assert_eq!(cp.variable.as_deref(), Some("brand"));

        // History-snapshot fields have no source — stay defaulted.
        assert!(ui.pending_pen_history.is_none());
        assert!(ui.pending_text_edit_history.is_none());
        assert!(cp.pre_snap.is_none());
    }

    #[test]
    fn default_inputs_yield_a_default_like_ui_state() {
        let eui = ec::EditorUiState::new();
        let draft = ec::UiDraftState::new();
        let ui = editor_ui_to_ui_state(&eui, &draft);
        let def = sc::document::UiState::default();
        assert_eq!(ui.sidebar_open, def.sidebar_open);
        assert_eq!(ui.theme_mode, def.theme_mode);
        assert_eq!(ui.locale, def.locale);
        assert!(ui.property_focus.is_none());
        assert!(ui.color_picker.is_none());
        assert!(ui.recent_files.is_empty());
        assert_eq!(ui.shape_tool, def.shape_tool);
    }

    #[test]
    fn file_action_maps_every_variant() {
        use ec::FileAction as E;
        use sc::document::FileAction as S;
        assert_eq!(file_action(E::New), S::New);
        assert_eq!(file_action(E::Open), S::Open);
        assert_eq!(file_action(E::Save), S::Save);
        assert_eq!(file_action(E::SaveAs), S::SaveAs);
        assert_eq!(file_action(E::ExportImage), S::ExportImage);
        assert_eq!(file_action(E::ExportImageConfirm), S::ExportImageConfirm);
        assert_eq!(file_action(E::ImportFigma), S::ImportFigma);
        assert_eq!(file_action(E::OpenRecent(4)), S::OpenRecent(4));
        assert_eq!(file_action(E::ClearRecent), S::ClearRecent);
    }
}
