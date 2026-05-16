//! Reverse enum translators for the paint `Document` → `EditorState`
//! bridge.
//!
//! [`crate::bridge_enums`] translates `op-editor-core` enums INTO
//! `openpencil-shell-core` enums (EC→SC) — the direction a host needs
//! to derive a paint snapshot. The native editor-host migration
//! (Task 6.1c-2) also needs the REVERSE: shell-core widget hit-test
//! helpers return shell-core types, which must be fed into
//! `op-editor-core` mutators. This module provides those SC→EC
//! translators.
//!
//! Same constraints as the forward module: `From` impls are forbidden
//! by the orphan rule (both source and target are foreign to this
//! crate), so each pair is a plain `fn`; every translator is an
//! exhaustive `match` with NO wildcard arm so a future variant added
//! to either side becomes a compile error.

use op_editor_core as ec;
use openpencil_shell_core as sc;

/// `shell-core` `NodeId` → `op_editor_core::NodeId`. Both crates model
/// `NodeId` as a string newtype, so this re-wraps the raw string.
pub fn node_id(id: &sc::document::NodeId) -> ec::NodeId {
    ec::NodeId::new(id.raw())
}

/// `shell-core` `Tool` → `op_editor_core::Tool`.
pub fn tool(t: sc::document::Tool) -> ec::Tool {
    match t {
        sc::document::Tool::Select => ec::Tool::Select,
        sc::document::Tool::Rect => ec::Tool::Rect,
        sc::document::Tool::Ellipse => ec::Tool::Ellipse,
        sc::document::Tool::Polygon => ec::Tool::Polygon,
        sc::document::Tool::Line => ec::Tool::Line,
        sc::document::Tool::Pen => ec::Tool::Pen,
        sc::document::Tool::Text => ec::Tool::Text,
        sc::document::Tool::Frame => ec::Tool::Frame,
        sc::document::Tool::Hand => ec::Tool::Hand,
    }
}

/// `shell-core` `ThemeMode` → `op_editor_core::ThemeMode`.
pub fn theme_mode(t: sc::document::ThemeMode) -> ec::ThemeMode {
    match t {
        sc::document::ThemeMode::Dark => ec::ThemeMode::Dark,
        sc::document::ThemeMode::Light => ec::ThemeMode::Light,
    }
}

/// `shell-core` `Locale` → `op_editor_core::Locale`. Both re-export
/// `op_i18n::Locale`, so this is the identity — kept explicit so a
/// future divergence surfaces here.
pub fn locale(l: sc::document::Locale) -> ec::Locale {
    l
}

/// `shell-core` `PropertyTab` → `op_editor_core::PropertyTab`.
pub fn property_tab(t: sc::document::PropertyTab) -> ec::PropertyTab {
    match t {
        sc::document::PropertyTab::Design => ec::PropertyTab::Design,
        sc::document::PropertyTab::Code => ec::PropertyTab::Code,
    }
}

/// `shell-core` `FlexLayout` → `op_editor_core::FlexLayout`.
pub fn flex_layout(f: sc::document::FlexLayout) -> ec::FlexLayout {
    match f {
        sc::document::FlexLayout::Free => ec::FlexLayout::Free,
        sc::document::FlexLayout::Vertical => ec::FlexLayout::Vertical,
        sc::document::FlexLayout::Horizontal => ec::FlexLayout::Horizontal,
    }
}

/// `shell-core` `FillType` → `op_editor_core::FillType`.
pub fn fill_type(f: sc::document::FillType) -> ec::FillType {
    match f {
        sc::document::FillType::Solid => ec::FillType::Solid,
        sc::document::FillType::LinearGradient => ec::FillType::LinearGradient,
        sc::document::FillType::RadialGradient => ec::FillType::RadialGradient,
        sc::document::FillType::Image => ec::FillType::Image,
    }
}

/// `shell-core` `PropertyFocus` → `op_editor_core::PropertyFocus`.
pub fn property_focus(f: sc::document::PropertyFocus) -> ec::PropertyFocus {
    match f {
        sc::document::PropertyFocus::PositionX => ec::PropertyFocus::PositionX,
        sc::document::PropertyFocus::PositionY => ec::PropertyFocus::PositionY,
        sc::document::PropertyFocus::Rotation => ec::PropertyFocus::Rotation,
        sc::document::PropertyFocus::PositionR => ec::PropertyFocus::PositionR,
        sc::document::PropertyFocus::SizeW => ec::PropertyFocus::SizeW,
        sc::document::PropertyFocus::SizeH => ec::PropertyFocus::SizeH,
        sc::document::PropertyFocus::Opacity => ec::PropertyFocus::Opacity,
        sc::document::PropertyFocus::FillHex => ec::PropertyFocus::FillHex,
        sc::document::PropertyFocus::StrokeHex => ec::PropertyFocus::StrokeHex,
        sc::document::PropertyFocus::StrokeWidth => ec::PropertyFocus::StrokeWidth,
    }
}

/// `shell-core` `VariableRowFocus` → `op_editor_core::VariableRowFocus`.
pub fn variable_row_focus(
    f: sc::document::VariableRowFocus,
) -> ec::VariableRowFocus {
    match f {
        sc::document::VariableRowFocus::Number(i) => ec::VariableRowFocus::Number(i),
        sc::document::VariableRowFocus::String(i) => ec::VariableRowFocus::String(i),
    }
}

/// `shell-core` `ExportFormat` → `op_editor_core::ExportFormat`.
pub fn export_format(
    f: sc::widgets::export_dialog::ExportFormat,
) -> ec::ExportFormat {
    use sc::widgets::export_dialog::ExportFormat as Sc;
    match f {
        Sc::Png => ec::ExportFormat::Png,
        Sc::Jpeg => ec::ExportFormat::Jpeg,
        Sc::Webp => ec::ExportFormat::Webp,
        Sc::Svg => ec::ExportFormat::Svg,
        Sc::Pdf => ec::ExportFormat::Pdf,
    }
}

/// `shell-core` `FileMenuChoice` → `op_editor_core::FileMenuChoice`.
pub fn file_menu_choice(
    c: sc::widgets::file_menu::FileMenuChoice,
) -> ec::FileMenuChoice {
    use sc::widgets::file_menu::FileMenuChoice as Sc;
    match c {
        Sc::NewFile => ec::FileMenuChoice::NewFile,
        Sc::OpenFile => ec::FileMenuChoice::OpenFile,
        Sc::Save => ec::FileMenuChoice::Save,
        Sc::SaveAs => ec::FileMenuChoice::SaveAs,
        Sc::ExportImage => ec::FileMenuChoice::ExportImage,
        Sc::OpenRecent(i) => ec::FileMenuChoice::OpenRecent(i),
        Sc::ClearRecent => ec::FileMenuChoice::ClearRecent,
    }
}

/// `shell-core` `ShapeChoice` → `op_editor_core::ShapeChoice`.
pub fn shape_choice(
    c: sc::widgets::shape_picker::ShapeChoice,
) -> ec::ShapeChoice {
    use sc::widgets::shape_picker::ShapeChoice as Sc;
    match c {
        Sc::Tool(t) => ec::ShapeChoice::Tool(tool(t)),
        Sc::OpenIconPicker => ec::ShapeChoice::OpenIconPicker,
        Sc::ImportImageOrSvg => ec::ShapeChoice::ImportImageOrSvg,
    }
}

/// `shell-core` `AlignAction` → `op_editor_core::AlignAction`.
pub fn align_action(a: sc::document::AlignAction) -> ec::AlignAction {
    match a {
        sc::document::AlignAction::Left => ec::AlignAction::Left,
        sc::document::AlignAction::CenterH => ec::AlignAction::CenterH,
        sc::document::AlignAction::Right => ec::AlignAction::Right,
        sc::document::AlignAction::Top => ec::AlignAction::Top,
        sc::document::AlignAction::CenterV => ec::AlignAction::CenterV,
        sc::document::AlignAction::Bottom => ec::AlignAction::Bottom,
        sc::document::AlignAction::DistributeH => ec::AlignAction::DistributeH,
        sc::document::AlignAction::DistributeV => ec::AlignAction::DistributeV,
    }
}

/// `shell-core` `AgentProvider` → `op_editor_core::AgentProvider`.
pub fn agent_provider(p: sc::document::AgentProvider) -> ec::AgentProvider {
    match p {
        sc::document::AgentProvider::ClaudeCode => ec::AgentProvider::ClaudeCode,
        sc::document::AgentProvider::CodexCli => ec::AgentProvider::CodexCli,
        sc::document::AgentProvider::OpenCode => ec::AgentProvider::OpenCode,
        sc::document::AgentProvider::GithubCopilot => ec::AgentProvider::GithubCopilot,
        sc::document::AgentProvider::GeminiCli => ec::AgentProvider::GeminiCli,
    }
}

/// `shell-core` `AgentSettingsTab` → `op_editor_core::AgentSettingsTab`.
pub fn agent_settings_tab(
    t: sc::document::AgentSettingsTab,
) -> ec::AgentSettingsTab {
    match t {
        sc::document::AgentSettingsTab::Agents => ec::AgentSettingsTab::Agents,
        sc::document::AgentSettingsTab::Mcp => ec::AgentSettingsTab::Mcp,
        sc::document::AgentSettingsTab::Images => ec::AgentSettingsTab::Images,
        sc::document::AgentSettingsTab::System => ec::AgentSettingsTab::System,
    }
}

/// `shell-core` `SettingsFocus` → `op_editor_core::SettingsFocus`.
pub fn settings_focus(f: sc::document::SettingsFocus) -> ec::SettingsFocus {
    match f {
        sc::document::SettingsFocus::McpPort => ec::SettingsFocus::McpPort,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_rewraps_the_raw_string() {
        let sc_id = sc::document::NodeId::new("n42");
        assert_eq!(node_id(&sc_id), ec::NodeId::new("n42"));
    }

    #[test]
    fn tool_maps_every_variant() {
        use sc::document::Tool as S;
        let cases = [
            (S::Select, ec::Tool::Select),
            (S::Rect, ec::Tool::Rect),
            (S::Ellipse, ec::Tool::Ellipse),
            (S::Polygon, ec::Tool::Polygon),
            (S::Line, ec::Tool::Line),
            (S::Pen, ec::Tool::Pen),
            (S::Text, ec::Tool::Text),
            (S::Frame, ec::Tool::Frame),
            (S::Hand, ec::Tool::Hand),
        ];
        for (s, e) in cases {
            assert_eq!(tool(s), e);
        }
    }

    #[test]
    fn theme_mode_maps_both_variants() {
        assert_eq!(theme_mode(sc::document::ThemeMode::Dark), ec::ThemeMode::Dark);
        assert_eq!(
            theme_mode(sc::document::ThemeMode::Light),
            ec::ThemeMode::Light
        );
    }

    #[test]
    fn locale_round_trips_identity() {
        assert_eq!(locale(sc::document::Locale::ZhCn), ec::Locale::ZhCn);
        assert_eq!(locale(sc::document::Locale::EnUs), ec::Locale::EnUs);
    }

    #[test]
    fn property_tab_maps_both_variants() {
        assert_eq!(
            property_tab(sc::document::PropertyTab::Design),
            ec::PropertyTab::Design
        );
        assert_eq!(
            property_tab(sc::document::PropertyTab::Code),
            ec::PropertyTab::Code
        );
    }

    #[test]
    fn flex_layout_maps_every_variant() {
        assert_eq!(
            flex_layout(sc::document::FlexLayout::Free),
            ec::FlexLayout::Free
        );
        assert_eq!(
            flex_layout(sc::document::FlexLayout::Vertical),
            ec::FlexLayout::Vertical
        );
        assert_eq!(
            flex_layout(sc::document::FlexLayout::Horizontal),
            ec::FlexLayout::Horizontal
        );
    }

    #[test]
    fn fill_type_maps_every_variant() {
        assert_eq!(
            fill_type(sc::document::FillType::Solid),
            ec::FillType::Solid
        );
        assert_eq!(
            fill_type(sc::document::FillType::LinearGradient),
            ec::FillType::LinearGradient
        );
        assert_eq!(
            fill_type(sc::document::FillType::RadialGradient),
            ec::FillType::RadialGradient
        );
        assert_eq!(
            fill_type(sc::document::FillType::Image),
            ec::FillType::Image
        );
    }

    #[test]
    fn property_focus_maps_every_variant() {
        use sc::document::PropertyFocus as S;
        let cases = [
            (S::PositionX, ec::PropertyFocus::PositionX),
            (S::PositionY, ec::PropertyFocus::PositionY),
            (S::Rotation, ec::PropertyFocus::Rotation),
            (S::PositionR, ec::PropertyFocus::PositionR),
            (S::SizeW, ec::PropertyFocus::SizeW),
            (S::SizeH, ec::PropertyFocus::SizeH),
            (S::Opacity, ec::PropertyFocus::Opacity),
            (S::FillHex, ec::PropertyFocus::FillHex),
            (S::StrokeHex, ec::PropertyFocus::StrokeHex),
            (S::StrokeWidth, ec::PropertyFocus::StrokeWidth),
        ];
        for (s, e) in cases {
            assert_eq!(property_focus(s), e);
        }
    }

    #[test]
    fn variable_row_focus_maps_both_variants() {
        assert_eq!(
            variable_row_focus(sc::document::VariableRowFocus::Number(3)),
            ec::VariableRowFocus::Number(3)
        );
        assert_eq!(
            variable_row_focus(sc::document::VariableRowFocus::String(7)),
            ec::VariableRowFocus::String(7)
        );
    }

    #[test]
    fn export_format_maps_every_variant() {
        use sc::widgets::export_dialog::ExportFormat as S;
        let cases = [
            (S::Png, ec::ExportFormat::Png),
            (S::Jpeg, ec::ExportFormat::Jpeg),
            (S::Webp, ec::ExportFormat::Webp),
            (S::Svg, ec::ExportFormat::Svg),
            (S::Pdf, ec::ExportFormat::Pdf),
        ];
        for (s, e) in cases {
            assert_eq!(export_format(s), e);
        }
    }

    #[test]
    fn file_menu_choice_maps_every_variant() {
        use sc::widgets::file_menu::FileMenuChoice as S;
        assert_eq!(file_menu_choice(S::NewFile), ec::FileMenuChoice::NewFile);
        assert_eq!(file_menu_choice(S::OpenFile), ec::FileMenuChoice::OpenFile);
        assert_eq!(file_menu_choice(S::Save), ec::FileMenuChoice::Save);
        assert_eq!(file_menu_choice(S::SaveAs), ec::FileMenuChoice::SaveAs);
        assert_eq!(
            file_menu_choice(S::ExportImage),
            ec::FileMenuChoice::ExportImage
        );
        assert_eq!(
            file_menu_choice(S::OpenRecent(2)),
            ec::FileMenuChoice::OpenRecent(2)
        );
        assert_eq!(
            file_menu_choice(S::ClearRecent),
            ec::FileMenuChoice::ClearRecent
        );
    }

    #[test]
    fn shape_choice_maps_every_variant() {
        use sc::widgets::shape_picker::ShapeChoice as S;
        assert_eq!(
            shape_choice(S::Tool(sc::document::Tool::Ellipse)),
            ec::ShapeChoice::Tool(ec::Tool::Ellipse)
        );
        assert_eq!(
            shape_choice(S::OpenIconPicker),
            ec::ShapeChoice::OpenIconPicker
        );
        assert_eq!(
            shape_choice(S::ImportImageOrSvg),
            ec::ShapeChoice::ImportImageOrSvg
        );
    }

    #[test]
    fn align_action_maps_every_variant() {
        use sc::document::AlignAction as S;
        let cases = [
            (S::Left, ec::AlignAction::Left),
            (S::CenterH, ec::AlignAction::CenterH),
            (S::Right, ec::AlignAction::Right),
            (S::Top, ec::AlignAction::Top),
            (S::CenterV, ec::AlignAction::CenterV),
            (S::Bottom, ec::AlignAction::Bottom),
            (S::DistributeH, ec::AlignAction::DistributeH),
            (S::DistributeV, ec::AlignAction::DistributeV),
        ];
        for (s, e) in cases {
            assert_eq!(align_action(s), e);
        }
    }

    #[test]
    fn agent_provider_maps_every_variant() {
        use sc::document::AgentProvider as S;
        let cases = [
            (S::ClaudeCode, ec::AgentProvider::ClaudeCode),
            (S::CodexCli, ec::AgentProvider::CodexCli),
            (S::OpenCode, ec::AgentProvider::OpenCode),
            (S::GithubCopilot, ec::AgentProvider::GithubCopilot),
            (S::GeminiCli, ec::AgentProvider::GeminiCli),
        ];
        for (s, e) in cases {
            assert_eq!(agent_provider(s), e);
        }
    }

    #[test]
    fn agent_settings_tab_maps_every_variant() {
        use sc::document::AgentSettingsTab as S;
        let cases = [
            (S::Agents, ec::AgentSettingsTab::Agents),
            (S::Mcp, ec::AgentSettingsTab::Mcp),
            (S::Images, ec::AgentSettingsTab::Images),
            (S::System, ec::AgentSettingsTab::System),
        ];
        for (s, e) in cases {
            assert_eq!(agent_settings_tab(s), e);
        }
    }

    #[test]
    fn settings_focus_maps_its_variant() {
        assert_eq!(
            settings_focus(sc::document::SettingsFocus::McpPort),
            ec::SettingsFocus::McpPort
        );
    }
}
