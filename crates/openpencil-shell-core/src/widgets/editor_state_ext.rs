//! Shell-core-side derivations over `op_editor_core::EditorUiState`.
//!
//! shell-core's `Document` carried `theme()` + `t()` accessors that the
//! widgets relied on. `EditorState` deliberately has no widget-layer
//! concerns, so the same two derivations live here as free functions
//! over the narrowest sub-struct that carries the inputs —
//! `EditorUiState` (which holds `theme_mode` + `locale`).
//!
//! Widgets that were ported off `Document` onto `EditorState` call
//! these instead of the old `doc.theme()` / `doc.t(key)`.

use crate::theme::Theme;
use op_editor_core::editor_ui_state::{EditorUiState, ThemeMode};

/// Resolve the active editor [`Theme`] from the UI theme mode.
/// Mirrors the old `Document::theme()`.
pub fn theme_for(ui: &EditorUiState) -> Theme {
    match ui.theme_mode {
        ThemeMode::Dark => Theme::dark(),
        ThemeMode::Light => Theme::light(),
    }
}

/// Translate a chrome string key against the active UI locale.
/// Mirrors the old `Document::t()`.
pub fn translate(ui: &EditorUiState, key: &'static str) -> &'static str {
    crate::i18n::translate(ui.locale, key)
}

/// Map an `op_editor_core::Tool` onto shell-core's `document::Tool`.
///
/// The two enums are variant-identical (the editor-state layer owns
/// its own `Tool`; widgets still speak `document::Tool` in their
/// hit-test API). This is the single conversion point so widgets
/// ported onto `EditorState` don't each re-implement the 9-arm match.
pub fn doc_tool(t: op_editor_core::Tool) -> crate::document::Tool {
    use crate::document::Tool as D;
    use op_editor_core::Tool as O;
    match t {
        O::Select => D::Select,
        O::Rect => D::Rect,
        O::Ellipse => D::Ellipse,
        O::Polygon => D::Polygon,
        O::Line => D::Line,
        O::Pen => D::Pen,
        O::Text => D::Text,
        O::Frame => D::Frame,
        O::Hand => D::Hand,
    }
}

/// Map an `op_editor_core::ShapeChoice` onto the widget-layer
/// `widgets::shape_picker::ShapeChoice`. The state-layer enum carries
/// `op_editor_core::Tool` in its `Tool` variant; the widget enum
/// carries `document::Tool` — [`doc_tool`] bridges that.
pub fn doc_shape_choice(
    c: op_editor_core::ShapeChoice,
) -> crate::widgets::shape_picker::ShapeChoice {
    use crate::widgets::shape_picker::ShapeChoice as D;
    use op_editor_core::ShapeChoice as O;
    match c {
        O::Tool(t) => D::Tool(doc_tool(t)),
        O::OpenIconPicker => D::OpenIconPicker,
        O::ImportImageOrSvg => D::ImportImageOrSvg,
    }
}

/// Map an `op_editor_core::AlignAction` onto shell-core's
/// `document::align::AlignAction`. Variant-identical; this is the
/// single bridge point for the align-toolbar hover state.
pub fn doc_align_action(a: op_editor_core::AlignAction) -> crate::document::AlignAction {
    use crate::document::AlignAction as D;
    use op_editor_core::AlignAction as O;
    match a {
        O::Left => D::Left,
        O::CenterH => D::CenterH,
        O::Right => D::Right,
        O::Top => D::Top,
        O::CenterV => D::CenterV,
        O::Bottom => D::Bottom,
        O::DistributeH => D::DistributeH,
        O::DistributeV => D::DistributeV,
    }
}

/// Map an `op_editor_core::FileMenuChoice` onto the widget-layer
/// `widgets::file_menu::FileMenuChoice`. Variant-identical; bridges
/// the file-menu hover state.
pub fn doc_file_menu_choice(
    c: op_editor_core::FileMenuChoice,
) -> crate::widgets::file_menu::FileMenuChoice {
    use crate::widgets::file_menu::FileMenuChoice as D;
    use op_editor_core::FileMenuChoice as O;
    match c {
        O::NewFile => D::NewFile,
        O::OpenFile => D::OpenFile,
        O::Save => D::Save,
        O::SaveAs => D::SaveAs,
        O::ExportImage => D::ExportImage,
        O::OpenRecent(i) => D::OpenRecent(i),
        O::ClearRecent => D::ClearRecent,
    }
}

/// Map an `op_editor_core::ExportFormat` onto the widget-layer
/// `widgets::export_dialog::ExportFormat`. Variant-identical.
pub fn doc_export_format(
    f: op_editor_core::ExportFormat,
) -> crate::widgets::export_dialog::ExportFormat {
    use crate::widgets::export_dialog::ExportFormat as D;
    use op_editor_core::ExportFormat as O;
    match f {
        O::Png => D::Png,
        O::Jpeg => D::Jpeg,
        O::Webp => D::Webp,
        O::Svg => D::Svg,
        O::Pdf => D::Pdf,
    }
}
