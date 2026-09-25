//! The browser generator runner's bookkeeping, free of any DOM so it is
//! unit-testable natively.
//!
//! The runner the editor calls (`GeneratorRunner` is a plain `fn`) cannot
//! wait for the daemon, so every run goes through this cache:
//!
//! * a **hit** returns the children (or the error) the daemon answered for
//!   exactly this request;
//! * a **miss** queues the request — once: an identical request already
//!   queued or in flight is not asked twice — and answers
//!   `GeneratorError::Pending`.
//!
//! The host drains [`GeneratorCache::take_queued`] into XHRs, feeds replies
//! back through [`GeneratorCache::complete`], and re-applies the requests
//! [`GeneratorCache::take_ready`] hands it (the re-run then hits).
//!
//! Keys hash only what the program's output depends on: the generator id
//! (the `Math.random` seed and the derived child ids), the spec minus its
//! bookkeeping `outputHash`, and the frame minus its `explain` (which is
//! the stored spec itself, and the runtime never reads it).

use std::collections::{HashMap, HashSet, VecDeque};

use jian_ops_schema::node::PenNode;
use op_editor_core::generator::{
    GeneratorError, GeneratorRequest, GeneratorRunReply, GeneratorRunWireRequest,
};
use op_editor_core::PenNodeExt;

/// Results kept at once; the oldest is dropped past this.
pub(crate) const CACHE_CAPACITY: usize = 32;

/// One request waiting for, or holding, a daemon answer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GeneratorJob {
    pub(crate) key: u64,
    pub(crate) wire: GeneratorRunWireRequest,
}

#[derive(Debug, Clone)]
struct CachedResult {
    key: u64,
    result: Result<Vec<PenNode>, GeneratorError>,
    /// A transient failure is served once (so it surfaces) and then
    /// forgotten, so the next attempt asks the daemon again.
    one_shot: bool,
}

#[derive(Debug, Default)]
pub(crate) struct GeneratorCache {
    results: VecDeque<CachedResult>,
    in_flight: HashSet<u64>,
    queued: Vec<GeneratorJob>,
    ready: Vec<GeneratorJob>,
    /// The key each generator last asked for. A reply for an older key is
    /// cached but not re-applied, so a slow answer can never overwrite a
    /// newer edit.
    latest: HashMap<String, u64>,
}

impl GeneratorCache {
    /// The runner's body: answer from the cache or queue the request.
    pub(crate) fn run(
        &mut self,
        request: &GeneratorRequest<'_>,
    ) -> Result<Vec<PenNode>, GeneratorError> {
        let wire = normalized_wire(request);
        let key = request_key(&wire);
        self.latest.insert(wire.generator_id.clone(), key);
        if let Some(index) = self.results.iter().position(|entry| entry.key == key) {
            let entry = &self.results[index];
            let result = entry.result.clone();
            if entry.one_shot {
                self.results.remove(index);
            }
            return result;
        }
        if self.in_flight.insert(key) {
            self.queued.push(GeneratorJob { key, wire });
        }
        Err(GeneratorError::Pending)
    }

    /// Requests to send now.
    pub(crate) fn take_queued(&mut self) -> Vec<GeneratorJob> {
        std::mem::take(&mut self.queued)
    }

    /// Store the daemon's `reply` to `job`; queue its re-apply unless a
    /// newer request for the same generator superseded it.
    pub(crate) fn complete(&mut self, job: GeneratorJob, reply: GeneratorRunReply) {
        self.in_flight.remove(&job.key);
        self.results.retain(|entry| entry.key != job.key);
        self.results.push_back(CachedResult {
            key: job.key,
            result: reply.result,
            one_shot: reply.transient,
        });
        while self.results.len() > CACHE_CAPACITY {
            self.results.pop_front();
        }
        if self.latest.get(&job.wire.generator_id) == Some(&job.key) {
            self.ready.push(job);
        }
    }

    /// Answered requests to re-apply through the editor.
    pub(crate) fn take_ready(&mut self) -> Vec<GeneratorJob> {
        std::mem::take(&mut self.ready)
    }

    #[cfg(test)]
    pub(crate) fn cached_len(&self) -> usize {
        self.results.len()
    }
}

/// The request as sent: the frame without its `explain` (the runtime
/// never reads it, and it would make every key unique per stored spec).
fn normalized_wire(request: &GeneratorRequest<'_>) -> GeneratorRunWireRequest {
    let mut wire = GeneratorRunWireRequest::from_request(request);
    wire.frame.base_mut().explain = None;
    wire
}

/// FNV-1a over the generator id, the spec without `outputHash`, and the
/// normalized frame. Stable within a page, which is all the cache needs.
fn request_key(wire: &GeneratorRunWireRequest) -> u64 {
    let mut spec = wire.spec.clone();
    spec.output_hash = None;
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    };
    feed(wire.generator_id.as_bytes());
    feed(serde_json::to_string(&spec).unwrap_or_default().as_bytes());
    feed(
        serde_json::to_string(&wire.frame)
            .unwrap_or_default()
            .as_bytes(),
    );
    hash
}

#[cfg(test)]
#[path = "web_generator_cache_tests.rs"]
mod tests;
