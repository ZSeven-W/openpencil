use crate::{Color, Rect, RenderBackend, Theme};
use jian_widgets::components::button::{Button, ButtonVariant};
use jian_widgets::{Density, Tokens};

pub(crate) fn tokens_from_theme(theme: &Theme) -> Tokens {
    Tokens {
        background: theme.background,
        foreground: theme.foreground,
        card: theme.card,
        card_foreground: theme.card_foreground,
        popover: theme.popover,
        popover_foreground: theme.popover_foreground,
        primary: theme.primary,
        primary_foreground: theme.primary_foreground,
        muted: theme.muted,
        muted_foreground: theme.muted_foreground,
        border: theme.border,
        accent: theme.accent,
        accent_foreground: theme.accent_foreground,
        destructive: theme.destructive,
        button_hover: theme.button_hover,
        row_selected: theme.row_selected,
        row_selected_primary: theme.row_selected_primary,
        density: Density::Desktop,
    }
}

pub(crate) fn paint_ghost_button_feedback(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    rect: Rect,
    hovered: bool,
    pressed: bool,
) -> Color {
    Button {
        label: "",
        icon_d: None,
        variant: ButtonVariant::Ghost,
        enabled: true,
        hovered,
        pressed,
        font_size: 0.0,
    }
    .paint(backend, rect, &tokens_from_theme(theme));

    if hovered || pressed {
        theme.foreground
    } else {
        theme.muted_foreground
    }
}

pub(crate) fn paint_button_feedback_wash(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    rect: Rect,
    radius: f32,
    hovered: bool,
    pressed: bool,
) -> Color {
    if hovered || pressed {
        let color = if pressed {
            theme.button_hover.with_alpha(theme.button_hover.a * 1.8)
        } else {
            theme.button_hover
        };
        backend.fill_round_rect(rect, radius, color);
        theme.foreground
    } else {
        theme.muted_foreground
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point2D, TextLayout};

    #[derive(Default)]
    struct CaptureBackend {
        fills: Vec<(Rect, f32, Color)>,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}

        fn end_frame(&mut self) {}

        fn fill_rect(&mut self, _rect: Rect, _color: Color) {}

        fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {}

        fn draw_text(&mut self, _layout: &TextLayout, _origin: Point2D) {}

        fn clip_rect(&mut self, _rect: Rect) {}

        fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {}

        fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
            self.fills.push((rect, radius, color));
        }

        fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {}

        fn stroke_svg_path(
            &mut self,
            _d: &str,
            _top_left: Point2D,
            _size: f32,
            _color: Color,
            _width: f32,
        ) {
        }

        fn save(&mut self) {}

        fn restore(&mut self) {}

        fn translate(&mut self, _offset: Point2D) {}

        fn resize(&mut self, _width: u32, _height: u32) {}

        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn ghost_feedback_paints_pressed_token() {
        let theme = Theme::dark();
        let mut backend = CaptureBackend::default();
        let rect = Rect::xywh(8.0, 10.0, 32.0, 32.0);

        let icon_color = paint_ghost_button_feedback(&mut backend, &theme, rect, true, true);

        assert_eq!(
            backend.fills,
            vec![(
                rect,
                6.0,
                theme.button_hover.with_alpha(theme.button_hover.a * 1.8)
            )]
        );
        assert_eq!(icon_color, theme.foreground);
    }

    #[test]
    fn ghost_feedback_hover_only_paints_hover_token() {
        let theme = Theme::dark();
        let mut backend = CaptureBackend::default();
        let rect = Rect::xywh(8.0, 10.0, 32.0, 32.0);

        let icon_color = paint_ghost_button_feedback(&mut backend, &theme, rect, true, false);

        assert_eq!(backend.fills, vec![(rect, 6.0, theme.button_hover)]);
        assert_eq!(icon_color, theme.foreground);
    }

    #[test]
    fn ghost_feedback_idle_paints_no_background() {
        let theme = Theme::dark();
        let mut backend = CaptureBackend::default();
        let rect = Rect::xywh(8.0, 10.0, 32.0, 32.0);

        let icon_color = paint_ghost_button_feedback(&mut backend, &theme, rect, false, false);

        assert!(backend.fills.is_empty());
        assert_eq!(icon_color, theme.muted_foreground);
    }
}
