//! Core translate / interpolate / plural behaviour tests.

use super::*;
use std::collections::HashMap;

#[test]
fn zh_cn_returns_chinese_chrome_strings() {
    assert_eq!(translate(Locale::ZhCn, "common.untitled"), "未命名");
}

#[test]
fn en_us_returns_english_chrome_strings() {
    assert_eq!(translate(Locale::EnUs, "common.untitled"), "Untitled");
}

#[test]
fn custom_agent_entry_is_presented_as_a_universal_integration() {
    assert_eq!(translate(Locale::ZhCn, "settings.agents.acp"), "通用接入");
    assert_eq!(
        translate(Locale::ZhCn, "settings.agents.addAcp"),
        "+ 添加自定义 Agent"
    );
    assert_eq!(
        translate(Locale::EnUs, "settings.agents.acp"),
        "Universal integration"
    );
    assert_eq!(
        translate(Locale::EnUs, "settings.agents.addAcp"),
        "+ Add custom Agent"
    );
}

#[test]
fn every_locale_has_a_direct_common_translation() {
    for locale in Locale::ALL {
        assert_ne!(translate(locale, "common.cancel"), "common.cancel");
    }
}

#[test]
fn native_thai_catalog_remains_embedded() {
    assert_eq!(translate(Locale::Th, "common.cancel"), "ยกเลิก");
}

#[test]
fn runtime_catalog_routes_use_stable_bcp47_codes() {
    assert_eq!(catalog_route(Locale::Ja), "/pkg/assets/i18n/ja.json");
    assert_eq!(catalog_route(Locale::ZhTw), "/pkg/assets/i18n/zh-TW.json");
    assert!(catalog_ready(Locale::EnUs));
    assert!(catalog_ready(Locale::ZhCn));
}

#[test]
fn runtime_catalog_install_is_bounded_to_known_keys_and_falls_back_per_key() {
    let _guard = runtime::test_lock();
    runtime::reset_for_test();

    assert!(!install_catalog(Locale::De, HashMap::new()));
    assert!(!runtime::catalog_installed(Locale::De));

    let invalid = HashMap::from([("not.an.i18n.key".to_string(), "Nein".to_string())]);
    assert!(!install_catalog(Locale::De, invalid));
    assert!(!runtime::catalog_installed(Locale::De));

    let partial = HashMap::from([("common.cancel".to_string(), "Abbrechen!".to_string())]);
    assert!(install_catalog(Locale::De, partial));
    assert!(runtime::catalog_installed(Locale::De));
    assert_eq!(
        runtime::lookup(Locale::De, "common.cancel"),
        Some("Abbrechen!")
    );
    assert_eq!(runtime::lookup(Locale::De, "common.ok"), None);
}

#[test]
fn unknown_key_falls_through_to_key() {
    for locale in Locale::ALL {
        assert_eq!(
            translate(locale, "this.key.does.not.exist"),
            "this.key.does.not.exist"
        );
    }
}

#[test]
fn interpolation_supports_canonical_and_legacy_placeholders() {
    assert_eq!(
        interpolate(
            "{{count}} items; {actual}, {missing}",
            &[("count", "3"), ("actual", "Inter")]
        ),
        "3 items; Inter, {missing}"
    );
    assert_eq!(
        translate_with(Locale::Fr, "common.selected", &[("count", "2")]),
        "2 sélectionné(s)"
    );
    assert_eq!(
        interpolate(
            "{{value}} and {other}",
            &[("value", "{other}"), ("other", "safe")]
        ),
        "{other} and safe",
        "replacement values must not be recursively interpolated"
    );
    assert_eq!(
        interpolate("before {{name} after", &[("name", "Alice")]),
        "before {{name} after",
        "unterminated canonical placeholders must remain verbatim"
    );
    assert_eq!(
        interpolate("before {name after", &[("name", "Alice")]),
        "before {name after",
        "unterminated legacy placeholders must remain verbatim"
    );
}

#[test]
fn plural_categories_cover_english_french_and_russian() {
    assert_eq!(plural_category(Locale::EnUs, 1), PluralCategory::One);
    assert_eq!(plural_category(Locale::EnUs, 0), PluralCategory::Other);
    assert_eq!(plural_category(Locale::EnUs, 2), PluralCategory::Other);

    assert_eq!(plural_category(Locale::Fr, 0), PluralCategory::One);
    assert_eq!(plural_category(Locale::Fr, 1), PluralCategory::One);
    assert_eq!(plural_category(Locale::Fr, 2), PluralCategory::Other);
    for locale in [Locale::Fr, Locale::Pt, Locale::Es] {
        assert_eq!(plural_category(locale, 1_000_000), PluralCategory::Many);
        assert_eq!(plural_category(locale, 2_000_000), PluralCategory::Many);
    }
    assert_eq!(plural_category(Locale::Tr, 1), PluralCategory::Other);

    for count in [1, 21, 101, -1] {
        assert_eq!(plural_category(Locale::Ru, count), PluralCategory::One);
    }
    for count in [2, 3, 4, 22, 23, 24] {
        assert_eq!(plural_category(Locale::Ru, count), PluralCategory::Few);
    }
    for count in [0, 5, 10, 11, 12, 14, 20, 25] {
        assert_eq!(plural_category(Locale::Ru, count), PluralCategory::Many);
    }
}

#[test]
fn russian_git_other_forms_are_safe_for_few_and_one_ending_counts() {
    for count in ["2", "21", "22"] {
        assert_eq!(
            translate_with(
                Locale::Ru,
                "git.history.diff.framesChanged_other",
                &[("count", count)]
            ),
            format!("Фреймов изменено: {count}")
        );
    }
}
