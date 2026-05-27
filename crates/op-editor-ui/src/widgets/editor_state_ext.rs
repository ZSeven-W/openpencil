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

/// Map an `op_editor_core::ShapeChoice` onto the widget-layer
/// `widgets::shape_picker::ShapeChoice`. The state-layer enum carries
/// `op_editor_core::Tool` in its `Tool` variant; the widget enum
/// carries the same `op_editor_core::Tool` — pass-through.
pub fn doc_shape_choice(
    c: op_editor_core::ShapeChoice,
) -> crate::widgets::shape_picker::ShapeChoice {
    use crate::widgets::shape_picker::ShapeChoice as D;
    use op_editor_core::ShapeChoice as O;
    match c {
        O::Tool(t) => D::Tool(t),
        O::OpenIconPicker => D::OpenIconPicker,
        O::ImportImageOrSvg => D::ImportImageOrSvg,
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

// ── Widget-layer → canonical reverse converters ───────────────────
//
// The host feeds widget hit-test results back into `EditorState`'s
// `editor_ui_state`. Most widget hit-tests already emit canonical
// `op_editor_core` types (`Tool`, `AlignAction`, `PropertyFocus`, …)
// so no conversion is needed. The three enums below stay widget-local
// (`file_menu` / `shape_picker` / `export_dialog` own them) and so
// still need a one-arm-per-variant bridge into the canonical
// `editor_ui_state` enums the hover / format state fields hold.

/// Map the widget-layer `widgets::file_menu::FileMenuChoice` onto the
/// canonical `op_editor_core::FileMenuChoice`. Reverse of
/// [`doc_file_menu_choice`].
pub fn file_menu_choice(
    c: crate::widgets::file_menu::FileMenuChoice,
) -> op_editor_core::FileMenuChoice {
    use crate::widgets::file_menu::FileMenuChoice as W;
    use op_editor_core::FileMenuChoice as O;
    match c {
        W::NewFile => O::NewFile,
        W::OpenFile => O::OpenFile,
        W::Save => O::Save,
        W::SaveAs => O::SaveAs,
        W::ExportImage => O::ExportImage,
        W::OpenRecent(i) => O::OpenRecent(i),
        W::ClearRecent => O::ClearRecent,
    }
}

/// Map the widget-layer `widgets::shape_picker::ShapeChoice` onto the
/// canonical `op_editor_core::ShapeChoice`. Reverse of
/// [`doc_shape_choice`]; the `Tool` variant carries the same
/// `op_editor_core::Tool` either way.
pub fn shape_choice(c: crate::widgets::shape_picker::ShapeChoice) -> op_editor_core::ShapeChoice {
    use crate::widgets::shape_picker::ShapeChoice as W;
    use op_editor_core::ShapeChoice as O;
    match c {
        W::Tool(t) => O::Tool(t),
        W::OpenIconPicker => O::OpenIconPicker,
        W::ImportImageOrSvg => O::ImportImageOrSvg,
    }
}

/// Map the widget-layer `widgets::toolbar::ToolbarAction` onto the
/// canonical `op_editor_core::ToolbarAction`. Variant-identical;
/// bridges the toolbar hover state.
pub fn toolbar_action(a: crate::widgets::toolbar::ToolbarAction) -> op_editor_core::ToolbarAction {
    use crate::widgets::toolbar::ToolbarAction as W;
    use op_editor_core::ToolbarAction as O;
    match a {
        W::Undo => O::Undo,
        W::Redo => O::Redo,
        W::ToggleCodePanel => O::ToggleCodePanel,
        W::ToggleDesignPanel => O::ToggleDesignPanel,
    }
}

/// Map a widget-layer `ToolbarHit` onto the canonical
/// `op_editor_core::ToolbarHover` so the host can store the
/// hovered item on `EditorUiState.toolbar_hover`.
pub fn toolbar_hover(hit: crate::widgets::toolbar::ToolbarHit) -> op_editor_core::ToolbarHover {
    use crate::widgets::toolbar::ToolbarHit as W;
    use op_editor_core::ToolbarHover as O;
    match hit {
        W::Tool(t) => O::Tool(t),
        W::Action(a) => O::Action(toolbar_action(a)),
        W::ToggleShapePicker => O::ShapeSlot,
    }
}

/// Map the widget-layer `widgets::export_dialog::ExportFormat` onto
/// the canonical `op_editor_core::ExportFormat`. Reverse of
/// [`doc_export_format`].
pub fn export_format(
    f: crate::widgets::export_dialog::ExportFormat,
) -> op_editor_core::ExportFormat {
    use crate::widgets::export_dialog::ExportFormat as W;
    use op_editor_core::ExportFormat as O;
    match f {
        W::Png => O::Png,
        W::Jpeg => O::Jpeg,
        W::Webp => O::Webp,
        W::Svg => O::Svg,
        W::Pdf => O::Pdf,
    }
}
