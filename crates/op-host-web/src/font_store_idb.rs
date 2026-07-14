// Browser boundary: IndexedDB persistence needs a real browser to exercise the
// open / put / delete / getAll round-trip; the pure key normalizer is unit-tested.
//! IndexedDB persistence for user-imported fonts (web host, Phase 4).
//!
//! The desktop host copies imported font files into a disk-backed
//! [`FontStore`](../../op-host-desktop/src/fonts.rs); the browser has no
//! filesystem, so we persist the raw font bytes in IndexedDB instead and
//! re-register them into the CanvasKit family registry on the next mount.
//!
//! DB `"openpencil"`, object store `"imported_fonts"` (out-of-line keys). Each
//! record is `{ family: string, bytes: Uint8Array }` stored under the family
//! key (see [`primary_key`]). Everything here is defensive: any IndexedDB error
//! logs to the console and degrades to session-only persistence — a failed
//! store never panics or blocks the editor.

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{IdbDatabase, IdbObjectStore, IdbOpenDbRequest, IdbTransactionMode};

const DB_NAME: &str = "openpencil";
const STORE: &str = "imported_fonts";
const DB_VERSION: u32 = 1;
type FontLoadCallback = Box<dyn FnOnce(Vec<(String, Vec<u8>)>)>;

/// CSS font stack → IndexedDB record key. Mirrors the JS `primaryFamilyKey`:
/// first family before a comma, quotes stripped, trimmed, lowercased; generic
/// keywords resolve to an empty key. The same value keys both `put_font` and
/// `delete_font`, so add / remove round-trip regardless of how the JS registry
/// keys its in-memory map.
pub(crate) fn primary_key(family: &str) -> String {
    let first = family
        .split(',')
        .next()
        .unwrap_or(family)
        .trim()
        .trim_matches(['"', '\''])
        .trim();
    let key = first.to_lowercase();
    const GENERIC: [&str; 5] = [
        "system-ui",
        "sans-serif",
        "serif",
        "monospace",
        "-apple-system",
    ];
    if GENERIC.contains(&key.as_str()) {
        String::new()
    } else {
        key
    }
}

fn console_warn(msg: &str) {
    web_sys::console::warn_1(&JsValue::from_str(msg));
}

/// Open (and, on first use, create) the imported-fonts DB, then invoke
/// `on_ready` with the live database. Any failure logs and drops the callback
/// (session-only degrade). One-shot closures are `forget()`-leaked — these fire
/// on rare user actions (import / remove) plus once at mount, so the leak is
/// negligible and keeps the event wiring simple.
fn open_db(on_ready: Box<dyn FnOnce(IdbDatabase)>) {
    let result = (|| -> Result<(), JsValue> {
        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("font-store: window unavailable"))?;
        let factory = window
            .indexed_db()?
            .ok_or_else(|| JsValue::from_str("font-store: IndexedDB unavailable"))?;
        let open_req: IdbOpenDbRequest = factory.open_with_u32(DB_NAME, DB_VERSION)?;

        // onupgradeneeded (first open / version bump) — create the store. The
        // request's `result()` is the upgrading database at this point.
        {
            let upgrade_req = open_req.clone();
            let upgrade = Closure::<dyn FnMut()>::once(move || {
                if let Ok(db) = upgrade_req.result().and_then(|v| {
                    v.dyn_into::<IdbDatabase>()
                        .map_err(|_| JsValue::from_str("upgrade: not a database"))
                }) {
                    // Ignore an "already exists" error — harmless on re-entry.
                    let _ = db.create_object_store(STORE);
                }
            });
            open_req.set_onupgradeneeded(Some(upgrade.as_ref().unchecked_ref()));
            upgrade.forget();
        }

        // onsuccess → hand the ready database to the caller.
        {
            let success_req = open_req.clone();
            let mut once = Some(on_ready);
            let success = Closure::<dyn FnMut()>::once(move || {
                let db = success_req
                    .result()
                    .ok()
                    .and_then(|v| v.dyn_into::<IdbDatabase>().ok());
                if let (Some(cb), Some(db)) = (once.take(), db) {
                    cb(db);
                }
            });
            open_req.set_onsuccess(Some(success.as_ref().unchecked_ref()));
            success.forget();
        }

        // onerror → log + drop the callback (session-only degrade).
        {
            let error = Closure::<dyn FnMut()>::once(move || {
                console_warn("[font-store] IndexedDB open failed; imported fonts are session-only");
            });
            open_req.set_onerror(Some(error.as_ref().unchecked_ref()));
            error.forget();
        }
        Ok(())
    })();
    if let Err(e) = result {
        web_sys::console::warn_1(&e);
    }
}

/// Open a read/write transaction on the store, returning `None` (after logging)
/// on any failure so callers can bail without panicking.
fn writable_store(db: &IdbDatabase) -> Option<IdbObjectStore> {
    match db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite) {
        Ok(tx) => tx.object_store(STORE).ok(),
        Err(e) => {
            web_sys::console::warn_1(&e);
            None
        }
    }
}

/// Attach a logging `onerror` to an IDB request so an ASYNC failure (quota
/// exceeded, transaction abort) surfaces in the console instead of being
/// silently dropped — the synchronous `Err` from `put`/`delete` only covers
/// request creation. `forget()`-leaked like the module's one-shot open
/// closures (fires only on rare import/remove writes).
fn log_request_errors(req: &web_sys::IdbRequest, what: &'static str) {
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| {
        console_warn(what);
    });
    req.set_onerror(Some(cb.as_ref().unchecked_ref()));
    cb.forget();
}

/// Persist a font under its family key. `{ family, bytes }`, keyed by
/// `family_key`; replaces any prior record for the same family. Fire-and-forget
/// (the transaction auto-commits) and non-blocking.
pub(crate) fn put_font(family_key: &str, family: &str, bytes: &[u8]) {
    if family_key.is_empty() {
        return;
    }
    let family_key = family_key.to_string();
    let family = family.to_string();
    // Copy into a fresh JS `Uint8Array` (own buffer) so the structured clone
    // that IndexedDB performs doesn't reference wasm linear memory.
    let arr = js_sys::Uint8Array::from(bytes);
    open_db(Box::new(move |db| {
        let Some(store) = writable_store(&db) else {
            return;
        };
        let record = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &record,
            &JsValue::from_str("family"),
            &JsValue::from_str(&family),
        );
        let _ = js_sys::Reflect::set(&record, &JsValue::from_str("bytes"), &arr);
        match store.put_with_key(&record, &JsValue::from_str(&family_key)) {
            Ok(req) => log_request_errors(&req, "font-store: IndexedDB put failed"),
            Err(e) => web_sys::console::warn_1(&e),
        }
    }));
}

/// Delete the persisted font for `family_key` (no-op if absent).
pub(crate) fn delete_font(family_key: &str) {
    if family_key.is_empty() {
        return;
    }
    let family_key = family_key.to_string();
    open_db(Box::new(move |db| {
        let Some(store) = writable_store(&db) else {
            return;
        };
        match store.delete(&JsValue::from_str(&family_key)) {
            Ok(req) => log_request_errors(&req, "font-store: IndexedDB delete failed"),
            Err(e) => web_sys::console::warn_1(&e),
        }
    }));
}

/// Load every persisted font, invoking `on_done` with `(family, bytes)` pairs.
/// `on_done` is only called on a successful read; any error path logs and drops
/// it (mount-time load simply skips — the editor still runs, imports just don't
/// survive the reload).
pub(crate) fn load_all(on_done: FontLoadCallback) {
    open_db(Box::new(move |db| {
        let store = match db.transaction_with_str(STORE) {
            Ok(tx) => match tx.object_store(STORE) {
                Ok(store) => store,
                Err(e) => {
                    web_sys::console::warn_1(&e);
                    return;
                }
            },
            Err(e) => {
                web_sys::console::warn_1(&e);
                return;
            }
        };
        let req = match store.get_all() {
            Ok(req) => req,
            Err(e) => {
                web_sys::console::warn_1(&e);
                return;
            }
        };
        let result_req = req.clone();
        let mut once = Some(on_done);
        let success = Closure::<dyn FnMut()>::once(move || {
            let value = result_req.result().unwrap_or(JsValue::NULL);
            let fonts = parse_records(&value);
            if let Some(cb) = once.take() {
                cb(fonts);
            }
        });
        req.set_onsuccess(Some(success.as_ref().unchecked_ref()));
        success.forget();
        let error = Closure::<dyn FnMut()>::once(move || {
            console_warn("[font-store] IndexedDB read failed; no persisted fonts loaded");
        });
        req.set_onerror(Some(error.as_ref().unchecked_ref()));
        error.forget();
    }));
}

/// Decode a `getAll()` result array into `(family, bytes)` pairs, skipping any
/// malformed record.
fn parse_records(value: &JsValue) -> Vec<(String, Vec<u8>)> {
    let array = js_sys::Array::from(value);
    let mut out = Vec::new();
    for entry in array.iter() {
        let family = js_sys::Reflect::get(&entry, &JsValue::from_str("family"))
            .ok()
            .and_then(|v| v.as_string())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let bytes = js_sys::Reflect::get(&entry, &JsValue::from_str("bytes"))
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Uint8Array>().ok())
            .map(|arr| arr.to_vec());
        if let (Some(family), Some(bytes)) = (family, bytes) {
            if !bytes.is_empty() {
                out.push((family, bytes));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_key_matches_js_normalization() {
        assert_eq!(primary_key("Inter"), "inter");
        assert_eq!(primary_key("\"My Font\", sans-serif"), "my font");
        assert_eq!(primary_key("  'Roboto Mono' "), "roboto mono");
    }

    #[test]
    fn primary_key_rejects_generic_families() {
        assert_eq!(primary_key("sans-serif"), "");
        assert_eq!(primary_key("system-ui"), "");
        assert_eq!(primary_key("-apple-system"), "");
        assert_eq!(primary_key(""), "");
    }
}
