//! Website → kit, with the network injected.
//!
//! The crate never opens a socket: a host supplies a [`BrandFetcher`]
//! (the desktop / MCP one is policy-screened, capped, and time-limited;
//! tests use an in-memory map). This module decides WHAT to fetch — the
//! page, its linked stylesheets in document order, and their `@import`s —
//! and caps how much. No script runs; only served HTML and CSS are read.

use op_html::dom::{parse_dom, StylesheetSource};
use op_html::resources::resolve_url;

use crate::css_scan::{scan_stylesheet, ScanOutput};
use crate::css_signals::analyze_site;
use crate::error::BrandError;
use crate::kit::{BrandKit, BrandSource};

/// Linked stylesheets fetched per page (imports included).
pub const MAX_STYLESHEETS: usize = 16;
/// Total CSS read per page.
pub const MAX_CSS_BYTES: usize = 6 * 1024 * 1024;
/// `@import` nesting followed.
const MAX_IMPORT_DEPTH: usize = 2;

/// What is being fetched — lets a host apply per-kind caps / accept headers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchKind {
    Page,
    Stylesheet,
}

/// A fetched resource. `final_url` is after redirects (relative links in
/// the body resolve against it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fetched {
    pub final_url: String,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
}

/// The host's network seam.
pub trait BrandFetcher {
    fn fetch(&self, url: &str, kind: FetchKind) -> Result<Fetched, BrandError>;
}

/// Fetch `url`, read its HTML + CSS, and resolve a brand kit.
pub fn extract_from_url(url: &str, fetcher: &dyn BrandFetcher) -> Result<BrandKit, BrandError> {
    let page = fetcher.fetch(url, FetchKind::Page)?;
    let looks_html = page
        .content_type
        .as_deref()
        .map(|t| t.to_ascii_lowercase().contains("html"))
        .unwrap_or(false)
        || page.body.iter().take(512).any(|b| *b == b'<');
    if !looks_html {
        return Err(BrandError::NotHtml {
            url: page.final_url,
        });
    }
    let html = op_html::html_encoding::decode_html_bytes(&page.body).into_owned();
    let dom = parse_dom(&html);
    let base = dom
        .base_hrefs
        .iter()
        .find_map(|href| resolve_url(Some(&page.final_url), href))
        .unwrap_or_else(|| page.final_url.clone());

    let mut budget = Budget {
        sheets: 0,
        bytes: 0,
    };
    let mut sheets = Vec::new();
    for source in &dom.stylesheet_sources {
        match source {
            StylesheetSource::Inline(css) => {
                expand_imports(css, &base, fetcher, &mut budget, 0, &mut sheets);
                sheets.push(css.clone());
            }
            StylesheetSource::Link(href) => {
                if let Some(css) = fetch_sheet(href, &base, fetcher, &mut budget) {
                    expand_imports(&css.1, &css.0, fetcher, &mut budget, 0, &mut sheets);
                    sheets.push(css.1);
                }
            }
        }
    }
    let signals = analyze_site(&html, &sheets);
    let host = host_of(&page.final_url);
    let kit = BrandKit::from_signals(signals, BrandSource::Url(page.final_url.clone()), &host);
    Ok(kit)
}

/// Kit from HTML already in hand (a saved page): only its inline styles
/// and any `extra_css` the caller read from disk are used.
pub fn extract_from_html(html: &str, extra_css: &[String], label: &str) -> BrandKit {
    let mut sheets = extra_css.to_vec();
    sheets.extend(crate::css_signals::inline_sheets(html));
    let signals = analyze_site(html, &sheets);
    BrandKit::from_signals(signals, BrandSource::Html(label.to_string()), label)
}

struct Budget {
    sheets: usize,
    bytes: usize,
}

/// Fetch one stylesheet by (possibly relative) href. Failures are skipped:
/// a missing sheet makes the kit poorer, not impossible.
fn fetch_sheet(
    href: &str,
    base: &str,
    fetcher: &dyn BrandFetcher,
    budget: &mut Budget,
) -> Option<(String, String)> {
    if budget.sheets >= MAX_STYLESHEETS || budget.bytes >= MAX_CSS_BYTES {
        return None;
    }
    let url = resolve_url(Some(base), href)?;
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    budget.sheets += 1;
    let fetched = fetcher.fetch(&url, FetchKind::Stylesheet).ok()?;
    let room = MAX_CSS_BYTES - budget.bytes;
    let body = &fetched.body[..fetched.body.len().min(room)];
    budget.bytes += body.len();
    Some((
        fetched.final_url,
        String::from_utf8_lossy(body).into_owned(),
    ))
}

/// Fetch a sheet's `@import`s (depth-first, before the sheet itself so the
/// cascade order is kept) into `out`.
fn expand_imports(
    css: &str,
    sheet_url: &str,
    fetcher: &dyn BrandFetcher,
    budget: &mut Budget,
    depth: usize,
    out: &mut Vec<String>,
) {
    if depth >= MAX_IMPORT_DEPTH {
        return;
    }
    let mut scan = ScanOutput::default();
    scan_stylesheet(css, &mut scan);
    for target in scan.imports {
        if let Some((url, body)) = fetch_sheet(&target, sheet_url, fetcher, budget) {
            expand_imports(&body, &url, fetcher, budget, depth + 1, out);
            out.push(body);
        }
    }
}

/// `https://www.acme.example/x` → `acme.example`.
pub fn host_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    host.strip_prefix("www.").unwrap_or(host).to_string()
}
