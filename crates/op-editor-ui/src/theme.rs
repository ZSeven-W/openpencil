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
    /// Text on a `destructive` surface (shadcn `--destructive-foreground`).
    pub destructive_foreground: Color,
    /// Secondary neutral surface + its foreground (shadcn `--secondary`).
    pub secondary: Color,
    pub secondary_foreground: Color,
    /// Form-field border (shadcn `--input`) — input box + switch off-track.
    pub input: Color,
    /// Focus-visible ring color (shadcn `--ring`).
    pub ring: Color,
    /// Solid background of an icon-only toolbar button on hover.
    pub button_hover: Color,
    /// Slightly lighter than `card` — used for the active page
    /// tab + neutral row highlights.
    pub row_selected: Color,
    /// Primary-tinted row highlight (e.g. selected layer row).
    /// Approximates `bg-blue-500/15` from the TS app.
    pub row_selected_primary: Color,
    /// Infinite-canvas surface behind document nodes. Slightly
    /// lighter than `background` so the canvas reads as a distinct
    /// surface (matches the TS app's `oklch(0.145 0 0)` ≈ `#252525`
    /// canvas tone, dialed back a touch for OP's chrome).
    pub canvas_surface: Color,
    /// Fill for user message bubbles in the AI chat transcript.
    /// Follows the overall neutral shell: a distinct graphite chip
    /// in both themes, with dedicated foreground for contrast.
    pub user_bubble: Color,
    pub user_bubble_foreground: Color,
    /// Success green for the ✓ ring on completed tool/step cards
    /// (#27 reference: ~#3FB950, matches GitHub's success green).
    pub status_success: Color,
    /// Gold accent for the speed/effort chip in the AI chat bottom
    /// toolbar (#27: ⚡ icon + "2x" label in this color, no bg).
    /// ~#FFD93D (warm yellow, readable on dark panels).
    pub speed_accent: Color,
}

impl Theme {
    /// Dark palette tuned to the TS screenshot. Values eyeballed from
    /// the screenshot + apps/web/src/styles.css `.dark` block — exact
    /// hex parity isn't a goal, semantic parity is.
    pub const fn dark() -> Self {
        Self {
            background: rgb(0x12, 0x12, 0x12),
            foreground: rgb(0xfa, 0xfa, 0xfa),
            card: rgb(0x1e, 0x1e, 0x1e),
            card_foreground: rgb(0xfa, 0xfa, 0xfa),
            popover: rgb(0x18, 0x18, 0x18),
            popover_foreground: rgb(0xfa, 0xfa, 0xfa),
            primary: rgb(0x3b, 0x82, 0xf6),
            primary_foreground: rgb(0xff, 0xff, 0xff),
            muted: rgb(0x27, 0x27, 0x27),
            muted_foreground: rgb(0xa3, 0xa3, 0xa3),
            border: rgb(0x31, 0x31, 0x31),
            accent: rgb(0x2d, 0x2d, 0x2d),
            accent_foreground: rgb(0xfa, 0xfa, 0xfa),
            destructive: rgb(0xef, 0x44, 0x44),
            destructive_foreground: rgb(0xfa, 0xfa, 0xfa),
            secondary: rgb(0x2a, 0x2a, 0x2a),
            secondary_foreground: rgb(0xfa, 0xfa, 0xfa),
            input: rgb(0x34, 0x34, 0x34),
            ring: rgb(0x3b, 0x82, 0xf6),
            button_hover: rgba(0xff, 0xff, 0xff, 0.06),
            row_selected: rgb(0x30, 0x30, 0x30),
            row_selected_primary: rgba(0x3b, 0x82, 0xf6, 0.22),
            // Pencil-like dark canvas, kept slightly warmer/lighter than the shell.
            canvas_surface: rgb(0x1b, 0x1b, 0x1b),
            user_bubble: rgb(0x50, 0x52, 0x60),
            user_bubble_foreground: rgb(0xff, 0xff, 0xff),
            // GitHub-style success green ~#3FB950 for completed ✓ rings.
            status_success: rgb(0x3f, 0xb9, 0x50),
            // #FFD93D — warm yellow for the ⚡ speed chip icon + label.
            speed_accent: rgb(0xff, 0xd9, 0x3d),
        }
    }

    /// Light palette stub. Step 4 boots the editor in dark; light is
    /// kept so the TopBar's theme toggle has somewhere to land in
    /// Step 5 without another schema change.
    pub const fn light() -> Self {
        Self {
            background: rgb(0xef, 0xef, 0xef),
            foreground: rgb(0x1d, 0x1d, 0x1f),
            card: rgb(0xf7, 0xf7, 0xf7),
            card_foreground: rgb(0x1d, 0x1d, 0x1f),
            popover: rgb(0xff, 0xff, 0xff),
            popover_foreground: rgb(0x1d, 0x1d, 0x1f),
            primary: rgb(0x3b, 0x82, 0xf6),
            primary_foreground: rgb(0xff, 0xff, 0xff),
            muted: rgb(0xe9, 0xe9, 0xe9),
            muted_foreground: rgb(0x68, 0x68, 0x6d),
            border: rgb(0xd8, 0xd8, 0xda),
            accent: rgb(0xe8, 0xe8, 0xea),
            accent_foreground: rgb(0x1d, 0x1d, 0x1f),
            destructive: rgb(0xef, 0x44, 0x44),
            destructive_foreground: rgb(0xfa, 0xfa, 0xfa),
            secondary: rgb(0xea, 0xea, 0xec),
            secondary_foreground: rgb(0x1d, 0x1d, 0x1f),
            input: rgb(0xe1, 0xe1, 0xe4),
            ring: rgb(0x3b, 0x82, 0xf6),
            button_hover: rgba(0x00, 0x00, 0x00, 0.06),
            row_selected: rgb(0xe4, 0xe4, 0xe7),
            row_selected_primary: rgba(0x3b, 0x82, 0xf6, 0.18),
            // Pencil-like light canvas, distinct from side panels but not stark white.
            canvas_surface: rgb(0xf3, 0xf3, 0xf3),
            user_bubble: rgb(0x5f, 0x62, 0x70),
            user_bubble_foreground: rgb(0xff, 0xff, 0xff),
            // Success green is the same in both themes.
            status_success: rgb(0x3f, 0xb9, 0x50),
            // Slightly muted gold for light backgrounds (same hue as dark).
            speed_accent: rgb(0xd4, 0xa0, 0x10),
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
    fn light_theme_uses_pencil_like_neutral_shell() {
        let t = Theme::light();
        assert!(t.background.r > 0.90 && t.background.r < 0.97);
        assert!(t.canvas_surface.r > t.background.r);
        assert!(t.card.r > t.background.r);
        assert!(t.border.r < t.card.r);
    }

    #[test]
    fn dark_theme_uses_pencil_like_neutral_shell() {
        let t = Theme::dark();
        assert!(t.background.r > 0.05 && t.background.r < t.card.r);
        assert!(t.canvas_surface.r > t.background.r);
        assert!(t.border.r > t.card.r);
    }

    #[test]
    fn light_user_bubble_is_a_contrasting_graphite_chip() {
        let t = Theme::light();
        assert!(
            t.user_bubble.r < 0.45 && t.user_bubble.g < 0.45 && t.user_bubble.b < 0.50,
            "light user bubble should read as a graphite chip: {:?}",
            t.user_bubble
        );
        assert_eq!(t.user_bubble_foreground, Color::WHITE);
    }

    #[test]
    fn default_is_dark() {
        let t = Theme::default();
        assert!(t.background.r < 0.1, "default theme should be dark");
    }
}
