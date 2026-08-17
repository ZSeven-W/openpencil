# OpenHarmony cross-compilation

Toolchain plumbing for `crates/op-engine-napi` → `libopenpencil.so`, the
native module the ArkTS app in `packaging/harmony` loads.

| File | Role |
| --- | --- |
| `ohos-clang.sh` | The NDK clang/clang++ wrapper: adds `--target` + `--sysroot` and execs the real driver. |
| `ohos-clang-aarch64.sh` / `ohos-clang-x86_64.sh` | Per-triple linker drivers referenced by `.cargo/config.toml`. |
| `../build-ohos.sh` | The supported build entry point. |

```sh
export OHOS_NDK_HOME="$HOME/command-line-tools/sdk/default/openharmony"
scripts/build-ohos.sh                       # aarch64, release, --features gl,editor
OHOS_TARGET=x86_64-unknown-linux-ohos scripts/build-ohos.sh   # emulator lane
```

## Where OHOS_NDK_HOME should point

The OpenHarmony SDK lays the toolchain out as `<sdk>/openharmony/native/{llvm,sysroot}`.
**Point `OHOS_NDK_HOME` at the directory containing `native/`** — that is what
skia-bindings expects (`build_support/platform/ohos.rs` reads `OHOS_NDK_HOME`
and appends `native/` itself, or takes `ohos_sdk_native` as the already-resolved
`native` path).

Both the wrapper and `build-ohos.sh` also accept `OHOS_NDK_HOME` pointing
directly at `native/` — they probe for whichever level actually holds a
`sysroot` — and the build script then exports `ohos_sdk_native` so
skia-bindings agrees. The parent-directory form is still the one to use.

## The Skia story

This is the part that usually breaks, so here is exactly what is and is not
known, read out of the vendored `skia-bindings 0.97.0` sources in
`~/.cargo/registry` rather than assumed.

**Good news: skia-bindings has a first-class OpenHarmony platform.**
`build_support/platform.rs` dispatches `(_, "unknown", "linux", Some("ohos"))`
to `platform/ohos.rs`, which:

* reads the NDK from `ohos_sdk_native`, else `OHOS_NDK_HOME` + `native/`;
* passes `--sysroot=<ndk>/sysroot`, `--target=<triple>`, and `-D__MUSL__` to
  bindgen (OHOS ships a musl libc);
* sets GN args `skia_use_egl=true`, `skia_gl_standard="gles"`,
  `skia_use_gl=true`, and disables fontconfig / X11 / DNG / system
  freetype+libpng+libwebp;
* links `c++_static`, `c++abi`, and — with the `gl` feature — `EGL` and
  `GLESv3`.

So no hand-rolled GN arguments are needed. What the build DOES need from us:

* **`CC` / `CXX` must carry `--target=`.** `build_support/skia/config.rs`
  parses the `--target=` substring out of `CC` to decide the build target,
  preferring it over the cargo triple. `build-ohos.sh` therefore exports
  `CC="…/clang --target=aarch64-linux-ohos --sysroot=…"` as one string rather
  than splitting the flag into `CFLAGS`.
* **`CLANGCC` / `CLANGCXX` win over `CC` / `CXX`** in the same file (they exist
  for Yocto SDKs, where `CC` is gcc). The script sets all four to the same
  command so neither branch can pick up a host compiler.
* **`AR_<triple>`, not `[target.<triple>].ar`.** Cargo ignores the config key;
  the `cc` crate reads the env var. The script exports `AR`, `RANLIB`, and the
  triple-suffixed forms.

**Skia is compiled from source on this lane.** rust-skia's prebuilt release
archives (`skia-binaries-{key}.tar.gz`) are published per target+feature key,
and OHOS triples are not among them — so the `pinned-skia-binaries` feature
(`skia-safe/no-compile`) CANNOT be used for OHOS until an archive exists.
Expect a long first build and a `python3` dependency for Skia's GN bootstrap.
`SKIA_NINJA_COMMAND` / `SKIA_GN_COMMAND` override the vendored tools if the
bundled ones do not run on your host.

## The GL gap (the real blocker)

`--features gl` currently buys **nothing on OHOS**, and this is a genuine
functional gap, not a configuration mistake:

* `op-engine-ffi`'s `gl` feature forwards to `jian-skia/gl`, but the EGL
  surface backend is gated `#[cfg(all(target_os = "android", feature = "gl"))]`
  in `vendor/jian/crates/jian-skia/src/surface/mod.rs`, and its `khronos-egl`
  dependency sits under `[target.'cfg(target_os = "android")'.dependencies]`.
* `op-engine-ffi`'s `build_surface` (`src/lifecycle.rs`) is gated the same way
  and falls through to
  `OpStatus::NotReady` — *"no GPU surface backend compiled for this
  target/features"* — on OHOS.

So `attachSurface` will return `NotReady` (10) on a real device until the gate
is widened. Closing it is a **`vendor/jian` submodule change** plus a two-line
gate widening in `op-engine-ffi`, and is deliberately out of scope for the
in-repo OHOS work:

1. `jian-skia/Cargo.toml`: make `khronos-egl` available for
   `cfg(any(target_os = "android", target_env = "ohos"))`.
2. `jian-skia/src/surface/mod.rs`: widen the `egl_android` module gate the same
   way (the module's body is plain EGL over a `void*` native window — nothing
   in it is Android-specific beyond the name).
3. `op-engine-ffi/src/lifecycle.rs`: widen the two
   `all(feature = "gl", target_os = "android")` gates (`SurfaceSlot::Egl`
   construction and the `draw_frame` arm) to accept `target_env = "ohos"`.

Until then the OHOS module builds, loads, and drives everything except
presenting frames.

## UNVERIFIED-UNTIL-NDK

No OpenHarmony NDK is installed on the machine this was written on, so the
following are reasoned from vendored sources and public headers but have not
been executed:

* **The full `cargo build -p op-engine-napi --target aarch64-unknown-linux-ohos`.**
  Everything downstream of `skia-bindings`' build script is unexecuted. What
  HAS been verified here is the NAPI/NDK-facing Rust: every module in
  `op-engine-napi` type-checks and passes `clippy -D warnings` for
  `aarch64-unknown-linux-ohos` against stubbed `op-engine-ffi` /
  `op-engine-jni` crates (see the crate README).
* **NDK directory layout.** `llvm/bin/{clang,clang++,llvm-ar,llvm-ranlib}` and
  `sysroot/` under `native/` are assumed from the SDK layout skia-bindings
  encodes. `build-ohos.sh` fails loudly with the exact missing path rather
  than proceeding.
* **API level.** skia-bindings' own comment recommends API 12 as the minimum.
  Nothing here pins one; if the sysroot is older, expect link failures in
  Skia rather than in this crate.
* **Library names.** `libace_ndk.z.so` (XComponent) and `libhilog_ndk.z.so`
  (HiLog) are declared via `#[link(name = "ace_ndk.z")]` /
  `#[link(name = "hilog_ndk.z")]`. The symbol declarations themselves are
  transcribed from the upstream OpenHarmony headers, but the link names are
  unproven until a real link step runs.
* **`napi-ohos` 1.2.0 / `napi-derive-ohos` 1.2.0.** Verified to exist on
  crates.io and to compile for `aarch64-unknown-linux-ohos`; the resulting
  `.so` has never been loaded by an ArkTS runtime.
* **`x86_64-unknown-linux-ohos`.** Wired for the emulator but never built.
