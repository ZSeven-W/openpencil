use std::collections::BTreeMap;
use std::net::IpAddr;
use std::time::Duration;

use jian_ops_schema::node::PenNode;
use op_editor_core::EditorCommand;
use op_html::{import_html_with_resources, HtmlImportOptions};
use op_mcp::import_common::{count_subtree_nodes, parse_import_placement, parse_viewport_height};
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};
use reqwest::header::{CONTENT_TYPE, LOCATION};

use crate::chat_runtime::block_on_anywhere;
use crate::import_html_url_error::ImportHtmlUrlError;
use crate::provider_dial::{client_for, EndpointDialPolicy};
use crate::web_image_search::{read_capped, ImageJobSlot};

const ALLOWLIST_ENV: &str = "OPENPENCIL_WEB_AI_ENDPOINT_ALLOWLIST";
const PAGE_BYTES_CAP: usize = 10 * 1024 * 1024;
const RESOURCE_BYTES_CAP: usize = 4 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_REDIRECTS: usize = 10;

pub(crate) struct ImportHtmlUrl;

pub(crate) struct ImportedPage {
    pub nodes: Vec<PenNode>,
    pub final_url: reqwest::Url,
    pub warnings: Vec<String>,
}

impl McpTool for ImportHtmlUrl {
    fn name(&self) -> &str {
        "import_html_url"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_url) = args.get("url") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "url is required".into());
        };
        let viewport_height = match parse_viewport_height(args) {
            Ok(value) => value,
            Err((code, message)) => return ToolOutcome::Err(code, message),
        };
        let placement = match parse_import_placement(args) {
            Ok(placement) => placement,
            Err((code, message)) => return ToolOutcome::Err(code, message),
        };

        let page = match import_page_from_url(raw_url, viewport_height) {
            Ok(page) => page,
            Err(error) => return ToolOutcome::Err(error.tool_error_code(), error.to_string()),
        };
        let mut nodes = page.nodes;
        if placement.x != 0 || placement.y != 0 {
            if let PenNode::Frame(frame) = &mut nodes[0] {
                frame.base.x = Some(placement.x as f64);
                frame.base.y = Some(placement.y as f64);
            }
        }
        // Diverges from `op_mcp::import_common::import_result_to_outcome`
        // only by the extra `sourceUrl` output entry.
        let mut output = BTreeMap::new();
        output.insert("wrote".into(), "true".into());
        output.insert("nodeCount".into(), count_subtree_nodes(&nodes).to_string());
        output.insert("sourceUrl".into(), page.final_url.to_string());
        if !page.warnings.is_empty() {
            output.insert("warnings".into(), page.warnings.join("\n"));
        }
        ToolOutcome::OkWithCommand(
            output,
            EditorCommand::InsertSubtree {
                nodes,
                parent_id: placement.parent_id,
                page_id: placement.page_id,
            },
        )
    }
}

/// Fetch and import a page without placing its nodes in the user's document.
/// This is the shared policy-screened importer used by the MCP tool and the
/// reference-context enrichment path.
pub(crate) fn import_page_from_url(
    raw_url: &str,
    viewport_height: Option<f64>,
) -> Result<ImportedPage, ImportHtmlUrlError> {
    let allowlist = std::env::var(ALLOWLIST_ENV).ok();
    let initial_url = screen_import_url_with_allowlist(raw_url, allowlist.as_deref())?;
    let Some(_job_slot) = ImageJobSlot::acquire() else {
        return Err(ImportHtmlUrlError::TooManyConcurrentJobs);
    };
    // `call` is a synchronous MCP tool that can be reached either from a
    // plain worker thread (CLI / stdio server) or from a tokio worker (the
    // web daemon's connection pool). A private runtime built here would
    // panic on `block_on` in the latter case, so both fetch paths below go
    // through the one sanctioned bridge, which sheds the worker instead.
    let page = block_on_anywhere(fetch_capped(
        initial_url,
        PAGE_BYTES_CAP,
        allowlist.as_deref(),
    ))?;
    let content_type_is_html = page
        .content_type
        .as_deref()
        .is_some_and(|value| value.to_ascii_lowercase().contains("text/html"));
    let body_looks_html = page.bytes.iter().take(512).any(|byte| *byte == b'<');
    if !content_type_is_html && !body_looks_html {
        return Err(ImportHtmlUrlError::NotHtmlPage);
    }

    let html = String::from_utf8_lossy(&page.bytes);
    let fetcher = |resource_url: &str| {
        let resource_url =
            screen_import_url_with_allowlist(resource_url, allowlist.as_deref()).ok()?;
        block_on_anywhere(fetch_capped(
            resource_url,
            RESOURCE_BYTES_CAP,
            allowlist.as_deref(),
        ))
        .ok()
        .map(|resource| resource.bytes)
    };
    let options = HtmlImportOptions {
        base_url: Some(page.final_url.to_string()),
        viewport_height,
        ..HtmlImportOptions::default()
    };
    let result = import_html_with_resources(&html, &options, Some(&fetcher), None);
    if result.nodes.is_empty() {
        let detail = result
            .warnings
            .first()
            .map(String::as_str)
            .unwrap_or("input produced no nodes");
        return Err(ImportHtmlUrlError::NoImportableContent {
            detail: detail.to_string(),
        });
    }
    Ok(ImportedPage {
        nodes: result.nodes,
        final_url: page.final_url,
        warnings: result.warnings,
    })
}

struct FetchedResource {
    bytes: Vec<u8>,
    content_type: Option<String>,
    final_url: reqwest::Url,
}

async fn fetch_capped(
    mut url: reqwest::Url,
    cap: usize,
    allowlist: Option<&str>,
) -> Result<FetchedResource, ImportHtmlUrlError> {
    for redirect_count in 0..=MAX_REDIRECTS {
        url = screen_import_url_with_allowlist(url.as_str(), allowlist)?;
        let policy = dial_policy(url.as_str(), allowlist);
        // `provider_dial::ProviderDialError` belongs to a sibling module this
        // pass does not own; render it with `to_string` so the adapter
        // survives if that module reshapes its error.
        let client = client_for(policy, url.as_str())
            .await
            .map_err(|error| ImportHtmlUrlError::Dial(error.to_string()))?;
        let response = client
            .get(url.clone())
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|error| ImportHtmlUrlError::Fetch {
                url: url.to_string(),
                detail: error.to_string(),
            })?;
        if response.status().is_redirection() {
            if redirect_count == MAX_REDIRECTS {
                return Err(ImportHtmlUrlError::TooManyRedirects);
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(ImportHtmlUrlError::RedirectMissingLocation)?;
            url = url
                .join(location)
                .map_err(|_| ImportHtmlUrlError::RedirectLocationInvalid)?;
            continue;
        }
        if !response.status().is_success() {
            return Err(ImportHtmlUrlError::HttpStatus {
                url: url.to_string(),
                status: response.status().to_string(),
            });
        }
        let final_url = screen_import_url_with_allowlist(response.url().as_str(), allowlist)?;
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let bytes = read_capped(response, cap).await.ok_or_else(|| {
            ImportHtmlUrlError::ResponseTooLarge {
                url: final_url.to_string(),
            }
        })?;
        return Ok(FetchedResource {
            bytes,
            content_type,
            final_url,
        });
    }
    Err(ImportHtmlUrlError::TooManyRedirects)
}

#[cfg(test)]
fn screen_import_url(url: &str) -> Result<reqwest::Url, ImportHtmlUrlError> {
    let allowlist = std::env::var(ALLOWLIST_ENV).ok();
    screen_import_url_with_allowlist(url, allowlist.as_deref())
}

fn screen_import_url_with_allowlist(
    url: &str,
    allowlist: Option<&str>,
) -> Result<reqwest::Url, ImportHtmlUrlError> {
    let parsed = reqwest::Url::parse(url.trim()).map_err(|_| ImportHtmlUrlError::UrlInvalid)?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(ImportHtmlUrlError::UrlNotAllowed);
    }
    if crate::web_credentials::base_url_is_explicitly_allowlisted(parsed.as_str(), allowlist) {
        return Ok(parsed);
    }
    let host = parsed
        .host_str()
        .expect("host presence checked above")
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host
        .parse::<IpAddr>()
        .is_ok_and(crate::web_credentials::is_restricted_ip)
        || is_restricted_hostname(&host)
    {
        return Err(ImportHtmlUrlError::UrlNotAllowed);
    }
    Ok(parsed)
}

fn is_restricted_hostname(host: &str) -> bool {
    host.is_empty()
        || !host.contains('.')
        || [
            "localhost",
            ".localhost",
            ".local",
            ".internal",
            ".home",
            ".lan",
            ".test",
            ".invalid",
        ]
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(suffix))
        || matches!(
            host,
            "metadata.google.internal"
                | "metadata.google"
                | "instance-data"
                | "instance-data.ec2.internal"
        )
}

fn dial_policy(url: &str, allowlist: Option<&str>) -> EndpointDialPolicy {
    if crate::web_credentials::base_url_is_explicitly_allowlisted(url, allowlist) {
        EndpointDialPolicy::Trusted
    } else {
        EndpointDialPolicy::PublicOnly
    }
}

pub(crate) fn import_html_url_snapshot() -> ImportHtmlUrl {
    ImportHtmlUrl
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_import_url_rejects_restricted_hosts() {
        assert!(screen_import_url("https://example.com/page").is_ok());
        assert!(screen_import_url("https://example.com/p?q=1#f").is_ok());
        assert!(screen_import_url("http://127.0.0.1:3000/").is_err());
        assert!(screen_import_url("http://169.254.169.254/meta").is_err());
        assert!(screen_import_url("http://localhost/x").is_err());
        assert!(screen_import_url("ftp://example.com/x").is_err());
        assert!(screen_import_url("https://user:pw@example.com/").is_err());
    }

    #[test]
    fn missing_url_is_typed_error() {
        let out = ImportHtmlUrl.call(&BTreeMap::new());
        assert!(matches!(out, ToolOutcome::Err(..)));
    }
}
