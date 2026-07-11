//! Cross-launch UI preferences (`~/.openpencil/ui.json`) — small,
//! non-secret editor chrome choices that should survive a restart.

use op_editor_core::PencilCursorStyle;

const FILE: &str = "ui.json";

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct UiPrefs {
    #[serde(default)]
    pencil_cursor: Option<String>,
}

pub(crate) fn save_pencil_cursor(style: PencilCursorStyle) {
    let value = UiPrefs {
        pencil_cursor: Some(style.id().to_string()),
    };
    if let Err(err) = op_config_store::write_json(FILE, &value) {
        eprintln!("[ui-prefs] write failed: {err}");
    }
}

pub(crate) fn load_pencil_cursor() -> Option<PencilCursorStyle> {
    let value: UiPrefs = op_config_store::read_json(FILE).ok().flatten()?;
    PencilCursorStyle::from_id(value.pencil_cursor.as_deref()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_ids_round_trip() {
        for style in PencilCursorStyle::ALL {
            assert_eq!(PencilCursorStyle::from_id(style.id()), Some(style));
        }
    }
}
