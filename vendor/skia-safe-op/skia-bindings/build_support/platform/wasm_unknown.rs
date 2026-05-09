//! `wasm32-unknown-unknown` platform — the OP "C-hard" path.
//!
//! Step 1b §2.2 P0 verdict (lock-in 2026-05-08) selected
//! `wasm32-unknown-emscripten` because Skia C++ needs libc/libc++ that
//! `wasm32-unknown-unknown` does not ship. However, the emscripten
//! target's wasm-bindgen output is incompatible with browser ES modules
//! (wasm-bindgen --target=web on emscripten-target wasm emits emscripten-
//! library glue, not a browser-loadable loader). The OP-side decision
//! (2026-05-09) is to fork skia-bindings, recompile Skia C++ directly
//! into `wasm32-unknown-unknown` ABI by:
//!
//!   - reusing emsdk's libc / libcxx headers via `-isystem`
//!   - dropping C++ exceptions + RTTI (`-fno-exceptions -fno-rtti`)
//!     so the resulting `.o` files do not import emscripten's
//!     exception runtime (`__THREW__`, `__wasm_setjmp`, etc.)
//!   - forcing clang's target via `--target=wasm32-unknown-unknown`
//!
//! The probe at `/tmp/skia-c-probe/test2.cpp` (2026-05-09) compiled
//! SkPaint + std::string + std::vector with this exact flag set into
//! a wasm32-unknown-unknown `.o` whose only undefs were:
//!   - libcxx allocator (`_Znwm`/`_ZdlPvm`, `__libcpp_verbose_abort`)
//!   - `__stack_pointer` (wasm-ld provides)
//!   - inter-skia name-mangled refs (resolved at static-link time)
//!
//! The wasm32-unknown-unknown bundle then links normally with
//! wasm-bindgen on the Rust side; runtime requires a libc/libcxx shim
//! crate (TODO `vendor/wasm-libc-shim`) to satisfy `_Znwm`/`_ZdlPvm` /
//! `__libcpp_verbose_abort` / a small libm subset. That shim is
//! Sub-phase C-hard.2.

use std::path::Path;

use super::prelude::*;

pub struct WasmUnknown;

impl PlatformDetails for WasmUnknown {
    fn uses_freetype(&self) -> bool {
        // Freetype path requires libc fopen/fread for font files, which
        // the libc shim does not provide; fall back to the custom empty
        // font manager (same approach as the emscripten platform) so
        // typeface creation still works without disk I/O.
        true
    }

    fn gn_args(&self, _config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        // Skia's GN target_cpu=wasm runs `skia/bin/activate-emsdk` to
        // bootstrap a parallel emsdk install under skia/third_party/
        // externals/emsdk. The OP C-hard path uses emsdk's clang
        // directly via the toolchain overrides below and does NOT need
        // a second emsdk inside skia/. Overwrite the upstream
        // activate-emsdk script with a no-op stub on every build so
        // (a) a fresh clone whose skia/ tree was just unpacked from
        // the tarball is patched before GN runs, and (b) an existing
        // skia/ tree that lost the stub for any reason gets it back.
        install_activate_emsdk_stub();

        let emsdk_base_dir = emsdk_base_dir();
        let llvm_bin = format!("{emsdk_base_dir}/upstream/emscripten/llvm/bin");
        let clang = format!("{llvm_bin}/clang");
        let clangxx = format!("{llvm_bin}/clang++");
        let llvm_ar = format!("{llvm_bin}/llvm-ar");

        builder
            .arg("target_cpu", quote("wasm"))
            .arg("skia_gl_standard", quote("webgl"))
            // GPU stays off for the C-hard kill-spike: WebGL2 backend
            // pulls more libc imports (file paths for shader cache,
            // pthread for queue submission) than the raster fallback.
            // Phase A round 2 / Phase B+ may revisit once raster ships.
            .arg("skia_use_webgl", no())
            // Use the custom empty font manager, no embedded fonts;
            // application supplies fonts via SkData buffers in widget
            // code (taffy + jian-skia textlayout path).
            .arg("skia_enable_fontmgr_custom_embedded", no())
            .arg("skia_enable_fontmgr_custom_empty", yes())
            // Skia's default GN toolchain uses the host system clang
            // which does not bundle a wasm32 LLVM target. Point cc/cxx
            // at emsdk's bundled clang (it has wasm32 + libcxx headers
            // built in).
            .arg("cc", quote(&clang))
            .arg("cxx", quote(&clangxx))
            .arg("ar", quote(&llvm_ar))
            // C-hard core: drop emscripten runtime imports.
            .cflag("-fno-exceptions")
            .cflag("-fno-rtti")
            // emsdk's clang running on macOS host auto-defines __APPLE__,
            // which makes Skia source try to pull <dispatch/dispatch.h>
            // (Grand Central Dispatch). Undefine so SkSemaphore et al
            // fall back to the wasm-friendly atomics path. SK_BUILD_FOR_
            // EMSCRIPTEN tells Skia source to take the emscripten code
            // path even though we're not linking emscripten runtime —
            // the impls there are wasm-32-friendly atomic / no-op stubs.
            .cflag("-U__APPLE__")
            .cflag("-D__EMSCRIPTEN__")
            .cflag("-DSK_BUILD_FOR_UNIX");

        // emsdk header path — same shim user installed for the
        // emscripten target build (per spec §2.2 P0 verdict).
        let sysroot = format!("{emsdk_base_dir}/upstream/emscripten/cache/sysroot");
        let libcxx_include = format!("{sysroot}/include/c++/v1");
        let sysroot_include = format!("{sysroot}/include");

        if Path::new(&libcxx_include).is_dir() {
            builder.cflag(format!("-isystem{libcxx_include}"));
        }
        if Path::new(&sysroot_include).is_dir() {
            builder.cflag(format!("-isystem{sysroot_include}"));
            builder.cflag(format!("--sysroot={sysroot}"));
        }
        // Newer libcxx pulls `xlocale.h` from the compat shim; mirror the
        // emscripten platform module's include order so `<vector>` etc.
        // resolve cleanly.
        let compat_include = format!("{sysroot}/include/compat");
        if Path::new(&compat_include).is_dir() {
            builder.cflag(format!("-isystem{compat_include}"));
        }

        // Override the linker target — Skia GN normally picks emscripten's
        // wasm32-unknown-emscripten when target_cpu=wasm, but C-hard wants
        // pure wasm32-unknown-unknown so the .o files do not pick up the
        // emscripten runtime imports. Note: target() sets `--target=` on
        // both clang and gcc-style invocations issued by gn/ninja.
        builder.target(Some("wasm32-unknown-unknown".to_string()));
    }

    fn bindgen_args(&self, _target: &cargo::Target, builder: &mut BindgenArgsBuilder) {
        builder.arg("-nobuiltininc");
        builder.arg("-fvisibility=default");
        builder.arg("-fno-exceptions");
        builder.arg("-fno-rtti");
        builder.arg("-U__APPLE__");
        builder.arg("-D__EMSCRIPTEN__");
        builder.override_target("wasm32-unknown-unknown");

        let emsdk_base_dir = emsdk_base_dir();
        let sysroot = format!("{emsdk_base_dir}/upstream/emscripten/cache/sysroot");
        let libcxx_include = format!("{sysroot}/include/c++/v1");
        let sysroot_include = format!("{sysroot}/include");
        let compat_include = format!("{sysroot}/include/compat");

        if Path::new(&libcxx_include).is_dir() {
            builder.arg(format!("-isystem{libcxx_include}"));
        }
        if Path::new(&sysroot_include).is_dir() {
            builder.arg(format!("-isystem{sysroot_include}"));
            builder.set_sysroot(sysroot);
        }
        if Path::new(&compat_include).is_dir() {
            builder.arg(format!("-isystem{compat_include}"));
        }

        // The skia-bindings FFI shim (binding_sources) is compiled by the
        // `cc` crate, NOT Skia's GN. cc crate picks the host compiler by
        // default, which on macOS is system clang lacking wasm32 LLVM
        // backend. Set CC_wasm32_unknown_unknown / CXX_wasm32_unknown_
        // unknown so cc::Build uses emsdk's bundled clang (which has
        // wasm32 + the libcxx headers we already configured above).
        let llvm_bin = format!("{}/upstream/emscripten/llvm/bin", emsdk_base_dir);
        std::env::set_var("CC_wasm32_unknown_unknown", format!("{llvm_bin}/clang"));
        std::env::set_var("CXX_wasm32_unknown_unknown", format!("{llvm_bin}/clang++"));
        std::env::set_var("AR_wasm32_unknown_unknown", format!("{llvm_bin}/llvm-ar"));
    }

    fn link_libraries(&self, _features: &Features) -> Vec<String> {
        // The libc shim crate (Sub-phase C-hard.2) provides operator
        // new/delete + libm; we do NOT pull system libraries here. The
        // wasm-ld step links the static .a files directly.
        Vec::new()
    }

    fn filter_platform_features(
        &self,
        _use_system_libraries: bool,
        mut features: Features,
    ) -> Features {
        features += feature::EMBED_FREETYPE;
        features
    }
}

fn emsdk_base_dir() -> String {
    match std::env::var("EMSDK") {
        Ok(val) => val,
        Err(_e) => panic!(
            "wasm32-unknown-unknown skia build requires the EMSDK env var \
             pointing at an emsdk install (used only for libc / libcxx \
             headers; no emscripten runtime is linked into the resulting \
             static archive)"
        ),
    }
}

/// Inline source of truth for the activate-emsdk no-op stub. Embedded
/// here (rather than committed at `skia/bin/activate-emsdk`) because the
/// `skia/` source tree is `.gitignore`d — it is fetched at build time
/// from rust-skia/skia.tar.gz and would overwrite any committed stub.
const ACTIVATE_EMSDK_STUB: &str = "#!/usr/bin/env python3\n\
    # OP C-hard fork override: Skia GN's `target_cpu=wasm` runs this script\n\
    # to install + activate emsdk under skia/third_party/externals/emsdk.\n\
    # The OP C-hard path uses emsdk's clang directly (see\n\
    # build_support/platform/wasm_unknown.rs) and does NOT need Skia to\n\
    # bootstrap a parallel emsdk install. This stub exits 0 so the GN\n\
    # emsdk_done.stamp action passes.\n\
    import sys\n\
    sys.exit(0)\n";

/// Overwrite `<skia-bindings crate>/skia/bin/activate-emsdk` with the
/// no-op stub. Idempotent and best-effort: if `skia/` does not exist
/// yet (binary-cache hasn't run) we do nothing — `gn_args` runs after
/// the source download in build.rs, so by the time GN actually invokes
/// the stub it is in place.
fn install_activate_emsdk_stub() {
    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(v) => v,
        Err(_) => return,
    };
    let stub_path = std::path::PathBuf::from(manifest_dir).join("skia/bin/activate-emsdk");
    let parent = match stub_path.parent() {
        Some(p) => p,
        None => return,
    };
    if !parent.exists() {
        // skia/ has not been unpacked yet; binary-cache will populate
        // it later. The next gn_args invocation (full build) will
        // re-run this and find the directory.
        return;
    }
    let _ = std::fs::write(&stub_path, ACTIVATE_EMSDK_STUB);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&stub_path, std::fs::Permissions::from_mode(0o755));
    }
}
