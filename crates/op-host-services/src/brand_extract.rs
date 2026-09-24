//! Brand extraction host arm: the policy-screened fetcher `op-brand` needs,
//! the desktop / CLI entry points, and the `brand_extract` MCP tool.
//!
//! `op-brand` decides what to read and how to map it; this module owns the
//! network (the same SSRF screen, redirect re-screening, byte caps, and
//! timeout the HTML importer uses) and the filesystem (image paths).

use std::collections::BTreeMap;

use op_brand::{BrandError, BrandFetcher, BrandKit, FetchKind, Fetched};
use op_editor_core::{BrandKitPayload, EditorState};
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};

use crate::import_html_url::{fetch_screened, PAGE_BYTES_CAP, RESOURCE_BYTES_CAP};
use crate::web_image_search::ImageJobSlot;

/// Screenshots above this are refused before decoding.
pub const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

/// The production fetcher: every request (and every redirect hop) passes the
/// importer's SSRF screen; pages cap at 10 MiB, stylesheets at 4 MiB.
pub struct ScreenedBrandFetcher;

impl BrandFetcher for ScreenedBrandFetcher {
    fn fetch(&self, url: &str, kind: FetchKind) -> Result<Fetched, BrandError> {
        let cap = match kind {
            FetchKind::Page => PAGE_BYTES_CAP,
            FetchKind::Stylesheet => RESOURCE_BYTES_CAP,
        };
        let fetched = fetch_screened(url, cap).map_err(|error| BrandError::Fetch {
            url: url.to_string(),
            detail: error.to_string(),
        })?;
        Ok(Fetched {
            final_url: fetched.final_url.to_string(),
            body: fetched.bytes,
            content_type: fetched.content_type,
        })
    }
}

/// Extract a kit from a live website (blocking; call off the UI thread).
pub fn extract_brand_from_url(url: &str) -> Result<BrandKit, BrandError> {
    let Some(_slot) = ImageJobSlot::acquire() else {
        return Err(BrandError::Fetch {
            url: url.to_string(),
            detail: "too many concurrent fetches; try again".into(),
        });
    };
    op_brand::extract_from_url(url, &ScreenedBrandFetcher)
}

/// Extract a kit from a local screenshot file.
pub fn extract_brand_from_image_path(path: &str) -> Result<BrandKit, BrandError> {
    let too_big = std::fs::metadata(path)
        .map(|m| m.len() > MAX_IMAGE_BYTES)
        .map_err(|e| BrandError::ImageDecode {
            detail: format!("{path}: {e}"),
        })?;
    if too_big {
        return Err(BrandError::ImageDecode {
            detail: format!("{path}: larger than {MAX_IMAGE_BYTES} bytes"),
        });
    }
    let bytes = std::fs::read(path).map_err(|e| BrandError::ImageDecode {
        detail: format!("{path}: {e}"),
    })?;
    op_brand::extract_from_image(&bytes, path)
}

/// Extract a kit from screenshot bytes already in memory (a staged chat
/// attachment). `name` labels the source and names the kit.
pub fn extract_brand_from_image_bytes(bytes: &[u8], name: &str) -> Result<BrandKit, BrandError> {
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(BrandError::ImageDecode {
            detail: format!("{name}: larger than {MAX_IMAGE_BYTES} bytes"),
        });
    }
    op_brand::extract_from_image(bytes, name)
}

/// The editor-side payload for a kit.
pub fn brand_kit_payload(kit: &BrandKit) -> BrandKitPayload {
    BrandKitPayload {
        label: kit.name.clone(),
        swatches: kit.swatches(),
        variables: kit.variables(),
        themes: kit.themes(),
        design_md: Some(kit.design_md()),
    }
}

/// `brand_extract` — `url` or `imagePath` → kit JSON; `apply=true` also
/// writes the kit into the document as one undo step.
pub(crate) struct BrandExtract {
    current_design_md: Option<jian_ops_schema::DesignMdSpec>,
}

pub(crate) fn brand_extract_snapshot(doc: &EditorState) -> BrandExtract {
    BrandExtract {
        current_design_md: doc.doc.design_md.clone(),
    }
}

impl BrandExtract {
    /// The tool body with the extraction injected, so tests run offline.
    pub(crate) fn call_with(
        &self,
        args: &BTreeMap<String, String>,
        extract: &dyn Fn(&BrandSourceArg) -> Result<BrandKit, BrandError>,
    ) -> ToolOutcome {
        let source = match (args.get("url"), args.get("imagePath")) {
            (Some(url), None) if !url.trim().is_empty() => BrandSourceArg::Url(url.trim().into()),
            (None, Some(path)) if !path.trim().is_empty() => {
                BrandSourceArg::ImagePath(path.trim().into())
            }
            (Some(_), Some(_)) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "pass exactly one of url or imagePath".into(),
                )
            }
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "url or imagePath is required".into(),
                )
            }
        };
        let apply = match args.get("apply").map(|v| v.trim().to_ascii_lowercase()) {
            None => false,
            Some(v) if v == "true" => true,
            Some(v) if v == "false" || v.is_empty() => false,
            Some(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "apply must be true or false".into(),
                )
            }
        };
        let kit = match extract(&source) {
            Ok(kit) => kit,
            Err(error) => {
                // A network failure is worth retrying; a source that is not
                // an HTML page / decodable image is a wrong argument.
                let code = match &error {
                    BrandError::Fetch { .. } => ToolErrorCode::ToolFailed,
                    _ => ToolErrorCode::InvalidArgument,
                };
                return ToolOutcome::Err(code, error.to_string());
            }
        };
        let payload = brand_kit_payload(&kit);
        let mut out = BTreeMap::new();
        out.insert("name".to_string(), kit.name.clone());
        out.insert("kit".to_string(), kit.to_json().to_string());
        out.insert(
            "variableCount".to_string(),
            payload.variables.len().to_string(),
        );
        out.insert("applied".to_string(), apply.to_string());
        if !apply {
            return ToolOutcome::Ok(out);
        }
        out.insert(
            "designMdWritten".to_string(),
            payload
                .writes_design_md(self.current_design_md.as_ref())
                .to_string(),
        );
        out.insert("wrote".to_string(), "true".to_string());
        let command = payload.to_command(self.current_design_md.as_ref());
        ToolOutcome::OkWithCommand(out, command)
    }
}

/// The parsed source argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BrandSourceArg {
    Url(String),
    ImagePath(String),
}

impl McpTool for BrandExtract {
    fn name(&self) -> &str {
        "brand_extract"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        self.call_with(args, &|source| match source {
            BrandSourceArg::Url(url) => extract_brand_from_url(url),
            BrandSourceArg::ImagePath(path) => extract_brand_from_image_path(path),
        })
    }
}

#[cfg(test)]
#[path = "brand_extract_tests.rs"]
mod tests;
