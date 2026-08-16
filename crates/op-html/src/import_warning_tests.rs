//! Guards for the typed import diagnostics.
//!
//! The `Display` assertions below are transcribed by hand from the strings
//! the importer emitted before `ImportWarning` existed. They are the contract
//! for CLI JSON and MCP output, so a failure here means user-visible wording
//! moved and the change must be deliberate.

use std::collections::BTreeSet;

use crate::import_warning::{ImportWarning, ALL_IMPORT_WARNING_CODES, IMPORT_WARNING_KEY_PREFIX};
use crate::{HtmlImportOptions, MediaQueryError};

#[test]
fn representative_variants_render_their_historical_text() {
    let cases: Vec<(ImportWarning, &str)> = vec![
        (
            ImportWarning::EmptyInput,
            "no importable content: input HTML is empty",
        ),
        (
            ImportWarning::NodeLimitTruncated,
            "node limit reached (40000), remaining content dropped",
        ),
        (
            ImportWarning::DomDepthTruncated { max_depth: 512 },
            "HTML nesting exceeded 512 levels and deeper content was dropped",
        ),
        (
            ImportWarning::FloatIgnored,
            "CSS float ignored during structured HTML import",
        ),
        (
            ImportWarning::FlexWrapIndefiniteWidth,
            "flex-wrap ignored: this container has no definite width, so rows cannot be \
             measured (per-child auto margins under this container stay unemulated)",
        ),
        (
            ImportWarning::GridColumnUnsupported {
                value: "span 2 / auto".to_string(),
            },
            "CSS grid-column `span 2 / auto` is not supported and was auto-placed",
        ),
        (
            ImportWarning::ConicGradientIgnored,
            "conic gradients are not representable and were ignored",
        ),
        (
            ImportWarning::PropertyNotRepresentable {
                property: "background-clip".to_string(),
            },
            "CSS background-clip is not representable and was ignored",
        ),
        (
            ImportWarning::ListStyleTypeUnsupported {
                value: "hebrew".to_string(),
            },
            "unsupported CSS list-style-type `hebrew` was approximated as a disc",
        ),
        (
            ImportWarning::WebFontNotDownloaded {
                family: "Inter".to_string(),
            },
            "web font 'Inter' declared via @font-face is not downloaded; text falls back to \
             available fonts",
        ),
        (
            ImportWarning::ElementPlaceholder {
                tag: "canvas".to_string(),
            },
            "<canvas> imported as a placeholder",
        ),
        (
            ImportWarning::CssImportDepthLimit {
                max_depth: 4,
                url: "https://example.test/a.css".to_string(),
            },
            "CSS @import depth limit (4) reached: https://example.test/a.css",
        ),
        (
            ImportWarning::MultipleHtmlEntries {
                count: 3,
                entry: "site/index.html".to_string(),
            },
            "multiple HTML entries found (3); selected site/index.html",
        ),
        (
            ImportWarning::SnapshotTaintedImages { count: 7 },
            "7 images kept as remote URLs (CORS-tainted)",
        ),
        (
            ImportWarning::MediaQuery(MediaQueryError::UnsupportedFeature("hover".to_string())),
            "unsupported @media feature 'hover' ignored",
        ),
    ];
    for (warning, expected) in cases {
        assert_eq!(warning.to_string(), expected, "code {}", warning.code());
    }
}

#[test]
fn every_code_is_unique_and_namespaced() {
    let unique: BTreeSet<&str> = ALL_IMPORT_WARNING_CODES.iter().copied().collect();
    assert_eq!(
        unique.len(),
        ALL_IMPORT_WARNING_CODES.len(),
        "duplicate warning code"
    );
    for code in ALL_IMPORT_WARNING_CODES {
        let (group, rest) = code.split_once('.').expect("code is `group.name`");
        assert!(
            !group.is_empty() && !rest.is_empty(),
            "malformed warning code `{code}`"
        );
        assert!(
            code.bytes()
                .all(|byte| byte.is_ascii_lowercase() || matches!(byte, b'_' | b'.')),
            "warning code `{code}` is not snake_case"
        );
    }
}

#[test]
fn codes_and_keys_agree_for_every_emitted_variant() {
    for warning in sample_of_every_variant() {
        let code = warning.code();
        assert!(
            ALL_IMPORT_WARNING_CODES.contains(&code),
            "`{code}` is missing from ALL_IMPORT_WARNING_CODES"
        );
        assert_eq!(
            warning.i18n_key(),
            format!("{IMPORT_WARNING_KEY_PREFIX}{code}"),
            "i18n key does not follow the code for `{code}`"
        );
        assert!(
            !warning.to_string().is_empty(),
            "`{code}` renders an empty message"
        );
    }
}

#[test]
fn the_sample_list_covers_every_declared_warning_code() {
    // `args()` and the other accessors end in wildcard arms, so a new variant
    // that nobody added to `import_warning_samples` would compile, ship an
    // empty argument list, and never reach the catalog tests. Comparing the
    // sampled codes against the declared catalog is what catches that.
    let sampled: BTreeSet<&str> = sample_of_every_variant()
        .iter()
        .map(ImportWarning::code)
        .collect();
    let declared: BTreeSet<&str> = ALL_IMPORT_WARNING_CODES.iter().copied().collect();
    let missing: Vec<_> = declared.difference(&sampled).collect();
    let extra: Vec<_> = sampled.difference(&declared).collect();
    assert!(
        missing.is_empty(),
        "add a sample to `import_warning_samples.rs` for {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "sampled codes not in the catalog: {extra:?}"
    );
}

#[test]
fn warn_once_deduplicates_on_the_rendered_message() {
    let options = HtmlImportOptions::default();
    let rules = [];
    let mut context = crate::mapper::MapCtx {
        opts: &options,
        rules: &rules,
        warnings: Vec::new(),
        warned: Default::default(),
        next_id: 0,
        node_count: 0,
        containing_width: 100.0,
        containing_height: 100.0,
        containing_width_is_definite: true,
        positioned_width: 100.0,
        positioned_height: 100.0,
        auto_margin_handled_by_parent: false,
        pending_base_outcome: Default::default(),
    };
    context.warn_once(ImportWarning::FloatIgnored);
    context.warn_once(ImportWarning::FloatIgnored);
    context.warn_once(ImportWarning::PositionStickyIgnored);
    assert_eq!(context.warnings.len(), 2);
    assert_eq!(context.warnings[0].code(), "layout.float_ignored");
    assert_eq!(context.warnings[1].code(), "layout.position_sticky_ignored");
}

#[test]
fn a_degrading_import_threads_typed_diagnostics_beside_the_strings() {
    let html = "<style>@container card (width>1px){p{color:red}}\
        .a{float:left;position:sticky;list-style-type:hebrew}</style>\
        <ul><li class=\"a\">one</li></ul><canvas></canvas>";
    let result = crate::import_html(html, &HtmlImportOptions::default());
    assert_eq!(
        result.warnings,
        crate::render_warnings(&result.diagnostics),
        "the string mirror drifted from the typed diagnostics"
    );
    let codes: BTreeSet<&str> = result.diagnostics.iter().map(ImportWarning::code).collect();
    for expected in [
        "css.unsupported_container_block",
        "layout.float_ignored",
        "layout.position_sticky_ignored",
        "list.style_type_unsupported",
        "media.element_placeholder",
    ] {
        assert!(
            codes.contains(expected),
            "missing `{expected}` in {codes:?}"
        );
    }
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("<canvas> imported as a placeholder")));
}

/// One value per variant, so `code` / `i18n_key` coverage cannot silently rot
/// when a variant is added — the compiler's exhaustiveness check on the
/// `match` inside `ident` plus this list keep the catalog honest.
fn sample_of_every_variant() -> Vec<ImportWarning> {
    let text = || "x".to_string();
    let mut all = vec![
        ImportWarning::MediaQuery(MediaQueryError::EmptyQuery),
        ImportWarning::MediaQuery(MediaQueryError::UnsupportedType(text())),
        ImportWarning::MediaQuery(MediaQueryError::UnsupportedCondition(text())),
        ImportWarning::MediaQuery(MediaQueryError::InvalidOrientation(text())),
        ImportWarning::MediaQuery(MediaQueryError::UnsupportedFeature(text())),
        ImportWarning::MediaQuery(MediaQueryError::UnsupportedRange(text())),
        ImportWarning::MediaQuery(MediaQueryError::InvalidRange(text())),
        ImportWarning::MediaQuery(MediaQueryError::InvalidLength(text())),
    ];
    all.extend(crate::import_warning_samples::sample_field_variants());
    all.extend(crate::import_warning_samples::sample_fixed_variants());
    all
}
