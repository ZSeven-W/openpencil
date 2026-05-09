/*
 * C-hard.2 stdio stubs for wasm32-unknown-unknown skia bundles.
 *
 * Skia uses snprintf/vsnprintf/vfprintf for SkString::printf style
 * formatting (e.g. shader debug logs, font path debug). On the raster
 * + custom_empty fontmgr pipeline none of these are exercised in
 * normal rendering — verified by running the post-link bundle and
 * confirming 0 env.* imports. If a path IS actually hit at runtime
 * we want to FAIL FAST with the symbol name in the panic message
 * rather than silently returning a successful empty string (codex
 * Round 1 C4: silent empty masked unexpected printf reachability).
 *
 * Variadic in stable Rust requires the `c_variadic` nightly feature.
 * Doing this in C keeps the wasm-libc-shim crate stable-compatible;
 * the Rust side (`imp.rs::wasm_libc_shim_stdio_panic`) does the
 * actual panic with `console_error_panic_hook` integration.
 */

#include <stddef.h>
#include <stdarg.h>

extern void wasm_libc_shim_stdio_panic(const char *name) __attribute__((noreturn));

int snprintf(char *buf, size_t size, const char *fmt, ...) {
    (void)fmt;
    if (buf && size > 0) {
        buf[0] = 0;
    }
    wasm_libc_shim_stdio_panic("snprintf");
}

int vsnprintf(char *buf, size_t size, const char *fmt, va_list ap) {
    (void)fmt;
    (void)ap;
    if (buf && size > 0) {
        buf[0] = 0;
    }
    wasm_libc_shim_stdio_panic("vsnprintf");
}

int vfprintf(void *stream, const char *fmt, va_list ap) {
    (void)stream;
    (void)fmt;
    (void)ap;
    wasm_libc_shim_stdio_panic("vfprintf");
}
