//! Cross-launch UI preferences (`~/.openpencil/ui.json`) — small,
//! non-secret editor chrome choices that should survive a restart.

use op_editor_core::PencilCursorStyle;

const FILE: &str = "ui.json";

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct UiPrefs {
    #[serde(default)]
    pencil_cursor: Option<String>,
    #[serde(default)]
    agent_swarm: Option<bool>,
}

/// Every persisted choice. `ui.json` is rewritten whole on each save, so a
/// save must carry all of them or it silently resets the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UiPrefsSnapshot {
    pub(crate) pencil_cursor: PencilCursorStyle,
    pub(crate) agent_swarm: bool,
}

pub(crate) fn save(snapshot: UiPrefsSnapshot) {
    crate::test_config_root::guard_user_config();
    let value = UiPrefs {
        pencil_cursor: Some(snapshot.pencil_cursor.id().to_string()),
        agent_swarm: Some(snapshot.agent_swarm),
    };
    if let Err(err) = op_config_store::write_json(FILE, &value) {
        eprintln!("[ui-prefs] write failed: {err}");
    }
}

/// Stored choices; a missing or unknown entry is `None` (keep the default).
pub(crate) fn load() -> (Option<PencilCursorStyle>, Option<bool>) {
    crate::test_config_root::guard_user_config();
    let value: UiPrefs = op_config_store::read_json(FILE)
        .ok()
        .flatten()
        .unwrap_or_default();
    (
        value
            .pencil_cursor
            .as_deref()
            .and_then(PencilCursorStyle::from_id),
        value.agent_swarm,
    )
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
