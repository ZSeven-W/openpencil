//! System font family enumeration for the native property panel.

use std::sync::mpsc::{self, Receiver};
use std::sync::OnceLock;

static SYSTEM_FONT_FAMILIES: OnceLock<Vec<String>> = OnceLock::new();

pub(crate) fn system_font_families() -> Vec<String> {
    SYSTEM_FONT_FAMILIES
        .get_or_init(enumerate_system_font_families)
        .clone()
}

pub(crate) fn spawn_system_font_loader() -> Receiver<Vec<String>> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("op-system-fonts".into())
        .spawn(move || {
            let _ = tx.send(system_font_families());
        });
    rx
}

fn enumerate_system_font_families() -> Vec<String> {
    let mgr = skia_safe::FontMgr::new();
    let families: Vec<String> = mgr
        .family_names()
        .map(|family| family.trim().to_string())
        .filter(|family| !family.is_empty())
        .collect();
    op_editor_ui::widgets::property_panel_font_picker::prepare_system_font_families(families)
}

#[cfg(test)]
mod tests {
    #[test]
    fn system_font_enumeration_returns_unique_names() {
        let families = super::enumerate_system_font_families();
        for family in &families {
            assert!(!family.trim().is_empty());
        }
        for pair in families.windows(2) {
            assert!(!pair[0].eq_ignore_ascii_case(&pair[1]));
        }
    }
}
