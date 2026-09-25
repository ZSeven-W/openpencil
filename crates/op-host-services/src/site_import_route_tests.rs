//! `/api/ai/site-import` over an in-memory site: no network.

use std::cell::RefCell;

use op_brand::{BrandError, BrandFetcher, FetchKind, Fetched};
use op_editor_core::{parse_site_import_reply, Locale};

use super::*;

const PAGE: &str = r#"<!doctype html><html><head><title>Acme</title></head><body>
<header><strong>Acme</strong></header>
<section style="padding:48px"><h1>Build faster</h1><p>Ship in days.</p></section>
</body></html>"#;

/// One HTML page at `https://acme.example/`; everything else is a 404.
/// Records every URL it was asked for, so a test can prove the route only
/// ever dialled through the injected fetcher.
#[derive(Default)]
struct MemorySite {
    asked: RefCell<Vec<String>>,
}

impl BrandFetcher for MemorySite {
    fn fetch(&self, url: &str, _kind: FetchKind) -> Result<Fetched, BrandError> {
        self.asked.borrow_mut().push(url.to_string());
        if url != "https://acme.example/" && url != "https://acme.example" {
            return Err(BrandError::Fetch {
                url: url.to_string(),
                detail: "404".into(),
            });
        }
        Ok(Fetched {
            final_url: "https://acme.example/".into(),
            body: PAGE.as_bytes().to_vec(),
            content_type: Some("text/html; charset=utf-8".into()),
        })
    }
}

fn serve(body: &str, site: &MemorySite) -> (&'static str, String) {
    serve_with(body, |url, locale| {
        crate::site_import::import_site(url, site, locale)
    })
}

#[test]
fn a_link_imports_through_the_shared_pipeline_and_the_browser_can_read_it() {
    let site = MemorySite::default();
    let (status, body) = serve(r#"{"url":"acme.example","locale":"zh-CN"}"#, &site);
    assert_eq!(status, "200 OK", "{body}");
    // A bare domain is normalized exactly like the Home button does.
    assert_eq!(site.asked.borrow()[0], "https://acme.example");

    let import = parse_site_import_reply(200, &body).expect("the browser parses the reply");
    assert_eq!(import.summary.host, "acme.example");
    assert_eq!(import.summary.source_url, "https://acme.example/");
    assert!(import.summary.node_count > 0);
    assert!(!import.document.children.is_empty());
    // The same finalize ran: the report carries its checks and an audit.
    assert!(
        import.report != op_editor_core::QualityReport::default(),
        "{body}"
    );
    // The reply is the whole result the desktop host installs, nothing
    // secret or host-local leaked into it.
    assert!(!body.contains("/Users/"), "{body}");
}

#[test]
fn requests_that_are_not_one_link_are_refused_before_any_fetch() {
    let site = MemorySite::default();
    for bad in [
        "",
        "not json",
        r#"{"url":"make it like https://acme.example"}"#,
        r#"{"url":"ftp://acme.example"}"#,
        r#"{"url":"https://acme.example","extra":1}"#,
    ] {
        let (status, body) = serve(bad, &site);
        assert_eq!(status, "400 Bad Request", "{bad} -> {body}");
        assert!(parse_site_import_reply(400, &body).is_err());
    }
    let oversized = format!(r#"{{"url":"https://acme.example/{}"}}"#, "a".repeat(8_000));
    assert_eq!(serve(&oversized, &site).0, "400 Bad Request");
    assert!(site.asked.borrow().is_empty(), "nothing was fetched");
}

#[test]
fn failures_map_to_a_status_and_a_detail_the_hint_can_show() {
    let site = MemorySite::default();
    let (status, body) = serve(r#"{"url":"https://elsewhere.example/"}"#, &site);
    assert_eq!(status, "502 Bad Gateway", "{body}");
    let detail = parse_site_import_reply(502, &body).expect_err("a failure");
    assert!(detail.contains("elsewhere.example"), "{detail}");

    let (status, _) = serve_with(r#"{"url":"https://acme.example"}"#, |_, _| {
        Err(SiteImportError::Busy)
    });
    assert_eq!(status, "429 Too Many Requests");

    let (status, _) = serve_with(r#"{"url":"https://acme.example"}"#, |_, _| {
        Err(SiteImportError::NoContent {
            detail: "empty".into(),
        })
    });
    assert_eq!(status, "422 Unprocessable Entity");
}

#[test]
fn an_unknown_locale_falls_back_instead_of_failing() {
    let mut seen = None;
    let _ = serve_with(
        r#"{"url":"https://acme.example","locale":"xx-YY"}"#,
        |_, locale| {
            seen = Some(locale);
            Err(SiteImportError::Busy)
        },
    );
    assert_eq!(seen, Some(Locale::EnUs));
}
