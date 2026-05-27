use crate::layout_scene::{stable_image_source_id, SceneNode};
use crate::widgets::PaintCx;
use crate::Rect;
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

/// Decode an inline-base64 `data:image/...;base64,...` URL into the raw
/// image bytes the backend's image decoder expects. The decoded bytes are
/// cached by the scene's precomputed source id, so paint frames clone an
/// `Arc` instead of cleaning + base64-decoding large data URLs again.
fn data_url_bytes(src: &str, image_src_id: u64) -> Option<Arc<[u8]>> {
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
    let bytes = data_url_bytes(src, node.image_src_id);
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
mod tests {
    use super::*;

    #[test]
    fn data_url_cache_reuses_decoded_bytes() {
        clear_data_url_cache_for_tests();
        let src = "data:image/png;base64,QUJD";

        let first = data_url_bytes(src, 7).expect("first decode");
        assert_eq!(first.as_ref(), b"ABC");
        assert_eq!(data_url_cache_len_for_tests(), 1);

        let second = data_url_bytes(src, 7).expect("cached decode");
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert_eq!(data_url_cache_len_for_tests(), 1);
    }
}
