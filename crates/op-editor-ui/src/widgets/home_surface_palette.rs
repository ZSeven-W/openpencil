use crate::Color;
use op_editor_core::ThemeMode;

/// The Home surface is a drafting table, so it deliberately does not reuse
/// editor chrome surfaces. Keeping these tokens together also makes the
/// light/dark contract testable without constructing a renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HomePalette {
    pub paper: Color,
    pub paper_2: Color,
    pub sheet: Color,
    pub ink: Color,
    pub graphite: Color,
    pub ash: Color,
    pub line: Color,
    pub blue: Color,
    pub blue_2: Color,
    pub blue_soft: Color,
    pub dots: Color,
    pub margin_rule: Color,
}

impl HomePalette {
    pub const fn light() -> Self {
        Self {
            paper: Color::rgb_u8(0xF4, 0xF1, 0xEA),
            paper_2: Color::rgb_u8(0xED, 0xE8, 0xDE),
            sheet: Color::rgb_u8(0xFF, 0xFD, 0xF9),
            ink: Color::rgb_u8(0x1B, 0x1A, 0x17),
            graphite: Color::rgb_u8(0x5E, 0x5A, 0x52),
            ash: Color::rgb_u8(0x9B, 0x95, 0x8A),
            line: Color::rgb_u8(0xD9, 0xD3, 0xC6),
            blue: Color::rgb_u8(0x2F, 0x3F, 0xD1),
            blue_2: Color::rgb_u8(0x25, 0x33, 0xA8),
            blue_soft: Color::rgb_u8(0xE4, 0xE7, 0xFA),
            dots: Color::rgba_u8(0x1B, 0x1A, 0x17, 0.10),
            margin_rule: Color::rgba_u8(0x2F, 0x3F, 0xD1, 0.35),
        }
    }

    pub const fn dark() -> Self {
        Self {
            paper: Color::rgb_u8(0x1E, 0x1C, 0x18),
            paper_2: Color::rgb_u8(0x26, 0x23, 0x20),
            sheet: Color::rgb_u8(0x2B, 0x29, 0x25),
            ink: Color::rgb_u8(0xF1, 0xED, 0xE4),
            graphite: Color::rgb_u8(0xB8, 0xB1, 0xA5),
            ash: Color::rgb_u8(0x7E, 0x78, 0x6D),
            line: Color::rgb_u8(0x3A, 0x36, 0x30),
            blue: Color::rgb_u8(0x5B, 0x6C, 0xFF),
            blue_2: Color::rgb_u8(0x74, 0x82, 0xFF),
            blue_soft: Color::rgb_u8(0x2A, 0x2E, 0x52),
            dots: Color::rgba_u8(0xF1, 0xED, 0xE4, 0.10),
            margin_rule: Color::rgba_u8(0x5B, 0x6C, 0xFF, 0.35),
        }
    }

    pub const fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }
}
