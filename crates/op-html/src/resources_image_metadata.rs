//! Bounded image metadata probing before structured DOM mapping.

use base64::Engine as _;
use op_util::{encoded_image_dimensions, encoded_svg_intrinsic_metadata, MAX_INTRINSIC_IMAGE_EDGE};

use crate::css::cascade::{compute_style_for_viewport, ComputedStyle, StyleRule};
use crate::dom::{DomElement, DomNode};
use crate::import_warning::ImportWarning;
use crate::mapper::MapCtx;
use crate::HtmlImportOptions;

use super::{embed_url, ImageResourceCache, ImageTransform, ResourceBudget, ResourceFetcher};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserImageMetadata {
    pub dimensions: (f64, f64),
    pub preferred_ratio: Option<f64>,
}

const INTRINSIC_WIDTH_ATTR: &str = "\0op-html-intrinsic-width";
const INTRINSIC_HEIGHT_ATTR: &str = "\0op-html-intrinsic-height";
const INTRINSIC_RATIO_ATTR: &str = "\0op-html-intrinsic-ratio";
// Raster dimensions occur near the header. JPEG permits metadata before SOF,
// and an SVG root can carry declarations, so retain a bounded generous head.
const MAX_IMAGE_METADATA_BYTES: usize = 1024 * 1024;
const MAX_DATA_URL_HEADER_BYTES: usize = 1024;
pub(super) const MAX_IMAGE_METADATA_BASE64_CHARS: usize = 4 * MAX_IMAGE_METADATA_BYTES.div_ceil(3);

pub(crate) fn browser_image_metadata(bytes: &[u8]) -> Option<BrowserImageMetadata> {
    if let Some(svg) = encoded_svg_intrinsic_metadata(bytes) {
        let dimensions = match (svg.width, svg.height, svg.view_box_ratio) {
            (Some(width), Some(height), _) => Some((width, height)),
            (Some(width), None, Some(ratio)) => Some((width, width / ratio)),
            (None, Some(height), Some(ratio)) => Some((height * ratio, height)),
            (None, None, Some(ratio)) if ratio >= 2.0 => Some((300.0, 300.0 / ratio)),
            (None, None, Some(ratio)) => Some((150.0 * ratio, 150.0)),
            (None, None, None) => Some((300.0, 150.0)),
            // One intrinsic axis without a preferred ratio uses the matching
            // dimension of the default object size for the other axis.
            (Some(width), None, None) => Some((width, 150.0)),
            (None, Some(height), None) => Some((300.0, height)),
        };
        let preferred_ratio = svg
            .view_box_ratio
            .or_else(|| match (svg.width, svg.height) {
                (Some(width), Some(height)) => Some(width / height),
                _ => None,
            });
        return dimensions
            .filter(|(width, height)| valid_layout_dimensions(*width, *height))
            .map(|dimensions| BrowserImageMetadata {
                dimensions,
                preferred_ratio,
            });
    }
    if let Some((width, height)) = encoded_image_dimensions(bytes) {
        return Some(BrowserImageMetadata {
            dimensions: (f64::from(width), f64::from(height)),
            preferred_ratio: Some(f64::from(width) / f64::from(height)),
        });
    }
    None
}

fn valid_layout_dimensions(width: f64, height: f64) -> bool {
    width.is_finite()
        && height.is_finite()
        && width > 0.0
        && height > 0.0
        && width <= f64::from(MAX_INTRINSIC_IMAGE_EDGE)
        && height <= f64::from(MAX_INTRINSIC_IMAGE_EDGE)
}

pub(super) fn data_url_image_metadata(url: &str) -> Option<BrowserImageMetadata> {
    let url = url.trim_start_matches(|character: char| character.is_ascii_whitespace());
    let data = url.get(5..).filter(|_| is_data_url(url))?;
    let comma = data
        .as_bytes()
        .iter()
        .take(MAX_DATA_URL_HEADER_BYTES + 1)
        .position(|byte| *byte == b',')?;
    let metadata = data.get(..comma)?;
    let payload = data.get(comma + 1..)?;
    let mut fields = metadata.split(';');
    let media_type = fields.next()?.trim();
    if media_type.len() > 128
        || !media_type.is_ascii()
        || !media_type
            .get(..6)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("image/"))
    {
        return None;
    }
    let base64 = fields.any(|field| field.trim().eq_ignore_ascii_case("base64"));
    let bytes = if base64 {
        let compact: Vec<u8> = payload
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .take(MAX_IMAGE_METADATA_BASE64_CHARS)
            .collect();
        base64::engine::general_purpose::STANDARD
            .decode(&compact)
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&compact))
            .ok()?
    } else if media_type.eq_ignore_ascii_case("image/svg+xml") {
        percent_decode_metadata(payload)?
    } else {
        return None;
    };
    if !declared_mime_matches(media_type, super::image_mime(&bytes)) {
        return None;
    }
    browser_image_metadata(&bytes)
}

#[cfg(test)]
pub(super) fn data_url_image_dimensions(url: &str) -> Option<(f64, f64)> {
    data_url_image_metadata(url).map(|metadata| metadata.dimensions)
}

fn declared_mime_matches(declared: &str, actual: &str) -> bool {
    declared.eq_ignore_ascii_case(actual)
        || (actual == "image/png" && declared.eq_ignore_ascii_case("image/apng"))
        || (actual == "image/jpeg"
            && (declared.eq_ignore_ascii_case("image/jpg")
                || declared.eq_ignore_ascii_case("image/pjpeg")))
}

pub(super) fn is_data_url(url: &str) -> bool {
    url.trim_start_matches(|character: char| character.is_ascii_whitespace())
        .get(..5)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:"))
}

pub(crate) fn normalize_image_source(mut source: String) -> String {
    let trimmed = source
        .trim_start_matches(|character: char| character.is_ascii_whitespace())
        .len();
    let leading = source.len().saturating_sub(trimmed);
    if leading > 0 {
        source.drain(..leading);
    }
    if is_data_url(&source) && source.get(..5) != Some("data:") {
        source.replace_range(..5, "data:");
    }
    source
}

fn percent_decode_metadata(payload: &str) -> Option<Vec<u8>> {
    let bytes = payload.as_bytes();
    let mut output = Vec::with_capacity(bytes.len().min(MAX_IMAGE_METADATA_BYTES));
    let mut index = 0;
    while index < bytes.len() && output.len() < MAX_IMAGE_METADATA_BYTES {
        if bytes[index] == b'%' {
            let high = hex_nibble(*bytes.get(index + 1)?)?;
            let low = hex_nibble(*bytes.get(index + 2)?)?;
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    Some(output)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Fetch selected `<img>` sources before mapping so intrinsic dimensions are
/// present when flex-wrap decides row boundaries. Resource warnings remain on
/// cache entries until final tree-order embedding consumes them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prefetch_image_metadata(
    root: &mut DomElement,
    options: &HtmlImportOptions,
    rules: &[StyleRule],
    base_url: Option<&str>,
    fetcher: Option<&ResourceFetcher<'_>>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
    cache: &mut ImageResourceCache,
) {
    let mut context = MapCtx {
        opts: options,
        rules,
        warnings: Vec::new(),
        warned: Default::default(),
        next_id: 0,
        node_count: 0,
        containing_width: options.viewport_width,
        containing_height: options.viewport_height(),
        containing_width_is_definite: true,
        positioned_width: options.viewport_width,
        positioned_height: options.viewport_height(),
        auto_margin_handled_by_parent: false,
        pending_base_outcome: Default::default(),
    };
    let mut metadata = Vec::new();
    collect_image_metadata(
        root,
        &mut Vec::new(),
        &mut context,
        None,
        false,
        base_url,
        fetcher,
        transform,
        budget,
        warnings,
        &mut cache.entries,
        &mut metadata,
    );
    apply_image_metadata(root, &mut metadata.into_iter());
}

#[allow(clippy::too_many_arguments)]
fn collect_image_metadata<'a>(
    element: &'a DomElement,
    path: &mut Vec<&'a DomElement>,
    context: &mut MapCtx<'_>,
    parent_style: Option<&ComputedStyle>,
    ancestor_hidden: bool,
    base_url: Option<&str>,
    fetcher: Option<&ResourceFetcher<'_>>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
    cache: &mut std::collections::HashMap<String, super::EmbeddedImage>,
    metadata: &mut Vec<Option<BrowserImageMetadata>>,
) {
    path.push(element);
    let style = (!ancestor_hidden).then(|| {
        compute_style_for_viewport(
            path,
            context.rules,
            parent_style,
            context.opts.base_font_size,
            context.opts.viewport_width,
            context.opts.viewport_height(),
        )
    });
    let hidden = ancestor_hidden
        || style
            .as_ref()
            .is_some_and(|style| style.get("display") == Some("none"));
    if element.tag == "img" {
        let source = (!hidden)
            .then(|| crate::srcset::resolve_image_candidate(context, path, element))
            .filter(|source| !source.url.trim().is_empty());
        let dimensions = source.and_then(|source| {
            let density = source.density;
            embed_url(
                &source.url,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
                false,
            )
            .and_then(|image| {
                image.dimensions.map(|dimensions| BrowserImageMetadata {
                    dimensions,
                    preferred_ratio: image.preferred_ratio,
                })
            })
            .and_then(|metadata| {
                let width = metadata.dimensions.0 / density;
                let height = metadata.dimensions.1 / density;
                valid_layout_dimensions(width, height)
                    .then_some((width, height))
                    .map(|dimensions| BrowserImageMetadata {
                        dimensions,
                        preferred_ratio: metadata.preferred_ratio,
                    })
            })
        });
        metadata.push(dimensions);
    }
    if crate::special::is_special_leaf_tag(&element.tag) {
        path.pop();
        return;
    }
    for child in &element.children {
        if let DomNode::Element(child) = child {
            collect_image_metadata(
                child,
                path,
                context,
                style.as_ref(),
                hidden,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
                metadata,
            );
        }
    }
    path.pop();
}

fn apply_image_metadata(
    element: &mut DomElement,
    metadata: &mut impl Iterator<Item = Option<BrowserImageMetadata>>,
) {
    if element.tag == "img" {
        element.attrs.retain(|(name, _)| {
            name != INTRINSIC_WIDTH_ATTR
                && name != INTRINSIC_HEIGHT_ATTR
                && name != INTRINSIC_RATIO_ATTR
        });
        if let Some(metadata) = metadata.next().flatten() {
            let (width, height) = metadata.dimensions;
            element
                .attrs
                .push((INTRINSIC_WIDTH_ATTR.to_string(), width.to_string()));
            element
                .attrs
                .push((INTRINSIC_HEIGHT_ATTR.to_string(), height.to_string()));
            element.attrs.push((
                INTRINSIC_RATIO_ATTR.to_string(),
                metadata
                    .preferred_ratio
                    .map(|ratio| ratio.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ));
        }
    }
    if crate::special::is_special_leaf_tag(&element.tag) {
        return;
    }
    for child in &mut element.children {
        if let DomNode::Element(child) = child {
            apply_image_metadata(child, metadata);
        }
    }
}

pub(crate) fn element_intrinsic_metadata(element: &DomElement) -> Option<BrowserImageMetadata> {
    Some(BrowserImageMetadata {
        dimensions: (
            element.attr(INTRINSIC_WIDTH_ATTR)?.parse().ok()?,
            element.attr(INTRINSIC_HEIGHT_ATTR)?.parse().ok()?,
        ),
        preferred_ratio: match element.attr(INTRINSIC_RATIO_ATTR)? {
            "none" => None,
            value => value.parse().ok(),
        },
    })
}
