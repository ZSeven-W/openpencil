use skia_bindings::SkGraphics;

pub fn init() {
    unsafe { SkGraphics::Init() };
}

pub fn font_cache_limit() -> usize {
    unsafe { SkGraphics::GetFontCacheLimit() }
}

pub fn set_font_cache_limit(bytes: usize) -> usize {
    unsafe { SkGraphics::SetFontCacheLimit(bytes) }
}

pub fn font_cache_used() -> usize {
    unsafe { SkGraphics::GetFontCacheUsed() }
}

pub fn font_cache_count_used() -> i32 {
    unsafe { SkGraphics::GetFontCacheCountUsed() }
}

pub fn font_cache_count_limit() -> i32 {
    unsafe { SkGraphics::GetFontCacheCountLimit() }
}

pub fn set_font_cache_count_limit(count: i32) -> i32 {
    unsafe { SkGraphics::SetFontCacheCountLimit(count) }
}

pub fn typeface_cache_count_limit() -> i32 {
    unsafe { SkGraphics::GetTypefaceCacheCountLimit() }
}

pub fn set_typeface_cache_count_limit(count: i32) -> i32 {
    unsafe { SkGraphics::SetTypefaceCacheCountLimit(count) }
}

pub fn purge_font_cache() {
    unsafe { SkGraphics::PurgeFontCache() }
}

pub fn purge_pinned_font_cache() {
    unsafe { SkGraphics::PurgePinnedFontCache() }
}

pub fn resource_cache_total_bytes_used() -> usize {
    unsafe { SkGraphics::GetResourceCacheTotalBytesUsed() }
}

pub fn resource_cache_total_bytes_limit() -> usize {
    unsafe { SkGraphics::GetResourceCacheTotalByteLimit() }
}

pub fn set_resource_cache_total_bytes_limit(new_limit: usize) -> usize {
    unsafe { SkGraphics::SetResourceCacheTotalByteLimit(new_limit) }
}

pub fn purge_resource_cache() {
    unsafe { SkGraphics::PurgeResourceCache() }
}

pub fn resource_cache_single_allocation_byte_limit() -> Option<usize> {
    let size = unsafe { SkGraphics::GetResourceCacheSingleAllocationByteLimit() };
    if size != 0 {
        Some(size)
    } else {
        None
    }
}

pub fn set_resource_cache_single_allocation_byte_limit(new_limit: Option<usize>) -> Option<usize> {
    let size = unsafe {
        SkGraphics::SetResourceCacheSingleAllocationByteLimit(new_limit.unwrap_or_default())
    };
    if size != 0 {
        Some(size)
    } else {
        None
    }
}

// TODO: dump_memory_statistics() (needs SkTraceMemoryDumpWrapper interop wrapper).

pub fn purge_all_caches() {
    unsafe { SkGraphics::PurgeAllCaches() }
}

// TODO: ImageGeneratorFromEncodedDataFactory
// TODO: SetOpenTypeSVGDecoderFactory & GetOpenTypeSVGDecoderFactory
