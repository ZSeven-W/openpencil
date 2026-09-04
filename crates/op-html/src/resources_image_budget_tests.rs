use super::*;
use jian_ops_schema::node::ImageSrc;
use std::cell::{Cell, RefCell};

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0; 24];
    bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    bytes[8..12].copy_from_slice(&13u32.to_be_bytes());
    bytes[12..16].copy_from_slice(b"IHDR");
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    bytes
}

#[test]
fn image_resource_budget_stops_fetching_and_retaining_unique_keys() {
    let fetched = Cell::new(0);
    let bytes = png(32, 32);
    let fetcher = |_: &str| {
        fetched.set(fetched.get() + 1);
        Some(bytes.clone())
    };
    let mut budget = ResourceBudget {
        max_fetched_images: 1,
        ..ResourceBudget::default()
    };
    let mut cache = ImageResourceCache::default();
    let mut warnings = Vec::new();
    for url in ["one.png", "two.png", "three.png"] {
        embed_url(
            url,
            Some("https://assets.test/page.html"),
            Some(&fetcher),
            None,
            &mut budget,
            &mut warnings,
            &mut cache.entries,
            true,
        )
        .unwrap();
    }
    assert_eq!(fetched.get(), 1);
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(warnings.len(), 1);

    let mut pending_budget = ResourceBudget {
        max_pending_image_bytes: 16,
        ..ResourceBudget::default()
    };
    let mut pending_cache = ImageResourceCache::default();
    let before = fetched.get();
    for url in ["large-one.png", "large-two.png"] {
        embed_url(
            url,
            Some("https://assets.test/page.html"),
            Some(&fetcher),
            None,
            &mut pending_budget,
            &mut Vec::new(),
            &mut pending_cache.entries,
            false,
        )
        .unwrap();
    }
    assert_eq!(fetched.get() - before, 1);
    assert_eq!(pending_cache.entries.len(), 0);

    let final_image = embed_url(
        "large-one.png",
        Some("https://assets.test/page.html"),
        Some(&fetcher),
        None,
        &mut pending_budget,
        &mut Vec::new(),
        &mut pending_cache.entries,
        true,
    )
    .unwrap();
    assert!(final_image
        .replacement
        .as_ref()
        .is_some_and(|source| source.as_str() != PLACEHOLDER_GRAY_PNG));
    assert_eq!(fetched.get() - before, 2);
}

#[test]
fn blocked_and_oversized_urls_do_not_retain_unbounded_keys_or_warnings() {
    let fetcher = |_: &str| Some(png(1, 1));
    let mut budget = ResourceBudget {
        max_prefetched_images: 1,
        ..ResourceBudget::default()
    };
    let mut cache = ImageResourceCache::default();
    let mut warnings = Vec::new();
    for url in [
        "https://outside.test/one.png".to_string(),
        "https://outside.test/two.png".to_string(),
        format!(
            "https://outside.test/{}.png",
            "x".repeat(MAX_RESOURCE_URL_BYTES + 1)
        ),
    ] {
        embed_url(
            &url,
            Some("https://openpencil.local/page.html"),
            Some(&fetcher),
            None,
            &mut budget,
            &mut warnings,
            &mut cache.entries,
            false,
        )
        .unwrap();
    }
    assert_eq!(cache.entries.len(), 1);
    assert!(warnings.is_empty());

    let oversized = format!(
        "https://outside.test/{}.png",
        "y".repeat(MAX_RESOURCE_URL_BYTES + 1)
    );
    embed_url(
        &oversized,
        Some("https://openpencil.local/page.html"),
        Some(&fetcher),
        None,
        &mut budget,
        &mut warnings,
        &mut cache.entries,
        true,
    )
    .unwrap();
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].to_string().len() < 256);
}

#[test]
fn pending_prefetch_preserves_final_tree_priority_and_releases_memory() {
    let fetched = RefCell::new(Vec::new());
    let transformed = Cell::new(0);
    let source = png(100, 50);
    let fetcher = |url: &str| {
        fetched.borrow_mut().push(url.to_string());
        Some(source.clone())
    };
    let transform = |_: &[u8]| {
        transformed.set(transformed.get() + 1);
        Some(png(1, 1))
    };
    let mut budget = ResourceBudget {
        max_fetched_images: 1,
        max_prefetched_images: 2,
        ..ResourceBudget::default()
    };
    let mut cache = ImageResourceCache::default();
    let child = embed_url(
        "child.png",
        Some("https://assets.test/page.html"),
        Some(&fetcher),
        Some(&transform),
        &mut budget,
        &mut Vec::new(),
        &mut cache.entries,
        false,
    )
    .unwrap();
    assert_eq!(child.dimensions, Some((100.0, 50.0)));
    assert!(child.pending.is_some());
    assert_eq!(budget.fetched_images, 0);
    assert_eq!(transformed.get(), 0);

    let background = embed_url(
        "background.png",
        Some("https://assets.test/page.html"),
        Some(&fetcher),
        Some(&transform),
        &mut budget,
        &mut Vec::new(),
        &mut cache.entries,
        true,
    )
    .unwrap();
    assert!(background
        .replacement
        .as_ref()
        .is_some_and(|source| source.as_str() != PLACEHOLDER_GRAY_PNG));
    assert_eq!(transformed.get(), 1);

    let child = embed_url(
        "child.png",
        Some("https://assets.test/page.html"),
        Some(&fetcher),
        Some(&transform),
        &mut budget,
        &mut Vec::new(),
        &mut cache.entries,
        true,
    )
    .unwrap();
    assert_eq!(
        child.replacement.as_ref().map(ImageSrc::as_str),
        Some(PLACEHOLDER_GRAY_PNG)
    );
    assert_eq!(budget.pending_image_bytes, 0);
    assert_eq!(transformed.get(), 1);
    assert_eq!(fetched.borrow().len(), 2);
}

#[test]
fn one_pending_source_finalizes_once_and_keeps_original_dimensions() {
    let fetched = Cell::new(0);
    let transformed = Cell::new(0);
    let fetcher = |_: &str| {
        fetched.set(fetched.get() + 1);
        Some(png(100, 50))
    };
    let transform = |_: &[u8]| {
        transformed.set(transformed.get() + 1);
        Some(png(1, 1))
    };
    let mut budget = ResourceBudget::default();
    let mut cache = ImageResourceCache::default();
    for emit in [false, true, true] {
        let image = embed_url(
            "shared.png",
            Some("https://assets.test/page.html"),
            Some(&fetcher),
            Some(&transform),
            &mut budget,
            &mut Vec::new(),
            &mut cache.entries,
            emit,
        )
        .unwrap();
        assert_eq!(image.dimensions, Some((100.0, 50.0)));
    }
    assert_eq!(fetched.get(), 1);
    assert_eq!(transformed.get(), 1);
    assert_eq!(budget.pending_image_bytes, 0);
}
