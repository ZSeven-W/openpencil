//! Chrome-string translation layer.
//!
//! 15 canonical, hand-maintained locale tables. Each per-locale module
//! exposes a single `lookup(key) -> Option<&'static str>`; this module
//! dispatches to the right one given a [`Locale`] variant. Integrity tests
//! keep every direct locale key set and placeholder set aligned with English.
//! Unknown keys fall through to the key itself for debug visibility.
//!
//! Key naming follows the TS app's dot.case convention
//! (`common.untitled`, `rightPanel.design`, `layout.flexLayout`,
//! …) so cross-walking strings between TS and Rust is mechanical.

use crate::Locale;

#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod de;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod de_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod de_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod de_panel;
mod en;
mod en_collab;
mod en_git;
mod en_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod es;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod es_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod es_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod es_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod fr;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod fr_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod fr_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod fr_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod hi;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod hi_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod hi_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod hi_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod id;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod id_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod id_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod id_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ja;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ja_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ja_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ja_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ko;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ko_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ko_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ko_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod pt;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod pt_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod pt_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod pt_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ru;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ru_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ru_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod ru_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod th;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod th_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod th_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod th_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod tr;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod tr_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod tr_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod tr_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod vi;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod vi_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod vi_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod vi_panel;
mod zh_cn;
mod zh_cn_collab;
mod zh_cn_git;
mod zh_cn_panel;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod zh_tw;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod zh_tw_collab;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod zh_tw_git;
#[cfg(any(
    not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
    test
))]
mod zh_tw_panel;

mod runtime;
pub use runtime::{catalog_ready, catalog_route, install_catalog};

/// Translate `key` for `locale`.
///
/// A missing locale entry falls back to English; a key absent from English
/// too is returned unchanged. `'static` keeps widget builders from cloning a
/// `String` per frame.
pub fn translate(locale: Locale, key: &'static str) -> &'static str {
    translate_dynamic(locale, key).unwrap_or(key)
}

/// Translate a key that is only known at run time.
///
/// `translate` cannot serve keys built at run time because its "unknown key
/// falls back to itself" contract needs a `'static` input. Callers that carry
/// a `String` key — the HTML-import diagnostics panel, whose keys come from
/// the importer's warning codes — use this instead and supply their own
/// fallback text when it returns `None`.
pub fn translate_dynamic(locale: Locale, key: &str) -> Option<&'static str> {
    let lookup = match locale {
        Locale::EnUs => en::lookup(key),
        Locale::ZhCn => zh_cn::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::ZhTw => zh_tw::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Ja => ja::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Ko => ko::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Fr => fr::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Es => es::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::De => de::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Pt => pt::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Ru => ru::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Hi => hi::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Tr => tr::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Th => th::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Vi => vi::lookup(key),
        #[cfg(any(
            not(all(target_arch = "wasm32", feature = "runtime-locale-catalog")),
            test
        ))]
        Locale::Id => id::lookup(key),
        #[cfg(all(target_arch = "wasm32", feature = "runtime-locale-catalog", not(test)))]
        Locale::ZhTw
        | Locale::Ja
        | Locale::Ko
        | Locale::Fr
        | Locale::Es
        | Locale::De
        | Locale::Pt
        | Locale::Ru
        | Locale::Hi
        | Locale::Tr
        | Locale::Th
        | Locale::Vi
        | Locale::Id => runtime::lookup(locale, key),
    };
    lookup.or_else(|| en::lookup(key))
}

/// CLDR-style cardinal plural category.
///
/// The complete category set keeps the API stable as locales are added even
/// though the current 15 locales do not exercise `Zero` or `Two`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

/// Return the locale-aware cardinal plural category for an integer.
pub fn plural_category(locale: Locale, count: i64) -> PluralCategory {
    let n = count.unsigned_abs();
    match locale {
        Locale::Ru => {
            let mod_10 = n % 10;
            let mod_100 = n % 100;
            if mod_10 == 1 && mod_100 != 11 {
                PluralCategory::One
            } else if (2..=4).contains(&mod_10) && !(12..=14).contains(&mod_100) {
                PluralCategory::Few
            } else if mod_10 == 0 || (5..=9).contains(&mod_10) || (11..=14).contains(&mod_100) {
                PluralCategory::Many
            } else {
                PluralCategory::Other
            }
        }
        // French and Portuguese use the singular category for the integer
        // values zero and one, and use `many` for non-zero exact millions.
        Locale::Fr | Locale::Pt if n != 0 && n.is_multiple_of(1_000_000) => PluralCategory::Many,
        Locale::Fr | Locale::Pt if n <= 1 => PluralCategory::One,
        // Spanish also has the exact-million `many` category.
        Locale::Es if n != 0 && n.is_multiple_of(1_000_000) => PluralCategory::Many,
        // CLDR cardinal rules for integer Hindi values treat both zero and
        // one as the singular category.
        Locale::Hi if n <= 1 => PluralCategory::One,
        // Chinese, Japanese, Korean, Thai, Vietnamese and Indonesian do not
        // vary cardinal nouns by integer count. Turkish likewise has only
        // the `other` cardinal category.
        Locale::ZhCn
        | Locale::ZhTw
        | Locale::Ja
        | Locale::Ko
        | Locale::Tr
        | Locale::Th
        | Locale::Vi
        | Locale::Id => PluralCategory::Other,
        _ if n == 1 => PluralCategory::One,
        _ => PluralCategory::Other,
    }
}

/// Substitute named placeholders in a translated template.
///
/// Both the canonical `{{name}}` form and the legacy `{name}` form used by
/// missing-font strings are supported. Unknown placeholders remain visible.
pub fn interpolate(template: &str, variables: &[(&str, &str)]) -> String {
    let mut output = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(open) = remaining.find('{') {
        output.push_str(&remaining[..open]);
        let token = &remaining[open..];
        let (name_start, closing) = if token.starts_with("{{") {
            (2, "}}")
        } else {
            (1, "}")
        };
        let Some(relative_end) = token[name_start..].find(closing) else {
            // Do not reinterpret the second `{` of an unterminated
            // canonical token as the start of a legacy token.
            output.push_str(token);
            return output;
        };
        let name_end = name_start + relative_end;
        let name = &token[name_start..name_end];
        let token_end = name_end + closing.len();
        let replacement = variables
            .iter()
            .find_map(|(candidate, value)| (*candidate == name).then_some(*value));
        if let Some(value) = replacement {
            output.push_str(value);
            remaining = &token[token_end..];
        } else {
            output.push_str(&token[..token_end]);
            remaining = &token[token_end..];
        }
    }
    output.push_str(remaining);
    output
}

/// Translate a key and substitute named placeholders in one call.
pub fn translate_with(locale: Locale, key: &'static str, variables: &[(&str, &str)]) -> String {
    interpolate(translate(locale, key), variables)
}

#[cfg(test)]
mod catalog_integrity_tests;
#[cfg(test)]
mod figma_property_panel_key_tests;
#[cfg(test)]
mod html_import_key_tests;

#[cfg(test)]
mod html_import_warning_key_tests;
#[cfg(test)]
mod missing_fonts_key_tests;
#[cfg(test)]
mod preview_device_key_tests;
#[cfg(test)]
mod prompt_center_key_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vector_fidelity_property_keys;
