//! On-demand re-discovery of the external CLI providers' model catalogs.
//!
//! Discovery otherwise happens exactly twice: once in the startup
//! [`crate::model_probe::ModelProbe`] sweep, and once per connect probe
//! (`provider_probe_host`, which is also a re-discovery). Neither fires
//! again while the app stays open, so a CLI that ships a new model
//! generation mid-session — the user runs `codex upgrade` in a terminal —
//! keeps showing last night's snapshot in the picker until the app is
//! restarted or the provider re-connected by hand.
//!
//! This module closes that gap from the other end: opening the model
//! picker requests a silent refresh of every *verified connected* CLI
//! provider, TTL-debounced so the request costs nothing when the catalog
//! is already fresh. Failures are swallowed on purpose — see
//! [`apply_refreshed_catalog`].

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use op_ai::chat_models::ModelEntry;
use op_editor_core::{AgentProvider, EditorState};

use crate::model_discovery::{discover_models_for_connected, model_entry_to_ec};

/// How long a provider's catalog is trusted before another picker open
/// may re-probe it.
///
/// Discovery spawns real subprocesses (`codex app-server`, `agy models`,
/// …), each costing hundreds of milliseconds of CPU and disk, so an
/// unthrottled hook would re-probe on every open — including the
/// reflexive open/close/open while a user scans the list. Five minutes
/// is far below the interval at which a CLI actually gains models (that
/// takes a user-initiated upgrade) and far above the interval at which
/// one editing session opens the picker, so a session pays at most one
/// probe round per provider per five minutes.
pub const MODEL_CATALOG_TTL: Duration = Duration::from_secs(5 * 60);

struct RefreshJob {
    /// The provider mask this job was spawned for — the landing step
    /// must only touch these slices, never the whole catalog.
    mask: [bool; 7],
    rx: Receiver<Vec<ModelEntry>>,
}

/// TTL bookkeeping + the single in-flight refresh worker.
///
/// One slot suffices: the request is a background nicety, and a second
/// picker open while a refresh runs wants the same answer the in-flight
/// job is already fetching.
#[derive(Default)]
pub struct ModelCatalogRefresh {
    /// When each provider was last *asked* — not last answered. Debouncing
    /// on the attempt is deliberate: a provider whose CLI is broken returns
    /// nothing every time, and keying the TTL off success would re-spawn
    /// that failing probe on every single picker open.
    last_attempt: [Option<Instant>; 7],
    job: Option<RefreshJob>,
}

impl ModelCatalogRefresh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_pending(&self) -> bool {
        self.job.is_some()
    }

    /// Providers in `connected` whose catalog has aged past the TTL.
    pub fn due_providers(&self, connected: [bool; 7], now: Instant) -> [bool; 7] {
        std::array::from_fn(|index| {
            connected[index]
                && self.last_attempt[index]
                    .is_none_or(|last| now.saturating_duration_since(last) >= MODEL_CATALOG_TTL)
        })
    }

    /// Spawn a silent refresh for the due subset of `connected`. Returns
    /// true when a worker started.
    pub fn request(&mut self, connected: [bool; 7], now: Instant) -> bool {
        self.request_with(connected, now, discover_models_for_connected)
    }

    /// [`request`](Self::request) with the discovery step injected, so tests
    /// can drive the whole request → land → splice path without depending on
    /// which CLIs the machine running them happens to have installed.
    pub fn request_with<F>(&mut self, connected: [bool; 7], now: Instant, discover: F) -> bool
    where
        F: FnOnce([bool; 7]) -> Vec<ModelEntry> + Send + 'static,
    {
        if self.is_pending() {
            return false;
        }
        let mask = self.due_providers(connected, now);
        if !mask.iter().any(|due| *due) {
            return false;
        }
        for (index, due) in mask.iter().enumerate() {
            if *due {
                self.last_attempt[index] = Some(now);
            }
        }
        let (tx, rx) = mpsc::channel();
        // Detached one-shot, same shape as `ModelProbe`: every CLI probe
        // under `discover_models_for_connected` is deadline-bounded, so the
        // thread always finishes on its own, and a dropped receiver only
        // makes the final send a no-op.
        std::thread::spawn(move || {
            let _ = tx.send(discover(mask));
        });
        self.job = Some(RefreshJob { mask, rx });
        true
    }

    /// Land a finished refresh into the live catalog. Returns true when a
    /// result was applied.
    pub fn poll_into(&mut self, state: &mut EditorState) -> bool {
        let Some(job) = self.job.as_ref() else {
            return false;
        };
        match job.rx.try_recv() {
            Ok(models) => {
                let mask = job.mask;
                self.job = None;
                apply_refreshed_catalog(state, mask, models);
                state.rebuild_chat_models();
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                false
            }
        }
    }
}

/// Replace the refreshed providers' slices, leaving every other provider —
/// and every provider that came back empty — exactly as it was.
///
/// The empty-slice skip is the whole failure policy: a refresh nobody
/// asked for must never be able to empty the picker the user is looking
/// at. A stale entry fails loudly at send time; a vanished list looks
/// like the app broke.
fn apply_refreshed_catalog(state: &mut EditorState, mask: [bool; 7], models: Vec<ModelEntry>) {
    let fresh: Vec<op_editor_core::ModelEntry> =
        models.into_iter().map(model_entry_to_ec).collect();
    for (index, provider) in AgentProvider::ALL.iter().enumerate() {
        if !mask[index] {
            continue;
        }
        let slice: Vec<op_editor_core::ModelEntry> = fresh
            .iter()
            .filter(|entry| entry.provider == *provider)
            .cloned()
            .collect();
        if slice.is_empty() {
            continue;
        }
        state
            .chat
            .discovered_models
            .retain(|entry| entry.provider != *provider);
        state.chat.discovered_models.extend(slice);
    }
    // Stable sort back into `AgentProvider::ALL` order — the spliced
    // slices were appended at the end, and the picker groups by provider.
    state.chat.discovered_models.sort_by_key(|entry| {
        AgentProvider::ALL
            .iter()
            .position(|provider| *provider == entry.provider)
            .unwrap_or(usize::MAX)
    });
}

#[cfg(test)]
#[path = "model_catalog_refresh_tests.rs"]
mod model_catalog_refresh_tests;
