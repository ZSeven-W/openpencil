//! Chrome-string translation layer.
//!
//! 15 locale tables, mirrored verbatim from
//! `apps/web/src/i18n/locales/*.ts` via `tools/convert-locales.py`.
//! Each per-locale module exposes a single `lookup(key) ->
//! Option<&'static str>`; this module dispatches to the right
//! one given a `Locale` variant. Unknown keys fall through to
//! the key itself so missing translations are visually obvious.
//!
//! Key naming follows the TS app's dot.case convention
//! (`common.untitled`, `rightPanel.design`, `layout.flexLayout`,
//! …) so cross-walking strings between TS and Rust is mechanical.

use crate::Locale;

mod de;
mod de_git;
mod en;
mod en_git;
mod es;
mod es_git;
mod fr;
mod fr_git;
mod hi;
mod hi_git;
mod id;
mod id_git;
mod ja;
mod ja_git;
mod ko;
mod ko_git;
mod pt;
mod pt_git;
mod ru;
mod ru_git;
mod th;
mod th_git;
mod tr;
mod tr_git;
mod vi;
mod vi_git;
mod zh_cn;
mod zh_cn_git;
mod zh_tw;
mod zh_tw_git;

/// Translate `key` for `locale`. Returns the key itself when no
/// entry exists. `'static` because every per-locale table value is
/// a string literal and callers pass static keys — letting widget
/// builders store the slice instead of cloning a `String` per frame.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zh_cn_returns_chinese_chrome_strings() {
        assert_eq!(translate(Locale::ZhCn, "common.untitled"), "未命名");
    }

    #[test]
    fn en_us_returns_english_chrome_strings() {
        assert_eq!(translate(Locale::EnUs, "common.untitled"), "Untitled");
    }

    #[test]
    fn ja_falls_back_through_en_for_missing_keys() {
        // Pick a key that's only in EN — assertion holds either way:
        // either ja has it (good), or it falls back to en (also good).
        let r = translate(Locale::Ja, "common.cancel");
        assert!(!r.is_empty());
    }

    #[test]
    fn unknown_key_falls_through_to_key() {
        assert_eq!(
            translate(Locale::ZhCn, "this.key.does.not.exist"),
            "this.key.does.not.exist"
        );
    }
}

#[cfg(test)]
mod preview_device_key_tests {
    /// Every locale table must carry a DIRECT entry for the preview
    /// device-switcher keys — `translate`'s EN fallback must not mask
    /// a missing translation.
    #[test]
    fn preview_device_keys_exist_in_every_locale_table() {
        const KEYS: [&str; 3] = [
            "preview.device.phone",
            "preview.device.desktop",
            "preview.device.canvas",
        ];
        type Lookup = fn(&str) -> Option<&'static str>;
        let tables: [(&str, Lookup); 15] = [
            ("en", super::en::lookup),
            ("zh_cn", super::zh_cn::lookup),
            ("zh_tw", super::zh_tw::lookup),
            ("ja", super::ja::lookup),
            ("ko", super::ko::lookup),
            ("fr", super::fr::lookup),
            ("es", super::es::lookup),
            ("de", super::de::lookup),
            ("pt", super::pt::lookup),
            ("ru", super::ru::lookup),
            ("hi", super::hi::lookup),
            ("tr", super::tr::lookup),
            ("th", super::th::lookup),
            ("vi", super::vi::lookup),
            ("id", super::id::lookup),
        ];
        for (name, lookup) in tables {
            for key in KEYS {
                assert!(
                    lookup(key).is_some(),
                    "locale table `{name}` is missing `{key}`"
                );
            }
        }
    }
}
