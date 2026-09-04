use crate::import_warning::ImportWarning;
use base64::Engine as _;
use jian_ops_schema::node::{ImageSrc, PenNode};
use jian_ops_schema::style::PenFill;
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

#[path = "resources_css_imports.rs"]
mod css_imports;
#[path = "resources_css_urls.rs"]
mod css_urls;
#[path = "resources_image_metadata.rs"]
mod image_metadata;
pub(crate) use css_imports::expand_stylesheet_imports;
#[cfg(test)]
use image_metadata::data_url_image_dimensions;
#[cfg(test)]
use image_metadata::MAX_IMAGE_METADATA_BASE64_CHARS;
pub(crate) use image_metadata::{
    browser_image_metadata, element_intrinsic_metadata, normalize_image_source,
    prefetch_image_metadata, BrowserImageMetadata,
};
use image_metadata::{data_url_image_metadata, is_data_url};

pub type ResourceFetcher<'a> = dyn Fn(&str) -> Option<Vec<u8>> + 'a;
pub type ImageTransform<'a> = dyn Fn(&[u8]) -> Option<Vec<u8>> + 'a;

const PLACEHOLDER_GRAY_PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
const MAX_FETCHED_IMAGE_RESOURCES: usize = 2_048;
const MAX_SOURCE_IMAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_CACHED_IMAGE_DATA_URL_BYTES: usize = 64 * 1024 * 1024;
const MAX_RESOURCE_URL_BYTES: usize = 8 * 1024;
const OVERSIZED_IMAGE_URL_LABEL: &str = "image URL exceeds importer limit";

pub(crate) struct ResourceBudget {
    fetched_images: usize,
    cached_image_bytes: usize,
    max_fetched_images: usize,
    max_source_image_bytes: usize,
    max_cached_image_bytes: usize,
    cache_exhausted: bool,
    exhaustion_warning_emitted: bool,
    prefetched_images: usize,
    pending_image_bytes: usize,
    max_prefetched_images: usize,
    max_pending_image_bytes: usize,
    prefetch_exhausted: bool,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            fetched_images: 0,
            cached_image_bytes: 0,
            max_fetched_images: MAX_FETCHED_IMAGE_RESOURCES,
            max_source_image_bytes: MAX_SOURCE_IMAGE_BYTES,
            max_cached_image_bytes: MAX_CACHED_IMAGE_DATA_URL_BYTES,
            cache_exhausted: false,
            exhaustion_warning_emitted: false,
            prefetched_images: 0,
            pending_image_bytes: 0,
            max_prefetched_images: MAX_FETCHED_IMAGE_RESOURCES,
            max_pending_image_bytes: MAX_CACHED_IMAGE_DATA_URL_BYTES,
            prefetch_exhausted: false,
        }
    }
}

impl ResourceBudget {
    pub(crate) fn take(&mut self, _warnings: &mut Vec<ImportWarning>) -> bool {
        true
    }

    fn take_image(&mut self) -> bool {
        if self.cache_exhausted || self.fetched_images >= self.max_fetched_images {
            return false;
        }
        self.fetched_images += 1;
        true
    }

    fn source_allowed(&self, bytes: usize) -> bool {
        bytes <= self.max_source_image_bytes
    }

    fn take_prefetch(&mut self) -> bool {
        if self.prefetch_exhausted || self.prefetched_images >= self.max_prefetched_images {
            return false;
        }
        self.prefetched_images += 1;
        true
    }

    fn reserve_pending(&mut self, bytes: usize) -> bool {
        let Some(next) = self.pending_image_bytes.checked_add(bytes) else {
            self.prefetch_exhausted = true;
            return false;
        };
        if next > self.max_pending_image_bytes {
            self.prefetch_exhausted = true;
            return false;
        }
        self.pending_image_bytes = next;
        true
    }

    fn release_pending(&mut self, bytes: usize) {
        self.pending_image_bytes = self.pending_image_bytes.saturating_sub(bytes);
    }

    fn reserve_data_url(&mut self, bytes: &[u8]) -> bool {
        let Some(encoded) = bytes.len().checked_add(2).map(|size| size / 3 * 4) else {
            return false;
        };
        let Some(total) = encoded.checked_add(64) else {
            return false;
        };
        let Some(next) = self.cached_image_bytes.checked_add(total) else {
            return false;
        };
        if next > self.max_cached_image_bytes {
            self.cache_exhausted = true;
            return false;
        }
        self.cached_image_bytes = next;
        true
    }

    fn exhausted_placeholder(
        &mut self,
        url: &str,
        warnings: &mut Vec<ImportWarning>,
        emit_warning: bool,
    ) -> EmbeddedImage {
        if emit_warning && !self.exhaustion_warning_emitted {
            warnings.push(ImportWarning::ImageUnavailable {
                url: url.to_string(),
            });
            self.exhaustion_warning_emitted = true;
        }
        EmbeddedImage::placeholder()
    }
}

#[derive(Default)]
pub(crate) struct ImageResourceCache {
    entries: HashMap<String, EmbeddedImage>,
}

pub fn resolve_url(base: Option<&str>, href: &str) -> Option<String> {
    let href = href.trim();
    if let Ok(url) = Url::parse(href) {
        return matches!(url.scheme(), "http" | "https" | "data").then(|| url.to_string());
    }
    let base = Url::parse(base?.trim()).ok()?;
    if !matches!(base.scheme(), "http" | "https") {
        return None;
    }
    base.join(href).ok().map(Into::into)
}

/// Resolves a resource URL while keeping virtual HTML projects self-contained.
///
/// Regular web imports may reference any HTTP(S) origin. The synthetic
/// `openpencil.local` origin is different: it represents files supplied by the
/// user, so an absolute `<base>`, stylesheet, import, or image URL must not turn
/// that virtual file lookup into an external request.
pub(crate) fn resolve_resource_url(base: Option<&str>, href: &str) -> Option<String> {
    let resolved = resolve_url(base, href)?;
    resource_url_allowed(base, &resolved).then_some(resolved)
}

pub(crate) fn select_document_base(
    document_url: Option<&str>,
    candidates: &[String],
    warnings: &mut Vec<ImportWarning>,
) -> Option<String> {
    for href in candidates {
        let Some(resolved) = resolve_url(document_url, href) else {
            warnings.push(ImportWarning::InvalidBaseHref {
                href: href.to_string(),
            });
            continue;
        };
        let Ok(parsed) = Url::parse(&resolved) else {
            continue;
        };
        if !matches!(parsed.scheme(), "http" | "https") {
            warnings.push(ImportWarning::InvalidBaseHref {
                href: href.to_string(),
            });
            continue;
        }
        if !resource_url_allowed(document_url, &resolved) {
            warnings.push(ImportWarning::BaseHrefOutsideOrigin {
                href: href.to_string(),
            });
            continue;
        }
        return Some(resolved);
    }
    document_url.map(str::to_string)
}

fn resource_url_allowed(base: Option<&str>, resolved: &str) -> bool {
    let Some(base) = base.and_then(|value| Url::parse(value).ok()) else {
        return true;
    };
    if !is_virtual_project_url(&base) {
        return true;
    }
    Url::parse(resolved)
        .ok()
        .is_some_and(|url| same_origin(&base, &url))
}

fn is_virtual_project_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("openpencil.local")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
        && right.username().is_empty()
        && right.password().is_none()
}

pub(crate) fn embed_images(
    nodes: &mut [PenNode],
    base_url: Option<&str>,
    fetcher: Option<&ResourceFetcher<'_>>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
    cache: &mut ImageResourceCache,
) -> usize {
    nodes
        .iter_mut()
        .map(|node| {
            embed_node_images(
                node,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                &mut cache.entries,
            )
        })
        .sum()
}

fn embed_node_images(
    node: &mut PenNode,
    base_url: Option<&str>,
    fetcher: Option<&ResourceFetcher<'_>>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
    cache: &mut HashMap<String, EmbeddedImage>,
) -> usize {
    let mut count = 0;
    let children = match node {
        PenNode::Frame(frame) => {
            count += embed_fills(
                &mut frame.container.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            frame.children.as_mut()
        }
        PenNode::Group(group) => {
            count += embed_fills(
                &mut group.container.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            group.children.as_mut()
        }
        PenNode::Rectangle(rectangle) => {
            count += embed_fills(
                &mut rectangle.container.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            rectangle.children.as_mut()
        }
        PenNode::Ellipse(ellipse) => {
            count += embed_fills(
                &mut ellipse.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            None
        }
        PenNode::Polygon(polygon) => {
            count += embed_fills(
                &mut polygon.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            None
        }
        PenNode::Path(path) => {
            count += embed_fills(
                &mut path.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            None
        }
        PenNode::Text(text) => {
            count += embed_fills(
                &mut text.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            None
        }
        PenNode::Image(image) => {
            // `data:` images were sized during DOM hydration and already own
            // their payload. Avoid a second decode and an unbounded URL copy.
            let embedded = (!is_data_url(image.src.as_str()))
                .then(|| {
                    embed_url(
                        image.src.as_str(),
                        base_url,
                        fetcher,
                        transform,
                        budget,
                        warnings,
                        cache,
                        true,
                    )
                })
                .flatten();
            if let Some(embedded) = embedded {
                if let Some(replacement) = embedded.replacement {
                    image.src = replacement;
                    count += 1;
                }
            }
            None
        }
        PenNode::Ref(reference) => reference.children.as_mut(),
        PenNode::Tabs(tabs) => tabs.children.as_mut(),
        _ => None,
    };
    if let Some(children) = children {
        for child in children {
            count +=
                embed_node_images(child, base_url, fetcher, transform, budget, warnings, cache);
        }
    }
    count
}

fn embed_fills(
    fills: &mut Option<Vec<PenFill>>,
    base_url: Option<&str>,
    fetcher: Option<&ResourceFetcher<'_>>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
    cache: &mut HashMap<String, EmbeddedImage>,
) -> usize {
    let mut count = 0;
    if let Some(fills) = fills {
        for fill in fills {
            if let PenFill::Image(image) = fill {
                if is_data_url(image.url.as_str()) {
                    continue;
                }
                if let Some(embedded) = embed_url(
                    image.url.as_str(),
                    base_url,
                    fetcher,
                    transform,
                    budget,
                    warnings,
                    cache,
                    true,
                ) {
                    if let Some(replacement) = embedded.replacement {
                        image.url = replacement;
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

#[allow(clippy::too_many_arguments)]
fn embed_url(
    url: &str,
    base_url: Option<&str>,
    fetcher: Option<&ResourceFetcher<'_>>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
    cache: &mut HashMap<String, EmbeddedImage>,
    emit_warnings: bool,
) -> Option<EmbeddedImage> {
    if is_data_url(url) {
        let metadata = data_url_image_metadata(url);
        return Some(EmbeddedImage {
            replacement: None,
            dimensions: metadata.map(|metadata| metadata.dimensions),
            preferred_ratio: metadata.and_then(|metadata| metadata.preferred_ratio),
            warning: None,
            pending: None,
        });
    }
    if url.trim().len() > MAX_RESOURCE_URL_BYTES {
        return Some(budget.exhausted_placeholder(
            OVERSIZED_IMAGE_URL_LABEL,
            warnings,
            emit_warnings,
        ));
    }
    let fetcher = fetcher?;
    let resolved = match resolve_resource_url(base_url, url) {
        Some(resolved) => resolved,
        None if base_url.is_some_and(is_virtual_project_base) => {
            let cache_key = format!("blocked:{url}");
            if let Some(cached) = cached_image(
                cache,
                &cache_key,
                transform,
                budget,
                warnings,
                emit_warnings,
            ) {
                return Some(cached);
            }
            let allowed = if emit_warnings {
                budget.take_image()
            } else {
                budget.take_prefetch()
            };
            if !allowed {
                return Some(budget.exhausted_placeholder(url, warnings, emit_warnings));
            }
            let mut embedded = EmbeddedImage::unavailable(ImportWarning::ImageOutsideOrigin {
                url: url.to_string(),
            });
            embedded.emit_warning(warnings, emit_warnings);
            cache.insert(cache_key, embedded.clone());
            return Some(embedded);
        }
        None => url.to_string(),
    };
    if let Some(cached) = cached_image(cache, &resolved, transform, budget, warnings, emit_warnings)
    {
        return Some(cached);
    }
    let (mut embedded, cacheable) = if emit_warnings {
        fetch_final_image(&resolved, fetcher, transform, budget, warnings)?
    } else {
        prefetch_image(&resolved, fetcher, budget)?
    };
    embedded.emit_warning(warnings, emit_warnings);
    if cacheable {
        cache.insert(resolved, embedded.clone());
    }
    Some(embedded)
}

fn prefetch_image(
    resolved: &str,
    fetcher: &ResourceFetcher<'_>,
    budget: &mut ResourceBudget,
) -> Option<(EmbeddedImage, bool)> {
    if !budget.take_prefetch() {
        return Some((EmbeddedImage::placeholder(), false));
    }
    let bytes = match fetcher(resolved) {
        Some(bytes) if budget.source_allowed(bytes.len()) => bytes,
        Some(_) | None => {
            return Some((
                EmbeddedImage::unavailable(ImportWarning::ImageUnavailable {
                    url: resolved.to_string(),
                }),
                true,
            ));
        }
    };
    let metadata = browser_image_metadata(&bytes);
    if !budget.reserve_pending(bytes.len()) {
        return Some((
            EmbeddedImage {
                replacement: None,
                dimensions: metadata.map(|metadata| metadata.dimensions),
                preferred_ratio: metadata.and_then(|metadata| metadata.preferred_ratio),
                warning: None,
                pending: None,
            },
            false,
        ));
    }
    Some((
        EmbeddedImage {
            replacement: None,
            dimensions: metadata.map(|metadata| metadata.dimensions),
            preferred_ratio: metadata.and_then(|metadata| metadata.preferred_ratio),
            warning: None,
            pending: Some(Arc::from(bytes)),
        },
        true,
    ))
}

fn fetch_final_image(
    resolved: &str,
    fetcher: &ResourceFetcher<'_>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
) -> Option<(EmbeddedImage, bool)> {
    if !budget.take_image() {
        return Some((
            budget.exhausted_placeholder(resolved, warnings, true),
            false,
        ));
    }
    let bytes = match fetcher(resolved) {
        Some(bytes) if budget.source_allowed(bytes.len()) => bytes,
        Some(_) | None => {
            return Some((
                EmbeddedImage::unavailable(ImportWarning::ImageUnavailable {
                    url: resolved.to_string(),
                }),
                true,
            ));
        }
    };
    let metadata = browser_image_metadata(&bytes);
    Some((
        finalize_image_bytes(resolved, &bytes, metadata, transform, budget),
        true,
    ))
}

fn finalize_image_bytes(
    resolved: &str,
    bytes: &[u8],
    metadata: Option<BrowserImageMetadata>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
) -> EmbeddedImage {
    let transformed = transform.and_then(|rewrite| rewrite(bytes));
    let payload = transformed.as_deref().unwrap_or(bytes);
    if budget.reserve_data_url(payload) {
        EmbeddedImage {
            replacement: Some(blob_to_data_url(payload).into()),
            dimensions: metadata.map(|metadata| metadata.dimensions),
            preferred_ratio: metadata.and_then(|metadata| metadata.preferred_ratio),
            warning: None,
            pending: None,
        }
    } else {
        EmbeddedImage::unavailable(ImportWarning::ImageUnavailable {
            url: resolved.to_string(),
        })
    }
}

#[derive(Clone)]
struct EmbeddedImage {
    replacement: Option<ImageSrc>,
    dimensions: Option<(f64, f64)>,
    preferred_ratio: Option<f64>,
    warning: Option<ImportWarning>,
    pending: Option<Arc<[u8]>>,
}

impl EmbeddedImage {
    fn placeholder() -> Self {
        Self {
            replacement: Some(ImageSrc::from(PLACEHOLDER_GRAY_PNG)),
            dimensions: None,
            preferred_ratio: None,
            warning: None,
            pending: None,
        }
    }

    fn unavailable(warning: ImportWarning) -> Self {
        Self {
            warning: Some(warning),
            ..Self::placeholder()
        }
    }

    fn emit_warning(&mut self, warnings: &mut Vec<ImportWarning>, emit: bool) {
        if emit {
            if let Some(warning) = self.warning.take() {
                warnings.push(warning);
            }
        }
    }
}

fn cached_image(
    cache: &mut HashMap<String, EmbeddedImage>,
    key: &str,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
    emit_warnings: bool,
) -> Option<EmbeddedImage> {
    let cached = cache.get_mut(key)?;
    if emit_warnings {
        if let Some(pending) = cached.pending.take() {
            budget.release_pending(pending.len());
            if budget.take_image() {
                let metadata = cached.dimensions.map(|dimensions| BrowserImageMetadata {
                    dimensions,
                    preferred_ratio: cached.preferred_ratio,
                });
                *cached = finalize_image_bytes(key, &pending, metadata, transform, budget);
            } else {
                *cached = budget.exhausted_placeholder(key, warnings, true);
            }
        }
    }
    cached.emit_warning(warnings, emit_warnings);
    Some(cached.clone())
}

fn is_virtual_project_base(base: &str) -> bool {
    Url::parse(base)
        .ok()
        .as_ref()
        .is_some_and(is_virtual_project_url)
}

fn blob_to_data_url(bytes: &[u8]) -> String {
    let mime = image_mime(bytes);
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:{mime};base64,{encoded}")
}

fn image_mime(bytes: &[u8]) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [0xff, 0xd8, ..] => "image/jpeg",
        [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => "image/gif",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "image/webp",
        [b'B', b'M', ..] => "image/bmp",
        [0, 0, 1 | 2, 0, ..] => "image/x-icon",
        [_, _, _, _, b'f', b't', b'y', b'p', b'a', b'v', b'i', b'f' | b's', ..] => "image/avif",
        _ if looks_like_svg(bytes) => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    op_util::encoded_svg_intrinsic_metadata(bytes).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_url_forms() {
        assert_eq!(
            resolve_url(Some("https://a.dev/x/y.html"), "s.css").as_deref(),
            Some("https://a.dev/x/s.css")
        );
        assert_eq!(
            resolve_url(Some("https://a.dev/x/y.html"), "/s.css").as_deref(),
            Some("https://a.dev/s.css")
        );
        assert_eq!(
            resolve_url(Some("https://a.dev/x/y.html"), "../s.css").as_deref(),
            Some("https://a.dev/s.css")
        );
        assert_eq!(
            resolve_url(Some("https://a.dev/x/"), "//cdn.b.io/s.css").as_deref(),
            Some("https://cdn.b.io/s.css")
        );
        assert_eq!(
            resolve_url(None, "https://c.io/s.css").as_deref(),
            Some("https://c.io/s.css")
        );
        assert!(resolve_url(None, "s.css").is_none());
    }

    #[test]
    fn virtual_project_resource_resolution_cannot_change_origin() {
        let base = Some("https://openpencil.local/site/pages/index.html");
        assert_eq!(
            resolve_resource_url(base, "../../assets/site.css").as_deref(),
            Some("https://openpencil.local/assets/site.css")
        );
        assert!(resolve_resource_url(base, "https://example.test/site.css").is_none());
        assert!(resolve_resource_url(base, "//example.test/site.css").is_none());
    }

    #[test]
    fn document_base_uses_first_valid_candidate_and_confines_projects() {
        let mut warnings = Vec::new();
        let selected = select_document_base(
            Some("https://openpencil.local/site/index.html"),
            &[
                "javascript:alert(1)".into(),
                "https://example.test/".into(),
                "assets/".into(),
                "/ignored/".into(),
            ],
            &mut warnings,
        );
        assert_eq!(
            selected.as_deref(),
            Some("https://openpencil.local/site/assets/")
        );
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn sniffs_common_image_formats_instead_of_mislabeling_them_as_png() {
        let cases: &[(&[u8], &str)] = &[
            (b"\x89PNG\r\n\x1a\n", "image/png"),
            (b"\xff\xd8\xff", "image/jpeg"),
            (b"GIF89a", "image/gif"),
            (b"RIFF\0\0\0\0WEBP", "image/webp"),
            (b"BM\0\0", "image/bmp"),
            (b"\0\0\x01\0", "image/x-icon"),
            (b"\0\0\0\x18ftypavif", "image/avif"),
            (b"\0\0\0\x18ftypavis", "image/avif"),
            (
                b" \n<svg xmlns='http://www.w3.org/2000/svg'/>",
                "image/svg+xml",
            ),
            (b"not an image", "application/octet-stream"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(image_mime(bytes), *expected, "bytes: {bytes:?}");
            assert!(blob_to_data_url(bytes).starts_with(&format!("data:{expected};base64,")));
        }
    }
}

#[cfg(test)]
#[path = "resources_image_tests.rs"]
mod image_tests;

#[cfg(test)]
#[path = "resources_image_budget_tests.rs"]
mod image_budget_tests;

#[cfg(test)]
#[path = "resources_image_edge_tests.rs"]
mod image_edge_tests;
