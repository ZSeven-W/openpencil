//! Enum translators for the EditorState → paint `Document` bridge.
//!
//! Every UI enum exists twice in the strangler reorg: once in the
//! wasm-clean `op-editor-core` state layer, once in
//! `openpencil-shell-core` (some declared under `shell-core/widgets/`,
//! they are still data enums). `op-pen-loader` is the only crate that
//! depends on both, so the translation lives here.
//!
//! `From` impls are forbidden by the orphan rule (both the source and
//! the target type are foreign to this crate), so each pair gets a
//! plain `fn`. Every translator is an exhaustive `match` with NO
//! wildcard arm — a future variant added to either side becomes a
//! compile error so the drift is caught immediately.

use op_editor_core as ec;
use openpencil_shell_core as sc;

/// `op_editor_core::Tool` → `shell-core` `Tool`.
pub fn tool(t: ec::Tool) -> sc::document::Tool {
    match t {
        ec::Tool::Select => sc::document::Tool::Select,
        ec::Tool::Rect => sc::document::Tool::Rect,
        ec::Tool::Ellipse => sc::document::Tool::Ellipse,
        ec::Tool::Polygon => sc::document::Tool::Polygon,
        ec::Tool::Line => sc::document::Tool::Line,
        ec::Tool::Pen => sc::document::Tool::Pen,
        ec::Tool::Text => sc::document::Tool::Text,
        ec::Tool::Frame => sc::document::Tool::Frame,
        ec::Tool::Hand => sc::document::Tool::Hand,
    }
}

/// `op_editor_core::ThemeMode` → `shell-core` `ThemeMode`.
pub fn theme_mode(t: ec::ThemeMode) -> sc::document::ThemeMode {
    match t {
        ec::ThemeMode::Dark => sc::document::ThemeMode::Dark,
        ec::ThemeMode::Light => sc::document::ThemeMode::Light,
    }
}

/// `op_editor_core::Locale` → `shell-core` `Locale`. Both re-export
/// `op_i18n::Locale`, so this is the identity — kept as an explicit
/// fn so the bridge call sites stay uniform and a future divergence
/// surfaces here rather than silently compiling.
pub fn locale(l: ec::Locale) -> sc::document::Locale {
    l
}

/// `op_editor_core::PropertyTab` → `shell-core` `PropertyTab`.
pub fn property_tab(t: ec::PropertyTab) -> sc::document::PropertyTab {
    match t {
        ec::PropertyTab::Design => sc::document::PropertyTab::Design,
        ec::PropertyTab::Code => sc::document::PropertyTab::Code,
    }
}

/// `op_editor_core::FlexLayout` → `shell-core` `FlexLayout`.
pub fn flex_layout(f: ec::FlexLayout) -> sc::document::FlexLayout {
    match f {
        ec::FlexLayout::Free => sc::document::FlexLayout::Free,
        ec::FlexLayout::Vertical => sc::document::FlexLayout::Vertical,
        ec::FlexLayout::Horizontal => sc::document::FlexLayout::Horizontal,
    }
}

/// `op_editor_core::FillType` → `shell-core` `FillType`.
pub fn fill_type(f: ec::FillType) -> sc::document::FillType {
    match f {
        ec::FillType::Solid => sc::document::FillType::Solid,
        ec::FillType::LinearGradient => sc::document::FillType::LinearGradient,
        ec::FillType::RadialGradient => sc::document::FillType::RadialGradient,
        ec::FillType::Image => sc::document::FillType::Image,
    }
}

/// `op_editor_core::PropertyFocus` → `shell-core` `PropertyFocus`.
pub fn property_focus(f: ec::PropertyFocus) -> sc::document::PropertyFocus {
    match f {
        ec::PropertyFocus::PositionX => sc::document::PropertyFocus::PositionX,
        ec::PropertyFocus::PositionY => sc::document::PropertyFocus::PositionY,
        ec::PropertyFocus::Rotation => sc::document::PropertyFocus::Rotation,
        ec::PropertyFocus::PositionR => sc::document::PropertyFocus::PositionR,
        ec::PropertyFocus::SizeW => sc::document::PropertyFocus::SizeW,
        ec::PropertyFocus::SizeH => sc::document::PropertyFocus::SizeH,
        ec::PropertyFocus::Opacity => sc::document::PropertyFocus::Opacity,
        ec::PropertyFocus::FillHex => sc::document::PropertyFocus::FillHex,
        ec::PropertyFocus::StrokeHex => sc::document::PropertyFocus::StrokeHex,
        ec::PropertyFocus::StrokeWidth => sc::document::PropertyFocus::StrokeWidth,
    }
}

/// `op_editor_core::VariableRowFocus` → `shell-core` `VariableRowFocus`.
pub fn variable_row_focus(f: ec::VariableRowFocus) -> sc::document::VariableRowFocus {
    match f {
        ec::VariableRowFocus::Number(i) => sc::document::VariableRowFocus::Number(i),
        ec::VariableRowFocus::String(i) => sc::document::VariableRowFocus::String(i),
    }
}

/// `op_editor_core::ExportFormat` → `shell-core` `ExportFormat`.
pub fn export_format(
    f: ec::ExportFormat,
) -> sc::widgets::export_dialog::ExportFormat {
    use sc::widgets::export_dialog::ExportFormat as Sc;
    match f {
        ec::ExportFormat::Png => Sc::Png,
        ec::ExportFormat::Jpeg => Sc::Jpeg,
        ec::ExportFormat::Webp => Sc::Webp,
        ec::ExportFormat::Svg => Sc::Svg,
        ec::ExportFormat::Pdf => Sc::Pdf,
    }
}

/// `op_editor_core::FileMenuChoice` → `shell-core` `FileMenuChoice`.
pub fn file_menu_choice(
    c: ec::FileMenuChoice,
) -> sc::widgets::file_menu::FileMenuChoice {
    use sc::widgets::file_menu::FileMenuChoice as Sc;
    match c {
        ec::FileMenuChoice::NewFile => Sc::NewFile,
        ec::FileMenuChoice::OpenFile => Sc::OpenFile,
        ec::FileMenuChoice::Save => Sc::Save,
        ec::FileMenuChoice::SaveAs => Sc::SaveAs,
        ec::FileMenuChoice::ExportImage => Sc::ExportImage,
        ec::FileMenuChoice::OpenRecent(i) => Sc::OpenRecent(i),
        ec::FileMenuChoice::ClearRecent => Sc::ClearRecent,
    }
}

/// `op_editor_core::ShapeChoice` → `shell-core` `ShapeChoice`.
pub fn shape_choice(c: ec::ShapeChoice) -> sc::widgets::shape_picker::ShapeChoice {
    use sc::widgets::shape_picker::ShapeChoice as Sc;
    match c {
        ec::ShapeChoice::Tool(t) => Sc::Tool(tool(t)),
        ec::ShapeChoice::OpenIconPicker => Sc::OpenIconPicker,
        ec::ShapeChoice::ImportImageOrSvg => Sc::ImportImageOrSvg,
    }
}

/// `op_editor_core::AlignAction` → `shell-core` `AlignAction`.
pub fn align_action(a: ec::AlignAction) -> sc::document::AlignAction {
    match a {
        ec::AlignAction::Left => sc::document::AlignAction::Left,
        ec::AlignAction::CenterH => sc::document::AlignAction::CenterH,
        ec::AlignAction::Right => sc::document::AlignAction::Right,
        ec::AlignAction::Top => sc::document::AlignAction::Top,
        ec::AlignAction::CenterV => sc::document::AlignAction::CenterV,
        ec::AlignAction::Bottom => sc::document::AlignAction::Bottom,
        ec::AlignAction::DistributeH => sc::document::AlignAction::DistributeH,
        ec::AlignAction::DistributeV => sc::document::AlignAction::DistributeV,
    }
}

/// `op_editor_core::AgentProvider` → `shell-core` `AgentProvider`.
pub fn agent_provider(p: ec::AgentProvider) -> sc::document::AgentProvider {
    match p {
        ec::AgentProvider::ClaudeCode => sc::document::AgentProvider::ClaudeCode,
        ec::AgentProvider::CodexCli => sc::document::AgentProvider::CodexCli,
        ec::AgentProvider::OpenCode => sc::document::AgentProvider::OpenCode,
        ec::AgentProvider::GithubCopilot => sc::document::AgentProvider::GithubCopilot,
        ec::AgentProvider::GeminiCli => sc::document::AgentProvider::GeminiCli,
    }
}

/// `op_editor_core::AgentSettingsTab` → `shell-core` `AgentSettingsTab`.
pub fn agent_settings_tab(
    t: ec::AgentSettingsTab,
) -> sc::document::AgentSettingsTab {
    match t {
        ec::AgentSettingsTab::Agents => sc::document::AgentSettingsTab::Agents,
        ec::AgentSettingsTab::Mcp => sc::document::AgentSettingsTab::Mcp,
        ec::AgentSettingsTab::Images => sc::document::AgentSettingsTab::Images,
        ec::AgentSettingsTab::System => sc::document::AgentSettingsTab::System,
    }
}

/// `op_editor_core::SettingsFocus` → `shell-core` `SettingsFocus`.
pub fn settings_focus(f: ec::SettingsFocus) -> sc::document::SettingsFocus {
    match f {
        ec::SettingsFocus::McpPort => sc::document::SettingsFocus::McpPort,
    }
}

/// `op_editor_core::McpServer` → `shell-core` `McpServer`. `McpServer`
/// is a plain `Copy` struct of primitives, so this is a field copy.
pub fn mcp_server(s: ec::McpServer) -> sc::document::McpServer {
    sc::document::McpServer {
        running: s.running,
        port: s.port,
    }
}

/// `op_editor_core::AgentSettings` → `shell-core` `AgentSettings`.
/// Field-by-field; `AgentSettings` carries no enum that lacks a
/// translator above. The `connected` / `mcp_cli_enabled` bool arrays
/// copy verbatim — their index order matches `AgentProvider::ALL` /
/// `McpCli::ALL`, which are identical across the two crates.
pub fn agent_settings(s: ec::AgentSettings) -> sc::document::AgentSettings {
    sc::document::AgentSettings {
        tab: agent_settings_tab(s.tab),
        connected: s.connected,
        scroll_y: s.scroll_y,
        mcp_server: mcp_server(s.mcp_server),
        mcp_cli_enabled: s.mcp_cli_enabled,
        images_advanced_open: s.images_advanced_open,
        images_search_ready: s.images_search_ready,
        focus: s.focus.map(settings_focus),
        hover_provider: s.hover_provider,
        hover_nav: s.hover_nav.map(agent_settings_tab),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_maps_every_variant() {
        for t in op_editor_core::Tool::ALL {
            // `tool` exhaustively matches; reaching here proves it
            // accepted the variant. Spot-check one mapping per call.
            let _ = tool(t);
        }
        assert_eq!(tool(ec::Tool::Select), sc::document::Tool::Select);
        assert_eq!(tool(ec::Tool::Pen), sc::document::Tool::Pen);
        assert_eq!(tool(ec::Tool::Hand), sc::document::Tool::Hand);
    }

    #[test]
    fn theme_mode_maps_both_variants() {
        assert_eq!(theme_mode(ec::ThemeMode::Dark), sc::document::ThemeMode::Dark);
        assert_eq!(
            theme_mode(ec::ThemeMode::Light),
            sc::document::ThemeMode::Light
        );
    }

    #[test]
    fn locale_round_trips_identity() {
        assert_eq!(locale(ec::Locale::ZhCn), sc::document::Locale::ZhCn);
        assert_eq!(locale(ec::Locale::EnUs), sc::document::Locale::EnUs);
    }

    #[test]
    fn property_tab_maps_both_variants() {
        assert_eq!(
            property_tab(ec::PropertyTab::Design),
            sc::document::PropertyTab::Design
        );
        assert_eq!(
            property_tab(ec::PropertyTab::Code),
            sc::document::PropertyTab::Code
        );
    }

    #[test]
    fn flex_layout_maps_every_variant() {
        assert_eq!(
            flex_layout(ec::FlexLayout::Free),
            sc::document::FlexLayout::Free
        );
        assert_eq!(
            flex_layout(ec::FlexLayout::Vertical),
            sc::document::FlexLayout::Vertical
        );
        assert_eq!(
            flex_layout(ec::FlexLayout::Horizontal),
            sc::document::FlexLayout::Horizontal
        );
    }

    #[test]
    fn fill_type_maps_every_variant() {
        assert_eq!(fill_type(ec::FillType::Solid), sc::document::FillType::Solid);
        assert_eq!(
            fill_type(ec::FillType::LinearGradient),
            sc::document::FillType::LinearGradient
        );
        assert_eq!(
            fill_type(ec::FillType::RadialGradient),
            sc::document::FillType::RadialGradient
        );
        assert_eq!(fill_type(ec::FillType::Image), sc::document::FillType::Image);
    }

    #[test]
    fn property_focus_maps_every_variant() {
        use ec::PropertyFocus as E;
        use sc::document::PropertyFocus as S;
        let cases = [
            (E::PositionX, S::PositionX),
            (E::PositionY, S::PositionY),
            (E::Rotation, S::Rotation),
            (E::PositionR, S::PositionR),
            (E::SizeW, S::SizeW),
            (E::SizeH, S::SizeH),
            (E::Opacity, S::Opacity),
            (E::FillHex, S::FillHex),
            (E::StrokeHex, S::StrokeHex),
            (E::StrokeWidth, S::StrokeWidth),
        ];
        for (e, s) in cases {
            assert_eq!(property_focus(e), s);
        }
    }

    #[test]
    fn variable_row_focus_maps_both_variants() {
        assert_eq!(
            variable_row_focus(ec::VariableRowFocus::Number(3)),
            sc::document::VariableRowFocus::Number(3)
        );
        assert_eq!(
            variable_row_focus(ec::VariableRowFocus::String(7)),
            sc::document::VariableRowFocus::String(7)
        );
    }

    #[test]
    fn export_format_maps_every_variant() {
        use ec::ExportFormat as E;
        use sc::widgets::export_dialog::ExportFormat as S;
        let cases = [
            (E::Png, S::Png),
            (E::Jpeg, S::Jpeg),
            (E::Webp, S::Webp),
            (E::Svg, S::Svg),
            (E::Pdf, S::Pdf),
        ];
        for (e, s) in cases {
            assert_eq!(export_format(e), s);
        }
    }

    #[test]
    fn file_menu_choice_maps_every_variant() {
        use ec::FileMenuChoice as E;
        use sc::widgets::file_menu::FileMenuChoice as S;
        assert_eq!(file_menu_choice(E::NewFile), S::NewFile);
        assert_eq!(file_menu_choice(E::OpenFile), S::OpenFile);
        assert_eq!(file_menu_choice(E::Save), S::Save);
        assert_eq!(file_menu_choice(E::SaveAs), S::SaveAs);
        assert_eq!(file_menu_choice(E::ExportImage), S::ExportImage);
        assert_eq!(file_menu_choice(E::OpenRecent(2)), S::OpenRecent(2));
        assert_eq!(file_menu_choice(E::ClearRecent), S::ClearRecent);
    }

    #[test]
    fn shape_choice_maps_every_variant() {
        use sc::widgets::shape_picker::ShapeChoice as S;
        assert_eq!(
            shape_choice(ec::ShapeChoice::Tool(ec::Tool::Ellipse)),
            S::Tool(sc::document::Tool::Ellipse)
        );
        assert_eq!(
            shape_choice(ec::ShapeChoice::OpenIconPicker),
            S::OpenIconPicker
        );
        assert_eq!(
            shape_choice(ec::ShapeChoice::ImportImageOrSvg),
            S::ImportImageOrSvg
        );
    }

    #[test]
    fn align_action_maps_every_variant() {
        use ec::AlignAction as E;
        use sc::document::AlignAction as S;
        let cases = [
            (E::Left, S::Left),
            (E::CenterH, S::CenterH),
            (E::Right, S::Right),
            (E::Top, S::Top),
            (E::CenterV, S::CenterV),
            (E::Bottom, S::Bottom),
            (E::DistributeH, S::DistributeH),
            (E::DistributeV, S::DistributeV),
        ];
        for (e, s) in cases {
            assert_eq!(align_action(e), s);
        }
    }

    #[test]
    fn agent_provider_maps_every_variant() {
        use ec::AgentProvider as E;
        use sc::document::AgentProvider as S;
        let cases = [
            (E::ClaudeCode, S::ClaudeCode),
            (E::CodexCli, S::CodexCli),
            (E::OpenCode, S::OpenCode),
            (E::GithubCopilot, S::GithubCopilot),
            (E::GeminiCli, S::GeminiCli),
        ];
        for (e, s) in cases {
            assert_eq!(agent_provider(e), s);
        }
    }

    #[test]
    fn agent_settings_tab_maps_every_variant() {
        use ec::AgentSettingsTab as E;
        use sc::document::AgentSettingsTab as S;
        let cases = [
            (E::Agents, S::Agents),
            (E::Mcp, S::Mcp),
            (E::Images, S::Images),
            (E::System, S::System),
        ];
        for (e, s) in cases {
            assert_eq!(agent_settings_tab(e), s);
        }
    }

    #[test]
    fn settings_focus_maps_its_variant() {
        assert_eq!(
            settings_focus(ec::SettingsFocus::McpPort),
            sc::document::SettingsFocus::McpPort
        );
    }

    #[test]
    fn agent_settings_struct_maps_field_by_field() {
        let mut src = ec::AgentSettings::default();
        src.tab = ec::AgentSettingsTab::Mcp;
        src.connected = [true, false, true, false, true];
        src.scroll_y = 42.0;
        src.mcp_server = ec::McpServer {
            running: true,
            port: 4200,
        };
        src.mcp_cli_enabled = [true, true, false, false, true, false];
        src.images_advanced_open = false;
        src.images_search_ready = false;
        src.focus = Some(ec::SettingsFocus::McpPort);
        src.hover_provider = 3;
        src.hover_nav = Some(ec::AgentSettingsTab::System);

        let out = agent_settings(src);
        assert_eq!(out.tab, sc::document::AgentSettingsTab::Mcp);
        assert_eq!(out.connected, [true, false, true, false, true]);
        assert_eq!(out.scroll_y, 42.0);
        assert!(out.mcp_server.running);
        assert_eq!(out.mcp_server.port, 4200);
        assert_eq!(out.mcp_cli_enabled, [true, true, false, false, true, false]);
        assert!(!out.images_advanced_open);
        assert!(!out.images_search_ready);
        assert_eq!(out.focus, Some(sc::document::SettingsFocus::McpPort));
        assert_eq!(out.hover_provider, 3);
        assert_eq!(out.hover_nav, Some(sc::document::AgentSettingsTab::System));
    }
}
