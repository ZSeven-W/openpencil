use super::*;

#[test]
fn only_a_link_and_nothing_else_is_an_import() {
    assert_eq!(
        site_import_url("  https://acme.example/pricing  ").as_deref(),
        Some("https://acme.example/pricing")
    );
    assert_eq!(
        site_import_url("acme.example").as_deref(),
        Some("https://acme.example")
    );
    assert_eq!(
        site_import_url("www.acme.example/about").as_deref(),
        Some("https://www.acme.example/about")
    );
    // A brief that mentions a link is a brief.
    assert_eq!(site_import_url("make it like https://acme.example"), None);
    assert_eq!(site_import_url("做一个类似 acme.example 的页面"), None);
    // Things that only look dotted are not domains.
    assert_eq!(site_import_url("v0.8"), None);
    assert_eq!(site_import_url("3.14"), None);
    assert_eq!(site_import_url("ftp://acme.example"), None);
    assert_eq!(site_import_url(""), None);
}

#[test]
fn a_press_queues_one_request_and_a_stale_result_is_dropped() {
    let mut state = HomeSiteImportState::default();
    assert!(!state.press("https://acme.example", 5), "unavailable host");
    state.available = true;
    assert!(state.press("https://www.acme.example/x", 5));
    assert_eq!(
        state.status,
        HomeSiteImportStatus::Importing {
            label: "acme.example".into()
        }
    );
    assert!(!state.press("https://other.example", 6), "one at a time");
    let (generation, url) = state.take_request().expect("queued");
    assert_eq!(url, "https://www.acme.example/x");
    assert!(state.take_request().is_none());

    state.cancel();
    assert!(!state.accept(generation), "cancelled import is ignored");

    assert!(state.press("https://acme.example", 10));
    let (generation, _) = state.take_request().unwrap();
    assert!(state.accept(generation));
    assert_eq!(state.status, HomeSiteImportStatus::Idle);
}

#[test]
fn a_failure_shows_a_timed_hint() {
    let mut state = HomeSiteImportState {
        available: true,
        ..Default::default()
    };
    state.press("https://acme.example", 100);
    let (generation, _) = state.take_request().unwrap();
    assert!(state.accept(generation));
    state.fail("HTTP 404", 200);
    assert!(state.hint_visible(200 + SITE_IMPORT_HINT_MS - 1));
    assert!(!state.hint_visible(200 + SITE_IMPORT_HINT_MS));
    assert_eq!(state.hint_deadline_ms(), Some(200 + SITE_IMPORT_HINT_MS));
}

#[test]
fn the_transcript_states_what_was_done() {
    let summary = SiteImportSummary {
        source_url: "https://acme.example/".into(),
        host: "acme.example".into(),
        node_count: 42,
        brand_name: Some("Acme".into()),
        brand_variables: 30,
        colors_bound: 12,
        components: vec![("Card".into(), 3), ("Button".into(), 2)],
        finalize_fixes: 4,
        warnings: 1,
    };
    let text = summary.transcript(crate::Locale::EnUs);
    assert!(text.contains("acme.example"), "{text}");
    assert!(text.contains("42"), "{text}");
    assert!(text.contains("Card ×3, Button ×2"), "{text}");
    assert!(text.contains("12"), "{text}");
    assert!(!text.contains("{{"), "{text}");
}

#[test]
fn the_import_origin_keeps_scheme_host_and_path_only() {
    assert_eq!(
        sanitize_import_origin("https://acme.example/pricing?utm=x&token=abc#plans").as_deref(),
        Some("https://acme.example/pricing")
    );
    // Credentials and the port go; the host is lowercased.
    assert_eq!(
        sanitize_import_origin("HTTPS://user:pw@Acme.Example:8443/a/b").as_deref(),
        Some("https://acme.example/a/b")
    );
    assert_eq!(
        sanitize_import_origin("http://acme.example?session=1").as_deref(),
        Some("http://acme.example")
    );
    assert_eq!(
        sanitize_import_origin("https://[::1]:3000/x").as_deref(),
        Some("https://[::1]/x")
    );
    // Not a web origin at all.
    assert_eq!(
        sanitize_import_origin("file:///Users/eve/secret.html"),
        None
    );
    assert_eq!(sanitize_import_origin("javascript:alert(1)"), None);
    assert_eq!(sanitize_import_origin("https://"), None);
    assert_eq!(sanitize_import_origin("https://bad host/x"), None);
    let long = format!("https://acme.example/{}", "a".repeat(2_000));
    assert_eq!(
        sanitize_import_origin(&long).map(|origin| origin.chars().count()),
        Some(IMPORT_ORIGIN_MAX_CHARS)
    );
}
