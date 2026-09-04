//! App-bundled design fonts for the browser host.
//!
//! The desktop binary embeds these OFL faces with `include_bytes!` and
//! registers them with jian-skia before the first frame
//! (`op-host-desktop/src/bundled_fonts.rs`). The browser can do neither: the
//! wasm bundle omits them to stay under its size ceiling, and CanvasKit ships
//! no fonts of its own. Without this module a document authored in Inter pops
//! the missing-fonts modal on web and paints a fallback face.
//!
//! So the mount fetches every bundled file from the daemon in parallel, patches
//! any thin variable default (`vf_normalize`), and registers each under the
//! family name parsed out of the file. Once ALL requests have settled — success
//! or failure — the registered families are published to the editor state,
//! which releases the missing-font detection gate.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::repaint_ctx::RepaintContext;

/// Route the daemon serves the staged fonts from. Must match the `fonts/`
/// directory `tools/stage-web-assets.sh` writes into the bundle's assets dir —
/// a mismatch is a silent 404 per file, not a build error.
const FONT_ROUTE_PREFIX: &str = "/pkg/assets/fonts/";

/// The bundled font files, mirroring desktop's `BUNDLED` list. Roboto is the
/// native backend's last-resort face and is registered here too so a document
/// that names it resolves the family instead of merely inheriting it.
///
/// Desktop's list additionally ships `DMMono-Medium.ttf`. It is deliberately
/// absent here: the CanvasKit bundled registry holds ONE face per family
/// (`registerBundledFont` replaces on the same key), so registering both DM
/// Mono weights would make the surviving face depend on which of two parallel
/// fetches settles last — DM Mono would render Regular or Medium by network
/// timing. Desktop keeps both because its provider matches faces by style.
const BUNDLED_FONT_FILES: &[&str] = &[
    "Roboto-Regular.ttf",
    "Inter-VF.ttf",
    "SpaceGrotesk-VF.ttf",
    "Manrope-VF.ttf",
    "Outfit-VF.ttf",
    "DMSans-VF.ttf",
    "DMSerifDisplay-Regular.ttf",
    "DMMono-Regular.ttf",
    "InstrumentSerif-Regular.ttf",
    "JetBrainsMono-VF.ttf",
    "CormorantGaramond-VF.ttf",
];

thread_local! {
    /// The settled family list waiting to reach the editor state.
    ///
    /// The last fetch to settle normally applies it inline. If the host happens
    /// to be borrowed at that instant, the list parks here instead of being
    /// dropped — leaving it unapplied would strand `bundled_fonts_pending` and
    /// suppress the missing-font modal for the rest of the session.
    static PENDING_APPLY: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Fetch and register every bundled design font. Call once at mount, after
/// `WidgetHost::begin_bundled_font_loading` has armed the detection gate.
pub(crate) fn load_bundled_fonts_at_mount<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    // All requests are fired at once: this is a small fixed set on the critical
    // path for correct text, unlike the unbounded preview-asset grid that
    // `web_asset_fetch` deliberately rate-limits.
    let settled = Rc::new(Cell::new(0usize));
    let families = Rc::new(RefCell::new(Vec::<String>::new()));
    for file in BUNDLED_FONT_FILES {
        let url = crate::daemon_base::daemon_url(&format!("{FONT_ROUTE_PREFIX}{file}"));
        let inner = inner.clone();
        let settled = settled.clone();
        let families = families.clone();
        crate::web_asset_fetch::fetch_bytes(&url, move |result| {
            if let Ok(bytes) = result {
                if let Some(family) = register_bundled_font(&inner, &bytes) {
                    families.borrow_mut().push(family);
                }
            }
            // Counted for failures too — a 404 for one face must not hold the
            // detection gate closed forever.
            settled.set(settled.get() + 1);
            if settled.get() == BUNDLED_FONT_FILES.len() {
                let families = std::mem::take(&mut *families.borrow_mut());
                PENDING_APPLY.with(|slot| *slot.borrow_mut() = Some(families));
                apply_pending(&inner);
            }
        });
    }
}

/// Apply a parked family list from the frame pump. Cheap when idle: one borrow
/// of an empty slot.
pub(crate) fn drain_pending_apply<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    if PENDING_APPLY.with(|slot| slot.borrow().is_none()) {
        return;
    }
    apply_pending(inner);
}

/// Register one fetched face and return the family it was registered under.
fn register_bundled_font<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    bytes: &[u8],
) -> Option<String> {
    // The vendored CanvasKit build reports no family name, so the family comes
    // from the file's own `name` table — the same parse the import path uses.
    let family = crate::font_meta::parse_family(bytes)?;
    // Patch here rather than in the backend: the imported paths normalize
    // inside `CanvasKitBackend::register_imported_font`, and one owner per
    // registry keeps the bytes from being rewritten twice.
    let patched = crate::vf_normalize::with_default_wght_400(bytes);
    let bytes = patched.as_deref().unwrap_or(bytes);
    let mut host = inner.try_borrow_mut().ok()?;
    host.register_bundled_font(&family, bytes).then_some(family)
}

fn apply_pending<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        // Retry from the next frame; ask for one so a quiet editor still gets it.
        crate::repaint_coalescer::request();
        return;
    };
    let Some(families) = PENDING_APPLY.with(|slot| slot.borrow_mut().take()) else {
        return;
    };
    b.host_mut().apply_bundled_font_families(families);
    // Newly registered faces change text metrics without touching the document,
    // and the web scene cache has no font-generation signal — force the rebuild
    // or the layout scene stays measured against the fallback glyphs.
    b.host_mut().invalidate_layout_scene();
    let _ = b.repaint();
}

#[cfg(test)]
mod tests {
    use super::{BUNDLED_FONT_FILES, FONT_ROUTE_PREFIX};

    /// Where each manifest entry lives in the source tree. Roboto sits with the
    /// native host (it is also that backend's last-resort face); everything else
    /// is in the desktop font directory. `tools/stage-web-assets.sh` copies from
    /// exactly these two places, so this pins the manifest ↔ staging coupling:
    /// a file added to one and not the other fails here.
    fn source_path(file: &str) -> String {
        let root = env!("CARGO_MANIFEST_DIR");
        if file == "Roboto-Regular.ttf" {
            format!("{root}/../op-host-native/assets/{file}")
        } else {
            format!("{root}/../op-host-desktop/assets/fonts/{file}")
        }
    }

    #[test]
    fn every_manifest_entry_exists_where_the_staging_script_copies_from() {
        for file in BUNDLED_FONT_FILES {
            let path = source_path(file);
            assert!(
                std::path::Path::new(&path).is_file(),
                "manifest names {file}, but {path} does not exist"
            );
        }
        assert_eq!(BUNDLED_FONT_FILES.len(), 11);
        assert!(FONT_ROUTE_PREFIX.ends_with('/'));
    }

    #[test]
    fn no_two_manifest_entries_share_a_family() {
        // The bundled registry keys one face per family; a duplicate would make
        // the surviving face depend on fetch-settle order (why DMMono-Medium is
        // excluded — see the manifest comment).
        let mut families: Vec<String> = BUNDLED_FONT_FILES
            .iter()
            .map(|file| {
                let bytes = std::fs::read(source_path(file)).expect("bundled font is readable");
                crate::font_meta::parse_family(&bytes).expect("bundled font has a family")
            })
            .collect();
        families.sort();
        let before = families.len();
        families.dedup();
        assert_eq!(families.len(), before, "duplicate family in {families:?}");
    }

    #[test]
    fn every_manifest_entry_carries_the_family_the_picker_expects() {
        // The family is what the document's `fontFamily` is matched against, so
        // a file swapped for one with a different name would silently stop
        // resolving.
        let expected = [
            ("Roboto-Regular.ttf", "Roboto"),
            ("Inter-VF.ttf", "Inter"),
            ("SpaceGrotesk-VF.ttf", "Space Grotesk"),
            ("Manrope-VF.ttf", "Manrope"),
            ("Outfit-VF.ttf", "Outfit"),
            ("DMSans-VF.ttf", "DM Sans"),
            ("DMSerifDisplay-Regular.ttf", "DM Serif Display"),
            ("DMMono-Regular.ttf", "DM Mono"),
            ("InstrumentSerif-Regular.ttf", "Instrument Serif"),
            ("JetBrainsMono-VF.ttf", "JetBrains Mono"),
            ("CormorantGaramond-VF.ttf", "Cormorant Garamond"),
        ];
        assert_eq!(expected.len(), BUNDLED_FONT_FILES.len());
        for (file, family) in expected {
            assert!(
                BUNDLED_FONT_FILES.contains(&file),
                "{file} is no longer in the manifest"
            );
            let bytes = std::fs::read(source_path(file)).expect("bundled font is readable");
            assert_eq!(
                crate::font_meta::parse_family(&bytes).as_deref(),
                Some(family),
                "{file} must register as {family}"
            );
        }
    }
}
