//! Footer toolbar geometry for the AI chat panel.

use super::ai_chat_panel::{
    footer_label_width, AIChatPlaceholder, FooterLayout, INPUT_TOOLBAR_HEIGHT, PAD,
};
use crate::Rect;

impl<'a> AIChatPlaceholder<'a> {
    pub(crate) fn footer_layout(
        &self,
        rect: Rect,
        input_rect: Rect,
        toolbar_top: f32,
    ) -> FooterLayout {
        let toolbar_center_y = toolbar_top + INPUT_TOOLBAR_HEIGHT / 2.0;
        let rx = rect.origin.x + rect.size.x - PAD;
        let attach = Rect::xywh(rx - 58.0, toolbar_center_y - 12.0, 24.0, 24.0);
        let send = Rect::xywh(rx - 24.0, toolbar_center_y - 12.0, 24.0, 24.0);
        let selected = self.state.selected_model_entry();
        let model_name: &str = selected
            .map(|m| m.display_name.as_str())
            .unwrap_or(self.label_no_models.as_str());
        let count = self.selected_count.to_string();
        let selected_label =
            op_i18n::translate(self.locale, "common.selected").replace("{{count}}", &count);
        let model_x = input_rect.origin.x - 6.0;
        let selected_w = footer_label_width(&selected_label, 10.0);
        let max_model_w = (attach.origin.x - selected_w - 62.0 - model_x).max(96.0);
        let desired_model_w = (26.0 + footer_label_width(model_name, 12.0) + 24.0).max(96.0);
        let model = Rect::xywh(
            model_x,
            toolbar_center_y - 14.0,
            desired_model_w.min(max_model_w),
            28.0,
        );
        let agent_team = Rect::xywh(
            model.origin.x + model.size.x + 8.0,
            toolbar_center_y - 11.0,
            36.0,
            22.0,
        );
        FooterLayout {
            model,
            agent_team,
            attach,
            send,
        }
    }
}

pub(crate) fn fit_footer_label(label: &str, size: f32, max_w: f32) -> String {
    if footer_label_width(label, size) <= max_w {
        return label.to_string();
    }
    let ellipsis_w = footer_label_width("…", size);
    let budget = (max_w - ellipsis_w).max(0.0);
    let mut out = String::new();
    let mut w = 0.0;
    for ch in label.chars() {
        let next = footer_label_width(&ch.to_string(), size);
        if w + next > budget {
            break;
        }
        out.push(ch);
        w += next;
    }
    out.push('…');
    out
}

pub(crate) fn footer_label_baseline(center_y: f32, size: f32) -> f32 {
    center_y + size * 0.35
}
