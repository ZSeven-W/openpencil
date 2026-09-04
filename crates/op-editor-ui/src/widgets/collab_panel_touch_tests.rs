//! Narrow touch-panel text regressions.

use super::*;
use crate::widgets::test_capture_backend::CaptureBackend;

#[test]
fn unavailable_message_fits_a_320pt_phone_in_every_locale() {
    for locale in op_i18n::Locale::ALL {
        let mut ui = EditorUiState {
            touch: true,
            locale,
            ..Default::default()
        };
        ui.collab.panel.open = true;
        let panel = CollabPanel::for_editor_ui(&ui).expect("open panel");
        let rect = panel.rect_at(
            Rect::xywh(268.0, 0.0, 44.0, 44.0),
            Rect::xywh(0.0, 0.0, 320.0, 568.0),
        );
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
        let message = backend.texts[1].0.clone();
        assert!(
            text_metrics::measure_chrome(&mut backend, &message, 12.0)
                <= rect.size.x - PAD * 2.0 + 0.01,
            "{} overflows: {message}",
            locale.code()
        );
    }
}
