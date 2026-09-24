//! Desktop half of the Studio Home document swap.
//!
//! A brief sent from Home swaps in a fresh page (op-host-native
//! `widget_host::home_document_swap`). Two things used to go wrong on
//! desktop: the replaced design's unsaved work vanished, and `current_path`
//! still named the replaced design's file, so the next ⌘S wrote the NEW
//! design over it. The shell drains the parked document here: the new design
//! starts untitled, and unsaved work is kept as a rescue copy that shows up
//! in Home's recent projects. No dialog: Home is where a new thing starts,
//! and a prompt at that moment would only be clicked through.

use std::path::{Path, PathBuf};

use op_host_native::widget_host::ReplacedHomeDocument;

use crate::DesktopApp;

/// `Documents/OpenPencil/Autosave` — somewhere a user can find by hand.
const RESCUE_DIR: [&str; 2] = ["OpenPencil", "Autosave"];

impl DesktopApp {
    /// Drain the document the last Home brief replaced. Returns true when
    /// state changed.
    pub(crate) fn drain_replaced_home_document(&mut self) -> bool {
        let Some(replaced) = self.host.take_replaced_home_document() else {
            return false;
        };
        self.current_path = None;
        crate::persistence::refresh_title(&self.current_path, self.window.as_ref());
        if replaced.had_unsaved_changes {
            let Some(dir) = rescue_dir() else {
                eprintln!("[home] no documents directory; the replaced design was not kept");
                return true;
            };
            match save_rescue_copy(&replaced, &dir, now_unix_seconds()) {
                Ok(path) => crate::settings_io::touch_recent(&mut self.host, &path),
                Err(error) => eprintln!("[home] could not keep the replaced design: {error}"),
            }
        }
        true
    }
}

fn rescue_dir() -> Option<PathBuf> {
    let base = dirs::document_dir().or_else(dirs::home_dir)?;
    Some(RESCUE_DIR.iter().fold(base, |dir, part| dir.join(part)))
}

fn now_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

/// Write the replaced design as `<title> <YYYY-MM-DD HHMMSS>.op` (UTC) in `dir`.
fn save_rescue_copy(
    replaced: &ReplacedHomeDocument,
    dir: &Path,
    unix_seconds: i64,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let path = dir.join(rescue_file_name(&replaced.title, unix_seconds));
    op_host_services::doc_io::save_to_path(&replaced.state, &path)
        .map_err(|error| error.to_string())?;
    Ok(path)
}

fn rescue_file_name(title: &str, unix_seconds: i64) -> String {
    let stem: String = title
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .filter(|c| !c.is_control())
        .take(60)
        .collect();
    let stem = stem.trim();
    let stem = if stem.is_empty() { "Untitled" } else { stem };
    let (y, m, d) = crate::git_jobs::civil_from_days(unix_seconds.div_euclid(86_400));
    let secs = unix_seconds.rem_euclid(86_400);
    format!(
        "{stem} {y:04}-{m:02}-{d:02} {:02}{:02}{:02}.op",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rescue_names_are_safe_and_dated() {
        // 2026-09-25 02:15:07 UTC
        let t = 1_790_302_507;
        assert_eq!(
            rescue_file_name("咖啡点单 App", t),
            "咖啡点单 App 2026-09-25 021507.op"
        );
        assert_eq!(rescue_file_name("a/b:c", t), "a-b-c 2026-09-25 021507.op");
        assert_eq!(rescue_file_name("   ", t), "Untitled 2026-09-25 021507.op");
    }

    #[test]
    fn a_rescue_copy_is_written_where_asked() {
        let dir = std::env::temp_dir().join(format!("op-home-rescue-{}", std::process::id()));
        let replaced = ReplacedHomeDocument {
            state: Box::new(op_editor_core::EditorState::starter()),
            had_unsaved_changes: true,
            title: "咖啡点单 App".into(),
        };
        let path = save_rescue_copy(&replaced, &dir, 1_790_302_507).expect("saved");
        assert!(path.starts_with(&dir));
        assert!(path.is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
