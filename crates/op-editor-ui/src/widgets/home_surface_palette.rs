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
    /// Top bar fill / hairline. Dark keeps the bar a shade above the
    /// page (`#141821` / `#262F3E`) instead of reusing the card fill.
    pub topbar: Color,
    pub topbar_line: Color,
    /// A raised surface: popovers, menus and secondary buttons. Dark
    /// lifts them off the card (`#222A38` / `#303A4D`); light keeps the
    /// original white-on-white with the hairline border.
    pub raised: Color,
    pub raised_line: Color,
    /// An inset surface: the composer's own input box. Dark recedes
    /// BELOW the card (`#131821`) so the box reads as carved in, which
    /// is the one layer a single `panel` token cannot express.
    pub surface_input: Color,
    /// Text blue. Kept separate from the primary-button blue: `#075BFF`
    /// is legible as a button fill under white text but too dark for
    /// small type on a dark surface, where the link tone is `#8BB3FF`.
    pub link: Color,
    /// Selected task-tab label and its 2 px underline bar.
    pub tab_selected_ink: Color,
    pub tab_selected_bar: Color,
    /// Label on the disabled primary button (white in light).
    pub disabled_primary_ink: Color,
    /// The headline marker. Dark uses a softer lime and paints it as a
    /// thin underline instead of a band, because the band would sit
    /// behind white glyphs.
    pub marker: Color,
    /// Whether the marker paints as an underline below the baseline
    /// (dark) instead of a band across the glyphs' lower half (light).
    pub marker_underline: bool,
    /// Ink on the yellow 示例 sticker. Always dark — the sticker keeps
    /// its own colour, so the theme's light `ink` may not be used here.
    pub sticker_ink: Color,
    /// Paper backing under the transparent tutorial thumbnail, so its
    /// black artwork keeps a light sheet to sit on in dark mode.
    pub tutorial_sheet: Color,
    /// The normal-mode workspace canvas backdrop and its dot grid.
    pub canvas: Color,
    pub canvas_dot: Color,
    /// The 查看示例 pill on a tinted example card. It sits on the card's
    /// own colour, not on the page, so it is a wash of white rather than
    /// a themed surface — nearly opaque in light, nearly invisible in
    /// dark, with its own ink either way.
    pub card_link_fill: Color,
    pub card_link_line: Color,
    pub card_link_ink: Color,
    /// Explore card tints (knowledge / tutorial / poster).
    pub tint_knowledge: Color,
    pub tint_tutorial: Color,
    pub tint_poster: Color,
    /// Ink of the glyph drawn on those tiles. The tiles keep their own
    /// bright colours in both themes, so the glyph — not the tile — is
    /// what has to flip to stay legible on lime.
    pub tile_ink: Color,
    /// The hover tooltip: a slab that contrasts with the PAGE, so in a
    /// dark theme it cannot be "ink at 90 %" (that is nearly white).
    pub tooltip_fill: Color,
    pub tooltip_ink: Color,
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
            topbar: Color::rgb_u8(0xFF, 0xFF, 0xFF),
            topbar_line: Color::rgb_u8(0xE1, 0xE8, 0xF2),
            raised: Color::rgb_u8(0xFF, 0xFF, 0xFF),
            raised_line: Color::rgb_u8(0xE1, 0xE8, 0xF2),
            surface_input: Color::rgb_u8(0xFF, 0xFF, 0xFF),
            link: Color::rgb_u8(0x07, 0x5B, 0xFF),
            tab_selected_ink: Color::rgb_u8(0x07, 0x5B, 0xFF),
            tab_selected_bar: Color::rgb_u8(0x07, 0x5B, 0xFF),
            disabled_primary_ink: Color::rgb_u8(0xFF, 0xFF, 0xFF),
            marker: Color::rgb_u8(0xF3, 0xFF, 0x23),
            marker_underline: false,
            sticker_ink: Color::rgb_u8(0x11, 0x1A, 0x32),
            tutorial_sheet: Color::rgb_u8(0xFF, 0xFF, 0xFF),
            canvas: Color::rgb_u8(0xF3, 0xF3, 0xF3),
            canvas_dot: Color::rgb_u8(0xD8, 0xDF, 0xEA),
            card_link_fill: Color::rgba_u8(0xFF, 0xFF, 0xFF, 0.85),
            card_link_line: Color::rgb_u8(0xFF, 0xFF, 0xFF),
            card_link_ink: Color::rgb_u8(0x11, 0x1A, 0x32),
            tint_knowledge: Color::rgb_u8(0xFF, 0xF3, 0xE8),
            tint_tutorial: Color::rgb_u8(0xED, 0xF5, 0xFF),
            tint_poster: Color::rgb_u8(0xF4, 0xFA, 0xDD),
            tile_ink: Color::rgb_u8(0xFF, 0xFF, 0xFF),
            tooltip_fill: Color::rgba_u8(0x11, 0x1A, 0x32, 0.9),
            tooltip_ink: Color::rgb_u8(0xFF, 0xFF, 0xFF),
            tile_knowledge: Color::rgb_u8(0xFF, 0x94, 0x42),
            tile_tutorial: Color::rgb_u8(0x68, 0xA6, 0xFF),
            tile_poster: Color::rgb_u8(0xC9, 0xFA, 0x15),
        }
    }

    /// The founder-approved dark theme (`dark-theme.css` in the
    /// `entry-home-studio` prototype). It is the same cool-blue design
    /// after dark, not a second design: the artwork, the yellow marker
    /// and the tinted example cards keep their own identity, and only
    /// the chrome inverts.
    pub const fn dark() -> Self {
        Self {
            page: Color::rgb_u8(0x10, 0x13, 0x1A),
            panel: Color::rgb_u8(0x19, 0x1E, 0x28),
            line: Color::rgb_u8(0x29, 0x32, 0x44),
            ink: Color::rgb_u8(0xED, 0xF1, 0xF8),
            muted: Color::rgb_u8(0xA0, 0xAE, 0xC3),
            sub: Color::rgb_u8(0xA0, 0xAE, 0xC3),
            context: Color::rgb_u8(0xA0, 0xAE, 0xC3),
            divider: Color::rgb_u8(0x30, 0x3A, 0x4D),
            blue: Color::rgb_u8(0x07, 0x5B, 0xFF),
            blue_hover: Color::rgb_u8(0x23, 0x69, 0xF2),
            blue_soft: Color::rgb_u8(0x20, 0x2F, 0x4C),
            segment_bg: Color::rgb_u8(0x13, 0x18, 0x21),
            segment_line: Color::rgb_u8(0x30, 0x3A, 0x4D),
            input_line: Color::rgb_u8(0x36, 0x41, 0x57),
            placeholder: Color::rgb_u8(0x8F, 0x9D, 0xB3),
            yellow: Color::rgb_u8(0xF3, 0xFF, 0x23),
            disabled_primary: Color::rgb_u8(0x25, 0x37, 0x54),
            preview: Color::rgb_u8(0x1B, 0x25, 0x38),
            preview_line: Color::rgb_u8(0x2A, 0x39, 0x52),
            selected_tab: Color::rgb_u8(0x20, 0x2F, 0x4C),
            tab_hover: Color::rgb_u8(0x22, 0x2A, 0x38),
            tab_fill: Color::rgb_u8(0x17, 0x1C, 0x26),
            tab_hover_fill: Color::rgb_u8(0x22, 0x2A, 0x38),
            tab_hover_line: Color::rgb_u8(0x30, 0x3A, 0x4D),
            tab_selected_fill: Color::rgb_u8(0x20, 0x2F, 0x4C),
            tab_selected_line: Color::rgb_u8(0x3B, 0x56, 0x82),
            tabs_hairline: Color::rgb_u8(0x29, 0x32, 0x44),
            button_hover: Color::rgb_u8(0x2A, 0x34, 0x45),
            button_hover_line: Color::rgb_u8(0x47, 0x56, 0x72),
            eyebrow: Color::rgb_u8(0xA0, 0xAE, 0xC3),
            preview_desc: Color::rgb_u8(0xA0, 0xAE, 0xC3),
            preview_footer: Color::rgb_u8(0xA0, 0xAE, 0xC3),
            status_green: Color::rgb_u8(0x4B, 0xCE, 0x98),
            chip_bg: Color::rgb_u8(0x24, 0x32, 0x4B),
            chip_line: Color::rgb_u8(0x35, 0x47, 0x68),
            topbar: Color::rgb_u8(0x14, 0x18, 0x21),
            topbar_line: Color::rgb_u8(0x26, 0x2F, 0x3E),
            raised: Color::rgb_u8(0x22, 0x2A, 0x38),
            raised_line: Color::rgb_u8(0x30, 0x3A, 0x4D),
            surface_input: Color::rgb_u8(0x13, 0x18, 0x21),
            link: Color::rgb_u8(0x8B, 0xB3, 0xFF),
            tab_selected_ink: Color::rgb_u8(0xA7, 0xC6, 0xFF),
            tab_selected_bar: Color::rgb_u8(0x6B, 0x9E, 0xFF),
            disabled_primary_ink: Color::rgb_u8(0x93, 0xA8, 0xC8),
            marker: Color::rgba_u8(0xD5, 0xE8, 0x44, 0.85),
            marker_underline: true,
            sticker_ink: Color::rgb_u8(0x18, 0x20, 0x19),
            tutorial_sheet: Color::rgb_u8(0xE6, 0xF1, 0xFF),
            canvas: Color::rgb_u8(0x11, 0x16, 0x20),
            canvas_dot: Color::rgb_u8(0x30, 0x3A, 0x4E),
            card_link_fill: Color::rgba_u8(0xFF, 0xFF, 0xFF, 0.035),
            card_link_line: Color::rgba_u8(0xFF, 0xFF, 0xFF, 0.11),
            card_link_ink: Color::rgb_u8(0xE2, 0xE8, 0xF0),
            // The three example tiles keep the original bright accents
            // with dark icons drawn on them; dimming them would make the
            // page read as a different design rather than the same one
            // after dark.
            tint_knowledge: Color::rgb_u8(0x2B, 0x23, 0x21),
            tint_tutorial: Color::rgb_u8(0x1D, 0x29, 0x3C),
            tint_poster: Color::rgb_u8(0x25, 0x2C, 0x1D),
            tile_ink: Color::rgb_u8(0x17, 0x21, 0x36),
            tooltip_fill: Color::rgb_u8(0x29, 0x36, 0x4C),
            tooltip_ink: Color::rgb_u8(0xED, 0xF1, 0xF8),
            tile_knowledge: Color::rgb_u8(0xFF, 0x94, 0x42),
            tile_tutorial: Color::rgb_u8(0x68, 0xA6, 0xFF),
            tile_poster: Color::rgb_u8(0xC9, 0xFA, 0x15),
        }
    }

    /// Every colour token at `factor` of its alpha — the entrance
    /// crossfade the Home and workspace surfaces play. It lives here so
    /// a new token cannot be forgotten by one of the three surfaces
    /// that fade the palette (they each used to keep their own copy of
    /// this list, and a token missing from one of them is invisible
    /// until someone watches that surface fade in).
    pub fn faded(self, factor: f32) -> Self {
        Self {
            page: fade(self.page, factor),
            panel: fade(self.panel, factor),
            line: fade(self.line, factor),
            ink: fade(self.ink, factor),
            muted: fade(self.muted, factor),
            sub: fade(self.sub, factor),
            context: fade(self.context, factor),
            divider: fade(self.divider, factor),
            blue: fade(self.blue, factor),
            blue_hover: fade(self.blue_hover, factor),
            blue_soft: fade(self.blue_soft, factor),
            segment_bg: fade(self.segment_bg, factor),
            segment_line: fade(self.segment_line, factor),
            input_line: fade(self.input_line, factor),
            placeholder: fade(self.placeholder, factor),
            yellow: fade(self.yellow, factor),
            disabled_primary: fade(self.disabled_primary, factor),
            preview: fade(self.preview, factor),
            preview_line: fade(self.preview_line, factor),
            selected_tab: fade(self.selected_tab, factor),
            tab_hover: fade(self.tab_hover, factor),
            tab_fill: fade(self.tab_fill, factor),
            tab_hover_fill: fade(self.tab_hover_fill, factor),
            tab_hover_line: fade(self.tab_hover_line, factor),
            tab_selected_fill: fade(self.tab_selected_fill, factor),
            tab_selected_line: fade(self.tab_selected_line, factor),
            tabs_hairline: fade(self.tabs_hairline, factor),
            button_hover: fade(self.button_hover, factor),
            button_hover_line: fade(self.button_hover_line, factor),
            eyebrow: fade(self.eyebrow, factor),
            preview_desc: fade(self.preview_desc, factor),
            preview_footer: fade(self.preview_footer, factor),
            status_green: fade(self.status_green, factor),
            chip_bg: fade(self.chip_bg, factor),
            chip_line: fade(self.chip_line, factor),
            topbar: fade(self.topbar, factor),
            topbar_line: fade(self.topbar_line, factor),
            raised: fade(self.raised, factor),
            raised_line: fade(self.raised_line, factor),
            surface_input: fade(self.surface_input, factor),
            link: fade(self.link, factor),
            tab_selected_ink: fade(self.tab_selected_ink, factor),
            tab_selected_bar: fade(self.tab_selected_bar, factor),
            disabled_primary_ink: fade(self.disabled_primary_ink, factor),
            marker: fade(self.marker, factor),
            sticker_ink: fade(self.sticker_ink, factor),
            tutorial_sheet: fade(self.tutorial_sheet, factor),
            canvas: fade(self.canvas, factor),
            canvas_dot: fade(self.canvas_dot, factor),
            card_link_fill: fade(self.card_link_fill, factor),
            card_link_line: fade(self.card_link_line, factor),
            card_link_ink: fade(self.card_link_ink, factor),
            tint_knowledge: fade(self.tint_knowledge, factor),
            tint_tutorial: fade(self.tint_tutorial, factor),
            tint_poster: fade(self.tint_poster, factor),
            tile_ink: fade(self.tile_ink, factor),
            tooltip_fill: fade(self.tooltip_fill, factor),
            tooltip_ink: fade(self.tooltip_ink, factor),
            tile_knowledge: fade(self.tile_knowledge, factor),
            tile_tutorial: fade(self.tile_tutorial, factor),
            tile_poster: fade(self.tile_poster, factor),
            marker_underline: self.marker_underline,
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
