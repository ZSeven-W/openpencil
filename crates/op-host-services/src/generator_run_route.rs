//! `POST /api/generator/run`: run one canvas generator program for the
//! browser shell.
//!
//! The wasm bundle carries no QuickJS, so the browser's editor hands the
//! run to its daemon (see `op_editor_core::generator` → `generator_remote`)
//! and lands the reply through the same command path a native regenerate
//! uses. The daemon runs the SAME runtime desktop installs —
//! [`op_mcp::generator_runtime::run_generator`]: the QuickJS sandbox with no
//! IO, a 64 MiB heap, a 1 500 ms wall-clock budget, and the recorder's line
//! caps. Nothing is read from or written to the daemon's own document: the
//! request carries everything the program sees, and the reply is only the
//! generated children.
//!
//! Request: `{"generatorId": "…", "spec": {…}, "frame": {…}}`
//! ([`GeneratorRunWireRequest`]). Reply (200): `{"children": [...]}`; a
//! refusal is `{"error": "…"}` — 400 for a malformed or oversized request,
//! 422 for a program that failed, 429 when every run slot is busy.
//!
//! Served in every [`ServeMode`](crate::web_canvas_server::ServeMode):
//! the run is pure, sandboxed compute over caller-supplied input — no
//! filesystem, no network, no process-global or tenant state — so none of
//! the reasons the online deployment shuts routes off applies. What a
//! shared process does need is a bound on CPU, which the per-run budget and
//! [`MAX_CONCURRENT_RUNS`] provide.

use std::sync::atomic::{AtomicUsize, Ordering};

use jian_ops_schema::node::PenNode;
use op_editor_core::generator::{
    generator_run_reply_body, GeneratorError, GeneratorRequest, GeneratorRunWireRequest,
};

pub use op_editor_core::generator::GENERATOR_RUN_ROUTE;

/// Whether this daemon serves [`GENERATOR_RUN_ROUTE`] — advertised as
/// `generators` in `GET /api/mcp/server`. Always: see the module docs for
/// why no deployment mode needs it off.
pub const SERVED: bool = true;

/// A program is capped at 64 KiB and sixteen text params at 4 096 chars
/// each; the frame is one node without children. 1 MiB leaves headroom for
/// a frame with an embedded image fill without accepting arbitrary bulk.
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// Runs executing at once, process-wide. Each holds its connection thread
/// for up to the 1 500 ms budget.
pub const MAX_CONCURRENT_RUNS: usize = 2;

static RUNNING: AtomicUsize = AtomicUsize::new(0);

/// One of the [`MAX_CONCURRENT_RUNS`] slots of `counter`, released on drop.
struct RunSlot<'a>(&'a AtomicUsize);

impl<'a> RunSlot<'a> {
    fn acquire(counter: &'a AtomicUsize) -> Option<Self> {
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |running| {
                (running < MAX_CONCURRENT_RUNS).then_some(running + 1)
            })
            .ok()
            .map(|_| RunSlot(counter))
    }
}

impl Drop for RunSlot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Why a request never reached the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestRefusal {
    TooLarge,
    Malformed,
    Busy,
}

impl RequestRefusal {
    fn status(self) -> &'static str {
        match self {
            Self::TooLarge | Self::Malformed => "400 Bad Request",
            Self::Busy => "429 Too Many Requests",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::TooLarge => "generator request too large",
            Self::Malformed => "invalid generator request",
            Self::Busy => "the generator server is busy; try again",
        }
    }
}

/// Serve one request through `run` (production: [`serve`]; tests inject
/// a runner). Returns `(status, body)`.
pub fn serve_with(
    body: &str,
    run: impl FnOnce(&GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError>,
) -> (&'static str, String) {
    serve_counted(body, &RUNNING, run)
}

/// [`serve_with`] against an explicit slot counter (tests use their own so
/// a held slot never leaks into a parallel test).
fn serve_counted(
    body: &str,
    slots: &AtomicUsize,
    run: impl FnOnce(&GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError>,
) -> (&'static str, String) {
    let refuse = |refusal: RequestRefusal| {
        (
            refusal.status(),
            serde_json::json!({ "error": refusal.message() }).to_string(),
        )
    };
    if body.len() > MAX_REQUEST_BYTES {
        return refuse(RequestRefusal::TooLarge);
    }
    let Ok(request) = serde_json::from_str::<GeneratorRunWireRequest>(body) else {
        return refuse(RequestRefusal::Malformed);
    };
    let Some(_slot) = RunSlot::acquire(slots) else {
        return refuse(RequestRefusal::Busy);
    };
    let result = run(&request.as_request());
    let status = if result.is_ok() {
        "200 OK"
    } else {
        "422 Unprocessable Entity"
    };
    (status, generator_run_reply_body(&result))
}

/// Serve one request with the production runtime. Blocking for up to the
/// runtime's budget: the dispatcher runs it on the connection's own thread,
/// never under the state lock.
pub fn serve(body: &str) -> (&'static str, String) {
    serve_with(body, op_mcp::generator_runtime::run_generator)
}

#[cfg(test)]
#[path = "generator_run_route_tests.rs"]
mod tests;
