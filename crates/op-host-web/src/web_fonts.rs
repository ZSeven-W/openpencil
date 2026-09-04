// Local Font Access permission and CanvasKit font-byte loading need browser smoke;
// pure classifiers and fallback routing are unit-tested.
//! Browser Local Font Access bridge for the web host.
//!
//! The wasm Skia backend cannot see operating-system fonts. After permission,
//! `window.queryLocalFonts()` supplies the active document and shell fallbacks.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use crate::repaint_ctx::RepaintContext;
use jian_ops_schema::node::{PenNode, TextContent};
use js_sys::{Array, Function, Promise, Reflect, Uint8Array};
use op_editor_core::pen_node_ext::PenNodeExt;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

type InnerRc<C> = Rc<RefCell<C>>;

/// Pure classifier (relocated from the skia-only `backend::emoji`) so the
/// CanvasKit build can use it without pulling the skia backend module.
fn is_emoji_font_family(family: &str) -> bool {
    let normalized = family.to_lowercase();
    normalized.contains("emoji") || normalized.contains("color symbol")
}

fn is_cjk_fallback_font_family(family: &str) -> bool {
    let normalized = family.to_lowercase();
    [
        "pingfang",
        "hiragino sans",
        "hiragino kaku gothic",
        "heiti",
        "stheiti",
        "songti",
        "noto sans cjk",
        "noto sans sc",
        "noto sans tc",
        "noto sans jp",
        "noto sans kr",
        "source han sans",
        "microsoft yahei",
        "microsoft jhenghei",
        "simhei",
        "simsun",
        "yu gothic",
        "yu mincho",
        "meiryo",
        "malgun gothic",
        "applegothic",
        "nanum gothic",
        "apple sd gothic neo",
        "arial unicode ms",
    ]
    .into_iter()
    .any(|needle| normalized.contains(needle))
}

fn is_text_fallback_font_family(family: &str) -> bool {
    if is_emoji_font_family(family) || is_cjk_fallback_font_family(family) {
        return true;
    }
    let normalized = family.to_lowercase();
    [
        "arial",
        "arial unicode ms",
        "helvetica neue",
        "sf pro",
        ".sf ns",
        "segoe ui",
        "segoe ui historic",
        "segoe ui symbol",
        "apple symbols",
        "noto sans",
        "kohinoor devanagari",
        "devanagari sangam mn",
        "itfdevanagari",
        "muktamahee",
        "mukta mahee",
        "noto sans devanagari",
        "nirmala ui",
        "mangal",
        "sfgeorgian",
        "sfhebrew",
        "sf hebrew",
        "arial hebrew",
        "geeza pro",
        "al nile",
        "thonburi",
        "sukhumvit",
        "noto sans thai",
        "noto sans thai ui",
        "leelawadee ui",
    ]
    .into_iter()
    .any(|needle| normalized.contains(needle))
}

thread_local! {
    static QUERY_IN_FLIGHT: Cell<bool> = const { Cell::new(false) };
    static FONT_DATA_BY_FAMILY: RefCell<HashMap<String, JsValue>> = RefCell::new(HashMap::new());
    static LOADING_FAMILIES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static LOADED_FAMILIES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

pub(crate) fn drain_font_requests<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    if should_query_system_fonts(inner) {
        start_system_font_query(inner);
    }
    load_used_system_fonts(inner);
    drain_missing_font_import_request(inner);
    drain_font_import_request(inner);
    drain_font_remove_request(inner);
}

/// Re-check deferred detection after the CanvasKit font drain. The async query
/// completion also calls this through `apply_browser_system_font_families`;
/// this synchronous re-check is the fallback if the query path changes later.
pub(crate) fn drain_missing_fonts_detection<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    b.host_mut().complete_pending_missing_fonts_detection();
}

fn should_query_system_fonts<C: RepaintContext + 'static>(inner: &InnerRc<C>) -> bool {
    let Ok(b) = inner.try_borrow() else {
        return false;
    };
    let ui = &b.host().editor_state().editor_ui;
    should_query_system_fonts_state(ui.font_picker.open, ui.system_fonts_loaded)
}

fn should_query_system_fonts_state(_font_picker_open: bool, system_fonts_loaded: bool) -> bool {
    !system_fonts_loaded
}

fn start_system_font_query<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    if QUERY_IN_FLIGHT.with(|flag| flag.replace(true)) {
        return;
    }
    let Some((window, query)) = query_local_fonts_function() else {
        QUERY_IN_FLIGHT.with(|flag| flag.set(false));
        apply_system_font_families(inner, Vec::new());
        return;
    };
    let promise = query
        .call0(window.as_ref())
        .ok()
        .and_then(|v| v.dyn_into::<Promise>().ok());
    let Some(promise) = promise else {
        QUERY_IN_FLIGHT.with(|flag| flag.set(false));
        apply_system_font_families(inner, Vec::new());
        return;
    };
    let inner_ok = inner.clone();
    let on_ok = Closure::<dyn FnMut(JsValue)>::once(move |value: JsValue| {
        QUERY_IN_FLIGHT.with(|flag| flag.set(false));
        let (families, handles) = parse_font_data_list(&value);
        FONT_DATA_BY_FAMILY.with(|slot| {
            *slot.borrow_mut() = handles;
        });
        apply_system_font_families(&inner_ok, families);
        load_used_system_fonts(&inner_ok);
    });
    let inner_err = inner.clone();
    let on_err = Closure::<dyn FnMut(JsValue)>::once(move |_err: JsValue| {
        QUERY_IN_FLIGHT.with(|flag| flag.set(false));
        // A present-but-rejecting queryLocalFonts usually means the
        // permission prompt needs a user gesture (or an embedding frame just
        // gained the delegation). Permanently recording an empty set here
        // left the editor without system fonts for the whole session; keep
        // the query retryable for a few later repaints (each retry is a
        // cheap immediate rejection until a gesture-carrying frame lands),
        // then settle on empty so an actually-denied permission stops asking.
        let exhausted = QUERY_REJECTIONS.with(|count| {
            let next = count.get() + 1;
            count.set(next);
            next >= MAX_QUERY_REJECTIONS
        });
        if exhausted && should_mark_system_fonts_loaded_after_query_rejection() {
            apply_system_font_families(&inner_err, Vec::new());
        }
    });
    let _ = promise.then2(&on_ok, &on_err);
    on_ok.forget();
    on_err.forget();
}

fn should_mark_system_fonts_loaded_after_query_rejection() -> bool {
    true
}

/// Rejections tolerated before the query settles on an empty system set.
const MAX_QUERY_REJECTIONS: u32 = 5;
thread_local! {
    static QUERY_REJECTIONS: Cell<u32> = const { Cell::new(0) };
}

fn query_local_fonts_function() -> Option<(web_sys::Window, Function)> {
    let window = web_sys::window()?;
    let query = Reflect::get(window.as_ref(), &JsValue::from_str("queryLocalFonts"))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    Some((window, query))
}

fn apply_system_font_families<C: RepaintContext + 'static>(
    inner: &InnerRc<C>,
    families: Vec<String>,
) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    b.host_mut().apply_browser_system_font_families(families);
    let _ = b.repaint();
}

fn load_used_system_fonts<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let mut families = used_font_families(inner);
    families.extend(available_text_fallback_font_families());
    for family in families {
        request_font_bytes(inner, family);
    }
}

fn available_text_fallback_font_families() -> Vec<String> {
    FONT_DATA_BY_FAMILY.with(|slot| {
        slot.borrow()
            .keys()
            .filter(|family| is_text_fallback_font_family(family))
            .cloned()
            .collect()
    })
}

fn used_font_families<C: RepaintContext + 'static>(inner: &InnerRc<C>) -> Vec<String> {
    let Ok(b) = inner.try_borrow() else {
        return Vec::new();
    };
    used_font_families_from_state(b.host().editor_state())
}

fn used_font_families_from_state(state: &op_editor_core::EditorState) -> Vec<String> {
    let mut families = Vec::new();
    let authored = state.active_children();
    let resolved;
    let active_canvas = if nodes_have_refs(authored) {
        resolved = op_editor_core::ref_resolve::resolve_refs_for_canvas_roots(authored, &state.doc);
        resolved.as_slice()
    } else {
        authored
    };
    collect_text_font_families(active_canvas, &mut families);
    if let Some(PenNode::Text(text)) = state.selected_node() {
        if let Some(family) = text.font_family.as_ref() {
            families.push(family.clone());
        }
    }
    families
        .into_iter()
        .flat_map(|family| op_editor_core::font_catalog::split_font_family_stack(&family))
        .filter(|family| !op_editor_core::font_catalog::is_generic_or_system_font_alias(family))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn nodes_have_refs(nodes: &[PenNode]) -> bool {
    nodes.iter().any(|node| {
        matches!(node, PenNode::Ref(_))
            || node
                .children()
                .is_some_and(|children| nodes_have_refs(children))
    })
}

fn collect_text_font_families(nodes: &[PenNode], out: &mut Vec<String>) {
    for node in nodes {
        if let PenNode::Text(text) = node {
            if let Some(family) = text.font_family.as_ref().filter(|s| !s.trim().is_empty()) {
                out.push(family.clone());
            }
            if let TextContent::Styled(segments) = &text.content {
                for segment in segments {
                    if let Some(family) = segment
                        .font_family
                        .as_ref()
                        .filter(|s| !s.trim().is_empty())
                    {
                        out.push(family.clone());
                    }
                }
            }
        }
        if let Some(children) = node.children() {
            collect_text_font_families(children, out);
        }
    }
}

fn request_font_bytes<C: RepaintContext + 'static>(inner: &InnerRc<C>, family: String) {
    let Some(key) = family_key(&family) else {
        return;
    };
    let should_start = LOADED_FAMILIES.with(|loaded| !loaded.borrow().contains(&key))
        && LOADING_FAMILIES.with(|loading| loading.borrow_mut().insert(key.clone()));
    if !should_start {
        return;
    }
    let Some(font_data) = FONT_DATA_BY_FAMILY.with(|slot| slot.borrow().get(&key).cloned()) else {
        LOADING_FAMILIES.with(|loading| {
            loading.borrow_mut().remove(&key);
        });
        return;
    };
    let Some(blob_promise) = call_promise_method(&font_data, "blob") else {
        mark_font_load_finished(&key, false);
        return;
    };
    let inner_blob = inner.clone();
    let key_blob = key.clone();
    let family_blob = family.clone();
    let on_blob = Closure::<dyn FnMut(JsValue)>::once(move |blob: JsValue| {
        let Some(bytes_promise) = call_promise_method(&blob, "arrayBuffer") else {
            mark_font_load_finished(&key_blob, false);
            return;
        };
        let inner_bytes = inner_blob.clone();
        let key_bytes = key_blob.clone();
        let family_bytes = family_blob.clone();
        let on_bytes = Closure::<dyn FnMut(JsValue)>::once(move |buffer: JsValue| {
            let bytes = Uint8Array::new(&buffer).to_vec();
            let Ok(mut b) = inner_bytes.try_borrow_mut() else {
                mark_font_load_finished(&key_bytes, false);
                return;
            };
            let ok = b.register_system_font(&family_bytes, &bytes);
            mark_font_load_finished(&key_bytes, ok);
            if ok {
                b.host_mut().mark_editor_state_dirty();
                // A newly-registered system-font face changes text metrics
                // without touching the doc; the web scene cache has no
                // font-generation signal, so force the rebuild or the layout
                // scene stays shaped/measured against the fallback glyphs.
                b.host_mut().invalidate_layout_scene();
                let _ = b.repaint();
            }
        });
        let key_err = key_blob.clone();
        let on_err = Closure::<dyn FnMut(JsValue)>::once(move |_err: JsValue| {
            mark_font_load_finished(&key_err, false);
        });
        let _ = bytes_promise.then2(&on_bytes, &on_err);
        on_bytes.forget();
        on_err.forget();
    });
    let key_err = key;
    let on_err = Closure::<dyn FnMut(JsValue)>::once(move |_err: JsValue| {
        mark_font_load_finished(&key_err, false);
    });
    let _ = blob_promise.then2(&on_blob, &on_err);
    on_blob.forget();
    on_err.forget();
}

fn call_promise_method(value: &JsValue, method: &str) -> Option<Promise> {
    let method = Reflect::get(value, &JsValue::from_str(method))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    method.call0(value).ok()?.dyn_into::<Promise>().ok()
}

fn mark_font_load_finished(key: &str, loaded: bool) {
    LOADING_FAMILIES.with(|loading| {
        loading.borrow_mut().remove(key);
    });
    if loaded {
        LOADED_FAMILIES.with(|families| {
            families.borrow_mut().insert(key.to_string());
        });
    }
}

fn parse_font_data_list(value: &JsValue) -> (Vec<String>, HashMap<String, JsValue>) {
    let array = Array::from(value);
    let mut families = Vec::new();
    let mut handles = HashMap::<String, JsValue>::new();
    for entry in array.iter() {
        let Some(family) = string_prop(&entry, "family").and_then(|s| normalized_family_name(&s))
        else {
            continue;
        };
        let Some(key) = family_key(&family) else {
            continue;
        };
        families.push(family.clone());
        let replace = !handles.contains_key(&key) || is_regular_face(&entry);
        if replace {
            handles.insert(key, entry);
        }
    }
    (families, handles)
}

fn string_prop(value: &JsValue, name: &str) -> Option<String> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()?
        .as_string()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn is_regular_face(value: &JsValue) -> bool {
    ["style", "fullName", "postscriptName"]
        .into_iter()
        .filter_map(|name| string_prop(value, name))
        .any(|s| {
            let s = s.to_lowercase();
            s == "regular" || s.ends_with("-regular") || s.ends_with(" regular")
        })
}

fn normalized_family_name(family: &str) -> Option<String> {
    // `queryLocalFonts().family` and the values supplied by
    // `used_font_families` are already one decoded family, not a CSS stack.
    // Splitting here would corrupt a legal raw name such as `ACME, Display`.
    let family = family.trim();
    let family = family
        .chars()
        .next()
        .filter(|quote| *quote == '"' || *quote == '\'')
        .and_then(|quote| {
            family
                .strip_prefix(quote)
                .and_then(|without_start| without_start.strip_suffix(quote))
        })
        .unwrap_or(family);
    (!family.is_empty()).then(|| family.to_string())
}

fn family_key(family: &str) -> Option<String> {
    normalized_family_name(family).map(|family| family.to_lowercase())
}

// ---------------------------------------------------------------------
// User font import / removal (Phase 4) — the web counterpart of the
// desktop `font_import_host`. `ImportFont` / `RemoveImportedFont` raise
// pending flags (mirrors native); this drain performs the browser IO:
// hidden file input → FileReader → CanvasKit family registry → IndexedDB.
// ---------------------------------------------------------------------

/// File-picker accept filter for imported fonts (matches the desktop's
/// `.ttf` / `.otf` rfd filter).
const FONT_ACCEPT: &str = ".ttf,.otf,font/ttf,font/otf";

/// Reject fonts larger than this before/after read — same 16 MiB ceiling
/// the desktop `FontStore` enforces (a CJK / variable font can reach a few
/// MiB; 16 MiB is a generous cap that still rejects a mis-picked huge file).
const MAX_FONT_BYTES: usize = 16 * 1024 * 1024;

/// Re-register every persisted imported font from IndexedDB at mount, then
/// refresh the picker snapshot + repaint. Per spec §C5: if the async read
/// resolves before the first paint completes the family is registered in time;
/// otherwise the repaint here re-paints any on-screen text with the newly
/// available typeface (a one-frame fallback flash is acceptable).
pub(crate) fn load_imported_fonts_at_mount<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let inner = inner.clone();
    crate::font_store_idb::load_all(Box::new(move |fonts| {
        if fonts.is_empty() {
            return;
        }
        {
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            // Don't clobber a font the user imported/changed this session before
            // the async IndexedDB read resolved: only register persisted
            // families not already present. (A family the user REMOVED in that
            // same pre-load window can still be re-added — a rare, self-
            // correcting race the user resolves by removing it again.)
            let present: std::collections::HashSet<String> = b
                .imported_family_list()
                .iter()
                .map(|f| crate::font_store_idb::primary_key(f))
                .collect();
            for (family, bytes) in &fonts {
                let key = crate::font_store_idb::primary_key(family);
                if key.is_empty() || present.contains(&key) {
                    continue;
                }
                b.register_imported_font(family, bytes);
            }
        }
        refresh_imported_font_snapshot(&inner);
    }));
}

/// Mirror the CanvasKit imported-font registry into the editor-state snapshot
/// the picker reads (`imported_font_families`), mark dirty, and repaint.
fn refresh_imported_font_snapshot<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    let families = b.imported_family_list();
    b.host_mut()
        .editor_state_mut()
        .editor_ui
        .imported_font_families = Arc::new(families);
    b.host_mut().mark_editor_state_dirty();
    b.host_mut().refresh_missing_fonts_prompt();
    // A font change alters text metrics/rendering without touching the doc, and
    // the web scene cache has no font-generation signal — force a rebuild so the
    // layout scene isn't left stale (measured/shaped against the old fonts).
    b.host_mut().invalidate_layout_scene();
    let _ = b.repaint();
}

fn drain_missing_font_import_request<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let row = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        b.host_mut().take_missing_fonts_import_row()
    };
    let Some(row) = row else {
        return;
    };
    open_font_import_picker(inner, Some(row));
}

/// Drain a pending `ImportFont`: open the hidden file input, read the chosen
/// font's bytes, register it (extracting its family), persist to IndexedDB, and
/// refresh the picker snapshot. All steps are non-fatal — a cancel / oversize /
/// parse failure logs and leaves the editor untouched.
fn drain_font_import_request<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let requested = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        std::mem::take(
            &mut b
                .host_mut()
                .editor_state_mut()
                .editor_ui
                .pending_font_import,
        )
    };
    if !requested {
        return;
    }
    open_font_import_picker(inner, None);
}

fn open_font_import_picker<C: RepaintContext + 'static>(
    inner: &InnerRc<C>,
    missing_row: Option<usize>,
) {
    let inner = inner.clone();
    crate::dom_io::open_file_picker(
        FONT_ACCEPT,
        Box::new(move |file| {
            // Reject an oversize file by `File.size` before reading it into
            // memory (the post-read cap is the backstop).
            if file.size() as usize > MAX_FONT_BYTES {
                console_warn_font(&format!(
                    "import rejected: font is too large ({:.1} MiB; max {} MiB)",
                    file.size() / (1024.0 * 1024.0),
                    MAX_FONT_BYTES / (1024 * 1024)
                ));
                return;
            }
            let inner = inner.clone();
            crate::dom_io::read_file(
                file,
                crate::dom_io::ReadMode::Bytes,
                Box::new(move |value| {
                    let Some(bytes) = crate::dom_io::js_bytes(&value) else {
                        console_warn_font("import failed: could not read the font file");
                        return;
                    };
                    if bytes.len() > MAX_FONT_BYTES {
                        console_warn_font("import rejected: font exceeds the 16 MiB cap");
                        return;
                    }
                    let actual_family =
                        missing_row.and_then(|_| crate::font_meta::parse_family(&bytes));
                    if import_font_bytes(&inner, bytes) {
                        if let Some(row) = missing_row {
                            let Ok(mut b) = inner.try_borrow_mut() else {
                                return;
                            };
                            b.host_mut()
                                .note_missing_font_supplied(row, actual_family.as_deref());
                            let _ = b.repaint();
                        }
                    }
                }),
            );
        }),
    );
}

/// Register imported font bytes: extract the family via CanvasKit, persist to
/// IndexedDB, and refresh the snapshot. A parse failure (no family name) logs
/// and does nothing else.
fn import_font_bytes<C: RepaintContext + 'static>(inner: &InnerRc<C>, bytes: Vec<u8>) -> bool {
    let family = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return false;
        };
        b.register_imported_font_from_bytes(&bytes)
    };
    let Some(family) = family else {
        console_warn_font("import rejected: could not parse the font (no family name)");
        return false;
    };
    // Persist (non-blocking) then reflect the new family in the picker.
    crate::font_store_idb::put_font(
        &crate::font_store_idb::primary_key(&family),
        &family,
        &bytes,
    );
    refresh_imported_font_snapshot(inner);
    true
}

/// Drain a pending `RemoveImportedFont`: drop the family from the CanvasKit
/// registry + IndexedDB, then refresh the snapshot.
fn drain_font_remove_request<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let family = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        b.host_mut()
            .editor_state_mut()
            .editor_ui
            .pending_font_remove
            .take()
    };
    let Some(family) = family else {
        return;
    };
    {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        b.remove_imported_font(&family);
    }
    crate::font_store_idb::delete_font(&crate::font_store_idb::primary_key(&family));
    refresh_imported_font_snapshot(inner);
}

fn console_warn_font(msg: &str) {
    web_sys::console::warn_1(&JsValue::from_str(&format!("[font-import] {msg}")));
}

#[cfg(test)]
#[path = "web_fonts_tests.rs"]
mod tests;
