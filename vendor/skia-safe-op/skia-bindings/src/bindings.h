#ifndef SKIA_BINDINGS_BINDINGS_H
#define SKIA_BINDINGS_BINDINGS_H

#include <vector>
#include <optional>
#include "include/core/SkRefCnt.h"
#include "include/core/SkString.h"

template<typename T>
inline sk_sp<T> spFromConst(const T* pt) {
    return sk_sp<T>(const_cast<T*>(pt));
}

template<typename T>
inline sk_sp<T> sp(T* pt) {
    return sk_sp<T>(pt);
}

template<typename T>
inline std::optional<T> opt(const T* pt) {
    return pt ? std::optional<T>(*pt) : std::nullopt;
}

extern "C" struct TraitObject {
    void* data;
    void* vtable;
};

/// A VecSink is passed from Rust to C++ for receiving a slice of values.
template<typename T> struct VecSink {
    TraitObject fn_trait;
    void (*set_fn)(T *, size_t, TraitObject);

    void set(T* ptr, size_t len) {
        set_fn(ptr, len, fn_trait);
    }

    void set(std::vector<T>& v) {
        if (v.empty()) {
            set_fn(nullptr, 0, fn_trait);
        } else {
            set_fn(v.data(), v.size(), fn_trait);
        }
    }
};

template<typename T> struct Sink {
    TraitObject fn_trait;
    void (*set_fn)(const T *, TraitObject);

    void set(const T& value) {
        set_fn(&value, fn_trait);
    }
};

struct SkStrings {
    std::vector<SkString> strings;
};

#endif //SKIA_BINDINGS_BINDINGS_H
