//! Every HTML-import diagnostic code must have panel copy in all 15 locales.
//!
//! The code list comes from `op-html` itself (a dev-dependency), so adding an
//! `ImportWarning` variant without its translations fails here rather than
//! shipping a raw key into the diagnostics panel.

use op_html::{ALL_IMPORT_WARNING_CODES, IMPORT_WARNING_KEY_PREFIX};

type Lookup = fn(&str) -> Option<&'static str>;

fn tables() -> [(&'static str, Lookup); 15] {
    [
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
    ]
}

/// Panel chrome around the warning rows.
const PANEL_KEYS: [&str; 6] = [
    "htmlImport.diagnostics.title",
    "htmlImport.diagnostics.summary",
    "htmlImport.diagnostics.dismiss",
    "htmlImport.diagnostics.expand",
    "htmlImport.diagnostics.collapse",
    "htmlImport.diagnostics.more",
];

#[test]
fn every_import_warning_code_has_copy_in_every_locale() {
    assert!(
        !ALL_IMPORT_WARNING_CODES.is_empty(),
        "the importer reported no warning codes"
    );
    for code in ALL_IMPORT_WARNING_CODES {
        let key = format!("{IMPORT_WARNING_KEY_PREFIX}{code}");
        for (name, lookup) in tables() {
            let value = lookup(&key).unwrap_or_else(|| {
                panic!("locale table `{name}` is missing `{key}` for warning code `{code}`")
            });
            assert!(
                !value.trim().is_empty(),
                "locale table `{name}` has empty copy for `{key}`"
            );
        }
    }
}

#[test]
fn the_diagnostics_panel_chrome_is_translated_everywhere() {
    for key in PANEL_KEYS {
        for (name, lookup) in tables() {
            assert!(
                lookup(key).is_some(),
                "locale table `{name}` is missing diagnostics panel key `{key}`"
            );
        }
    }
}

/// The count guard mirrors `catalog_integrity_tests`: it makes an accidental
/// key deletion loud instead of letting the fall-through mask it.
#[test]
fn the_warning_key_family_stays_the_size_the_importer_declares() {
    let expected = ALL_IMPORT_WARNING_CODES.len();
    assert_eq!(
        expected, 142,
        "update the intentional HTML-import warning catalog size"
    );
    let localized = ALL_IMPORT_WARNING_CODES
        .iter()
        .filter(|code| super::en::lookup(&format!("{IMPORT_WARNING_KEY_PREFIX}{code}")).is_some())
        .count();
    assert_eq!(localized, expected);
}
