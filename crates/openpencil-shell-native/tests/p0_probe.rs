//! P0 probe — verifications (2) and (3) per Step 1a spec §7.2.
//!
//! TRANSIENT: this test exists only to drive the three-OS CI matrix for the
//! P0 dep-stack probe gate. After CI is green and probe pin versions are
//! recorded in `openpencil-docs/superpowers/notes/2026-05-05-skia-glow-loader-compat-probe.md`,
//! this file plus `examples/p0_probe.rs` plus the matching dev-dep block in
//! `Cargo.toml` are reverted. Task 1 owns the permanent integration.
//!
//! Why this test shells out to `examples/p0_probe.rs` instead of running the
//! winit `EventLoop` inline: on macOS, winit refuses to construct an
//! `EventLoop` off the OS main thread (panics with "EventLoop must be
//! created on the main thread"). `cargo test` always runs `#[test]` fns on
//! a libtest worker thread. The cross-OS-portable workaround is to put the
//! `fn main()` driver in `examples/p0_probe.rs` (where rustc owns the real
//! main thread) and invoke it as a subprocess from each test. Each
//! `#[test]` subprocess gets a fresh process, side-stepping winit's
//! "EventLoop already created" guard for the second test.
//!
//! `#[ignore = "P0_PROBE_GATE"]` keeps the default `cargo test --workspace`
//! green; CI runs a separate `cargo test --workspace -- --ignored
//! P0_PROBE_GATE` step.

use std::process::Command;

fn run_example(arg: &str) {
    if cfg!(target_os = "windows") {
        // spec §8.2: standard GitHub Actions Windows runner has no GL driver
        // (WINDOWS_GPU_DEFERRED_NO_RUNNER); manual smoke required there.
        eprintln!("WINDOWS_GPU_DEFERRED_NO_RUNNER: skipping GL probe on Windows runner");
        return;
    }

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.args([
        "run",
        "--quiet",
        "-p",
        "openpencil-shell-native",
        "--example",
        "p0_probe",
        "--",
        arg,
    ]);
    let output = cmd
        .output()
        .expect("failed to spawn `cargo run --example p0_probe`");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "probe example `{arg}` failed (status {:?}):\n--- stderr ---\n{}\n--- stdout ---\n{}",
            output.status.code(),
            stderr,
            stdout
        );
    }
}

#[test]
#[ignore = "P0_PROBE_GATE"]
fn cross_api_gl_state_visibility() {
    run_example("stencil");
}

#[test]
#[ignore = "P0_PROBE_GATE"]
fn gpu_readback() {
    run_example("readback");
}
