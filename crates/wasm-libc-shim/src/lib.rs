//! C-hard.2 — minimal libc / libcxx / libm shim for wasm32-unknown-unknown
//! Skia bundles (Step 1b §3.2 P0.5B Run path).
//!
//! The wasm32-unknown-unknown skia build produced by the
//! `vendor/skia-safe-op` fork imports ~61 `env.*` symbols at runtime
//! that wasm-bindgen does not synthesize. This crate provides those
//! symbols as Rust `extern "C"` functions so wasm-ld resolves them
//! inside the bundle and the wasm becomes loadable in browsers.
//!
//! Categories (counts after dedup against the actual import list):
//! - allocator:    malloc / free / calloc / realloc / malloc_usable_size
//!                 → dlmalloc-rs
//! - libm:         asinh / acosh / atanh / nextafterf / remainder
//!                 → libm crate
//! - libc string:  memchr / wmemchr / strcmp / strcpy / strtoull
//!                 → hand-rolled byte-wise impls
//! - libc stdio:   snprintf / vsnprintf / vfprintf
//!                 → C-side stubs (in `src/stdio_stub.c`) that route
//!                   into Rust extern `wasm_libc_shim_stdio_panic` and
//!                   panic with the symbol name. Skia's font printf
//!                   paths are NOT exercised on the raster +
//!                   custom_empty fontmgr pipeline (verified: 0 env.*
//!                   imports in the post-link bundle); a runtime panic
//!                   here would mean a new Skia call site landed and
//!                   needs a real impl, not a silent empty success.
//! - libc misc:    abort / __errno_location
//! - C++ ABI:      __cxa_atexit / __cxa_guard_acquire / __cxa_guard_release
//!                 / __cxa_pure_virtual
//! - operator new/delete (`_Znwm`/`_Znam`/`_ZdlPv*`/`_ZdaPv*`)
//!                 → forward to malloc/free
//! - threads:      sem_init / sem_destroy / sem_post / sem_wait
//!                 → no-op (single-threaded wasm)
//! - libcxx string / locale / iostream / shared_weak_count / to_string
//!                 → panic stubs; these symbols are linker-pulled by
//!                   templated code that the skia raster + empty
//!                   fontmgr pipeline does not exercise at runtime.
//!                   If any IS hit, the panic becomes a JS exception
//!                   via console_error_panic_hook.
//!
//! IMPORTANT: this crate must NOT be a Cargo workspace member of any
//! native (non-wasm) build — its symbols intentionally collide with
//! the host C library. Gate via `cfg(target_arch = "wasm32")` at the
//! consumer side and via `cfg(target_os = "unknown")` to avoid the
//! wasm32-unknown-emscripten path (which has its own libc).
//!
//! Tested with: rustc 1.85, dlmalloc 0.2, libm 0.2,
//! `--target wasm32-unknown-unknown`, vendor/skia-safe-op fork
//! commit (current working-tree, 2026-05-09).

#![no_std]
#![allow(non_snake_case, non_upper_case_globals)]

// Single-threaded wasm only. The errno backing storage and __cxa guard
// implementations use plain `static mut` and non-atomic loads/stores;
// wasm threads (atomics target feature) would race them. Block the
// build with a clear message instead of producing UB at runtime — the
// path forward is real TLS errno + atomic guard variables, which is
// a separate work item (Sub-phase C-hard.3 if/when wasm threads land).
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
compile_error!(
    "wasm-libc-shim assumes single-threaded wasm; the `atomics` target \
     feature requires real thread-safe impls (TLS errno, atomic C++ \
     guard variables) which this kill-spike crate intentionally omits."
);

// Active only on wasm32-unknown-unknown to avoid colliding with the
// host platform's libc / libcxx during native builds. The shim is a
// link-time static whose symbols overwrite C runtime symbols on
// purpose; on macOS / Linux / Windows that would break the world.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod imp;
