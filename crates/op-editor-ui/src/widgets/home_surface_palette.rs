use crate::Color;
use op_editor_core::ThemeMode;

/// The Studio Home surface's blue/white design tokens (the founder-
/// approved `entry-home-studio` prototype). The surface deliberately
/// does not reuse editor chrome tokens; keeping them together makes the
/// light/dark contract testable without constructing a renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StudioPalette {
    /// Page background `#F9FBFD`.
    pub page: Color,
    /// Panel / button white `#FFFFFF`.
    pub panel: Color,
    /// Hairline border `#E1E8F2`.
    pub line: Color,
    /// Primary ink `#111A32`.
    pub ink: Color,
    /// Muted ink `#78859C`.
    pub muted: Color,
    /// Welcome sub copy `#61748E`.
    pub sub: Color,
    /// Top-bar context copy `#626E81`.
    pub context: Color,
    /// The 1×17 brand divider `#DAE0E8`.
    pub divider: Color,
    /// Primary blue `#075BFF`.
    pub blue: Color,
    /// Primary hover `#004CE0`.
    pub blue_hover: Color,
    /// Selected segment fill `#E6EFFF`.
    pub blue_soft: Color,
    /// Segmented control track `#F6F9FD`.
    pub segment_bg: Color,
    /// Segmented control border `#DCE7F6`.
    pub segment_line: Color,
    /// Input box border `#D5DEEC`.
    pub input_line: Color,
    /// Input placeholder `#929DAF`.
    pub placeholder: Color,
    /// Marker yellow `#F3FF23` (decoration only).
    pub yellow: Color,
    /// Disabled primary fill `#9ABBFF`.
    pub disabled_primary: Color,
    /// Preview panel fill `#EEF4FF`.
    pub preview: Color,
    /// Preview panel border `#EDF3FD`.
    pub preview_line: Color,
    /// Selected tab fill `#EDF4FF`.
    pub selected_tab: Color,
    /// Tab hover `#F4F7FB`.
    pub tab_hover: Color,
    /// Wide tab rest fill: white at 49 % (`#ffffff7d`).
    pub tab_fill: Color,
    /// Wide tab hover fill `#F0F5FD` / border `#E1EAF7`.
    pub tab_hover_fill: Color,
    pub tab_hover_line: Color,
    /// Wide selected tab fill `#EAF2FF` / border `#C8DBFF`.
    pub tab_selected_fill: Color,
    pub tab_selected_line: Color,
    /// The task-selector row's bottom hairline `#DFE7F2`.
    pub tabs_hairline: Color,
    /// Outline-button hover `#F2F6FC`.
    pub button_hover: Color,
    /// Outline-button hover border `#CBD7EA`.
    pub button_hover_line: Color,
    /// Preview eyebrow `#7487A1`.
    pub eyebrow: Color,
    /// Preview description `#6A7D97`.
    pub preview_desc: Color,
    /// Preview footer `#6C7F97`.
    pub preview_footer: Color,
    /// Model status dot green `#22B878`.
    pub status_green: Color,
    /// Attachment chip fill `#F7FAFF`.
    pub chip_bg: Color,
    /// Attachment chip border `#DBE4F1`.
    pub chip_line: Color,
    /// Explore card tints (knowledge / tutorial / poster).
    pub tint_knowledge: Color,
    pub tint_tutorial: Color,
    pub tint_poster: Color,
    /// Explore card icon tiles (knowledge / tutorial / poster).
    pub tile_knowledge: Color,
    pub tile_tutorial: Color,
    pub tile_poster: Color,
}

impl StudioPalette {
    pub const fn light() -> Self {
        Self {
            page: Color::rgb_u8(0xF9, 0xFB, 0xFD),
            panel: Color::rgb_u8(0xFF, 0xFF, 0xFF),
            line: Color::rgb_u8(0xE1, 0xE8, 0xF2),
            ink: Color::rgb_u8(0x11, 0x1A, 0x32),
            muted: Color::rgb_u8(0x78, 0x85, 0x9C),
            sub: Color::rgb_u8(0x61, 0x74, 0x8E),
            context: Color::rgb_u8(0x62, 0x6E, 0x81),
            divider: Color::rgb_u8(0xDA, 0xE0, 0xE8),
            blue: Color::rgb_u8(0x07, 0x5B, 0xFF),
            blue_hover: Color::rgb_u8(0x00, 0x4C, 0xE0),
            blue_soft: Color::rgb_u8(0xE6, 0xEF, 0xFF),
            segment_bg: Color::rgb_u8(0xF6, 0xF9, 0xFD),
            segment_line: Color::rgb_u8(0xDC, 0xE7, 0xF6),
            input_line: Color::rgb_u8(0xD5, 0xDE, 0xEC),
            placeholder: Color::rgb_u8(0x92, 0x9D, 0xAF),
            yellow: Color::rgb_u8(0xF3, 0xFF, 0x23),
            disabled_primary: Color::rgb_u8(0x9A, 0xBB, 0xFF),
            preview: Color::rgb_u8(0xEE, 0xF4, 0xFF),
            preview_line: Color::rgb_u8(0xED, 0xF3, 0xFD),
            selected_tab: Color::rgb_u8(0xED, 0xF4, 0xFF),
            tab_hover: Color::rgb_u8(0xF4, 0xF7, 0xFB),
            tab_fill: Color::rgba_u8(0xFF, 0xFF, 0xFF, 0.49),
            tab_hover_fill: Color::rgb_u8(0xF0, 0xF5, 0xFD),
            tab_hover_line: Color::rgb_u8(0xE1, 0xEA, 0xF7),
            tab_selected_fill: Color::rgb_u8(0xEA, 0xF2, 0xFF),
            tab_selected_line: Color::rgb_u8(0xC8, 0xDB, 0xFF),
            tabs_hairline: Color::rgb_u8(0xDF, 0xE7, 0xF2),
            button_hover: Color::rgb_u8(0xF2, 0xF6, 0xFC),
            button_hover_line: Color::rgb_u8(0xCB, 0xD7, 0xEA),
            eyebrow: Color::rgb_u8(0x74, 0x87, 0xA1),
            preview_desc: Color::rgb_u8(0x6A, 0x7D, 0x97),
            preview_footer: Color::rgb_u8(0x6C, 0x7F, 0x97),
            status_green: Color::rgb_u8(0x22, 0xB8, 0x78),
            chip_bg: Color::rgb_u8(0xF7, 0xFA, 0xFF),
            chip_line: Color::rgb_u8(0xDB, 0xE4, 0xF1),
            tint_knowledge: Color::rgb_u8(0xFF, 0xF3, 0xE8),
            tint_tutorial: Color::rgb_u8(0xED, 0xF5, 0xFF),
            tint_poster: Color::rgb_u8(0xF4, 0xFA, 0xDD),
            tile_knowledge: Color::rgb_u8(0xFF, 0x94, 0x42),
            tile_tutorial: Color::rgb_u8(0x68, 0xA6, 0xFF),
            tile_poster: Color::rgb_u8(0xC9, 0xFA, 0x15),
        }
    }

    pub const fn dark() -> Self {
        Self {
            page: Color::rgb_u8(0x0F, 0x15, 0x24),
            panel: Color::rgb_u8(0x16, 0x1D, 0x30),
            line: Color::rgb_u8(0x25, 0x30, 0x4A),
            ink: Color::rgb_u8(0xEE, 0xF2, 0xFA),
            muted: Color::rgb_u8(0x8B, 0x97, 0xAE),
            sub: Color::rgb_u8(0x93, 0xA1, 0xB8),
            context: Color::rgb_u8(0x8B, 0x97, 0xAE),
            divider: Color::rgb_u8(0x2B, 0x36, 0x50),
            blue: Color::rgb_u8(0x07, 0x5B, 0xFF),
            blue_hover: Color::rgb_u8(0x2F, 0x76, 0xFF),
            blue_soft: Color::rgb_u8(0x1B, 0x2C, 0x50),
            segment_bg: Color::rgb_u8(0x18, 0x21, 0x36),
            segment_line: Color::rgb_u8(0x2C, 0x38, 0x52),
            input_line: Color::rgb_u8(0x33, 0x40, 0x5E),
            placeholder: Color::rgb_u8(0x67, 0x74, 0x8E),
            yellow: Color::rgb_u8(0xF3, 0xFF, 0x23),
            disabled_primary: Color::rgb_u8(0x3D, 0x51, 0x77),
            preview: Color::rgb_u8(0x13, 0x1C, 0x31),
            preview_line: Color::rgb_u8(0x25, 0x30, 0x4A),
            selected_tab: Color::rgb_u8(0x1C, 0x27, 0x40),
            tab_hover: Color::rgb_u8(0x18, 0x20, 0x33),
            tab_fill: Color::rgba_u8(0x16, 0x1D, 0x30, 0.49),
            tab_hover_fill: Color::rgb_u8(0x18, 0x22, 0x36),
            tab_hover_line: Color::rgb_u8(0x2C, 0x38, 0x52),
            tab_selected_fill: Color::rgb_u8(0x1C, 0x27, 0x40),
            tab_selected_line: Color::rgb_u8(0x2C, 0x3E, 0x68),
            tabs_hairline: Color::rgb_u8(0x25, 0x30, 0x4A),
            button_hover: Color::rgb_u8(0x1C, 0x25, 0x3A),
            button_hover_line: Color::rgb_u8(0x38, 0x46, 0x64),
            eyebrow: Color::rgb_u8(0x7E, 0x8C, 0xA6),
            preview_desc: Color::rgb_u8(0x7E, 0x8C, 0xA6),
            preview_footer: Color::rgb_u8(0x7E, 0x8C, 0xA6),
            status_green: Color::rgb_u8(0x22, 0xB8, 0x78),
            chip_bg: Color::rgb_u8(0x1A, 0x23, 0x38),
            chip_line: Color::rgb_u8(0x30, 0x3D, 0x5A),
            tint_knowledge: Color::rgb_u8(0x33, 0x28, 0x1E),
            tint_tutorial: Color::rgb_u8(0x1B, 0x27, 0x40),
            tint_poster: Color::rgb_u8(0x25, 0x30, 0x1C),
            tile_knowledge: Color::rgb_u8(0xB8, 0x6A, 0x2E),
            tile_tutorial: Color::rgb_u8(0x4A, 0x7B, 0xC8),
            tile_poster: Color::rgb_u8(0x93, 0xB4, 0x0C),
        }
    }

    pub const fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }
}

/// Multiply a colour's alpha by `factor` (composes with baked alpha).
pub(crate) fn fade(color: Color, factor: f32) -> Color {
    Color {
        a: color.a * factor,
        ..color
    }
}
