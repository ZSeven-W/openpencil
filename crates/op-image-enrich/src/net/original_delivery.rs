//! Upgrade only the selected artwork; previews remain available on every failure.

use std::collections::HashSet;
use std::future::Future;
use std::io::Cursor;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::Engine as _;

use super::fetch::claim_unused_image_src;
use super::resolve::ResolvedImage;

const DELIVERY_MARGIN: Duration = Duration::from_millis(250);
const ORIGINAL_FETCH_BUDGET: Duration = Duration::from_secs(8);

pub(super) async fn upgrade_selected_image(
    selected: ResolvedImage,
    used_urls: &Mutex<HashSet<String>>,
    deadline: Option<Instant>,
) -> ResolvedImage {
    upgrade_selected_with(selected, used_urls, deadline, |url, deadline| async move {
        super::providers::fetch_original_image_data_url(&url, Some(deadline)).await
    })
    .await
}

async fn upgrade_selected_with<F, Fut>(
    mut selected: ResolvedImage,
    used_urls: &Mutex<HashSet<String>>,
    deadline: Option<Instant>,
    fetch: F,
) -> ResolvedImage
where
    F: FnOnce(String, Instant) -> Fut,
    Fut: Future<Output = Option<String>>,
{
    let Some(original) = selected.original.as_ref() else {
        trace_retained(&selected, "no-original-source");
        return selected;
    };
    let Some(preview_size) = raster_dimensions(&selected.src) else {
        trace_retained(&selected, "preview-dimensions-unavailable");
        return selected;
    };
    if matches!((original.width, original.height), (Some(w), Some(h)) if w <= preview_size.0 && h <= preview_size.1)
    {
        trace_retained(&selected, "catalogue-source-not-larger");
        return selected;
    }
    let now = Instant::now();
    let deadline = deadline.map_or(now + ORIGINAL_FETCH_BUDGET, |end| {
        (now + ORIGINAL_FETCH_BUDGET).min(end.checked_sub(DELIVERY_MARGIN).unwrap_or(end))
    });
    let remaining = deadline.saturating_duration_since(now);
    if remaining.is_zero() {
        trace_retained(&selected, "deadline");
        return selected;
    }
    let fetched = tokio::time::timeout(remaining, fetch(original.url.clone(), deadline)).await;
    let Ok(Some(src)) = fetched else {
        trace_retained(
            &selected,
            if Instant::now() >= deadline {
                "deadline"
            } else {
                "original-unavailable"
            },
        );
        return selected;
    };
    let Some((width, height)) = raster_dimensions(&src) else {
        trace_retained(&selected, "original-dimensions-unavailable");
        return selected;
    };
    if width.max(height) > 2048
        || width < preview_size.0
        || height < preview_size.1
        || (width, height) == preview_size
    {
        trace_retained(&selected, "original-not-larger-or-over-limit");
        return selected;
    }
    if Instant::now() >= deadline {
        trace_retained(&selected, "deadline");
        return selected;
    }
    let preview = selected.src.clone();
    let candidate = src.clone();
    let consistent = tokio::time::timeout_at(
        deadline.into(),
        tokio::task::spawn_blocking(move || same_artwork(&preview, &candidate)),
    )
    .await;
    if !matches!(consistent, Ok(Ok(true))) {
        trace_retained(
            &selected,
            if Instant::now() >= deadline {
                "deadline"
            } else {
                "original-composition-unverified"
            },
        );
        return selected;
    }
    if !claim_unused_image_src(used_urls, &src) {
        trace_retained(&selected, "original-already-used");
        return selected;
    }
    // Keep the preview's content claim and provider identity too: another
    // slot must not reuse this artwork through its lower-resolution variant.
    eprintln!(
        "[ENRICH] original-delivered: {}x{} -> {width}x{height} {}",
        preview_size.0,
        preview_size.1,
        selected.describe()
    );
    selected.src = src;
    selected
}

/// A larger file is not necessarily the image the relevance judge approved:
/// catalogues can expose a crop as the thumbnail. This conservative pixel gate
/// keeps the preview whenever matching content/composition cannot be verified.
fn same_artwork(preview: &str, original: &str) -> bool {
    let Some(preview) =
        raster_bytes(preview).and_then(|bytes| image::load_from_memory(&bytes).ok())
    else {
        return false;
    };
    let Some(original) =
        raster_bytes(original).and_then(|bytes| image::load_from_memory(&bytes).ok())
    else {
        return false;
    };
    let cross_a = u64::from(preview.width()) * u64::from(original.height());
    let cross_b = u64::from(original.width()) * u64::from(preview.height());
    if cross_a.min(cross_b) == 0 || cross_a.abs_diff(cross_b) * 100 > cross_a.max(cross_b) {
        return false;
    }
    let preview = comparison_raster(preview);
    let original = comparison_raster(original);
    let mut color_error = 0u32;
    let mut alpha_error = 0u32;
    let mut changed_pixels = 0usize;
    for (a, b) in preview.pixels().zip(original.pixels()) {
        let errors = std::array::from_fn::<_, 4, _>(|channel| a[channel].abs_diff(b[channel]));
        color_error += errors[..3]
            .iter()
            .map(|error| u32::from(*error))
            .sum::<u32>();
        alpha_error += u32::from(errors[3]);
        changed_pixels += usize::from(errors.into_iter().any(|error| error > 32));
    }
    // Permit small resizing/JPEG differences, but reject both broad color
    // changes and localized subject/composition changes. Alpha is independent
    // of the white-matted RGB channels, so transparency cannot silently change.
    color_error <= 32 * 32 * 3 * 8
        && alpha_error <= 32 * 32 * 8
        && changed_pixels * 100 <= 32 * 32 * 2
}

fn comparison_raster(image: image::DynamicImage) -> image::RgbaImage {
    let mut pixels = image.to_rgba8();
    for pixel in pixels.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in 0..3 {
            pixel[channel] =
                ((u16::from(pixel[channel]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
        }
    }
    image::imageops::resize(&pixels, 32, 32, image::imageops::FilterType::Triangle)
}

fn trace_retained(selected: &ResolvedImage, reason: &str) {
    eprintln!(
        "[ENRICH] original-retained: reason={reason} {}",
        selected.describe()
    );
}

fn raster_dimensions(src: &str) -> Option<(u32, u32)> {
    image::ImageReader::new(Cursor::new(raster_bytes(src)?))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

fn raster_bytes(src: &str) -> Option<Vec<u8>> {
    let encoded = src.strip_prefix("data:image/")?.split_once(";base64,")?.1;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()
}

#[cfg(test)]
#[path = "original_delivery_tests.rs"]
mod tests;
