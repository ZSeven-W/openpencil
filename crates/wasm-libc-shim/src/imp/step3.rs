//! Step 3 wasm-libc-shim additions — extracted from `imp/mod.rs`
//! to keep that file under the repo's 800-line ceiling
//! (`openpencil/CLAUDE.md` "Single files must not exceed 800
//! lines"). Functionally a continuation of `mod.rs`: every symbol
//! here is `#[no_mangle] pub extern "C"` so wasm-ld resolves them
//! the same way as the original C-hard.2 shim entries.
//!
//! Categories — Skia font path imports (FontMgr::custom_empty +
//! new_from_data + Canvas::draw_str). All called by Skia /
//! freetype internals; for our happy path (in-memory TTF, no
//! disk font lookup, no error path) most of these never actually
//! fire — they exist as linker references. We provide minimum-
//! viable behaviour: file I/O returns sentinel error values, env
//! vars are absent, mmap fails, setjmp returns 0, longjmp panics
//! if ever reached.

use core::ffi::{c_char, c_int, c_long, c_void};

// `is_space` lives in `imp/mod.rs` (private) and is shared with
// `strtoull` there; we re-use it for `strtol` here.
use super::is_space;

// --- string ops ---
#[no_mangle]
pub unsafe extern "C" fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int {
    for i in 0..n {
        let ca = *a.add(i) as u8;
        let cb = *b.add(i) as u8;
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    let mut i = 0;
    let mut copying = true;
    while i < n {
        if copying {
            let c = *src.add(i);
            *dst.add(i) = c;
            if c == 0 {
                copying = false;
            }
        } else {
            *dst.add(i) = 0;
        }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char {
    if *needle == 0 {
        return haystack as *mut c_char;
    }
    let mut h = haystack;
    while *h != 0 {
        let mut a = h;
        let mut b = needle;
        while *a != 0 && *b != 0 && *a == *b {
            a = a.add(1);
            b = b.add(1);
        }
        if *b == 0 {
            return h as *mut c_char;
        }
        h = h.add(1);
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn strrchr(s: *const c_char, c: c_int) -> *mut c_char {
    let target = c as u8;
    let mut last: *mut c_char = core::ptr::null_mut();
    let mut p = s;
    loop {
        let cur = *p as u8;
        if cur == target {
            last = p as *mut c_char;
        }
        if cur == 0 {
            return last;
        }
        p = p.add(1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn strcat(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    let mut d = dst;
    while *d != 0 {
        d = d.add(1);
    }
    let mut s = src;
    loop {
        let c = *s;
        *d = c;
        if c == 0 {
            return dst;
        }
        d = d.add(1);
        s = s.add(1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn strtol(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> c_long {
    // Returns `c_long`, NOT `i64` (codex Step 3 R1 CONCERN —
    // wasm32-unknown-unknown sizes `long` as 32-bit, so an i64
    // return triggered a `signature_mismatch:strtol` trap stub
    // in wasm-ld). Mirrors strtoull but signed; explicit `0x`
    // prefix accepted when `base == 16` (codex NIT-2).
    let mut p = nptr;
    while is_space(*p) {
        p = p.add(1);
    }
    let mut neg = false;
    if *p == b'+' as c_char {
        p = p.add(1);
    } else if *p == b'-' as c_char {
        neg = true;
        p = p.add(1);
    }
    let mut effective_base = base;
    if effective_base == 0 {
        if *p == b'0' as c_char
            && (*p.add(1) == b'x' as c_char || *p.add(1) == b'X' as c_char)
        {
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
        // Codex Step 3 R1 NIT-2: explicit `0x` prefix when caller
        // passed base=16. strtoull already handles this; mirror
        // for parity.
        p = p.add(2);
    }
    let mut acc: c_long = 0;
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
        acc = acc.wrapping_mul(effective_base as c_long);
        acc = acc.wrapping_add(digit as c_long);
        p = p.add(1);
    }
    if !endptr.is_null() {
        *endptr = p as *mut c_char;
    }
    if neg {
        -acc
    } else {
        acc
    }
}

#[no_mangle]
pub extern "C" fn tolower(c: c_int) -> c_int {
    if (b'A' as c_int) <= c && c <= (b'Z' as c_int) {
        c + 32
    } else {
        c
    }
}

#[no_mangle]
pub unsafe extern "C" fn qsort(
    base: *mut c_void,
    nmemb: usize,
    size: usize,
    compar: extern "C" fn(*const c_void, *const c_void) -> c_int,
) {
    // Insertion sort — O(n^2) but Skia calls this only on tiny
    // arrays (font feature lists, glyph runs); avoids the
    // recursion-stack hit of quicksort on wasm where stack is
    // small.
    if nmemb < 2 || size == 0 {
        return;
    }
    let base = base as *mut u8;
    let mut tmp_buf = [0u8; 256];
    let tmp = tmp_buf.as_mut_ptr();
    if size > 256 {
        // Codex Step 3 R1 NIT-3: previous version silently
        // returned unsorted data on size > 256, which would
        // corrupt the caller's array invariants. Panic loudly
        // so a real call site gets a usable diagnostic; tiny
        // arrays (font feature lists / glyph runs) stay under
        // the threshold.
        panic!(
            "wasm-libc-shim: qsort element size {} > 256 — bump tmp_buf in imp/step3.rs::qsort",
            size
        );
    }
    for i in 1..nmemb {
        // Copy element i into tmp.
        core::ptr::copy_nonoverlapping(base.add(i * size), tmp, size);
        let mut j = i;
        while j > 0 {
            let prev = base.add((j - 1) * size);
            if compar(prev as *const c_void, tmp as *const c_void) <= 0 {
                break;
            }
            // Shift previous element right.
            core::ptr::copy(prev, base.add(j * size), size);
            j -= 1;
        }
        // Place tmp at j.
        core::ptr::copy_nonoverlapping(tmp, base.add(j * size), size);
    }
}

// --- file I/O — all return error sentinels (no filesystem on wasm) ---
#[no_mangle]
pub extern "C" fn fopen(_path: *const c_char, _mode: *const c_char) -> *mut c_void {
    core::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn fread(
    _ptr: *mut c_void,
    _size: usize,
    _nmemb: usize,
    _stream: *mut c_void,
) -> usize {
    0
}

#[no_mangle]
pub extern "C" fn fclose(_stream: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn fputc(c: c_int, _stream: *mut c_void) -> c_int {
    c
}

#[no_mangle]
pub extern "C" fn fileno(_stream: *mut c_void) -> c_int {
    -1
}

#[no_mangle]
pub extern "C" fn fstat(_fd: c_int, _statbuf: *mut c_void) -> c_int {
    -1
}

#[no_mangle]
pub extern "C" fn pread(
    _fd: c_int,
    _buf: *mut c_void,
    _count: usize,
    _offset: i64,
) -> isize {
    -1
}

#[no_mangle]
pub extern "C" fn ftell(_stream: *mut c_void) -> c_long {
    // c_long (32-bit on wasm32) per the C standard. Codex Step 3
    // R1 CONCERN: prior i64 return triggered a wasm-ld
    // `signature_mismatch:ftell` trap stub.
    -1
}

#[no_mangle]
pub extern "C" fn fseek(_stream: *mut c_void, _offset: c_long, _whence: c_int) -> c_int {
    // Same as ftell — `offset` is `long`, NOT `int64_t`. Codex
    // Step 3 R1 CONCERN.
    -1
}

// --- env ---
#[no_mangle]
pub extern "C" fn getenv(_name: *const c_char) -> *const c_char {
    core::ptr::null()
}

// --- mmap (always fails — MAP_FAILED = (void*)-1) ---
#[no_mangle]
pub extern "C" fn mmap(
    _addr: *mut c_void,
    _length: usize,
    _prot: c_int,
    _flags: c_int,
    _fd: c_int,
    _offset: i64,
) -> *mut c_void {
    !0usize as *mut c_void
}

#[no_mangle]
pub extern "C" fn munmap(_addr: *mut c_void, _length: usize) -> c_int {
    -1
}

// --- setjmp/longjmp (Skia / freetype error path) ---
// setjmp returns 0 on the initial call; longjmp panics if it
// fires (the happy path through the in-memory TTF parse should
// never reach a longjmp).
#[no_mangle]
pub extern "C" fn setjmp(_env: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn longjmp(_env: *mut c_void, _val: c_int) -> ! {
    panic!("wasm-libc-shim: longjmp() called — freetype / skia error path triggered, no recovery available on wasm32-unknown-unknown");
}

// --- C++ operator new (nothrow) — forwards to malloc, returns
//     null on OOM (the nothrow contract). Mangled name:
//     `_ZnwmRKSt9nothrow_t` = `operator new(size_t,
//     std::nothrow_t const&)`.
#[no_mangle]
pub unsafe extern "C" fn _ZnwmRKSt9nothrow_t(
    size: usize,
    _nothrow: *const c_void,
) -> *mut c_void {
    super::malloc(size.max(1))
}
