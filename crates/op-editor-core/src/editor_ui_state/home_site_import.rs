//! Studio Home's one-click website import.
//!
//! When the composer holds nothing but a link, the primary button offers
//! "Import this site" instead of generating from the URL text. The press
//! only records a request here; fetching and the post-import pipeline
//! (finalize, brand kit + variable binding, component recognition) run in
//! the shell that owns the network, which reports back with
//! [`HomeSiteImportState::finish`]. The finished import arrives as a whole
//! document ([`SiteImportResult`]) that the host swaps in through the same
//! fresh-document path a Home brief uses.

use jian_ops_schema::PenDocument;

use crate::quality_report::QualityReport;

/// How long the failure hint stays up.
pub const SITE_IMPORT_HINT_MS: u64 = 4_000;

/// What the pipeline did, as facts — the host turns it into the chat
/// transcript line and the quality panel rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteImportSummary {
    /// The page's final URL (after redirects).
    pub source_url: String,
    /// Host label (`acme.example`).
    pub host: String,
    /// Layers in the imported page.
    pub node_count: usize,
    /// The applied brand kit's name, when one could be read.
    pub brand_name: Option<String>,
    /// Variables the kit added / updated.
    pub brand_variables: usize,
    /// Colour literals bound to those variables.
    pub colors_bound: usize,
    /// `(component name, instance count)` for every component created.
    pub components: Vec<(String, usize)>,
    /// Edits the finalize passes applied.
    pub finalize_fixes: usize,
    /// Importer warnings (parts of the page that degraded).
    pub warnings: usize,
}

impl SiteImportSummary {
    /// The assistant line the conversation dock shows after an import.
    pub fn transcript(&self, locale: crate::Locale) -> String {
        let t = |key| op_i18n::translate(locale, key);
        let mut lines = vec![t("workspace.siteImport.done")
            .replace("{{host}}", &self.host)
            .replace("{{nodes}}", &self.node_count.to_string())];
        match &self.brand_name {
            Some(name) => lines.push(
                t("workspace.siteImport.brand")
                    .replace("{{name}}", name)
                    .replace("{{variables}}", &self.brand_variables.to_string())
                    .replace("{{bound}}", &self.colors_bound.to_string()),
            ),
            None => lines.push(t("workspace.siteImport.brandFailed").to_string()),
        }
        if self.components.is_empty() {
            lines.push(t("workspace.siteImport.noComponents").to_string());
        } else {
            let list = self
                .components
                .iter()
                .map(|(name, count)| format!("{name} ×{count}"))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(t("workspace.siteImport.components").replace("{{list}}", &list));
        }
        lines.push(
            t("workspace.siteImport.finalize")
                .replace("{{fixed}}", &self.finalize_fixes.to_string()),
        );
        if self.warnings > 0 {
            lines.push(
                t("workspace.siteImport.warnings").replace("{{count}}", &self.warnings.to_string()),
            );
        }
        lines.join("\n")
    }
}

/// A finished import, ready to install.
#[derive(Debug, Clone)]
pub struct SiteImportResult {
    pub document: Box<PenDocument>,
    pub summary: SiteImportSummary,
    /// The workspace quality panel's report for this import.
    pub report: QualityReport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum HomeSiteImportStatus {
    #[default]
    Idle,
    /// A request is queued or running.
    Importing { label: String },
    /// The last import failed (hint shown for [`SITE_IMPORT_HINT_MS`]).
    Failed { detail: String },
}

#[derive(Debug, Clone, Default)]
pub struct HomeSiteImportState {
    /// Host capability: this host can fetch and import (desktop). Off, a
    /// link-only draft is an ordinary brief.
    pub available: bool,
    pub status: HomeSiteImportStatus,
    /// Wall-clock ms the status last changed (drives the hint timeout).
    pub status_at_ms: u64,
    pending: Option<(u64, String)>,
    generation: u64,
}

impl HomeSiteImportState {
    /// Queue an import of `url`. Returns whether a request was queued (a
    /// running import ignores further presses).
    pub fn press(&mut self, url: &str, now_ms: u64) -> bool {
        if self.is_importing() || !self.available {
            return false;
        }
        self.generation += 1;
        self.status = HomeSiteImportStatus::Importing {
            label: host_label(url),
        };
        self.status_at_ms = now_ms.max(1);
        self.pending = Some((self.generation, url.to_string()));
        true
    }

    /// Host: take the queued request `(generation, url)`.
    pub fn take_request(&mut self) -> Option<(u64, String)> {
        self.pending.take()
    }

    /// Host: whether a result for `generation` is still wanted. Settles
    /// the status to Idle when it is.
    pub fn accept(&mut self, generation: u64) -> bool {
        if generation != self.generation || !self.is_importing() {
            return false;
        }
        self.status = HomeSiteImportStatus::Idle;
        true
    }

    /// Host: record a failed import (after [`Self::accept`] or for a
    /// result that could not be installed).
    pub fn fail(&mut self, detail: impl Into<String>, now_ms: u64) {
        self.status = HomeSiteImportStatus::Failed {
            detail: detail.into(),
        };
        self.status_at_ms = now_ms.max(1);
    }

    /// Drop a running import (its result will be ignored).
    pub fn cancel(&mut self) {
        self.generation += 1;
        self.pending = None;
        self.status = HomeSiteImportStatus::Idle;
    }

    pub fn is_importing(&self) -> bool {
        matches!(self.status, HomeSiteImportStatus::Importing { .. })
    }

    /// Whether the failure hint is still up.
    pub fn hint_visible(&self, now_ms: u64) -> bool {
        matches!(self.status, HomeSiteImportStatus::Failed { .. })
            && now_ms.saturating_sub(self.status_at_ms) < SITE_IMPORT_HINT_MS
    }

    /// When the failure hint expires (the host wakes to erase it).
    pub fn hint_deadline_ms(&self) -> Option<u64> {
        matches!(self.status, HomeSiteImportStatus::Failed { .. })
            .then(|| self.status_at_ms.saturating_add(SITE_IMPORT_HINT_MS))
    }
}

/// The URL to import when the draft is a link and nothing else: an
/// `http(s)://` URL, or a bare domain (`acme.example/pricing`), which gets
/// `https://`. Anything with other words in it is a brief, not a link.
pub fn site_import_url(draft: &str) -> Option<String> {
    let text = draft.trim();
    if text.is_empty() || text.chars().any(char::is_whitespace) {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return super::brand::first_url(text);
    }
    if text.contains("://") {
        return None;
    }
    let host = text.split(['/', '?', '#']).next().unwrap_or(text);
    let labels: Vec<&str> = host.split('.').collect();
    let tld = labels.last().copied().unwrap_or("");
    let domain_like = labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && tld.len() >= 2
        && tld.chars().all(|c| c.is_ascii_alphabetic());
    domain_like.then(|| format!("https://{text}"))
}

/// `https://www.acme.example/x` → `acme.example`.
pub fn host_label(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

/// Longest origin kept on a document. Real page URLs are far shorter; the
/// cap only bounds what a hand-edited file can make the editor carry.
pub const IMPORT_ORIGIN_MAX_CHARS: usize = 512;

/// The import origin a document may carry (`editorMeta.importedFrom`):
/// scheme + host + path of `url`, and nothing else.
///
/// Query strings, fragments, credentials (`user:pass@`) and ports are
/// dropped — they are where session tokens and tracking ids live, and the
/// origin is saved into a file that gets shared. Only `http(s)` URLs with a
/// plausible host survive; anything else is `None`, never an error, because
/// the origin is a repair-policy hint, not a reason to refuse a file.
pub fn sanitize_import_origin(url: &str) -> Option<String> {
    let text = url.trim();
    let (scheme, rest) = text.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let rest = rest.split(['?', '#']).next().unwrap_or("");
    let (authority, path) = match rest.find('/') {
        Some(index) => rest.split_at(index),
        None => (rest, ""),
    };
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let host = match host_port.strip_prefix('[') {
        // IPv6 literal: keep the brackets, drop the port.
        Some(inner) => format!("[{}]", inner.split_once(']')?.0),
        None => host_port.split(':').next().unwrap_or("").to_string(),
    }
    .to_ascii_lowercase();
    let host_ok = host.len() > 2
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '[' | ']' | ':'));
    if !host_ok {
        return None;
    }
    let path: String = path
        .chars()
        .take_while(|c| !c.is_whitespace() && !c.is_control())
        .collect();
    let origin = format!("{scheme}://{host}{path}");
    Some(origin.chars().take(IMPORT_ORIGIN_MAX_CHARS).collect())
}

#[cfg(test)]
#[path = "home_site_import_tests.rs"]
mod tests;
