//! Thin, status-aware localStorage adapter for browser-host preferences.

#[cfg(target_arch = "wasm32")]
pub(crate) fn storage_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn storage_get(_key: &str) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn storage_set_checked(key: &str, value: &str) -> bool {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        return storage.set_item(key, value).is_ok();
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn storage_set_checked(_key: &str, _value: &str) -> bool {
    false
}

pub(crate) fn report_storage_failure() {
    #[cfg(target_arch = "wasm32")]
    STORAGE_FAILURE_REPORTED.with(|reported| {
        if !reported.replace(true) {
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
                "OpenPencil could not persist browser settings to localStorage",
            ));
        }
    });
}

pub(crate) fn report_unsupported_credential_version() {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
        "OpenPencil found newer or incompatible browser settings; they were left unchanged",
    ));
}

pub(crate) fn clear_storage_failure() {
    #[cfg(target_arch = "wasm32")]
    STORAGE_FAILURE_REPORTED.with(|reported| reported.set(false));
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static STORAGE_FAILURE_REPORTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
