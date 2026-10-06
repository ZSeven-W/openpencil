//! Runtime locale catalogs used by the slim browser bundle.
//!
//! The browser host parses JSON before calling this module, keeping
//! `op-i18n` dependency-free. Each accepted lazy catalog is installed once
//! and leaked for the process lifetime so translation keeps returning
//! `&'static str`. The leak is bounded to the 13 non-embedded locales.

use crate::Locale;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

type Catalog = HashMap<&'static str, &'static str>;

#[derive(Default)]
struct Registry {
    catalogs: HashMap<Locale, Catalog>,
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

fn lock() -> MutexGuard<'static, Registry> {
    registry()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

/// Install one parsed lazy-locale catalog.
///
/// Embedded catalogs are immutable, duplicate installs are ignored, and the
/// whole map is rejected before leaking anything when it contains an unknown
/// English key. Missing known keys are valid and fall back to English during
/// lookup.
pub fn install_catalog(locale: Locale, entries: HashMap<String, String>) -> bool {
    if matches!(locale, Locale::EnUs | Locale::ZhCn)
        || entries.is_empty()
        || entries
            .keys()
            .any(|key| super::en::lookup(key.as_str()).is_none())
    {
        return false;
    }

    let mut registry = lock();
    if registry.catalogs.contains_key(&locale) {
        return false;
    }

    let catalog = entries
        .into_iter()
        .map(|(key, value)| {
            let key: &'static str = Box::leak(key.into_boxed_str());
            let value: &'static str = Box::leak(value.into_boxed_str());
            (key, value)
        })
        .collect();
    registry.catalogs.insert(locale, catalog);
    true
}

/// Whether the active build can translate `locale` without fetching it.
pub fn catalog_ready(locale: Locale) -> bool {
    catalog_embedded(locale) || catalog_installed(locale)
}

/// Stable staged-asset route for a locale catalog.
pub fn catalog_route(locale: Locale) -> String {
    format!("/pkg/assets/i18n/{}.json", locale.code())
}

#[cfg(any(all(target_arch = "wasm32", feature = "runtime-locale-catalog"), test))]
pub(super) fn lookup(locale: Locale, key: &str) -> Option<&'static str> {
    lock()
        .catalogs
        .get(&locale)
        .and_then(|catalog| catalog.get(key))
        .copied()
}

pub(super) fn catalog_installed(locale: Locale) -> bool {
    lock().catalogs.contains_key(&locale)
}

#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
const fn catalog_embedded(_locale: Locale) -> bool {
    true
}

#[cfg(all(target_arch = "wasm32", feature = "runtime-locale-catalog", not(test)))]
const fn catalog_embedded(locale: Locale) -> bool {
    matches!(locale, Locale::EnUs | Locale::ZhCn)
}

#[cfg(test)]
pub(super) fn reset_for_test() {
    lock().catalogs.clear();
}

#[cfg(test)]
pub(super) fn test_lock() -> MutexGuard<'static, ()> {
    static TEST_LOCK: Mutex<()> = Mutex::new(());
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}
