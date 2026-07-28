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

mod de;
mod de_collab;
mod de_git;
mod de_panel;
mod en;
mod en_collab;
mod en_git;
mod en_panel;
mod es;
mod es_collab;
mod es_git;
mod es_panel;
mod fr;
mod fr_collab;
mod fr_git;
mod fr_panel;
mod hi;
mod hi_collab;
mod hi_git;
mod hi_panel;
mod id;
mod id_collab;
mod id_git;
mod id_panel;
mod ja;
mod ja_collab;
mod ja_git;
mod ja_panel;
mod ko;
mod ko_collab;
mod ko_git;
mod ko_panel;
mod pt;
mod pt_collab;
mod pt_git;
mod pt_panel;
mod ru;
mod ru_collab;
mod ru_git;
mod ru_panel;
mod th;
mod th_collab;
mod th_git;
mod th_panel;
mod tr;
mod tr_collab;
mod tr_git;
mod tr_panel;
mod vi;
mod vi_collab;
mod vi_git;
mod vi_panel;
mod zh_cn;
mod zh_cn_collab;
mod zh_cn_git;
mod zh_cn_panel;
mod zh_tw;
mod zh_tw_collab;
mod zh_tw_git;
mod zh_tw_panel;

/// Translate `key` for `locale`.
///
/// A missing locale entry falls back to English; a key absent from English
/// too is returned unchanged. `'static` keeps widget builders from cloning a
/// `String` per frame.
pub fn translate(locale: Locale, key: &'static str) -> &'static str {
    let lookup = match locale {
        Locale::EnUs => en::lookup(key),
        Locale::ZhCn => zh_cn::lookup(key),
        Locale::ZhTw => zh_tw::lookup(key),
        Locale::Ja => ja::lookup(key),
        Locale::Ko => ko::lookup(key),
        Locale::Fr => fr::lookup(key),
        Locale::Es => es::lookup(key),
        Locale::De => de::lookup(key),
        Locale::Pt => pt::lookup(key),
        Locale::Ru => ru::lookup(key),
        Locale::Hi => hi::lookup(key),
        Locale::Tr => tr::lookup(key),
        Locale::Th => th::lookup(key),
        Locale::Vi => vi::lookup(key),
        Locale::Id => id::lookup(key),
    };
    lookup.or_else(|| en::lookup(key)).unwrap_or(key)
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
mod missing_fonts_key_tests;
#[cfg(test)]
mod preview_device_key_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vector_fidelity_property_keys;
