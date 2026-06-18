use std::sync::Arc;

use op_editor_ui::widgets::property_panel_typography::BUNDLED_FONT_FAMILIES;

use super::WidgetHost;

pub(crate) fn normalize_browser_system_font_families(families: Vec<String>) -> Vec<String> {
    let bundled = BUNDLED_FONT_FAMILIES
        .iter()
        .map(|family| family.to_lowercase())
        .collect::<std::collections::HashSet<_>>();
    let mut out = families
        .into_iter()
        .map(|family| family.trim().to_string())
        .filter(|family| !family.is_empty())
        .filter(|family| !bundled.contains(&family.to_lowercase()))
        .collect::<Vec<_>>();
    out.sort_by_key(|family| family.to_lowercase());
    out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    out
}

impl WidgetHost {
    pub(crate) fn apply_browser_system_font_families(&mut self, families: Vec<String>) {
        let families = normalize_browser_system_font_families(families);
        let ui = &mut self.editor_state.editor_ui;
        ui.system_font_families = Arc::new(families);
        ui.system_fonts_loaded = true;
        ui.font_picker.hover = None;
        self.mark_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_browser_system_fonts_like_native_host() {
        let families = normalize_browser_system_font_families(vec![
            "PingFang SC".to_string(),
            "Inter".to_string(),
            " pingfang sc ".to_string(),
            "".to_string(),
            "SF Pro".to_string(),
        ]);

        assert_eq!(families, vec!["PingFang SC", "SF Pro"]);
    }

    #[test]
    fn applies_browser_system_fonts_to_picker_state() {
        let mut host = crate::widget_host::WidgetHost::new();

        host.apply_browser_system_font_families(vec![
            "Inter".to_string(),
            "PingFang SC".to_string(),
            "SF Pro".to_string(),
        ]);

        let ui = &host.editor_state.editor_ui;
        assert!(ui.system_fonts_loaded);
        assert_eq!(
            ui.system_font_families.as_ref(),
            &vec!["PingFang SC".to_string(), "SF Pro".to_string()]
        );
    }
}
