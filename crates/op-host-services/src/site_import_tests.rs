//! End-to-end website import over an in-memory site: no network.

use std::cell::RefCell;
use std::collections::HashMap;

use jian_ops_schema::node::PenNode;
use op_brand::{BrandError, BrandFetcher, FetchKind, Fetched};
use op_editor_core::{EditorState, Locale, NodeId, PenNodeExt, QualityTopic};

use super::*;

const PAGE: &str = r#"<!doctype html><html><head><title>Acme</title>
<link rel="stylesheet" href="/site.css"></head><body>
<header class="nav"><strong>Acme</strong></header>
<section class="hero"><h1>Build faster</h1><a class="btn" href="/start">Get started</a></section>
<section class="grid">
  <div class="card"><h3>Fast</h3><p>Ship in days.</p></div>
  <div class="card"><h3>Safe</h3><p>Audited code.</p></div>
  <div class="card"><h3>Open</h3><p>MIT licensed.</p></div>
</section>
<section class="cta"><h2>Ready?</h2><a class="btn" href="/sales">Contact sales</a></section>
</body></html>"#;

const CSS: &str = "
body{margin:0;font-family:Inter,sans-serif;background:#FFFDF8;color:#1C1917}
.nav{padding:16px 64px;display:flex}
.hero,.cta{padding:64px;display:flex;flex-direction:column;gap:16px}
.btn{background:#0E7C66;color:#fff;border-radius:10px;padding:12px 20px;display:inline-block;text-decoration:none}
.grid{display:flex;gap:24px;padding:32px}
.card{background:#fff;border:1px solid #E7E5E4;border-radius:12px;padding:24px;width:280px;display:flex;flex-direction:column;gap:8px}
.card h3{margin:0;font-size:20px}
.card p{margin:0;font-size:14px;color:#57534E}
";

/// A site served from memory, counting requests per URL.
struct MemorySite {
    pages: HashMap<String, (&'static str, &'static str)>,
    hits: RefCell<HashMap<String, usize>>,
}

impl MemorySite {
    fn acme() -> Self {
        let mut pages = HashMap::new();
        pages.insert("https://acme.example/".to_string(), ("text/html", PAGE));
        pages.insert(
            "https://acme.example/site.css".to_string(),
            ("text/css", CSS),
        );
        Self {
            pages,
            hits: RefCell::new(HashMap::new()),
        }
    }

    fn hits(&self, url: &str) -> usize {
        self.hits.borrow().get(url).copied().unwrap_or(0)
    }
}

impl BrandFetcher for MemorySite {
    fn fetch(&self, url: &str, _kind: FetchKind) -> Result<Fetched, BrandError> {
        *self.hits.borrow_mut().entry(url.to_string()).or_insert(0) += 1;
        let Some((content_type, body)) = self.pages.get(url) else {
            return Err(BrandError::Fetch {
                url: url.to_string(),
                detail: "404".into(),
            });
        };
        Ok(Fetched {
            final_url: url.to_string(),
            body: body.as_bytes().to_vec(),
            content_type: Some(content_type.to_string()),
        })
    }
}

fn texts(nodes: &[PenNode], out: &mut Vec<String>) {
    for node in nodes {
        if let PenNode::Text(text) = node {
            out.push(serde_json::to_string(&text.content).unwrap());
        }
        if let Some(children) = node.children() {
            texts(children, out);
        }
    }
}

fn rendered_texts(state: &EditorState) -> Vec<String> {
    let doc = op_editor_core::ref_resolve::resolve_refs_for_canvas(&state.doc);
    let mut out = Vec::new();
    texts(&doc.children, &mut out);
    out
}

#[test]
fn a_site_imports_as_a_branded_componentized_document() {
    let site = MemorySite::acme();
    let result = import_site("https://acme.example/", &site, Locale::EnUs).expect("imports");

    // One request per resource: the brand step re-used the import's reads.
    assert_eq!(site.hits("https://acme.example/"), 1);
    assert_eq!(site.hits("https://acme.example/site.css"), 1);

    let summary = &result.summary;
    assert_eq!(summary.host, "acme.example");
    assert!(summary.node_count > 10, "{summary:?}");
    assert!(summary.brand_name.is_some(), "{summary:?}");
    assert!(summary.brand_variables > 0);
    assert!(
        summary.colors_bound > 0,
        "brand literals bound: {summary:?}"
    );
    let names: Vec<&str> = summary.components.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"Card"), "{summary:?}");
    assert!(names.contains(&"Button"), "{summary:?}");
    assert!(summary.components.contains(&("Card".to_string(), 3)));
    assert!(summary.components.contains(&("Button".to_string(), 2)));

    let state = EditorState::from_document(*result.document);
    // The kit's variables are on the document and the brand green is one
    // of them; the button fill follows it now.
    let variables = state.doc.variables.as_ref().expect("kit variables");
    assert!(variables.contains_key("--primary"));
    let json = serde_json::to_string(&state.doc).unwrap();
    assert!(json.contains("\"$--primary\""), "button fill bound");
    // The components registry rebuilds from the saved document.
    assert_eq!(state.components.components.len(), 2);

    // Nothing the page said is lost: every text renders, refs expanded.
    let rendered = rendered_texts(&state);
    for needle in [
        "Acme",
        "Build faster",
        "Get started",
        "Fast",
        "Ship in days.",
        "Safe",
        "Audited code.",
        "Open",
        "MIT licensed.",
        "Ready?",
        "Contact sales",
    ] {
        assert!(
            rendered.iter().any(|t| t.contains(needle)),
            "{needle} missing from {rendered:?}"
        );
    }

    // Instances are real refs to the master.
    let card = &state
        .components
        .components
        .iter()
        .find(|c| c.name == "Card")
        .unwrap()
        .id;
    let mut refs = 0;
    fn count_refs(nodes: &[PenNode], target: &NodeId, refs: &mut usize) {
        for node in nodes {
            if let PenNode::Ref(r) = node {
                if r.target == target.as_str() {
                    *refs += 1;
                }
            }
            if let Some(children) = node.children() {
                count_refs(children, target, refs);
            }
        }
    }
    count_refs(state.active_children(), card, &mut refs);
    assert_eq!(refs, 2);

    // The report is audited and names the import's two steps.
    let report = &result.report;
    assert!(report.audited);
    assert!(!report.is_empty());
    let topic = |t: QualityTopic| report.topics.iter().find(|e| e.topic == t);
    let consistency = topic(QualityTopic::Consistency).expect("components row");
    assert!(consistency
        .fixed
        .iter()
        .any(|item| item.node_name.as_deref() == Some("Card")));
    let palette = topic(QualityTopic::Palette).expect("binding row");
    assert!(palette
        .fixed
        .iter()
        .any(|item| item.source == "site-import:brand-binding"));

    let transcript = summary.transcript(Locale::EnUs);
    assert!(transcript.contains("Card ×3"), "{transcript}");
}

#[test]
fn the_finalize_defers_to_the_site_as_authored() {
    let site = MemorySite::acme();
    let result = import_site("https://acme.example/", &site, Locale::EnUs).unwrap();
    // The intent tier skip is recorded, never silent.
    assert!(
        result
            .report
            .notes
            .iter()
            .any(|note| note.contains("authored import")),
        "{:?}",
        result.report.notes
    );
}

#[test]
fn a_page_that_is_not_html_is_refused() {
    let mut site = MemorySite::acme();
    site.pages.insert(
        "https://acme.example/data".into(),
        ("application/json", "{\"a\":1}"),
    );
    let error = import_site("https://acme.example/data", &site, Locale::EnUs).unwrap_err();
    assert!(matches!(error, SiteImportError::NotHtml { .. }), "{error}");
}

#[test]
fn a_fetch_failure_is_typed() {
    let site = MemorySite::acme();
    let error = import_site("https://acme.example/missing", &site, Locale::EnUs).unwrap_err();
    assert!(matches!(error, SiteImportError::Fetch(_)), "{error}");
}

#[test]
fn a_site_without_readable_brand_still_imports() {
    let mut site = MemorySite::acme();
    site.pages.insert(
        "https://plain.example/".into(),
        ("text/html", "<html><body><p>Hello</p></body></html>"),
    );
    let result = import_site("https://plain.example/", &site, Locale::EnUs).unwrap();
    assert!(result.summary.components.is_empty());
    let state = EditorState::from_document(*result.document);
    assert!(rendered_texts(&state).iter().any(|t| t.contains("Hello")));
}
