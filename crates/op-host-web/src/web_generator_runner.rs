// Browser boundary: the capability probe and the run XHRs need a live page;
// the cache decisions they drive are unit-tested in
// `web_generator_cache_tests.rs`, the editor half in op-editor-core's
// `generator_remote_tests.rs`, and the route in op-host-services'
// `generator_run_route_tests.rs`.
//! The browser's generator runtime: the daemon runs the program.
//!
//! The wasm bundle has no QuickJS. When the daemon advertises
//! `generators` in `GET /api/mcp/server`, this installs [`run`] as the
//! process-wide `GeneratorRunner`; until then (and for good against a daemon
//! without the route) nothing is installed and the editor keeps generators
//! read-only. `run` answers from [`GeneratorCache`] or queues the request and
//! answers `Pending`; [`pump`] — once per frame — re-applies answered
//! requests through the editor and sends the queued ones.

use std::cell::RefCell;
use std::rc::Rc;

use op_editor_core::generator::{
    installed_generator_runner, parse_generator_run_reply, GeneratorError, GeneratorRequest,
    GENERATOR_RUN_ROUTE,
};
use op_editor_core::NodeId;

use crate::web_generator_cache::{GeneratorCache, GeneratorJob};

/// The program has a 1 500 ms budget; the rest is transport slack. Bounded
/// so a hung daemon surfaces as a failure instead of "running" forever.
const RUN_TIMEOUT_MS: u32 = 15_000;

thread_local! {
    /// wasm is single-threaded; the runner is a plain `fn`, so its state
    /// lives here.
    static CACHE: RefCell<GeneratorCache> = RefCell::new(GeneratorCache::default());
}

/// The installed `GeneratorRunner`.
fn run(
    request: &GeneratorRequest<'_>,
) -> Result<Vec<jian_ops_schema::node::PenNode>, GeneratorError> {
    CACHE.with(|cache| cache.borrow_mut().run(request))
}

/// Ask the daemon whether it runs generators, and install [`run`] if so.
/// Separate from the Studio `fileBound` probe because that one is skipped
/// for the VS Code embed, which still edits generators.
pub(crate) fn probe_capability() {
    let url = crate::daemon_base::daemon_url("/api/mcp/server");
    crate::live_sync::get(
        &url,
        Rc::new(|body: String| {
            if parse_generators(&body) {
                op_editor_core::generator::install_generator_runner(run);
                crate::repaint_coalescer::request();
            }
        }),
    );
}

/// `generators` out of `GET /api/mcp/server`. An older daemon (no field)
/// has no run route.
fn parse_generators(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|parsed| parsed.get("generators")?.as_bool())
        .unwrap_or(false)
}

/// Once per frame: land answered runs, then send queued ones. Returns
/// whether the editor state changed.
pub(crate) fn pump(host: &mut crate::widget_host::WidgetHost) -> bool {
    let ready = CACHE.with(|cache| cache.borrow_mut().take_ready());
    let mut changed = false;
    for job in ready {
        let Some(id) = NodeId::new_opt(&job.wire.generator_id) else {
            continue;
        };
        // The runner answers this re-run from the cache, so the write goes
        // through the same command + one-undo-step path a native run does.
        let state = host.editor_state_mut();
        let _ =
            state.apply_remote_generator_result(&id, &job.wire.spec, installed_generator_runner());
        changed = true;
    }
    if changed {
        host.mark_editor_state_dirty();
    }
    send_queued();
    changed
}

fn send_queued() {
    let queued = CACHE.with(|cache| cache.borrow_mut().take_queued());
    for job in queued {
        let Ok(body) = serde_json::to_string(&job.wire) else {
            complete(job, 0, String::new());
            continue;
        };
        let url = crate::daemon_base::daemon_url(GENERATOR_RUN_ROUTE);
        let answered = job.clone();
        let started = crate::web_image_panel::post_json_with_timeout(
            &url,
            &body,
            RUN_TIMEOUT_MS,
            Rc::new(move |status, text| complete(answered.clone(), status, text)),
        );
        if !started {
            complete(job, 0, String::new());
        }
    }
}

fn complete(job: GeneratorJob, status: u16, body: String) {
    let reply = parse_generator_run_reply(status, &body);
    CACHE.with(|cache| cache.borrow_mut().complete(job, reply));
    crate::repaint_coalescer::request();
}

#[cfg(test)]
mod tests {
    use super::parse_generators;

    #[test]
    fn capability_is_read_from_the_server_info() {
        assert!(parse_generators(r#"{"running":true,"generators":true}"#));
        assert!(!parse_generators(r#"{"running":true,"generators":false}"#));
        assert!(!parse_generators(r#"{"running":true}"#), "older daemon");
        assert!(!parse_generators("not json"));
    }
}
