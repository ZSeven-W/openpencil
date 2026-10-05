//! Runtime-preview entry on completed normal-mode App and Web works.

use super::*;

impl WorkspaceSurface<'_> {
    /// Finished App/Web works expose the same real runtime as professional
    /// Play, without requiring a mode switch first.
    pub fn preview_button(&self, layout: &WorkspaceLayout) -> Option<Rect> {
        if !matches!(self.state.family, HomeFamily::AppUi | HomeFamily::Web) || !self.play_enabled()
        {
            return None;
        }
        let anchor = self.share_button(layout).unwrap_or(layout.export);
        let width =
            header_button_width(op_i18n::translate(self.ui.locale, "tooltip.topbar.preview"));
        Some(Rect::xywh(
            anchor.origin.x - HEADER_BUTTON_GAP - width,
            anchor.origin.y,
            width,
            anchor.size.y,
        ))
    }
}

/// Width of a header outline button holding a 15 px icon and `label`
/// at 13 px. Estimated per char (wide scripts at a full em) so layout,
/// hit-test and paint agree without a text measurer; never narrower
/// than the 导出 button beside it.
pub fn header_button_width(label: &str) -> f32 {
    let label_w: f32 = label
        .chars()
        .map(|c| if (c as u32) >= 0x1100 { 13.0 } else { 7.4 })
        .sum();
    (label_w + 15.0 + 8.0 + 18.0).max(EXPORT_BUTTON_W)
}
