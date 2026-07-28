//! Bounded host/widget handoff for verified collaboration avatars.
//!
//! URLs arrive only from the authenticated collaboration roster and stay in
//! this ephemeral process-local registry; encoded bytes stay in its LRU.
//! Neither enters `EditorState`, document serialization, or debug output.
//! Widgets read only by epoch-local participant key; the desktop host owns
//! HTTPS and the existing image decode workers own rasterization.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

pub const MAX_AVATAR_ENCODED_BYTES: usize = 512 * 1024;
pub const MAX_AVATAR_SOURCE_EDGE_PX: u32 = 1_024;
pub const MAX_AVATAR_SOURCE_PIXELS: u64 = 1_048_576;
pub const AVATAR_DECODE_EDGE_PX: u32 = 64;

const MAX_AVATAR_URL_BYTES: usize = 2_048;
const MAX_PARTICIPANT_KEY_BYTES: usize = 256;
const AVATAR_CACHE_BYTE_BUDGET: usize = 8 * 1024 * 1024;
const AVATAR_CACHE_MAX_ENTRIES: usize = 64;
const AVATAR_PENDING_CAP: usize = 32;
const IMAGE_ID_PREFIX: u64 = 0xa710_0000_0000_0000;

/// Opaque request token. Its custom `Debug` deliberately omits the URL and
/// participant identity.
#[derive(Clone)]
pub struct CollabAvatarFetchRequest {
    session_generation: u64,
    participant_key: String,
    profile_generation: u64,
    image_id: u64,
    url: String,
}

impl CollabAvatarFetchRequest {
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl fmt::Debug for CollabAvatarFetchRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollabAvatarFetchRequest")
            .field("session_generation", &self.session_generation)
            .field("profile_generation", &self.profile_generation)
            .field("image_id", &self.image_id)
            .field("url", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone)]
pub struct CollabAvatarImage {
    pub image_id: u64,
    pub encoded: Arc<[u8]>,
}

impl fmt::Debug for CollabAvatarImage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollabAvatarImage")
            .field("image_id", &self.image_id)
            .field("encoded_bytes", &self.encoded.len())
            .finish()
    }
}

enum SlotState {
    Waiting,
    Queued,
    InFlight,
    Ready,
    Failed,
}

struct AvatarSlot {
    url: String,
    profile_generation: u64,
    image_id: u64,
    state: SlotState,
    last_used: u64,
}

struct AvatarRegistry {
    slots: HashMap<String, AvatarSlot>,
    /// Encoded cache keyed only by a process-local opaque id. Participant
    /// names and raw URLs are never cache keys.
    encoded: HashMap<u64, Arc<[u8]>>,
    pending: VecDeque<CollabAvatarFetchRequest>,
    cached_bytes: usize,
    session_generation: u64,
    tick: u64,
    next_image_id: u64,
    byte_budget: usize,
    max_entries: usize,
    pending_cap: usize,
}

impl AvatarRegistry {
    fn new() -> Self {
        Self::with_limits(
            AVATAR_CACHE_BYTE_BUDGET,
            AVATAR_CACHE_MAX_ENTRIES,
            AVATAR_PENDING_CAP,
        )
    }

    fn with_limits(byte_budget: usize, max_entries: usize, pending_cap: usize) -> Self {
        Self {
            slots: HashMap::new(),
            encoded: HashMap::new(),
            pending: VecDeque::new(),
            cached_bytes: 0,
            session_generation: 0,
            tick: 0,
            next_image_id: 1,
            byte_budget,
            max_entries,
            pending_cap,
        }
    }

    fn lookup(&mut self, participant_key: &str, url: &str) -> Option<CollabAvatarImage> {
        if !valid_lookup(participant_key, url) || self.max_entries == 0 {
            return None;
        }
        self.tick = self.tick.wrapping_add(1);
        let tick = self.tick;
        let changed = self
            .slots
            .get(participant_key)
            .is_some_and(|slot| slot.url != url);
        let profile_generation = if changed {
            self.slots
                .get(participant_key)
                .map(|slot| slot.profile_generation.wrapping_add(1).max(1))
                .unwrap_or(1)
        } else {
            1
        };
        if changed {
            self.remove_slot(participant_key);
        }
        if !self.slots.contains_key(participant_key) {
            self.evict_to_entry_limit(self.max_entries.saturating_sub(1));
            let image_id = IMAGE_ID_PREFIX | self.next_image_id;
            self.next_image_id = self.next_image_id.wrapping_add(1).max(1);
            self.slots.insert(
                participant_key.to_string(),
                AvatarSlot {
                    url: url.to_string(),
                    profile_generation,
                    image_id,
                    state: SlotState::Waiting,
                    last_used: tick,
                },
            );
        }

        let needs_queue = self
            .slots
            .get(participant_key)
            .is_some_and(|slot| matches!(slot.state, SlotState::Waiting));
        if needs_queue && self.pending.len() < self.pending_cap {
            let slot = self
                .slots
                .get_mut(participant_key)
                .expect("slot was inserted above");
            slot.state = SlotState::Queued;
            self.pending.push_back(CollabAvatarFetchRequest {
                session_generation: self.session_generation,
                participant_key: participant_key.to_string(),
                profile_generation: slot.profile_generation,
                image_id: slot.image_id,
                url: slot.url.clone(),
            });
        }
        let slot = self
            .slots
            .get_mut(participant_key)
            .expect("slot was inserted above");
        slot.last_used = tick;
        match &slot.state {
            SlotState::Ready => self
                .encoded
                .get(&slot.image_id)
                .map(|encoded| CollabAvatarImage {
                    image_id: slot.image_id,
                    encoded: Arc::clone(encoded),
                }),
            _ => None,
        }
    }

    fn ready(&mut self, participant_key: &str) -> Option<CollabAvatarImage> {
        if participant_key.is_empty()
            || participant_key.len() > MAX_PARTICIPANT_KEY_BYTES
            || participant_key.chars().any(char::is_control)
        {
            return None;
        }
        self.tick = self.tick.wrapping_add(1);
        let tick = self.tick;
        let slot = self.slots.get_mut(participant_key)?;
        slot.last_used = tick;
        matches!(slot.state, SlotState::Ready)
            .then(|| {
                self.encoded
                    .get(&slot.image_id)
                    .map(|encoded| CollabAvatarImage {
                        image_id: slot.image_id,
                        encoded: Arc::clone(encoded),
                    })
            })
            .flatten()
    }

    fn take_requests(&mut self, max: usize) -> Vec<CollabAvatarFetchRequest> {
        let mut output = Vec::with_capacity(max.min(self.pending.len()));
        while output.len() < max {
            let Some(request) = self.pending.pop_front() else {
                break;
            };
            let valid = self
                .slots
                .get_mut(&request.participant_key)
                .is_some_and(|slot| {
                    if request.session_generation == self.session_generation
                        && slot.profile_generation == request.profile_generation
                        && slot.image_id == request.image_id
                        && slot.url == request.url
                        && matches!(slot.state, SlotState::Queued)
                    {
                        slot.state = SlotState::InFlight;
                        true
                    } else {
                        false
                    }
                });
            if valid {
                output.push(request);
            }
        }
        output
    }

    fn complete(&mut self, request: &CollabAvatarFetchRequest, bytes: Option<Vec<u8>>) -> bool {
        let valid = self
            .slots
            .get(&request.participant_key)
            .is_some_and(|slot| {
                request.session_generation == self.session_generation
                    && slot.profile_generation == request.profile_generation
                    && slot.image_id == request.image_id
                    && slot.url == request.url
                    && matches!(slot.state, SlotState::InFlight)
            });
        if !valid {
            return false;
        }
        let encoded = bytes.and_then(validate_encoded_avatar);
        self.tick = self.tick.wrapping_add(1);
        let slot = self
            .slots
            .get_mut(&request.participant_key)
            .expect("validated slot still exists");
        slot.last_used = self.tick;
        match encoded {
            Some(encoded) => {
                self.cached_bytes = self.cached_bytes.saturating_add(encoded.len());
                self.encoded.insert(slot.image_id, encoded);
                slot.state = SlotState::Ready;
                self.evict_over_budget();
                self.slots
                    .get(&request.participant_key)
                    .is_some_and(|slot| matches!(slot.state, SlotState::Ready))
            }
            None => {
                slot.state = SlotState::Failed;
                true
            }
        }
    }

    fn bytes_for(&self, image_id: u64) -> Option<Arc<[u8]>> {
        self.encoded.get(&image_id).map(Arc::clone)
    }

    fn begin_session_generation(&mut self, generation: u64) -> bool {
        let changed = self.session_generation != generation
            || !self.pending.is_empty()
            || !self.slots.is_empty()
            || !self.encoded.is_empty();
        self.session_generation = generation;
        self.pending.clear();
        self.slots.clear();
        self.encoded.clear();
        self.cached_bytes = 0;
        self.tick = self.tick.wrapping_add(1);
        // Keep `next_image_id` monotonic: a late decode result may still land
        // in the backend LRU, and a fresh generation must never reuse its id.
        changed
    }

    fn remove_slot(&mut self, participant_key: &str) {
        self.pending
            .retain(|request| request.participant_key != participant_key);
        if let Some(slot) = self.slots.remove(participant_key) {
            if let Some(bytes) = self.encoded.remove(&slot.image_id) {
                self.cached_bytes = self.cached_bytes.saturating_sub(bytes.len());
            }
        }
    }

    fn evict_to_entry_limit(&mut self, limit: usize) {
        while self.slots.len() > limit {
            let Some(oldest) = self
                .slots
                .iter()
                .min_by_key(|(_, slot)| slot.last_used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            self.remove_slot(&oldest);
        }
    }

    fn evict_over_budget(&mut self) {
        while self.cached_bytes > self.byte_budget || self.slots.len() > self.max_entries {
            let Some(oldest) = self
                .slots
                .iter()
                .min_by_key(|(_, slot)| slot.last_used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            self.remove_slot(&oldest);
        }
    }
}

static AVATARS: OnceLock<Mutex<AvatarRegistry>> = OnceLock::new();

fn avatars() -> &'static Mutex<AvatarRegistry> {
    AVATARS.get_or_init(|| Mutex::new(AvatarRegistry::new()))
}

/// Register or replace one URL obtained from the authenticated roster.
///
/// The URL never crosses into `EditorState`; widgets retain only the
/// epoch-local participant key. Invalid/absent profiles remove any stale
/// image for that participant.
pub fn register_collab_avatar_url(participant_key: &str, url: Option<&str>) -> bool {
    let Ok(mut registry) = avatars().lock() else {
        return false;
    };
    let Some(url) = url else {
        registry.remove_slot(participant_key);
        return true;
    };
    if !valid_lookup(participant_key, url) {
        registry.remove_slot(participant_key);
        return false;
    }
    let _ = registry.lookup(participant_key, url);
    true
}

/// Resolve a ready avatar by opaque participant key only.
pub fn collab_avatar_image(participant_key: &str) -> Option<CollabAvatarImage> {
    avatars().lock().ok()?.ready(participant_key)
}

/// Begin a process-local avatar epoch for a collaboration runtime.
/// Pending, failed, and ready entries are dropped on every call, including
/// when a newly constructed runtime reuses the same numeric generation.
/// In-flight workers may finish, but their request tokens can no longer
/// complete because opaque image ids are never reused.
pub fn begin_collab_avatar_generation(generation: u64) -> bool {
    avatars()
        .lock()
        .map(|mut registry| registry.begin_session_generation(generation))
        .unwrap_or(false)
}

pub fn take_collab_avatar_requests(max: usize) -> Vec<CollabAvatarFetchRequest> {
    avatars()
        .lock()
        .map(|mut registry| registry.take_requests(max))
        .unwrap_or_default()
}

/// Land one worker result if its participant generation is still current.
/// Invalid, oversized, or stale results are discarded without retaining raw
/// bytes.
pub fn complete_collab_avatar_request(
    request: &CollabAvatarFetchRequest,
    bytes: Option<Vec<u8>>,
) -> bool {
    avatars()
        .lock()
        .map(|mut registry| registry.complete(request, bytes))
        .unwrap_or(false)
}

pub fn cached_collab_avatar_bytes(image_id: u64) -> Option<Arc<[u8]>> {
    avatars().lock().ok()?.bytes_for(image_id)
}

pub fn has_pending_collab_avatar_requests() -> bool {
    avatars()
        .lock()
        .map(|registry| !registry.pending.is_empty())
        .unwrap_or(false)
}

fn valid_lookup(participant_key: &str, url: &str) -> bool {
    !participant_key.is_empty()
        && participant_key.len() <= MAX_PARTICIPANT_KEY_BYTES
        && !participant_key.chars().any(char::is_control)
        && !url.is_empty()
        && url.len() <= MAX_AVATAR_URL_BYTES
        && url.starts_with("https://")
        && !url.chars().any(char::is_whitespace)
}

fn validate_encoded_avatar(bytes: Vec<u8>) -> Option<Arc<[u8]>> {
    if bytes.is_empty() || bytes.len() > MAX_AVATAR_ENCODED_BYTES {
        return None;
    }
    let (width, height) = crate::image_runtime::encoded_image_dimensions(&bytes)?;
    if width > MAX_AVATAR_SOURCE_EDGE_PX
        || height > MAX_AVATAR_SOURCE_EDGE_PX
        || u64::from(width) * u64::from(height) > MAX_AVATAR_SOURCE_PIXELS
    {
        return None;
    }
    Some(Arc::from(bytes.into_boxed_slice()))
}

#[cfg(test)]
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub(crate) fn lock_collab_avatar_registry_for_tests() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    if let Ok(mut registry) = avatars().lock() {
        *registry = AvatarRegistry::new();
    }
    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_header(width: u32, height: u32, payload_bytes: usize) -> Vec<u8> {
        let mut bytes = vec![0; payload_bytes.max(24)];
        bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes[12..16].copy_from_slice(b"IHDR");
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        bytes
    }

    fn fetch(registry: &mut AvatarRegistry, key: &str, url: &str) -> CollabAvatarFetchRequest {
        assert!(registry.lookup(key, url).is_none());
        registry.take_requests(1).pop().expect("request queued")
    }

    #[test]
    fn success_is_cached_and_failures_keep_the_fallback() {
        let mut registry = AvatarRegistry::with_limits(1_024, 4, 4);
        let ok = fetch(&mut registry, "p1", "https://cdn.example/a.png");
        assert!(registry.complete(&ok, Some(png_header(16, 16, 48))));
        let image = registry
            .lookup("p1", "https://cdn.example/a.png")
            .expect("valid response is ready");
        assert_eq!(image.image_id, ok.image_id);

        let failed = fetch(&mut registry, "p2", "https://cdn.example/b.png");
        assert!(registry.complete(&failed, None));
        assert!(registry.lookup("p2", "https://cdn.example/b.png").is_none());
        assert!(registry.take_requests(1).is_empty());
    }

    #[test]
    fn encoded_bytes_dimensions_and_queue_are_bounded() {
        let mut registry = AvatarRegistry::with_limits(2_048, 64, 1);
        let large_body = fetch(&mut registry, "p1", "https://cdn.example/large.png");
        assert!(registry.complete(&large_body, Some(vec![0; MAX_AVATAR_ENCODED_BYTES + 1])));
        assert!(registry
            .lookup("p1", "https://cdn.example/large.png")
            .is_none());

        let large_raster = fetch(&mut registry, "p2", "https://cdn.example/wide.png");
        assert!(registry.complete(
            &large_raster,
            Some(png_header(MAX_AVATAR_SOURCE_EDGE_PX + 1, 1, 24))
        ));
        assert!(registry
            .lookup("p2", "https://cdn.example/wide.png")
            .is_none());

        assert!(registry.lookup("p3", "https://cdn.example/c.png").is_none());
        assert!(registry.lookup("p4", "https://cdn.example/d.png").is_none());
        assert_eq!(registry.take_requests(8).len(), 1);
    }

    #[test]
    fn lru_eviction_and_generation_switch_drop_stale_results() {
        let mut registry = AvatarRegistry::with_limits(80, 2, 4);
        let first = fetch(&mut registry, "p1", "https://cdn.example/one.png");
        assert!(registry.complete(&first, Some(png_header(2, 2, 40))));
        let second = fetch(&mut registry, "p2", "https://cdn.example/two.png");
        assert!(registry.complete(&second, Some(png_header(2, 2, 40))));
        assert!(registry
            .lookup("p1", "https://cdn.example/one.png")
            .is_some());
        let third = fetch(&mut registry, "p3", "https://cdn.example/three.png");
        assert!(registry.complete(&third, Some(png_header(2, 2, 40))));
        assert!(
            !registry.slots.contains_key("p2"),
            "least recently used entry is evicted"
        );

        let old = fetch(&mut registry, "p4", "https://cdn.example/old.png");
        assert!(registry
            .lookup("p4", "https://cdn.example/new.png")
            .is_none());
        let new = registry.take_requests(1).pop().expect("new generation");
        assert!(!registry.complete(&old, Some(png_header(2, 2, 32))));
        assert!(registry
            .lookup("p4", "https://cdn.example/new.png")
            .is_none());
        assert!(registry.complete(&new, Some(png_header(2, 2, 32))));
        assert_eq!(
            registry
                .lookup("p4", "https://cdn.example/new.png")
                .expect("new generation wins")
                .image_id,
            new.image_id
        );
    }

    #[test]
    fn session_generation_rotation_clears_cache_and_rejects_late_workers() {
        let mut registry = AvatarRegistry::with_limits(1_024, 4, 4);
        assert!(registry.begin_session_generation(7));
        let stale = fetch(
            &mut registry,
            "same-participant",
            "https://cdn.example/same.png",
        );
        assert!(registry.begin_session_generation(8));
        assert!(registry.slots.is_empty());
        assert!(registry.pending.is_empty());
        assert!(!registry.complete(&stale, Some(png_header(16, 16, 32))));

        let current = fetch(
            &mut registry,
            "same-participant",
            "https://cdn.example/same.png",
        );
        assert_ne!(stale.image_id, current.image_id);
        assert!(
            registry.begin_session_generation(8),
            "a new runtime may reuse the numeric generation and must still reset"
        );
        assert!(!registry.complete(&current, Some(png_header(16, 16, 32))));
        let refreshed = fetch(
            &mut registry,
            "same-participant",
            "https://cdn.example/same.png",
        );
        assert_ne!(current.image_id, refreshed.image_id);
        assert!(registry.complete(&refreshed, Some(png_header(16, 16, 32))));
        assert!(registry
            .lookup("same-participant", "https://cdn.example/same.png")
            .is_some());
    }

    #[test]
    fn request_debug_redacts_url_and_identity() {
        let mut registry = AvatarRegistry::with_limits(100, 1, 1);
        let request = fetch(
            &mut registry,
            "private-participant",
            "https://cdn.example/secret.png",
        );
        let debug = format!("{request:?}");
        assert!(!debug.contains("private-participant"));
        assert!(!debug.contains("cdn.example"));
        assert!(debug.contains("[REDACTED]"));
    }
}
