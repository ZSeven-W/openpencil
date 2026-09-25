//! One-click website import: URL → an editable, branded, componentized
//! document, plus the facts the workspace reports about it.
//!
//! The pipeline, in order:
//!
//! 1. **Fetch + import.** The page and its stylesheets / images go through
//!    one injected [`BrandFetcher`] (production: the importer's SSRF-screened,
//!    size-capped fetcher) and `op-html` maps the DOM to a `.op` tree. A
//!    per-import cache means the brand step below re-reads nothing.
//! 2. **Brand kit.** `op-brand` reads the same page + CSS into a kit, which
//!    is applied to the document as design variables (+ design.md).
//! 3. **Finalize.** The finalizer's contract tier (text contrast, geometry:
//!    overflow / collapsed boxes) over the imported boards. The intent-tier
//!    passes MCP sessions also run — heading re-weighting, entrance motion,
//!    hero gaps, surface restyling — are skipped: the site is authored input
//!    (see `op_orchestrator::finalize_authored_import`).
//! 4. **Bind.** Colour literals that EQUAL a kit variable's value become
//!    `$variable` references — nothing changes on screen, but the design
//!    now follows the brand when it is swapped.
//! 5. **Components.** Structurally identical repeated parts (cards,
//!    buttons, list items) become one reusable master + proven `ref`
//!    instances (see `op_editor_core::component_recognition`).
//! 6. **Audit.** The end-of-run detectors run on the final boards, so the
//!    quality panel reports what still needs attention.
//!
//! Everything here is blocking; hosts call it off the UI thread.

use std::cell::RefCell;
use std::collections::HashMap;

use op_brand::{BrandError, BrandFetcher, FetchKind, Fetched};
use op_editor_core::component_recognition::{
    componentize_repeated_structures, RecognizedComponent,
};
use op_editor_core::{
    host_label, EditorState, Locale, PenNodeExt, QualityRepairRecord, QualityReport,
    SiteImportResult, SiteImportSummary,
};
use op_html::HtmlImportOptions;

use crate::brand_extract::{brand_kit_payload, ScreenedBrandFetcher};
use crate::site_import_error::SiteImportError;
use crate::web_image_search::ImageJobSlot;

/// Import `url` with the production (policy-screened) fetcher.
pub fn import_site_screened(
    url: &str,
    locale: Locale,
) -> Result<SiteImportResult, SiteImportError> {
    let Some(_slot) = ImageJobSlot::acquire() else {
        return Err(SiteImportError::Busy);
    };
    import_site(url, &ScreenedBrandFetcher, locale)
}

/// Import `url` through `fetcher` (tests inject an in-memory one).
pub fn import_site(
    url: &str,
    fetcher: &dyn BrandFetcher,
    locale: Locale,
) -> Result<SiteImportResult, SiteImportError> {
    let cache = CachingFetcher::new(fetcher);
    let page = cache.fetch(url, FetchKind::Page)?;
    let looks_html = page
        .content_type
        .as_deref()
        .is_some_and(|t| t.to_ascii_lowercase().contains("html"))
        || page.body.iter().take(512).any(|b| *b == b'<');
    if !looks_html {
        return Err(SiteImportError::NotHtml {
            url: page.final_url,
        });
    }
    let html = op_html::html_encoding::decode_html_bytes(&page.body).into_owned();
    let resources = |resource_url: &str| {
        cache
            .fetch(resource_url, FetchKind::Stylesheet)
            .ok()
            .map(|fetched| fetched.body)
    };
    let options = HtmlImportOptions {
        base_url: Some(page.final_url.clone()),
        ..HtmlImportOptions::default()
    };
    let imported = op_html::import_html_document(&html, &options, Some(&resources), None);
    if imported.document.children.is_empty() {
        return Err(SiteImportError::NoContent {
            detail: imported
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "input produced no nodes".into()),
        });
    }
    let warnings = imported.warnings.len();
    let host = host_label(&page.final_url);
    let mut state = EditorState::from_document(imported.document);
    let node_count = count_nodes(state.active_children());

    // Brand kit from the same page + CSS (cache hits, no second fetch).
    let kit = op_brand::extract_from_url(&page.final_url, &cache).ok();
    let mut brand_variables = 0;
    if let Some(kit) = &kit {
        let payload = brand_kit_payload(kit);
        brand_variables = payload.variables.len();
        state.apply_brand_kit(&payload);
    }

    let repairs = op_orchestrator::finalize_authored_import(&mut state, &host);

    let mut nodes = state.active_children().to_vec();
    let colors_bound = if kit.is_some() {
        op_orchestrator::bind_exact_color_variables(&mut nodes, &state)
    } else {
        0
    };
    let components = componentize_repeated_structures(&mut nodes);
    *state.active_children_mut() = nodes;

    let summary = SiteImportSummary {
        source_url: page.final_url.clone(),
        host,
        node_count,
        brand_name: kit.as_ref().map(|kit| kit.name.clone()),
        brand_variables,
        colors_bound,
        components: components
            .iter()
            .map(|c| (c.name.clone(), c.instance_ids.len() + 1))
            .collect(),
        finalize_fixes: repairs.total_repairs(),
        warnings,
    };
    let report = build_report(&state, &repairs, &summary, &components, locale);
    Ok(SiteImportResult {
        document: Box::new(state.doc),
        summary,
        report,
    })
}

/// The quality panel's report: the finalize's own checks and edits, the
/// import's two structural steps as applied fixes, then the final audit.
fn build_report(
    state: &EditorState,
    repairs: &op_orchestrator::RepairSummary,
    summary: &SiteImportSummary,
    components: &[RecognizedComponent],
    locale: Locale,
) -> QualityReport {
    let mut report = QualityReport::default();
    let checks: Vec<String> = repairs
        .checked()
        .into_iter()
        .map(|check| check.key().to_string())
        .collect();
    let mut items: Vec<QualityRepairRecord> = repairs
        .records()
        .iter()
        .map(op_orchestrator::RepairRecord::quality_item)
        .collect();
    if summary.colors_bound > 0 {
        items.push(QualityRepairRecord {
            pass: "site-import:brand-binding".into(),
            family: "palette".into(),
            node_id: String::new(),
            node_name: summary.brand_name.clone(),
            detail: op_i18n::translate(locale, "workspace.siteImport.bound")
                .replace("{{bound}}", &summary.colors_bound.to_string()),
        });
    }
    for component in components {
        items.push(QualityRepairRecord {
            pass: "site-import:components".into(),
            family: "structure".into(),
            node_id: component.master_id.clone(),
            node_name: Some(component.name.clone()),
            detail: op_i18n::translate(locale, "workspace.siteImport.instances")
                .replace("{{count}}", &(component.instance_ids.len() + 1).to_string()),
        });
    }
    report.ingest_repairs(&checks, &items, repairs.notes());
    let boards: Vec<String> = state
        .active_children()
        .iter()
        .filter(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
        .map(|node| node.id_str().to_string())
        .collect();
    let audit = op_orchestrator::quality_audit::audit_final_quality(state, &boards);
    report.ingest_audit(&audit.audited_topics, audit.remaining);
    report.attribute_boards(state, &boards);
    report
}

fn count_nodes(nodes: &[jian_ops_schema::node::PenNode]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + node.children().map_or(0, |c| count_nodes(c)))
        .sum()
}

/// Memoizes every response (under both the requested and the final URL)
/// for the length of one import, so the brand step and repeated resource
/// references never hit the network twice.
struct CachingFetcher<'a> {
    inner: &'a dyn BrandFetcher,
    cache: RefCell<HashMap<String, Result<Fetched, BrandError>>>,
}

impl<'a> CachingFetcher<'a> {
    fn new(inner: &'a dyn BrandFetcher) -> Self {
        Self {
            inner,
            cache: RefCell::new(HashMap::new()),
        }
    }
}

impl BrandFetcher for CachingFetcher<'_> {
    fn fetch(&self, url: &str, kind: FetchKind) -> Result<Fetched, BrandError> {
        if let Some(hit) = self.cache.borrow().get(url) {
            return hit.clone();
        }
        let result = self.inner.fetch(url, kind);
        let mut cache = self.cache.borrow_mut();
        if let Ok(fetched) = &result {
            cache
                .entry(fetched.final_url.clone())
                .or_insert_with(|| result.clone());
        }
        cache.insert(url.to_string(), result.clone());
        result
    }
}

#[cfg(test)]
#[path = "site_import_tests.rs"]
mod tests;
