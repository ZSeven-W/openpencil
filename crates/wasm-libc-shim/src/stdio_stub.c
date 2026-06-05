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

/* Step 3 codex stop-hook addition: fprintf is variadic so it
 * needs the C-side stub treatment. Skia uses it for diagnostic
 * messages on freetype error paths; happy text-rendering path
 * doesn't reach it. Same fail-fast policy as the snprintf
 * family — easier to find a real call than to debug an empty
 * silent return. */
int fprintf(void *stream, const char *fmt, ...) {
    (void)stream;
    (void)fmt;
    wasm_libc_shim_stdio_panic("fprintf");
}

/*
 * Step 4 (2026-06-05): additional libc + libc++ symbols imported by the
 * Skia C++ when the bundle is built with the brew-emscripten clang (a
 * slightly different libc++ revision than the canonical emsdk the shim
 * was first written against). All defined here in C so the wasm
 * signature + ABI exactly match Skia's call sites (one toolchain emits
 * both). On the canonical real-emsdk build these definitions are simply
 * unreferenced and dead-stripped, so adding them is harmless there and
 * keeps the "0 env.* imports" bundle invariant under either toolchain.
 *
 * Policy split from the printf family above: the printf stubs fail-fast
 * because a silent empty return would MASK an unexpected format call.
 * These, by contrast, are real (string/number parsing) or are
 * single-threaded no-ops (mutex / condition_variable) / lookup
 * failures (file I/O on a shell that ships fonts in memory and never
 * touches a real fd), so a correct value / benign failure is the right
 * answer, not a panic.
 */

/* ---- libc string / number ---- */

char *strchr(const char *s, int c) {
    char ch = (char)c;
    for (;; s++) {
        if (*s == ch) {
            return (char *)s;
        }
        if (*s == 0) {
            return (char *)0;
        }
    }
}

char *strncat(char *dst, const char *src, size_t n) {
    char *d = dst;
    while (*d) {
        d++;
    }
    size_t i = 0;
    while (i < n && src[i]) {
        d[i] = src[i];
        i++;
    }
    d[i] = 0;
    return dst;
}

unsigned long strtoul(const char *s, char **endptr, int base) {
    const char *p = s;
    while (*p == ' ' || (*p >= '\t' && *p <= '\r')) {
        p++;
    }
    int neg = 0;
    if (*p == '+') {
        p++;
    } else if (*p == '-') {
        neg = 1; /* C: strtoul applies unary minus in unsigned arithmetic */
        p++;
    }
    if (base == 0) {
        if (p[0] == '0' && (p[1] == 'x' || p[1] == 'X')) {
            base = 16;
        } else if (p[0] == '0') {
            base = 8;
        } else {
            base = 10;
        }
    }
    /* Consume the 0x prefix ONLY when a hex digit actually follows, so "0x" /
     * "0xG" convert the bare 0 and leave endptr at the 'x' (per the standard). */
    if (base == 16 && p[0] == '0' && (p[1] == 'x' || p[1] == 'X')) {
        int h = p[2];
        int is_hex = (h >= '0' && h <= '9') || (h >= 'a' && h <= 'f') || (h >= 'A' && h <= 'F');
        if (is_hex) {
            p += 2;
        }
    }
    const unsigned long ulmax = (unsigned long)-1;
    unsigned long cutoff = ulmax / (unsigned long)base;
    int cutlim = (int)(ulmax % (unsigned long)base);
    unsigned long val = 0;
    int anydig = 0;
    int overflow = 0;
    for (;;) {
        int c = *p;
        int d;
        if (c >= '0' && c <= '9') {
            d = c - '0';
        } else if (c >= 'a' && c <= 'z') {
            d = c - 'a' + 10;
        } else if (c >= 'A' && c <= 'Z') {
            d = c - 'A' + 10;
        } else {
            break;
        }
        if (d >= base) {
            break;
        }
        if (val > cutoff || (val == cutoff && d > cutlim)) {
            overflow = 1; /* keep scanning digits so endptr lands past them */
        } else {
            val = val * (unsigned long)base + (unsigned long)d;
        }
        p++;
        anydig = 1;
    }
    if (endptr) {
        *endptr = anydig ? (char *)p : (char *)s;
    }
    if (overflow) {
        return ulmax; /* ERANGE saturation (errno not wired in this shim) */
    }
    return neg ? (0ul - val) : val;
}

double strtod(const char *s, char **endptr) {
    const char *p = s;
    while (*p == ' ' || (*p >= '\t' && *p <= '\r')) {
        p++;
    }
    int sign = 1;
    if (*p == '+' || *p == '-') {
        if (*p == '-') {
            sign = -1;
        }
        p++;
    }
    double val = 0.0;
    int anydig = 0;
    while (*p >= '0' && *p <= '9') {
        val = val * 10.0 + (double)(*p - '0');
        p++;
        anydig = 1;
    }
    if (*p == '.') {
        p++;
        double frac = 0.1;
        while (*p >= '0' && *p <= '9') {
            val += (double)(*p - '0') * frac;
            frac *= 0.1;
            p++;
            anydig = 1;
        }
    }
    if (anydig && (*p == 'e' || *p == 'E')) {
        const char *e = p;
        p++;
        int esign = 1;
        if (*p == '+' || *p == '-') {
            if (*p == '-') {
                esign = -1;
            }
            p++;
        }
        int edig = 0;
        int exp = 0;
        while (*p >= '0' && *p <= '9') {
            /* Clamp so a pathologically long exponent can't signed-overflow
             * `exp`; 10^1000 already underflows/overflows the double to 0/inf. */
            if (exp < 1000) {
                exp = exp * 10 + (*p - '0');
            }
            p++;
            edig = 1;
        }
        if (edig) {
            double scale = 1.0;
            for (int i = 0; i < exp; i++) {
                scale *= 10.0;
            }
            if (esign < 0) {
                val /= scale;
            } else {
                val *= scale;
            }
        } else {
            p = e; /* no exponent digits: do not consume the 'e' */
        }
    }
    if (endptr) {
        *endptr = anydig ? (char *)p : (char *)s;
    }
    return sign * val;
}

/* ---- libc misc / file I/O (no real fs on the web shell) ---- */

/* Fail-fast like the printf family: faking full success (returning nmemb)
 * would silently lose data if a real fwrite reach ever encoded to a stream a
 * caller reads back. The raster + in-memory-font web path never reaches it;
 * a panic surfaces any new call site instead of masking it. */
size_t fwrite(const void *ptr, size_t size, size_t nmemb, void *stream) {
    (void)ptr;
    (void)size;
    (void)nmemb;
    (void)stream;
    wasm_libc_shim_stdio_panic("fwrite");
}

char *setlocale(int category, const char *locale) {
    (void)category;
    (void)locale;
    static char c_locale[2] = {'C', 0};
    return c_locale; /* always the "C" locale */
}

int open(const char *path, int flags, ...) {
    (void)path;
    (void)flags;
    return -1;
}

int close(int fd) {
    (void)fd;
    return 0;
}

int stat(const char *path, void *buf) {
    (void)path;
    (void)buf;
    return -1;
}

void *opendir(const char *name) {
    (void)name;
    return (void *)0;
}

void *readdir(void *dir) {
    (void)dir;
    return (void *)0;
}

int closedir(void *dir) {
    (void)dir;
    return 0;
}

/* C++ RTTI dynamic_cast runtime — not exercised on the raster path. Returning
 * NULL is wrong for BOTH a successful checked cast (caller sees spurious
 * failure) and a `dynamic_cast<T&>` (NULL deref instead of std::bad_cast), so
 * fail-fast instead: any real RTTI reach is surfaced loudly. */
void *__dynamic_cast(const void *sub, const void *src, const void *dst, long off) {
    (void)sub;
    (void)src;
    (void)dst;
    (void)off;
    wasm_libc_shim_stdio_panic("__dynamic_cast");
}

/* ---- libc++ (std::__2) symbols, named via asm labels so the exact
 * Itanium-mangled names are emitted with matching wasm signatures.
 * Single-threaded wasm: mutex / condition_variable are no-ops. ---- */

/* std::__2::__hash_memory(void const*, size_t) -> size_t (32-bit FNV-1a) */
size_t op_shim_hash_memory(const void *p, size_t n) __asm__("_ZNSt3__213__hash_memoryEPKvm");
size_t op_shim_hash_memory(const void *p, size_t n) {
    const unsigned char *b = (const unsigned char *)p;
    size_t h = 2166136261u;
    for (size_t i = 0; i < n; i++) {
        h ^= b[i];
        h *= 16777619u;
    }
    return h;
}

/* std::__2::__next_prime(size_t) -> size_t (next odd prime >= n) */
size_t op_shim_next_prime(size_t n) __asm__("_ZNSt3__212__next_primeEm");
size_t op_shim_next_prime(size_t n) {
    if (n <= 2) {
        return 2;
    }
    if ((n & 1) == 0) {
        n++;
    }
    for (;;) {
        int prime = 1;
        for (size_t d = 3; d * d <= n; d += 2) {
            if (n % d == 0) {
                prime = 0;
                break;
            }
        }
        if (prime) {
            return n;
        }
        n += 2;
    }
}

/* std::__2::__call_once(volatile unsigned long&, void*, void(*)(void*))
 * libc++ once_flag states: 0 = not run, 1 = in-progress, ~0ul = complete.
 * The inline call_once fast path is `if (flag != ~0ul) __call_once(...)`, so
 * the completed flag MUST be ~0ul (not 1) — otherwise every later call_once
 * on the same flag needlessly re-enters this slow path. Single-threaded:
 * run fn once, then publish the complete state. */
void op_shim_call_once(unsigned long *flag, void *arg, void (*fn)(void *))
    __asm__("_ZNSt3__211__call_onceERVmPvPFvS2_E");
void op_shim_call_once(unsigned long *flag, void *arg, void (*fn)(void *)) {
    if (flag && *flag != (unsigned long)-1) {
        if (*flag == 0) {
            fn(arg);
        }
        *flag = (unsigned long)-1; /* ~0ul = libc++ once_flag "complete" */
    }
}

/* std::__2::locale::name() const -> std::string (zeroed = valid empty
 * SSO string on the wasm32 libc++ layout; the sret buffer is 12 bytes). */
void op_shim_locale_name(void *ret, const void *self) __asm__("_ZNKSt3__26locale4nameEv");
void op_shim_locale_name(void *ret, const void *self) {
    (void)self;
    char *r = (char *)ret;
    for (int i = 0; i < 12; i++) {
        r[i] = 0;
    }
}

/* std::__2::mutex::lock() / unlock() / ~mutex() — no-ops (single-thread) */
void op_shim_mutex_lock(void *self) __asm__("_ZNSt3__25mutex4lockEv");
void op_shim_mutex_lock(void *self) { (void)self; }

void op_shim_mutex_unlock(void *self) __asm__("_ZNSt3__25mutex6unlockEv");
void op_shim_mutex_unlock(void *self) { (void)self; }

void op_shim_mutex_dtor(void *self) __asm__("_ZNSt3__25mutexD1Ev");
void op_shim_mutex_dtor(void *self) { (void)self; }

/* std::__2::condition_variable::wait(unique_lock<mutex>&) / notify_all()
 * / ~condition_variable(). notify_all + dtor are correct no-ops (no waiters
 * on a single thread); wait, however, could only spin forever here (no other
 * thread can ever satisfy the predicate), so fail-fast instead of hanging. */
void op_shim_cv_wait(void *self, void *lk) __asm__("_ZNSt3__218condition_variable4waitERNS_11unique_lockINS_5mutexEEE");
void op_shim_cv_wait(void *self, void *lk) {
    (void)self;
    (void)lk;
    wasm_libc_shim_stdio_panic("condition_variable::wait");
}

void op_shim_cv_notify_all(void *self) __asm__("_ZNSt3__218condition_variable10notify_allEv");
void op_shim_cv_notify_all(void *self) { (void)self; }

void op_shim_cv_dtor(void *self) __asm__("_ZNSt3__218condition_variableD1Ev");
void op_shim_cv_dtor(void *self) { (void)self; }
