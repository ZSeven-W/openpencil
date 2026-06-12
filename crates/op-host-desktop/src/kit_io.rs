//! Component-Browser kit IO host logic — drains the panel's kit
//! Import / Export requests (which need the native file dialogs the
//! widget layer cannot reach) and flushes imported-kit persistence.
//!
//! Mirrors `design_md_host.rs`'s drain pattern; the pure kit logic
//! lives in `op_editor_core::uikit_io` (TS `kit-import-export.ts`)
//! and the on-disk store in `kit_persistence.rs` (TS
//! `uikit-store.ts` persist/hydrate).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use op_editor_core::uikit_io;
use op_editor_core::KitIoRequest;

use crate::DesktopApp;

static KIT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Mint a unique imported-kit id. TS uses `nanoid()`; uniqueness is
/// the only contract (the id never leaves the local store), so a
/// timestamp + counter suffices — documented divergence.
fn mint_kit_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let counter = KIT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("kit-{nanos:016x}-{counter:04x}")
}

impl DesktopApp {
    /// Run a queued kit Import / Export, then flush persistence when
    /// the kit list or the browser-open flag changed. Called from the
    /// per-event drain block (after `drain_component_browser_insert`).
    pub(crate) fn drain_kit_io(&mut self) {
        let request = self
            .host
            .editor_state_mut()
            .editor_ui
            .component_browser_kit_request
            .take();
        match request {
            Some(KitIoRequest::Import) => self.import_kit_file(),
            Some(KitIoRequest::Export) => self.export_kit_file(),
            None => {}
        }
        // Persistence: `ui_kits_changed` (import / delete) or a
        // browser-open flip both rewrite `uikits.json` — TS persists
        // `importedKits` + `browserOpen` together on every mutation.
        let open = self.host.editor_state().editor_ui.component_browser_open;
        let kits_changed =
            std::mem::take(&mut self.host.editor_state_mut().editor_ui.ui_kits_changed);
        if kits_changed || self.kit_browser_open_persisted != Some(open) {
            // Tests must never write a developer machine's real
            // `uikits.json` (the load side is skipped the same way).
            if !cfg!(test) {
                crate::kit_persistence::save(self.host.editor_state());
            }
            self.kit_browser_open_persisted = Some(open);
        }
    }

    /// Pick a `.op` / `.pen` / `.json` file and import its reusable
    /// components as a kit (TS `importKitFromFile`: cancel and
    /// parse-failure are silent, a file with no reusable components
    /// imports nothing).
    fn import_kit_file(&mut self) {
        let locale = self.host.editor_state().editor_ui.locale;
        let picked = rfd::FileDialog::new()
            .set_title(op_i18n::translate(locale, "componentBrowser.importKit"))
            .add_filter("OpenPencil File", &["op", "pen", "json"])
            .pick_file();
        let Some(path) = picked else {
            return;
        };
        let src = match std::fs::read_to_string(&path) {
            Ok(src) => src,
            Err(err) => {
                eprintln!("openpencil-desktop: kit import read failed: {err}");
                return;
            }
        };
        let doc = match op_pen_loader::load_canonical(&src) {
            Ok(loaded) => loaded.value,
            Err(err) => {
                eprintln!("openpencil-desktop: kit import parse failed: {err}");
                return;
            }
        };
        let Some(kit) = uikit_io::import_kit_from_document(&doc, mint_kit_id()) else {
            // No reusable components — TS returns null silently.
            return;
        };
        self.host.editor_state_mut().import_kit(kit);
        self.host.mark_editor_state_dirty();
    }

    /// Export every reusable component in the open document as a kit
    /// file (TS `handleExport`: kit name = document name or "My
    /// Kit"; nothing reusable exports nothing).
    fn export_kit_file(&mut self) {
        let state = self.host.editor_state();
        let kit_name = state
            .doc
            .name
            .clone()
            .unwrap_or_else(|| "My Kit".to_string());
        let Some(kit_doc) = uikit_io::build_kit_document(&state.doc, &[], &kit_name) else {
            // No reusable components — TS returns false silently.
            return;
        };
        let locale = state.editor_ui.locale;
        let picked = rfd::FileDialog::new()
            .set_title(op_i18n::translate(locale, "componentBrowser.exportKit"))
            .add_filter("OpenPencil", &["op", "pen"])
            .set_file_name(uikit_io::kit_export_file_name(&kit_name))
            .save_file();
        let Some(path) = picked else {
            return;
        };
        let json = match serde_json::to_string_pretty(&kit_doc) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("openpencil-desktop: kit export encode failed: {err}");
                return;
            }
        };
        if let Err(err) = std::fs::write(&path, json) {
            eprintln!("openpencil-desktop: kit export write failed: {err}");
        }
    }
}
