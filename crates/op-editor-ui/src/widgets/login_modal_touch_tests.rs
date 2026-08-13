//! Focused responsive geometry coverage for the mobile sign-in surface.

use super::*;

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn phone_widths_clamp_with_gutters_and_keep_44pt_close_target() {
    let mut state = EditorState::new();
    state.editor_ui.touch = true;
    let modal = LoginModal::for_editor(&state);

    for (viewport_w, viewport_h, expected_w) in [
        (320.0, 568.0, 288.0),
        (390.0, 844.0, 358.0),
        (568.0, 320.0, MODAL_WIDTH),
    ] {
        let panel = modal.rect(viewport_w, viewport_h);
        let close = close_rect(panel, modal.touch);
        let sign_in = sign_in_rect(panel);
        let status = status_rect(panel);

        assert_eq!(panel.origin.x, (viewport_w - expected_w) / 2.0);
        assert_eq!(panel.size.x, expected_w);
        assert_eq!(close.size, Point2D::new(44.0, 44.0));
        for rect in [close, sign_in, status] {
            assert!(
                panel.contains(rect.origin),
                "{viewport_w}x{viewport_h}: {rect:?}"
            );
            assert!(panel.contains(Point2D::new(
                rect.origin.x + rect.size.x,
                rect.origin.y + rect.size.y,
            )));
        }
        assert!(status.size.y > 0.0);
        assert!(status.origin.y + status.size.y <= sign_in.origin.y - 8.0);
        assert_eq!(modal.hit_test(panel, center(close)), LoginModalHit::Close);
        assert!(!modal.sign_in_enabled);
        assert_eq!(
            modal.hit_test(panel, center(sign_in)),
            LoginModalHit::Inside
        );
    }
}
