//! The process-wide chat tokio runtime + the sanctioned sync→async bridge —
//! moved verbatim from `op-host-services/src/chat_runtime.rs`, which
//! re-exports both under their original paths
//! (`op_host_services::chat_runtime::{shared_runtime, block_on_anywhere}`).

use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};

/// Process-wide tokio runtime used for every BuiltIn chat turn. We
/// own a single multi-thread runtime instead of spinning one up per
/// provider so abort controllers + reqwest connection pools stay
/// shared. Initialized lazily on first chat send so cold startup
/// (open file menu, draw chrome) doesn't pay the spawn cost.
pub fn shared_runtime() -> &'static Runtime {
    static RUNTIME: std::sync::OnceLock<Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .enable_all()
            .thread_name("op-chat")
            .build()
            .expect("chat runtime build")
    })
}

/// Drive `fut` to completion from a **synchronous** function, wherever that
/// function happens to be called from.
///
/// The workspace is full of sync entry points (probe workers, widget-host
/// pumps, orchestrator worker threads) that need one `async` call. Writing
/// `shared_runtime().block_on(fut)` there is a latent panic: the moment such a
/// function is reached from inside a tokio worker, tokio aborts with
/// *"Cannot start a runtime from within a runtime"*. This helper is the one
/// sanctioned bridge; prefer it over any bare `Runtime::block_on` in a sync fn.
///
/// # Contract
///
/// * **No ambient runtime** (a plain `std::thread` worker, `main`, a test) —
///   the future runs on the process-wide [`shared_runtime`]. Unchanged from
///   the historical behavior.
/// * **Ambient multi-thread runtime** — [`tokio::task::block_in_place`] hands
///   this worker's queued tasks to a sibling worker and *exits* the runtime
///   context, so the captured [`Handle`]'s `block_on` is legal and no other
///   task on the runtime is starved while we block. The future keeps running
///   on the ambient reactor, so any IO/timer it created stays valid.
/// * **Ambient current-thread runtime** — panics with an actionable message.
///   There is no sound rescue: `block_in_place` is rejected outright by tokio
///   on that flavor, and blocking the scheduler's *only* thread with a foreign
///   executor (`futures::executor::block_on`) parks its IO/timer driver, so
///   any future doing real IO would hang forever. Failing loudly at the call
///   site beats a silent deadlock. Callers that genuinely run on a
///   current-thread runtime must `.await` instead of reaching for this bridge.
///
/// Deliberately unbounded by `Send` / `'static`: several call sites (the
/// orchestrator's `RemoteDocSink` runs, which hold `&dyn` trait objects across
/// the await points) pass borrowing, non-`Send` futures, so the helper can
/// never offload work to another thread — it always blocks the caller.
#[track_caller]
pub fn block_on_anywhere<F: std::future::Future>(fut: F) -> F::Output {
    match Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::CurrentThread => {
            panic!(
                "block_on_anywhere was called from a current-thread tokio runtime; \
                 blocking its only worker would park the IO/timer driver. \
                 Await the future directly instead."
            )
        }
        // Multi-thread runtime: shed the worker, then block on the ambient
        // handle (block_in_place has exited the runtime context for us).
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
        // No runtime on this thread — the plain, historical path.
        Err(_) => shared_runtime().block_on(fut),
    }
}
