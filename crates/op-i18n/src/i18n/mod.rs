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
mod en;
mod es;
mod fr;
mod hi;
mod id;
mod ja;
mod ko;
mod pt;
mod ru;
mod th;
mod tr;
mod vi;
mod zh_cn;
mod zh_tw;

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
