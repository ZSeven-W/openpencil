// Compile the C-variadic stdio stubs for wasm32-unknown-unknown.
// On any other target this is a no-op so the crate links empty.

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target != "wasm32-unknown-unknown" {
        return;
    }

    let emsdk = match std::env::var("EMSDK") {
        Ok(v) => v,
        Err(_) => panic!(
            "wasm-libc-shim requires the EMSDK env var pointing at an emsdk install \
             (used only for libcxx headers + emsdk's bundled wasm32-aware clang)"
        ),
    };
    let llvm_bin = format!("{emsdk}/upstream/emscripten/llvm/bin");
    let sysroot = format!("{emsdk}/upstream/emscripten/cache/sysroot");

    let mut build = cc::Build::new();
    build
        .compiler(format!("{llvm_bin}/clang"))
        .archiver(format!("{llvm_bin}/llvm-ar"))
        .file("src/stdio_stub.c")
        .flag(format!("--sysroot={sysroot}"))
        .flag(format!("-isystem{sysroot}/include"))
        .flag(format!("-isystem{sysroot}/include/compat"))
        .flag("--target=wasm32-unknown-unknown")
        .flag("-fvisibility=default")
        .flag("-fno-exceptions")
        .flag("-O2")
        .opt_level(2)
        .warnings(false);
    build.compile("wasm_libc_shim_stdio");
    println!("cargo:rerun-if-changed=src/stdio_stub.c");
    println!("cargo:rerun-if-env-changed=EMSDK");
}
