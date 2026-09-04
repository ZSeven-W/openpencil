use crate::layout_scene::{stable_image_source_id, SceneNode};
use crate::widgets::canvas_viewport_overlay::scaled_non_uniform_corner_radii;
use crate::widgets::PaintCx;
use crate::Color;
use crate::Rect;
use std::sync::{Arc, Mutex, OnceLock};

mod decode_registry;
mod picture_glyph;
mod svg_raster;
#[cfg(test)]
mod test_support;

use picture_glyph::paint_picture_glyph;
#[cfg(test)]
use test_support::{
    clear_data_url_cache_for_tests, clear_remote_registry_for_tests, data_url_cache_len_for_tests,
    lock_statics,
};

#[cfg(test)]
pub(crate) use decode_registry::lock_decode_registry_for_tests;
pub use decode_registry::{
    has_in_flight_decodes, has_pending_decodes, mark_decode_done, mark_decode_failed,
    note_pending_decode, pending_decode_count, take_pending_decodes, PendingDecode,
};

/// Byte budget for cached encoded-image payloads. The cache must hold
/// the working set of an image-heavy document (a Figma import easily
/// carries hundreds of bitmaps) — a small entry cap caused every frame
/// to re-run the base64 decode for whatever fell off the end. Budget
/// by bytes instead: wasm gets a smaller budget to respect the 32-bit
/// heap.
#[cfg(target_arch = "wasm32")]
const DATA_URL_CACHE_BYTE_BUDGET: usize = 96 * 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const DATA_URL_CACHE_BYTE_BUDGET: usize = 256 * 1024 * 1024;
/// Safety cap on entry count — bounds accumulation of tiny payloads.
const DATA_URL_CACHE_MAX_ENTRIES: usize = 4096;

struct DataUrlCache {
    entries: std::collections::HashMap<u64, (Arc<[u8]>, u64)>,
    /// Sum of payload lengths across `entries`.
    bytes: usize,
    /// Monotonic use counter driving LRU eviction.
    tick: u64,
}

impl DataUrlCache {
    fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            bytes: 0,
            tick: 0,
        }
    }

    fn get(&mut self, id: u64) -> Option<Arc<[u8]>> {
        self.tick += 1;
        let tick = self.tick;
        let (bytes, last_used) = self.entries.get_mut(&id)?;
        *last_used = tick;
        Some(Arc::clone(bytes))
    }

    /// Presence check that does not refresh the entry's LRU position.
    fn contains(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    fn insert(&mut self, id: u64, bytes: Arc<[u8]>) {
        self.tick += 1;
        let tick = self.tick;
        if let Some((old, _)) = self.entries.insert(id, (bytes, tick)) {
            self.bytes = self.bytes.saturating_sub(old.len());
        }
        if let Some((fresh, _)) = self.entries.get(&id) {
            self.bytes += fresh.len();
        }
        self.evict_over(DATA_URL_CACHE_BYTE_BUDGET, DATA_URL_CACHE_MAX_ENTRIES);
    }

    /// Evict least-recently-used entries until the cache fits both
    /// `byte_budget` and `max_entries`. Separated from `insert` so
    /// tests can exercise eviction with small budgets.
    fn evict_over(&mut self, byte_budget: usize, max_entries: usize) {
        while self.bytes > byte_budget || self.entries.len() > max_entries {
            let Some((&oldest, _)) = self
                .entries
                .iter()
                .min_by_key(|(_, (_, last_used))| *last_used)
            else {
                break;
            };
            if let Some((evicted, _)) = self.entries.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(evicted.len());
            }
        }
    }
}

static DATA_URL_CACHE: OnceLock<Mutex<DataUrlCache>> = OnceLock::new();

fn data_url_cache() -> &'static Mutex<DataUrlCache> {
    DATA_URL_CACHE.get_or_init(|| Mutex::new(DataUrlCache::new()))
}

/// Most remote misses a single frame can queue before the rest wait
/// for the next paint (the host drains a few per frame anyway).
const REMOTE_MISS_QUEUE_CAP: usize = 32;
/// Negative-cache bound — failed source ids beyond this evict oldest
/// (an evicted failure may refetch once; it just fails again).
const REMOTE_FAILED_CAP: usize = 256;

/// Paint-time registry for remote (`http(s)`) image sources. The
/// platform-free painter can only RECORD a cache miss here; a host
/// with network access (desktop) drains [`take_remote_image_requests`]
/// per frame, fetches, and stores the bytes back into the shared
/// data-url cache via [`store_remote_image_bytes`]. Hosts without a
/// fetch path (web, for now) simply never drain — the bounded queue
/// caps memory and the placeholder keeps painting.
#[derive(Default)]
struct RemoteImageRegistry {
    /// Misses recorded by paint, in first-seen order, awaiting a host.
    pending: std::collections::VecDeque<(u64, String)>,
    /// Ids a host has taken (in-flight) — paint must not re-queue.
    requested: std::collections::HashSet<u64>,
    /// Ids whose fetch failed — permanent placeholder, never re-queued.
    failed: std::collections::HashSet<u64>,
    failed_order: std::collections::VecDeque<u64>,
}

static REMOTE_IMAGES: OnceLock<Mutex<RemoteImageRegistry>> = OnceLock::new();

fn remote_images() -> &'static Mutex<RemoteImageRegistry> {
    REMOTE_IMAGES.get_or_init(|| Mutex::new(RemoteImageRegistry::default()))
}

/// Record a paint-time miss for a remote image source. Deduplicated
/// against already-queued / in-flight / failed ids, and bounded — a
/// dropped miss is simply re-noted on a later paint.
pub fn note_remote_image_miss(id: u64, url: &str) {
    let Ok(mut reg) = remote_images().lock() else {
        return;
    };
    if reg.failed.contains(&id)
        || reg.requested.contains(&id)
        || reg.pending.iter().any(|(queued, _)| *queued == id)
        || reg.pending.len() >= REMOTE_MISS_QUEUE_CAP
    {
        return;
    }
    reg.pending.push_back((id, url.to_string()));
}

/// Host-side drain: pop up to `max` recorded misses and mark them
/// in-flight so paint stops re-queuing them while the fetch runs.
pub fn take_remote_image_requests(max: usize) -> Vec<(u64, String)> {
    let Ok(mut reg) = remote_images().lock() else {
        return Vec::new();
    };
    let take = max.min(reg.pending.len());
    let mut out = Vec::with_capacity(take);
    for _ in 0..take {
        let Some((id, url)) = reg.pending.pop_front() else {
            break;
        };
        reg.requested.insert(id);
        out.push((id, url));
    }
    out
}

/// Host-side store: put fetched image bytes into the SAME cache the
/// data-url decode path uses, keyed by the source id — the next paint
/// hits the cache and draws the bitmap. Empty bytes are treated as a
/// failed fetch. Clears the in-flight mark so a later cache eviction
/// re-queues (and refetches) the source instead of sticking on the
/// placeholder forever.
pub fn store_remote_image_bytes(id: u64, bytes: Vec<u8>) {
    if bytes.is_empty() {
        mark_remote_image_failed(id);
        return;
    }
    // A remote `.svg` decodes in neither skia nor CanvasKit — rasterize at
    // the seam so the cache only ever holds bitmap codecs.
    let bytes = svg_raster::ensure_raster_bytes(Arc::from(bytes.into_boxed_slice()));
    if let Ok(mut cache) = data_url_cache().lock() {
        cache.insert(id, bytes);
    }
    if let Ok(mut reg) = remote_images().lock() {
        reg.requested.remove(&id);
    }
}

/// Host-side negative cache: a failed fetch keeps the placeholder
/// permanently — paint never re-queues a failed id, so a dead URL
/// doesn't refetch every frame.
pub fn mark_remote_image_failed(id: u64) {
    let Ok(mut reg) = remote_images().lock() else {
        return;
    };
    reg.requested.remove(&id);
    if reg.failed.insert(id) {
        reg.failed_order.push_back(id);
        while reg.failed.len() > REMOTE_FAILED_CAP {
            match reg.failed_order.pop_front() {
                Some(oldest) => {
                    reg.failed.remove(&oldest);
                }
                None => break,
            }
        }
    }
}

/// Whether the shared byte cache holds an entry for `id` — lets hosts
/// (and their tests) verify a stored fetch result without re-running
/// a paint pass.
pub fn has_cached_image_bytes(id: u64) -> bool {
    data_url_cache()
        .lock()
        .map(|cache| cache.contains(id))
        .unwrap_or(false)
}

pub fn cached_bytes_for(id: u64) -> Option<Arc<[u8]>> {
    data_url_cache().lock().ok()?.get(id)
}

/// Whether paint has recorded remote misses no host has taken yet —
/// the desktop host uses this to keep its event loop waking until the
/// queue is drained.
pub fn has_pending_remote_image_requests() -> bool {
    remote_images()
        .lock()
        .map(|reg| !reg.pending.is_empty())
        .unwrap_or(false)
}

fn is_remote_image_url(src: &str) -> bool {
    let lower = src.get(..8).unwrap_or(src).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Resolve the encoded image bytes for `src`: cache first (covers both
/// decoded data URLs and host-fetched remote bytes), then an inline
/// `data:` decode; a remote `http(s)` miss is recorded for the host
/// fetcher and paints the placeholder this frame.
pub(crate) fn image_source_bytes(src: &str, image_src_id: u64) -> Option<Arc<[u8]>> {
    let id = if image_src_id == 0 {
        stable_image_source_id(src)
    } else {
        image_src_id
    };
    if let Ok(mut cache) = data_url_cache().lock() {
        if let Some(bytes) = cache.get(id) {
            return Some(bytes);
        }
    }

    if is_remote_image_url(src) {
        note_remote_image_miss(id, src);
        return None;
    }
    // An SVG data URI decodes in neither skia nor CanvasKit — rasterize at
    // the seam so the cache only ever holds bitmap codecs.
    let decoded = svg_raster::ensure_raster_bytes(decode_data_url_bytes(src)?);
    if let Ok(mut cache) = data_url_cache().lock() {
        cache.insert(id, decoded.clone());
    }
    Some(decoded)
}

fn decode_data_url_bytes(src: &str) -> Option<Arc<[u8]>> {
    let after_scheme = src.strip_prefix("data:")?;
    let comma = after_scheme.find(',')?;
    let meta = &after_scheme[..comma];
    let payload = &after_scheme[comma + 1..];
    if !meta.contains(";base64") {
        // The only textual `data:` payload worth carrying is SVG markup —
        // CSS commonly embeds icons as `data:image/svg+xml,%3Csvg...`
        // percent-encoded rather than base64. Anything else stays undecoded
        // as before.
        if !meta.contains("image/svg") {
            return None;
        }
        let decoded = percent_decode(payload)?;
        return Some(Arc::from(decoded.into_boxed_slice()));
    }

    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    let decoded = if payload.bytes().any(|b| b.is_ascii_whitespace()) {
        let clean: Vec<u8> = payload
            .bytes()
            .filter(|b| !b.is_ascii_whitespace())
            .collect();
        B64.decode(clean.as_slice()).ok()?
    } else {
        B64.decode(payload.as_bytes()).ok()?
    };
    Some(Arc::from(decoded.into_boxed_slice()))
}

/// Decode `%XX` escapes; every other byte passes through verbatim (a data
/// URI does not use `+` for space).
fn percent_decode(payload: &str) -> Option<Vec<u8>> {
    let bytes = payload.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let hex = bytes.get(at + 1..at + 3)?;
            let hex = std::str::from_utf8(hex).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    Some(out)
}

/// Paint a raster image inside `world_rect`. The source bytes and decoded
/// backend image are both cached, so repeated canvas paints do not re-decode
/// data URLs while importing or panning a Figma-heavy document.
/// Longest raster edge (device px) a node drawn at `world_rect` needs,
/// rounded up to a power of two so a slow zoom re-decodes in steps
/// instead of on every frame, and clamped to a sane ceiling.
pub(crate) fn required_raster_edge(world_rect: Rect, dpi_scale: f32) -> u32 {
    required_raster_edge_with_transform(world_rect, dpi_scale, None)
}

/// Transform-aware raster requirement for image crops.
///
/// A crop transform whose linear part maps the node unit square to a small
/// source-UV window magnifies that source window on screen. Scale the decode
/// request by the inverse smallest singular value so the backend does not
/// downsample the source before the renderer enlarges the crop.
pub(crate) fn required_raster_edge_with_transform(
    world_rect: Rect,
    dpi_scale: f32,
    transform: Option<[f32; 6]>,
) -> u32 {
    const MIN_EDGE: u32 = 64;
    const MAX_EDGE: u32 = 4096;
    let dpi = if dpi_scale.is_finite() && dpi_scale > 0.0 {
        dpi_scale
    } else {
        1.0
    };
    let crop_scale = transform
        .and_then(inverse_smallest_singular_value)
        .unwrap_or(1.0);
    let longest = world_rect.size.x.abs().max(world_rect.size.y.abs()) * dpi * crop_scale;
    if !longest.is_finite() || longest <= 0.0 {
        return MIN_EDGE;
    }
    let needed = longest.ceil() as u32;
    needed
        .clamp(MIN_EDGE, MAX_EDGE)
        .next_power_of_two()
        .min(MAX_EDGE)
}

fn inverse_smallest_singular_value(transform: [f32; 6]) -> Option<f32> {
    let [a, b, _, c, d, _] = transform;
    if ![a, b, c, d].iter().all(|value| value.is_finite()) {
        return None;
    }
    let trace = a * a + b * b + c * c + d * d;
    let determinant_squared = (a * d - b * c).powi(2);
    let discriminant = (trace * trace - 4.0 * determinant_squared).max(0.0);
    let lambda_min = (trace - discriminant.sqrt()) * 0.5;
    let sigma_min = lambda_min.max(0.0).sqrt();
    if sigma_min <= 1e-6 {
        Some(4096.0)
    } else {
        Some((1.0 / sigma_min).max(1.0))
    }
}

pub(super) fn paint_image_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    src: &str,
    show_placeholder: bool,
) {
    paint_image_node_with_stroke(cx, node, world_rect, zoom, src, show_placeholder, true);
}

/// Paint an image node's contents without its outline. Recursing image
/// containers use this so their outline can overlay children exactly once.
pub(super) fn paint_image_node_without_stroke(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    src: &str,
    show_placeholder: bool,
) {
    paint_image_node_with_stroke(cx, node, world_rect, zoom, src, show_placeholder, false);
}

fn paint_image_node_with_stroke(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    src: &str,
    show_placeholder: bool,
    paint_stroke: bool,
) {
    let bytes = image_source_bytes(src, node.image_src_id);
    let id = if node.image_src_id == 0 {
        stable_image_source_id(src)
    } else {
        node.image_src_id
    };
    // Raster only what the view can show. A node drawn 40 px wide needs
    // a 40 px raster, not the source's 2048 px: decoding every visible
    // image at full size is what made a zoomed-out image-dense page
    // thrash its cache (hundreds of 16 MB rasters against a 384 MB
    // budget) while the same page zoomed in stayed smooth.
    let decode_transform = if node.image_fit == crate::layout_scene::SceneImageFit::Tile {
        None
    } else {
        node.image_transform
    };
    let max_edge_px =
        required_raster_edge_with_transform(world_rect, cx.backend.dpi_scale(), decode_transform);
    let sharp_enough = bytes
        .as_deref()
        .is_some_and(|encoded| cx.backend.image_decoded(id, encoded, max_edge_px));
    if bytes.is_some() && !sharp_enough {
        note_pending_decode(id, max_edge_px);
    }
    // A raster that is merely too coarse still draws: sharpening must
    // refine the picture in place, never flash back to placeholder art.
    let decode_ready = sharp_enough || (bytes.is_some() && cx.backend.image_resident(id));
    let r = node.corner_radius * zoom;
    let use_round = r > 0.5;
    let per_corner = scaled_non_uniform_corner_radii(node, zoom);
    if !decode_ready && !show_placeholder {
        return;
    }
    if !decode_ready {
        // Keep every placeholder layer inside the image node's authored
        // corners. The thumb is deliberately first: a translucent node fill
        // can tint it before the neutral placeholder art lands on top.
        if let Some(radii) = per_corner {
            cx.backend.save();
            cx.backend.clip_round_rect_per_corner(world_rect, radii);
        } else if use_round {
            cx.backend.save();
            cx.backend.clip_round_rect(world_rect, r);
        }
        if let Some(thumb) = jian_ops_schema::image_thumbs::thumb_for(id) {
            cx.backend.draw_image_thumb(world_rect, id, thumb.as_ref());
        }
        if let Some(fill) = node.fill {
            if let Some(radii) = per_corner {
                cx.backend
                    .fill_round_rect_per_corner(world_rect, radii, fill);
            } else if use_round {
                cx.backend.fill_round_rect(world_rect, r, fill);
            } else {
                cx.backend.fill_rect(world_rect, fill);
            }
        }
        // Missing / undecodable / still-fetching source — dashed border +
        // a centred picture glyph so the node reads as "image placeholder",
        // not a plain grey box. Also the terminal look for a FAILED image
        // search (`placeholder://` sentinel src). Placeholder art adapts to
        // the slot's own fill: a readable mid-grey on light slots, a lifted
        // grey on dark ones — one fixed grey was invisible on dark designs.
        let luminance = node
            .fill
            .map(|fill| 0.299 * fill.r + 0.587 * fill.g + 0.114 * fill.b)
            .unwrap_or(0.9);
        let placeholder = if luminance >= 0.5 {
            Color {
                r: 0.45,
                g: 0.48,
                b: 0.53,
                a: 1.0,
            }
        } else {
            Color {
                r: 0.55,
                g: 0.58,
                b: 0.64,
                a: 1.0,
            }
        };
        super::canvas_viewport::paint_dashed_rect(cx, world_rect, placeholder, 1.0);
        paint_picture_glyph(cx, world_rect, placeholder);
        if per_corner.is_some() || use_round {
            cx.backend.restore();
        }
    }
    if decode_ready {
        let bytes = bytes.expect("decode-ready image has encoded bytes");
        if let Some(radii) = per_corner {
            cx.backend.save();
            cx.backend.clip_round_rect_per_corner(world_rect, radii);
        }
        let authored_tile_scale =
            if node.image_tile_scale.is_finite() && node.image_tile_scale > 0.0 {
                node.image_tile_scale
            } else {
                1.0
            };
        let tile_zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        let tile_scale = authored_tile_scale * tile_zoom;
        cx.backend
            .draw_image_with_options_transform_blend_and_tile_scale(
                world_rect,
                id,
                bytes.as_ref(),
                node.image_fit.to_draw_mode(),
                node.image_adjustments,
                node.opacity,
                if per_corner.is_none() && use_round {
                    r
                } else {
                    0.0
                },
                node.image_transform,
                node.image_blend_mode,
                node.image_original_size,
                tile_scale,
            );
        if per_corner.is_some() {
            cx.backend.restore();
        }
    }
    if let Some(stroke) = node.stroke.filter(|_| paint_stroke) {
        let width = stroke.width * zoom;
        if let Some(radii) = per_corner {
            cx.backend
                .stroke_round_rect_per_corner(world_rect, radii, stroke.color, width);
        } else if use_round {
            cx.backend
                .stroke_round_rect(world_rect, r, stroke.color, width);
        } else {
            cx.backend.stroke_rect(world_rect, stroke.color, width);
        }
    }
}

#[cfg(test)]
mod blur_up_tests;

#[cfg(test)]
mod tests;
