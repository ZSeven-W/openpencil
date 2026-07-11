use crate::layout_scene::{stable_image_source_id, SceneNode};
use crate::widgets::PaintCx;
use crate::Rect;
use crate::{Color, Point2D};
use std::sync::{Arc, Mutex, OnceLock};

const DATA_URL_CACHE_CAP: usize = 64;

struct DataUrlCache {
    entries: std::collections::HashMap<u64, Arc<[u8]>>,
    order: std::collections::VecDeque<u64>,
}

impl DataUrlCache {
    fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            order: std::collections::VecDeque::new(),
        }
    }

    fn get(&self, id: u64) -> Option<Arc<[u8]>> {
        self.entries.get(&id).cloned()
    }

    fn insert(&mut self, id: u64, bytes: Arc<[u8]>) {
        if !self.entries.contains_key(&id) {
            self.order.push_back(id);
        }
        self.entries.insert(id, bytes);
        while self.entries.len() > DATA_URL_CACHE_CAP {
            match self.order.pop_front() {
                Some(oldest) => {
                    self.entries.remove(&oldest);
                }
                None => break,
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
    if let Ok(mut cache) = data_url_cache().lock() {
        cache.insert(id, Arc::from(bytes.into_boxed_slice()));
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
        .map(|cache| cache.get(id).is_some())
        .unwrap_or(false)
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
    if let Ok(cache) = data_url_cache().lock() {
        if let Some(bytes) = cache.get(id) {
            return Some(bytes);
        }
    }

    if is_remote_image_url(src) {
        note_remote_image_miss(id, src);
        return None;
    }
    let decoded = decode_data_url_bytes(src)?;
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
        return None;
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

/// Paint a raster image inside `world_rect`. The source bytes and decoded
/// backend image are both cached, so repeated canvas paints do not re-decode
/// data URLs while importing or panning a Figma-heavy document.
pub(super) fn paint_image_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    src: &str,
) {
    let bytes = image_source_bytes(src, node.image_src_id);
    let r = node.corner_radius * zoom;
    let use_round = r > 0.5;
    if bytes.is_none() {
        if let Some(fill) = node.fill {
            if use_round {
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
        // Honor the node's corner radius like the real-image path
        // below: clip the placeholder art into the rounded rect (the
        // dashed edges lose their corner segments, which reads as a
        // rounded dashed border without a dashed-arc primitive).
        if use_round {
            cx.backend.save();
            cx.backend.clip_round_rect(world_rect, r);
        }
        super::canvas_viewport::paint_dashed_rect(cx, world_rect, placeholder, 1.0);
        paint_picture_glyph(cx, world_rect, placeholder);
        if use_round {
            cx.backend.restore();
        }
    }
    if let Some(bytes) = bytes {
        let id = if node.image_src_id == 0 {
            stable_image_source_id(src)
        } else {
            node.image_src_id
        };
        cx.backend.draw_image_with_options(
            world_rect,
            id,
            bytes.as_ref(),
            node.image_fit.to_draw_mode(),
            node.image_adjustments,
            node.opacity,
            if use_round { r } else { 0.0 },
        );
    }
    if let Some(stroke) = node.stroke {
        let width = stroke.width * zoom;
        if use_round {
            cx.backend
                .stroke_round_rect(world_rect, r, stroke.color, width);
        } else {
            cx.backend.stroke_rect(world_rect, stroke.color, width);
        }
    }
}

#[cfg(test)]
fn clear_data_url_cache_for_tests() {
    if let Ok(mut cache) = data_url_cache().lock() {
        *cache = DataUrlCache::new();
    }
}

#[cfg(test)]
fn data_url_cache_len_for_tests() -> usize {
    data_url_cache()
        .lock()
        .map(|cache| cache.entries.len())
        .unwrap_or(0)
}

#[cfg(test)]
fn clear_remote_registry_for_tests() {
    if let Ok(mut reg) = remote_images().lock() {
        *reg = RemoteImageRegistry::default();
    }
}

/// Minimal "picture" glyph — frame + sun + mountain strokes scaled
/// into the centre of `rect` (24px reference art, like the lucide
/// `image` icon but hand-stroked to avoid an icon-catalog dependency
/// in the paint path).
fn paint_picture_glyph(cx: &mut PaintCx<'_>, rect: Rect, color: Color) {
    let size = (rect.size.x.min(rect.size.y) * 0.4).clamp(12.0, 48.0);
    let cx0 = rect.origin.x + rect.size.x / 2.0 - size / 2.0;
    let cy0 = rect.origin.y + rect.size.y / 2.0 - size / 2.0;
    let w = 1.5;
    // Frame.
    cx.backend.stroke_round_rect(
        Rect {
            origin: Point2D::new(cx0, cy0),
            size: Point2D::new(size, size),
        },
        size * 0.12,
        color,
        w,
    );
    // Sun — small circle approximated by a tight round-rect.
    let sun = size * 0.16;
    cx.backend.stroke_round_rect(
        Rect {
            origin: Point2D::new(cx0 + size * 0.2, cy0 + size * 0.2),
            size: Point2D::new(sun, sun),
        },
        sun / 2.0,
        color,
        w,
    );
    // Mountain.
    cx.backend.stroke_line(
        Point2D::new(cx0 + size * 0.12, cy0 + size * 0.85),
        Point2D::new(cx0 + size * 0.45, cy0 + size * 0.45),
        color,
        w,
    );
    cx.backend.stroke_line(
        Point2D::new(cx0 + size * 0.45, cy0 + size * 0.45),
        Point2D::new(cx0 + size * 0.7, cy0 + size * 0.7),
        color,
        w,
    );
    cx.backend.stroke_line(
        Point2D::new(cx0 + size * 0.7, cy0 + size * 0.7),
        Point2D::new(cx0 + size * 0.88, cy0 + size * 0.52),
        color,
        w,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The caches + registry are process-wide statics; serialize the
    /// tests that mutate them so parallel test threads don't race.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock_statics() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_data_url_cache_for_tests();
        clear_remote_registry_for_tests();
        guard
    }

    #[test]
    fn data_url_cache_reuses_decoded_bytes() {
        let _guard = lock_statics();
        let src = "data:image/png;base64,QUJD";

        let first = image_source_bytes(src, 7).expect("first decode");
        assert_eq!(first.as_ref(), b"ABC");
        assert_eq!(data_url_cache_len_for_tests(), 1);

        let second = image_source_bytes(src, 7).expect("cached decode");
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert_eq!(data_url_cache_len_for_tests(), 1);
    }

    #[test]
    fn remote_miss_is_queued_once_and_drained_by_the_host() {
        let _guard = lock_statics();
        let url = "https://example.com/cat.png";

        // First paint records the miss; repeated paints don't dup it.
        assert!(image_source_bytes(url, 101).is_none());
        assert!(image_source_bytes(url, 101).is_none());
        assert!(has_pending_remote_image_requests());
        let taken = take_remote_image_requests(8);
        assert_eq!(taken, vec![(101, url.to_string())]);
        assert!(!has_pending_remote_image_requests());

        // In-flight (taken) — paint still must not re-queue.
        assert!(image_source_bytes(url, 101).is_none());
        assert!(take_remote_image_requests(8).is_empty());
    }

    #[test]
    fn stored_remote_bytes_hit_the_shared_cache_on_next_paint() {
        let _guard = lock_statics();
        let url = "https://example.com/dog.png";

        assert!(image_source_bytes(url, 102).is_none());
        let _ = take_remote_image_requests(8);
        store_remote_image_bytes(102, b"PNGBYTES".to_vec());

        let bytes = image_source_bytes(url, 102).expect("cache hit after store");
        assert_eq!(bytes.as_ref(), b"PNGBYTES");
        // Satisfied — nothing further queued.
        assert!(!has_pending_remote_image_requests());
    }

    #[test]
    fn failed_fetch_is_negative_cached_and_never_requeued() {
        let _guard = lock_statics();
        let url = "https://example.com/404.png";

        assert!(image_source_bytes(url, 103).is_none());
        let _ = take_remote_image_requests(8);
        mark_remote_image_failed(103);

        // Paint keeps missing but the miss never re-queues.
        assert!(image_source_bytes(url, 103).is_none());
        assert!(!has_pending_remote_image_requests());
        assert!(take_remote_image_requests(8).is_empty());
    }

    #[test]
    fn empty_stored_bytes_count_as_failure() {
        let _guard = lock_statics();
        let url = "https://example.com/empty.png";

        assert!(image_source_bytes(url, 104).is_none());
        let _ = take_remote_image_requests(8);
        store_remote_image_bytes(104, Vec::new());

        assert!(image_source_bytes(url, 104).is_none());
        assert!(!has_pending_remote_image_requests());
    }

    #[test]
    fn miss_queue_is_bounded() {
        let _guard = lock_statics();
        for i in 0..(REMOTE_MISS_QUEUE_CAP as u64 + 10) {
            note_remote_image_miss(200 + i, "https://example.com/n.png");
        }
        let taken = take_remote_image_requests(usize::MAX);
        assert_eq!(taken.len(), REMOTE_MISS_QUEUE_CAP);
    }
}
