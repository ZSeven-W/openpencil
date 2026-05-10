//! Implementation module for the wasm32-unknown-unknown libc/libcxx/libm shim.
//! See `lib.rs` for the high-level rationale and category breakdown.
//!
//! Split layout (per repo `openpencil/CLAUDE.md` 800-line ceiling):
//! - this `mod.rs` carries the original C-hard.2 surface (allocator
//!   + libm + libc string + libc misc + C++ ABI + operator new/delete +
//!   sem / threads + libcxx panic stubs)
//! - `step3` carries the Skia font-path additions (extra string ops,
//!   file I/O sentinels, mmap, setjmp/longjmp, `_ZnwmRKSt9nothrow_t`)
//!
//! Both files emit `#[no_mangle]` symbols at the same flat namespace —
//! wasm-ld resolves them identically regardless of which Rust module
//! they came from.

mod step3;

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------
// Allocator — dlmalloc-rs with a 16-byte size header per allocation so
// `free` and `realloc` can satisfy `GlobalAlloc`'s contract that the
// dealloc layout match the alloc layout. Without the header we'd be
// passing a placeholder size to `dealloc`, which is formally UB even
// though dlmalloc-rs ignores the size at runtime; storing the original
// size keeps us sound and unblocks a real `realloc` that copies
// `min(old_size, new_size)` instead of reading past the old block.
// ---------------------------------------------------------------------

const ALIGN: usize = 16;
// 16 bytes of overhead so the user pointer stays 16-byte aligned. Only
// the first `size_of::<usize>()` bytes are used to store the size; the
// remainder is padding. Keep ≥ 16 to preserve user alignment.
const HEADER_BYTES: usize = 16;

#[global_allocator]
static GLOBAL: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

// SAFETY: dlmalloc::malloc / free / etc. are documented as thread-safe via
// internal locking; our wasm32 single-threaded usage trivially satisfies
// any aliasing requirements. The size-header round-trip preserves the
// alloc/dealloc layout pair.
#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let total = match size.checked_add(HEADER_BYTES) {
        Some(t) => t,
        None => return core::ptr::null_mut(),
    };
    let layout = match core::alloc::Layout::from_size_align(total, ALIGN) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };
    let raw = alloc_compat::alloc(layout);
    if raw.is_null() {
        return core::ptr::null_mut();
    }
    *(raw as *mut usize) = size;
    raw.add(HEADER_BYTES).cast()
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let raw = (ptr as *mut u8).sub(HEADER_BYTES);
    let size = *(raw as *mut usize);
    let layout = core::alloc::Layout::from_size_align_unchecked(size + HEADER_BYTES, ALIGN);
    alloc_compat::dealloc(raw, layout);
}

#[no_mangle]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void {
    let total = match nmemb.checked_mul(size) {
        Some(t) if t > 0 => t,
        _ => return core::ptr::null_mut(),
    };
    let p = malloc(total);
    if !p.is_null() {
        core::ptr::write_bytes(p as *mut u8, 0, total);
    }
    p
}

#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    if ptr.is_null() {
        return malloc(new_size);
    }
    if new_size == 0 {
        free(ptr);
        return core::ptr::null_mut();
    }
    let raw = (ptr as *mut u8).sub(HEADER_BYTES);
    let old_size = *(raw as *mut usize);
    let new = malloc(new_size);
    if new.is_null() {
        return core::ptr::null_mut();
    }
    let copy = if old_size < new_size { old_size } else { new_size };
    core::ptr::copy_nonoverlapping(ptr.cast::<u8>(), new.cast::<u8>(), copy);
    free(ptr);
    new
}

#[no_mangle]
pub unsafe extern "C" fn malloc_usable_size(ptr: *mut c_void) -> usize {
    // Now that malloc stores the original size in a 16-byte header we
    // can return the exact request size. Skia's call site uses this
    // for TLS arena budgeting; surfacing the truth is strictly better
    // than the conservative 0 we returned previously.
    if ptr.is_null() {
        return 0;
    }
    let raw = (ptr as *mut u8).sub(HEADER_BYTES);
    *(raw as *mut usize)
}

mod alloc_compat {
    //! Re-exposes core::alloc::GlobalAlloc methods without an extra
    //! crate dep — dlmalloc-rs implements the trait on
    //! `dlmalloc::GlobalDlmalloc` so these forward through.
    use core::alloc::{GlobalAlloc, Layout};

    pub unsafe fn alloc(layout: Layout) -> *mut u8 {
        super::GLOBAL.alloc(layout)
    }
    pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
        super::GLOBAL.dealloc(ptr, layout);
    }
}

// ---------------------------------------------------------------------
// libm — pure Rust math.
// ---------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn asinh(x: f64) -> f64 {
    libm::asinh(x)
}

#[no_mangle]
pub extern "C" fn acosh(x: f64) -> f64 {
    libm::acosh(x)
}

#[no_mangle]
pub extern "C" fn atanh(x: f64) -> f64 {
    libm::atanh(x)
}

#[no_mangle]
pub extern "C" fn nextafterf(x: f32, y: f32) -> f32 {
    libm::nextafterf(x, y)
}

#[no_mangle]
pub extern "C" fn remainder(x: f64, y: f64) -> f64 {
    libm::remainder(x, y)
}

// ---------------------------------------------------------------------
// libc string — byte-wise impls.
// ---------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void {
    let bytes = core::slice::from_raw_parts(s.cast::<u8>(), n);
    match bytes.iter().position(|&b| b == c as u8) {
        Some(i) => (s as *mut u8).add(i).cast(),
        None => core::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wmemchr(
    s: *const u32, // wchar_t = i32 on most platforms; we use u32 for byte-pattern equality
    c: u32,
    n: usize,
) -> *mut u32 {
    let chars = core::slice::from_raw_parts(s, n);
    match chars.iter().position(|&ch| ch == c) {
        Some(i) => (s as *mut u32).add(i),
        None => core::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn strcmp(a: *const c_char, b: *const c_char) -> c_int {
    let mut i = 0isize;
    loop {
        let ca = *a.offset(i) as u8;
        let cb = *b.offset(i) as u8;
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    let mut i = 0isize;
    loop {
        let c = *src.offset(i);
        *dst.offset(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strtoull(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> u64 {
    // Skip leading whitespace.
    let mut p = nptr;
    while is_space(*p) {
        p = p.add(1);
    }
    // Optional sign — strtoull ignores '+' / '-' per spec but treats '-'
    // as wrap-around. We match: '+' OK, '-' wraps.
    let mut neg = false;
    if *p == b'+' as c_char {
        p = p.add(1);
    } else if *p == b'-' as c_char {
        neg = true;
        p = p.add(1);
    }
    // Auto-detect base if 0.
    let mut effective_base = base;
    if effective_base == 0 {
        if *p == b'0' as c_char && (*p.add(1) == b'x' as c_char || *p.add(1) == b'X' as c_char) {
            effective_base = 16;
            p = p.add(2);
        } else if *p == b'0' as c_char {
            effective_base = 8;
            p = p.add(1);
        } else {
            effective_base = 10;
        }
    } else if effective_base == 16
        && *p == b'0' as c_char
        && (*p.add(1) == b'x' as c_char || *p.add(1) == b'X' as c_char)
    {
        p = p.add(2);
    }
    let mut acc: u64 = 0;
    loop {
        let c = *p as u8;
        let digit = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'z' => c - b'a' + 10,
            b'A'..=b'Z' => c - b'A' + 10,
            _ => break,
        };
        if digit as c_int >= effective_base {
            break;
        }
        acc = acc.wrapping_mul(effective_base as u64);
        acc = acc.wrapping_add(digit as u64);
        p = p.add(1);
    }
    if !endptr.is_null() {
        *endptr = p as *mut c_char;
    }
    if neg {
        0u64.wrapping_sub(acc)
    } else {
        acc
    }
}

// `pub(super)` so `imp::step3::strtol` can reuse it without
// re-implementing the whitespace check.
pub(super) unsafe fn is_space(c: c_char) -> bool {
    matches!(c as u8, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

// ---------------------------------------------------------------------
// libc stdio — variadic stubs implemented in C (src/stdio_stub.c) and
// linked via build.rs + cc crate. snprintf / vsnprintf / vfprintf are
// declared there with proper C-variadic ABI; before returning they
// call `wasm_libc_shim_stdio_panic` below so any actual invocation is
// loud (per-codex Round 1 C4 — we previously returned an empty success
// which masked unexpected printf paths in the Skia raster pipeline).
// ---------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn wasm_libc_shim_stdio_panic(name: *const c_char) -> ! {
    // Walk the C string up to a sane bound so a corrupt pointer does
    // not turn the diagnostic into a second crash. 64 bytes is enough
    // for any of `snprintf` / `vsnprintf` / `vfprintf`.
    let mut len = 0usize;
    while len < 64 && *name.add(len) != 0 {
        len += 1;
    }
    let bytes = core::slice::from_raw_parts(name as *const u8, len);
    let label = core::str::from_utf8_unchecked(bytes);
    panic!(
        "wasm-libc-shim: unimplemented stdio stub `{}` called — \
         add a real impl or remove the call site (Skia raster + \
         custom_empty fontmgr was supposed to never reach here)",
        label
    );
}

// libc misc.
// ---------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn abort() -> ! {
    // Routes through std's panic handler → `console_error_panic_hook`
    // (installed by shell-web's mount entry) so the developer sees a
    // proper stack + symbol name instead of an opaque "RuntimeError:
    // unreachable" wasm trap.
    panic!("wasm-libc-shim: abort() called");
}

// errno backing storage. Single-threaded wasm — no TLS needed. Storing
// via `static mut` is safe because the wasm host is single-threaded;
// tighten via #[thread_local] when wasm threads land.
static mut ERRNO: c_int = 0;

#[no_mangle]
pub unsafe extern "C" fn __errno_location() -> *mut c_int {
    &raw mut ERRNO
}

// ---------------------------------------------------------------------
// C++ ABI — __cxa_*.
// ---------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn __cxa_atexit(
    _func: extern "C" fn(*mut c_void),
    _arg: *mut c_void,
    _dso_handle: *mut c_void,
) -> c_int {
    // Wasm has no atexit semantics; return 0 success.
    0
}

#[no_mangle]
pub unsafe extern "C" fn __cxa_guard_acquire(guard: *mut u8) -> c_int {
    // C++ guard variable: byte 0 = init flag, byte 1 = in-progress.
    // Single-threaded: if init flag is set, return 0 (skip init); else
    // mark in-progress + return 1 (run init).
    if *guard != 0 {
        return 0;
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn __cxa_guard_release(guard: *mut u8) {
    *guard = 1;
}

#[no_mangle]
pub extern "C" fn __cxa_pure_virtual() -> ! {
    panic!("wasm-libc-shim: __cxa_pure_virtual called (pure-virtual method invoked on a partially-constructed C++ object)");
}

// ---------------------------------------------------------------------
// C++ operator new / delete — forward to malloc / free.
// _Znwm = operator new(size_t)
// _Znam = operator new[](size_t)
// _ZdlPv  = operator delete(void*)
// _ZdlPvm = operator delete(void*, size_t)
// _ZdaPv  = operator delete[](void*)
// _ZdaPvm = operator delete[](void*, size_t)
// ---------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _Znwm(size: usize) -> *mut c_void {
    // C++ guarantees `operator new(0)` returns a non-null pointer that
    // is distinct from any other live allocation. Our `malloc(0)`
    // returns null, so route through `malloc(1)` for the zero case.
    // (C++ allows the standard library to call `new(0)` for empty
    // containers' end-iterator sentinels and similar zero-size objects.)
    let p = malloc(size.max(1));
    if p.is_null() {
        // -fno-exceptions skia cannot throw std::bad_alloc; abort is the
        // documented fallback. Routes through panic for diagnostic context.
        abort();
    }
    p
}

#[no_mangle]
pub unsafe extern "C" fn _Znam(size: usize) -> *mut c_void {
    _Znwm(size)
}

#[no_mangle]
pub unsafe extern "C" fn _ZdlPv(ptr: *mut c_void) {
    free(ptr);
}

#[no_mangle]
pub unsafe extern "C" fn _ZdlPvm(ptr: *mut c_void, _size: usize) {
    free(ptr);
}

#[no_mangle]
pub unsafe extern "C" fn _ZdaPv(ptr: *mut c_void) {
    free(ptr);
}

#[no_mangle]
pub unsafe extern "C" fn _ZdaPvm(ptr: *mut c_void, _size: usize) {
    free(ptr);
}

// ---------------------------------------------------------------------
// Threads — semaphore no-ops (single-threaded wasm).
// ---------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn sem_init(_sem: *mut c_void, _pshared: c_int, _value: u32) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn sem_destroy(_sem: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn sem_post(_sem: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn sem_wait(_sem: *mut c_void) -> c_int {
    0
}

// ---------------------------------------------------------------------
// libcxx panic stubs.
//
// All ~23 mangled symbols below (basic_string ops, locale, ios_base,
// basic_iostream, shared_weak_count, to_string) are linker-pulled by
// templated code that the skia raster + custom_empty fontmgr pipeline
// does not actually exercise at runtime. Stubbing as `unreachable!`
// keeps the wasm linker happy; if a runtime path DOES hit one of
// these, the panic surfaces as a JS exception via console_error_panic_
// hook (installed by shell-web's mount entry).
//
// Future C-hard.3 may replace specific stubs with real implementations
// if a widget path needs the underlying functionality.
// ---------------------------------------------------------------------

macro_rules! libcxx_stub {
    ($name:ident) => {
        #[no_mangle]
        pub extern "C" fn $name() -> ! {
            // `panic!` routes through std's panic handler →
            // `console_error_panic_hook` (installed by shell-web's
            // mount entry) which prints the message + a JS stack trace
            // to the browser console. This is strictly better than a
            // bare `wasm32::unreachable()` trap for diagnosing which
            // libcxx call site is actually reaching the stub.
            panic!(
                "wasm-libc-shim: unimplemented libcxx stub `{}` called — \
                 a Skia / libcxx code path needs a real impl",
                stringify!($name)
            );
        }
    };
    ($($name:ident),+ $(,)?) => {
        $(libcxx_stub!($name);)+
    };
}

libcxx_stub!(
    // basic_string<char,...> instance methods
    _ZNSt3__212basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEE17__assign_no_aliasILb1EEERS5_PKcm,
    _ZNSt3__212basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEE25__init_copy_ctor_externalEPKcm,
    _ZNSt3__212basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEE6appendEmc,
    _ZNSt3__212basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEE6appendEPKcm,
    _ZNSt3__212basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEE6insertEmPKcm,
    _ZNSt3__212basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEE9push_backEc,
    // istream / ostream
    _ZNSt3__213basic_istreamIcNS_11char_traitsIcEEE6sentryC1ERS3_b,
    _ZNSt3__213basic_istreamIcNS_11char_traitsIcEEErsERd,
    _ZNSt3__213basic_istreamIcNS_11char_traitsIcEEErsERf,
    _ZNSt3__213basic_ostreamIcNS_11char_traitsIcEEElsEf,
    _ZNSt3__214basic_iostreamIcNS_11char_traitsIcEEED2Ev,
    _ZNSt3__29basic_iosIcNS_11char_traitsIcEEED2Ev,
    // locale
    _ZNSt3__26locale7classicEv,
    _ZNSt3__26localeaSERKS0_,
    _ZNSt3__26localeC1ERKS0_,
    _ZNSt3__26localeC1Ev,
    _ZNSt3__26localeD1Ev,
    // ios_base
    _ZNSt3__28ios_base4initEPv,
    _ZNSt3__28ios_base5clearEj,
    _ZNSt3__28ios_base5imbueERKNS_6localeE,
    _ZNKSt3__28ios_base6getlocEv,
    // shared_ptr internals (control block)
    _ZNSt3__219__shared_weak_count14__release_weakEv,
    _ZNSt3__219__shared_weak_countD2Ev,
    _ZNKSt3__219__shared_weak_count13__get_deleterERKSt9type_info,
    // to_string (int / unsigned long / long long)
    _ZNSt3__29to_stringEi,
    _ZNSt3__29to_stringEm,
    _ZNSt3__29to_stringEx,
);
