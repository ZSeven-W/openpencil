//! OpenPencil editor theme tokens — Step 4 visual lift.
//!
//! Mirrors the TS app's shadcn-dark palette (apps/web/src/styles.css) so
//! the Rust shell renders the same color semantics without literal hex
//! strings sprinkled through widget code. Light theme is stubbed for
//! parity with the TS theme switch but Step 4 only ships the dark
//! palette (the TS app boots in dark by default — see the `dark`
//! class on `<html>`).
//!
//! Tokens follow shadcn naming so a TS reader maps them 1:1:
//! - `background` — root canvas behind everything
//! - `foreground` — primary text on `background`
//! - `card` / `card_foreground` — panel surfaces (LayerPanel, RightPanel)
//! - `popover` / `popover_foreground` — floating overlays (AIChatPanel)
//! - `primary` / `primary_foreground` — accent (selected tool, active row)
//! - `muted` / `muted_foreground` — subdued text + dividers
//! - `border` — 1px hairlines between sections
//! - `accent` — hover background on neutral controls
//! - `destructive` — error red (kept here for future use)

use crate::Color;

/// Construct a `Color` from RGB byte triples + a 0..=1 alpha.
const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}

/// Editor color tokens. Constructed once via `Theme::dark()`; widgets
/// receive a `&Theme` through `LayoutCx` / a host-owned theme cache so
/// changing the theme is one swap, not a rebuild of every widget.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub card: Color,
    pub card_foreground: Color,
    pub popover: Color,
    pub popover_foreground: Color,
    pub primary: Color,
    pub primary_foreground: Color,
    pub muted: Color,
    pub muted_foreground: Color,
    pub border: Color,
    pub accent: Color,
    pub accent_foreground: Color,
    pub destructive: Color,
    /// Solid background of an icon-only toolbar button on hover.
    pub button_hover: Color,
    /// Slightly lighter than `card` — used for the active page
    /// tab + neutral row highlights.
    pub row_selected: Color,
    /// Primary-tinted row highlight (e.g. selected layer row).
    /// Approximates `bg-blue-500/15` from the TS app.
    pub row_selected_primary: Color,
}

impl Theme {
    /// Dark palette tuned to the TS screenshot. Values eyeballed from
    /// the screenshot + apps/web/src/styles.css `.dark` block — exact
    /// hex parity isn't a goal, semantic parity is.
    pub const fn dark() -> Self {
        Self {
            background: rgb(0x0a, 0x0a, 0x0a),
            foreground: rgb(0xfa, 0xfa, 0xfa),
            card: rgb(0x14, 0x14, 0x14),
            card_foreground: rgb(0xfa, 0xfa, 0xfa),
            popover: rgb(0x1a, 0x1a, 0x1a),
            popover_foreground: rgb(0xfa, 0xfa, 0xfa),
            primary: rgb(0x3b, 0x82, 0xf6),
            primary_foreground: rgb(0xff, 0xff, 0xff),
            muted: rgb(0x1f, 0x1f, 0x1f),
            muted_foreground: rgb(0x9a, 0x9a, 0x9a),
            border: rgb(0x26, 0x26, 0x26),
            accent: rgb(0x26, 0x26, 0x26),
            accent_foreground: rgb(0xfa, 0xfa, 0xfa),
            destructive: rgb(0xef, 0x44, 0x44),
            button_hover: rgba(0xff, 0xff, 0xff, 0.06),
            row_selected: rgb(0x26, 0x26, 0x26),
            row_selected_primary: rgba(0x3b, 0x82, 0xf6, 0.18),
        }
    }

    /// Light palette stub. Step 4 boots the editor in dark; light is
    /// kept so the TopBar's theme toggle has somewhere to land in
    /// Step 5 without another schema change.
    pub const fn light() -> Self {
        Self {
            background: rgb(0xff, 0xff, 0xff),
            foreground: rgb(0x0a, 0x0a, 0x0a),
            card: rgb(0xff, 0xff, 0xff),
            card_foreground: rgb(0x0a, 0x0a, 0x0a),
            popover: rgb(0xff, 0xff, 0xff),
            popover_foreground: rgb(0x0a, 0x0a, 0x0a),
            primary: rgb(0x3b, 0x82, 0xf6),
            primary_foreground: rgb(0xff, 0xff, 0xff),
            muted: rgb(0xf5, 0xf5, 0xf5),
            muted_foreground: rgb(0x73, 0x73, 0x73),
            border: rgb(0xe5, 0xe5, 0xe5),
            accent: rgb(0xf5, 0xf5, 0xf5),
            accent_foreground: rgb(0x0a, 0x0a, 0x0a),
            destructive: rgb(0xef, 0x44, 0x44),
            button_hover: rgba(0x00, 0x00, 0x00, 0.06),
            row_selected: rgb(0xe5, 0xe5, 0xe5),
            row_selected_primary: rgba(0x3b, 0x82, 0xf6, 0.15),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_have_distinct_backgrounds() {
        let d = Theme::dark();
        let l = Theme::light();
        assert!(d.background.r < 0.5, "dark bg should be near-black");
        assert!(l.background.r > 0.9, "light bg should be near-white");
    }

    #[test]
    fn primary_is_blue_in_both_themes() {
        // Primary stays the same accent across themes (matches TS:
        // shadcn primary doesn't flip on theme).
        let d = Theme::dark();
        let l = Theme::light();
        assert_eq!(d.primary.r, l.primary.r);
        assert_eq!(d.primary.b, l.primary.b);
    }

    #[test]
    fn default_is_dark() {
        let t = Theme::default();
        assert!(t.background.r < 0.1, "default theme should be dark");
    }
}
