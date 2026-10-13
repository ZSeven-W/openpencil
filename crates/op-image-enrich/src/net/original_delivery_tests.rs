use super::*;
use crate::net::providers::{
    parse_openverse_results, parse_wikimedia_results, OriginalImageSource,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn png(width: u32, height: u32) -> String {
    let image = image::RgbImage::from_pixel(width, height, image::Rgb([190, 150, 120]));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    )
}

fn artwork(shift: i32) -> image::DynamicImage {
    image::RgbImage::from_fn(600, 400, |x, y| {
        let x = x as i32;
        let y = y as i32;
        if (x - 150 - shift).pow(2) + (y - 200).pow(2) < 65 * 65 {
            image::Rgb([65, 100, 125])
        } else if (x - 450).pow(2) + (y - 200).pow(2) < 50 * 50 {
            image::Rgb([150, 90, 110])
        } else {
            image::Rgb([230, 220, 200])
        }
    })
    .into()
}

fn image_src(image: &image::DynamicImage, jpeg_quality: Option<u8>) -> String {
    let mut bytes = Cursor::new(Vec::new());
    let mime = if let Some(quality) = jpeg_quality {
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality)
            .encode_image(&image.to_rgb8())
            .unwrap();
        "jpeg"
    } else {
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        "png"
    };
    format!(
        "data:image/{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    )
}

fn assert_preview_retained(preview: String, original: String) {
    let mut chosen = selected();
    chosen.src = preview.clone();
    let used = Mutex::new(HashSet::new());
    let result =
        crate::net::block_on_image_runtime(upgrade_selected_with(chosen, &used, None, |_, _| {
            std::future::ready(Some(original.clone()))
        }));
    assert_eq!(result.src, preview);
    assert!(claim_unused_image_src(&used, &original));
}

fn selected() -> ResolvedImage {
    ResolvedImage {
        src: png(60, 40),
        provider: "openverse",
        provider_query: "cream texture".into(),
        title: "Cream".into(),
        attribution: "Photographer / CC BY 4.0".into(),
        original: Some(OriginalImageSource {
            url: "https://images.example/cream-original.jpg".into(),
            width: Some(1440),
            height: Some(960),
        }),
    }
}

#[test]
fn catalogue_keeps_original_dimensions_and_attribution_beside_the_preview() {
    let openverse = serde_json::json!({"results": [{
        "id":"artwork", "title":"Cream", "thumbnail":"https://thumb.example/600.jpg",
        "url":"https://source.example/original.jpg", "width":3000, "height":2000,
        "attribution":"Photographer / CC BY 4.0"
    }]});
    let hit = parse_openverse_results(&openverse).remove(0);
    assert_eq!(hit.thumb_url, "https://thumb.example/600.jpg");
    assert_eq!(hit.attribution, "Photographer / CC BY 4.0");
    assert_eq!(
        hit.original.unwrap(),
        OriginalImageSource {
            url: "https://source.example/original.jpg".into(),
            width: Some(3000),
            height: Some(2000),
        }
    );

    let wiki = serde_json::json!({"query":{"pages":{"1":{
        "pageid":1,"title":"File:Cream.jpg","imageinfo":[{
            "mime":"image/jpeg", "thumburl":"https://upload.wikimedia.org/800px-Cream.jpg",
            "url":"https://upload.wikimedia.org/Cream.jpg", "width":2400,"height":1600,
            "extmetadata":{"LicenseShortName":{"value":"CC BY-SA 4.0"}}
        }]
    }}}});
    let hit = parse_wikimedia_results(&wiki).remove(0);
    assert_eq!(hit.attribution, "CC BY-SA 4.0");
    assert_eq!(hit.original.unwrap().width, Some(2400));
}

#[test]
fn only_the_selected_original_is_delivered_with_metadata_and_preview_claim_retained() {
    let selected = selected();
    let preview = selected.src.clone();
    let original = png(1440, 960);
    let used = Mutex::new(HashSet::new());
    assert!(claim_unused_image_src(&used, &preview));
    let calls = AtomicUsize::new(0);
    let result = crate::net::block_on_image_runtime(upgrade_selected_with(
        selected,
        &used,
        Some(Instant::now() + Duration::from_secs(120)),
        |url, _| {
            calls.fetch_add(1, Ordering::Relaxed);
            assert_eq!(url, "https://images.example/cream-original.jpg");
            std::future::ready(Some(original.clone()))
        },
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(result.src, original);
    assert_eq!(result.attribution, "Photographer / CC BY 4.0");
    assert_eq!(result.title, "Cream");
    assert_eq!(result.original.as_ref().unwrap().width, Some(1440));
    assert!(!claim_unused_image_src(&used, &preview));
    assert!(!claim_unused_image_src(&used, &original));
}

#[test]
fn failed_smaller_or_already_claimed_originals_keep_the_completed_preview() {
    for fetched in [None, Some(png(30, 20)), Some(png(1440, 960))] {
        let chosen = selected();
        let preview = chosen.src.clone();
        let used = Mutex::new(HashSet::new());
        if let Some(src) = &fetched {
            claim_unused_image_src(&used, src);
        }
        let result = crate::net::block_on_image_runtime(upgrade_selected_with(
            chosen,
            &used,
            None,
            |_, _| std::future::ready(fetched),
        ));
        assert_eq!(result.src, preview);
    }
}

#[test]
fn oversized_original_or_smaller_catalogue_source_cannot_replace_the_preview() {
    let chosen = selected();
    let preview = chosen.src.clone();
    let used = Mutex::new(HashSet::new());
    let oversized = png(2049, 100);
    let result =
        crate::net::block_on_image_runtime(upgrade_selected_with(chosen, &used, None, |_, _| {
            std::future::ready(Some(oversized.clone()))
        }));
    assert_eq!(result.src, preview);
    assert!(claim_unused_image_src(&used, &oversized));

    let mut chosen = selected();
    chosen.original.as_mut().unwrap().width = Some(30);
    chosen.original.as_mut().unwrap().height = Some(20);
    let result =
        crate::net::block_on_image_runtime(upgrade_selected_with(chosen, &used, None, |_, _| {
            panic!("catalogue says the original is smaller than the preview");
            #[allow(unreachable_code)]
            std::future::ready(None)
        }));
    assert_eq!(result.src, preview);
}

#[test]
fn exhausted_deadline_skips_original_fetch_and_retains_the_preview() {
    let chosen = selected();
    let preview = chosen.src.clone();
    let used = Mutex::new(HashSet::new());
    let result = crate::net::block_on_image_runtime(upgrade_selected_with(
        chosen,
        &used,
        Some(Instant::now() + Duration::from_millis(200)),
        |_, _| {
            panic!("the remaining 250ms belongs to delivery, not another download");
            #[allow(unreachable_code)]
            std::future::ready(None)
        },
    ));
    assert_eq!(result.src, preview);
}

#[test]
fn slow_original_cannot_overrun_the_original_run_deadline() {
    let chosen = selected();
    let preview = chosen.src.clone();
    let used = Mutex::new(HashSet::new());
    let deadline = Instant::now() + Duration::from_millis(270);
    let result = crate::net::block_on_image_runtime(upgrade_selected_with(
        chosen,
        &used,
        Some(deadline),
        |_, effective_deadline| {
            assert_eq!(effective_deadline, deadline - DELIVERY_MARGIN);
            std::future::pending::<Option<String>>()
        },
    ));
    assert_eq!(result.src, preview);
}

#[test]
fn different_aspect_crop_cannot_replace_the_judged_preview() {
    assert_preview_retained(png(60, 40), png(180, 180));
}

#[test]
fn same_aspect_crop_with_another_subject_cannot_replace_the_preview() {
    let original = artwork(0);
    let preview = original.crop_imm(0, 100, 300, 200).resize_exact(
        60,
        40,
        image::imageops::FilterType::Triangle,
    );
    assert_preview_retained(image_src(&preview, None), image_src(&original, None));
}

#[test]
fn same_palette_with_shifted_subjects_is_not_the_same_composition() {
    let preview = artwork(0).resize_exact(60, 40, image::imageops::FilterType::Triangle);
    assert_preview_retained(image_src(&preview, None), image_src(&artwork(110), None));
}

#[test]
fn normal_resize_and_jpeg_compression_preserve_the_selected_artwork() {
    let source = artwork(0);
    let preview = source.resize_exact(60, 40, image::imageops::FilterType::Triangle);
    let original = source.resize_exact(1200, 800, image::imageops::FilterType::Triangle);
    let preview = image_src(&preview, Some(75));
    let original = image_src(&original, Some(90));
    assert!(same_artwork(&preview, &original));
    let mut chosen = selected();
    chosen.src = preview;
    let used = Mutex::new(HashSet::new());
    let result =
        crate::net::block_on_image_runtime(upgrade_selected_with(chosen, &used, None, |_, _| {
            std::future::ready(Some(original.clone()))
        }));
    assert_eq!(result.src, original);
}

#[test]
fn transparent_resize_matches_but_white_matte_does_not_replace_alpha() {
    let original: image::DynamicImage = image::RgbaImage::from_fn(600, 400, |x, _| {
        image::Rgba([80, 110, 130, if x < 300 { 0 } else { 128 }])
    })
    .into();
    let preview = original.resize_exact(60, 40, image::imageops::FilterType::Triangle);
    let preview_src = image_src(&preview, None);
    assert!(same_artwork(&preview_src, &image_src(&original, None)));
    // White-matted RGB can look identical while the original loses alpha.
    let mut flattened = original.to_rgba8();
    for pixel in flattened.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in 0..3 {
            pixel[channel] =
                ((u16::from(pixel[channel]) * alpha + 255 * (255 - alpha)) / 255) as u8;
        }
        pixel[3] = 255;
    }
    assert_preview_retained(preview_src, image_src(&flattened.into(), None));
}

#[test]
fn header_valid_but_truncated_original_cannot_replace_a_complete_preview() {
    let original = png(120, 80);
    let mut bytes = raster_bytes(&original).unwrap();
    // Retain IHDR and the beginning of IDAT, enough for header-only dimensions
    // while removing raster data and the final PNG chunks.
    let idat = bytes.windows(4).position(|chunk| chunk == b"IDAT").unwrap();
    bytes.truncate(idat + 8);
    let truncated = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
    assert_eq!(raster_dimensions(&truncated), Some((120, 80)));
    assert_preview_retained(png(60, 40), truncated);
}
