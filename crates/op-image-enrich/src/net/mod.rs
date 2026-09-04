//! Networked image resolution (feature `net`): the provider ladder
//! (Openverse → two-keyword retry → Wikimedia), the first-usable-image
//! fetch with skia down-scaling, and the shared blocking bridge.
//!
//! Moved here (pure code motion) from `op-host-services/src/web_image_search.rs`
//! (providers) and `op-host-desktop/src/{image_search_session/fetch.rs,
//! image_downscale.rs}` — both re-export under their old paths — so the
//! mobile FFI host resolves design image slots through the exact desktop
//! ladder instead of a drifting port.

pub mod downscale;
pub mod fetch;
pub mod providers;

/// Drive one image job's future to completion from a synchronous worker
/// thread. A dedicated small runtime rather than a per-call one: image jobs
/// run on plain `std::thread` workers (never inside another runtime), and a
/// shared reactor keeps reqwest's connection pool warm across jobs.
pub fn block_on_image_runtime<F: std::future::Future>(fut: F) -> F::Output {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .thread_name("op-image-fetch")
                .build()
                .expect("image fetch runtime build")
        })
        .block_on(fut)
}
