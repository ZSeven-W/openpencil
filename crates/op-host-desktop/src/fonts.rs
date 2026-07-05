//! Native persistence for user-imported fonts.
//!
//! User-imported `.ttf` / `.otf` faces live under `<config>/fonts/` next to a
//! small `index.json`. On startup [`FontStore::rescan_and_register`]
//! re-registers every stored face with `jian-skia` so an imported family
//! survives a restart; [`FontStore::import`] validates + copies a new file in
//! and registers it; [`FontStore::remove`] drops a whole family from both disk
//! and the live registry. The in-memory registry + generation-aware
//! invalidation live in `jian-skia`; this module only owns the disk side.

use std::path::{Path, PathBuf};

use jian_core::layout::measure::FontStyleKind;
use op_config_store::ConfigStore;
use serde::{Deserialize, Serialize};

/// Reject absurd font files early. A normal face is well under 1 MiB; a large
/// CJK / variable font can reach a few MiB, so 16 MiB is a generous ceiling
/// that still bounds disk + parse cost and a hostile input.
pub(crate) const MAX_FONT_BYTES: usize = 16 * 1024 * 1024;

const FONTS_SUBDIR: &str = "fonts";
const INDEX_FILE: &str = "fonts/index.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FontIndexEntry {
    family: String,
    italic: bool,
    weight: u16,
    hash: u64,
    /// File name within the fonts dir (e.g. `"a1b2c3d4e5f60718.ttf"`).
    file: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FontIndex {
    fonts: Vec<FontIndexEntry>,
}

/// Disk-backed store for user-imported fonts.
pub struct FontStore {
    dir: PathBuf,
    index_path: PathBuf,
}

impl FontStore {
    /// The per-user store under the shared OpenPencil config dir.
    pub fn user() -> std::io::Result<Self> {
        Ok(Self::at(ConfigStore::user()?.root()))
    }

    /// Store rooted at `config_root/fonts` — used by tests with a temp dir.
    pub fn at(config_root: impl AsRef<Path>) -> Self {
        let root = config_root.as_ref();
        Self {
            dir: root.join(FONTS_SUBDIR),
            index_path: root.join(INDEX_FILE),
        }
    }

    fn load_index(&self) -> FontIndex {
        op_config_store::read_json_path(&self.index_path)
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    fn save_index(&self, index: &FontIndex) -> std::io::Result<()> {
        op_config_store::write_json_path(&self.index_path, index)
    }

    /// Re-register every persisted font with `jian-skia`. A missing or
    /// corrupt entry is dropped from the index (and logged), never fatal — a
    /// bad file must not block startup.
    pub fn rescan_and_register(&self) {
        let index = self.load_index();
        let mut kept: Vec<FontIndexEntry> = Vec::with_capacity(index.fonts.len());
        let mut changed = false;
        for entry in index.fonts {
            let path = self.dir.join(&entry.file);
            match std::fs::read(&path) {
                Ok(bytes) => match jian_skia::register_imported_font(bytes) {
                    Ok(_) => kept.push(entry),
                    Err(err) => {
                        eprintln!(
                            "[fonts] dropping unparseable imported font {}: {err}",
                            path.display()
                        );
                        let _ = std::fs::remove_file(&path);
                        changed = true;
                    }
                },
                Err(err) => {
                    eprintln!(
                        "[fonts] dropping missing imported font {}: {err}",
                        path.display()
                    );
                    changed = true;
                }
            }
        }
        if changed {
            if let Err(err) = self.save_index(&FontIndex { fonts: kept }) {
                eprintln!("[fonts] failed to rewrite font index: {err}");
            }
        }
    }

    /// Validate, persist, then register a font file. Returns the registered
    /// blob on success. Rejects oversize / unparseable input without touching
    /// disk.
    ///
    /// Persistence happens BEFORE the live registration: the bytes are parsed
    /// (no registry mutation) to learn the file/index key, written to disk,
    /// and only then committed to the process-global registry. Registering
    /// first would leave a live-but-unpersisted font — visible + rendered
    /// this session but gone on restart — if a disk write then failed while
    /// the caller reported the import as failed.
    pub fn import(&self, bytes: Vec<u8>) -> Result<jian_skia::FontBlob, String> {
        if bytes.len() > MAX_FONT_BYTES {
            return Err(format!(
                "font file is too large ({:.1} MiB; max {} MiB)",
                bytes.len() as f64 / (1024.0 * 1024.0),
                MAX_FONT_BYTES / (1024 * 1024)
            ));
        }
        // Parse WITHOUT registering — validates the bytes and yields the
        // family/style/weight/hash the on-disk entry is keyed on.
        let meta = jian_skia::parse_imported_font_meta(&bytes)
            .ok_or_else(|| "not a valid ttf/otf font file".to_string())?;

        let file = format!("{:016x}.ttf", meta.hash);
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("create fonts dir: {e}"))?;
        std::fs::write(self.dir.join(&file), &bytes)
            .map_err(|e| format!("write font file: {e}"))?;

        let italic = matches!(meta.style, FontStyleKind::Italic);
        let mut index = self.load_index();
        // Replace any prior entry for the same face (last-import-wins, matching
        // the in-memory registry), pruning its now-orphaned file.
        index.fonts.retain(|e| {
            let same = e.family == meta.family && e.italic == italic && e.weight == meta.weight;
            if same && e.file != file {
                let _ = std::fs::remove_file(self.dir.join(&e.file));
            }
            !same
        });
        index.fonts.push(FontIndexEntry {
            family: meta.family.clone(),
            italic,
            weight: meta.weight,
            hash: meta.hash,
            file,
        });
        self.save_index(&index)
            .map_err(|e| format!("write font index: {e}"))?;

        // Persistence is durable — now commit the live registration. This
        // re-parses the same bytes, so it cannot fail after `parse_*` above
        // succeeded, but propagate any error rather than unwrap.
        jian_skia::register_imported_font(bytes).map_err(|e| format!("register font: {e}"))
    }

    /// Remove every imported face of `family` from the live registry and disk.
    /// Returns whether anything was removed.
    pub fn remove(&self, family: &str) -> bool {
        let removed_live = jian_skia::remove_imported_font(family);
        let mut index = self.load_index();
        let before = index.fonts.len();
        index.fonts.retain(|e| {
            if e.family == family {
                let _ = std::fs::remove_file(self.dir.join(&e.file));
                false
            } else {
                true
            }
        });
        let removed_disk = index.fonts.len() != before;
        if removed_disk {
            let _ = self.save_index(&index);
        }
        removed_live || removed_disk
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // A bundled family distinct from those used by op-host-native's font
    // tests (different test binary anyway) and unlikely to be system-installed.
    const FONT: &[u8] = include_bytes!("../assets/fonts/InstrumentSerif-Regular.ttf");
    const FAMILY: &str = "Instrument Serif";

    // A second distinct family for the failed-persist test, so it can't race
    // the round-trip test's family in the shared process-global registry.
    const FONT2: &[u8] = include_bytes!("../assets/fonts/Outfit-VF.ttf");
    const FAMILY2: &str = "Outfit";

    static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempRoot(PathBuf);
    impl TempRoot {
        fn new() -> Self {
            let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("op-fonts-test-{}-{seq}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn import_persists_registers_then_rescan_and_remove_round_trip() {
        let root = TempRoot::new();
        // Start clean in the process-global registry too.
        jian_skia::remove_imported_font(FAMILY);

        let store = FontStore::at(&root.0);

        // --- import: validates, registers, writes the file + index ---------
        let blob = store.import(FONT.to_vec()).expect("import a valid font");
        assert_eq!(blob.family, FAMILY);
        let stored_file = root
            .0
            .join(FONTS_SUBDIR)
            .join(format!("{:016x}.ttf", blob.hash));
        assert!(stored_file.exists(), "font file copied into the store");
        assert!(root.0.join(INDEX_FILE).exists(), "index written");
        assert_eq!(
            jian_skia::list_families()
                .iter()
                .filter(|m| m.family == FAMILY)
                .count(),
            1,
            "family registered in the live registry"
        );

        // --- rescan: a fresh store over the same dir re-registers ----------
        jian_skia::remove_imported_font(FAMILY);
        assert_eq!(
            jian_skia::list_families()
                .iter()
                .filter(|m| m.family == FAMILY)
                .count(),
            0,
            "cleared before rescan"
        );
        FontStore::at(&root.0).rescan_and_register();
        assert_eq!(
            jian_skia::list_families()
                .iter()
                .filter(|m| m.family == FAMILY)
                .count(),
            1,
            "rescan re-registers the persisted family"
        );

        // --- remove: drops the family from disk + registry -----------------
        assert!(store.remove(FAMILY), "remove reports a change");
        assert!(!stored_file.exists(), "font file deleted");
        assert_eq!(
            jian_skia::list_families()
                .iter()
                .filter(|m| m.family == FAMILY)
                .count(),
            0,
            "family gone from the live registry"
        );
        // Index no longer lists it, so a rescan is a no-op.
        FontStore::at(&root.0).rescan_and_register();
        assert_eq!(
            jian_skia::list_families()
                .iter()
                .filter(|m| m.family == FAMILY)
                .count(),
            0,
            "nothing to re-register after removal"
        );
    }

    #[test]
    fn oversize_and_corrupt_imports_are_rejected_without_touching_disk() {
        let root = TempRoot::new();
        let store = FontStore::at(&root.0);

        let too_big = vec![0u8; MAX_FONT_BYTES + 1];
        assert!(store.import(too_big).is_err(), "oversize rejected");

        let not_a_font = b"this is definitely not a font file".to_vec();
        assert!(store.import(not_a_font).is_err(), "corrupt bytes rejected");

        // Neither wrote an index (no successful import happened).
        assert!(
            !root.0.join(INDEX_FILE).exists(),
            "a rejected import must not create the index"
        );
    }

    #[test]
    fn failed_persist_does_not_leak_into_the_live_registry() {
        // Root is a FILE, not a directory, so `create_dir_all(root/fonts)`
        // fails and the import errors AFTER parse but BEFORE the live
        // registration — proving persistence precedes registration.
        let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let file_root =
            std::env::temp_dir().join(format!("op-fonts-notadir-{}-{seq}", std::process::id()));
        let _ = std::fs::remove_dir_all(&file_root);
        std::fs::write(&file_root, b"x").unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _cleanup = Cleanup(file_root.clone());

        jian_skia::remove_imported_font(FAMILY2); // clean slate in the shared registry

        let store = FontStore::at(&file_root);
        assert!(
            store.import(FONT2.to_vec()).is_err(),
            "import into a file-rooted store must fail at the disk write"
        );
        assert_eq!(
            jian_skia::list_families()
                .iter()
                .filter(|m| m.family == FAMILY2)
                .count(),
            0,
            "a failed persist must NOT leave the font registered in the live registry"
        );
    }
}
