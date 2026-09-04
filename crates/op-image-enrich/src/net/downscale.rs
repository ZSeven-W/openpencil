//! Down-scale oversized raster images at import time so the document
//! never carries a multi-megabyte `src`.
//!
//! A 24-megapixel photo dropped onto the canvas bloats every later
//! operation: each scene rebuild clones + compares the node's base64
//! `src`, every `.op` save serializes it, and the canvas decodes it to
//! a full-resolution GPU bitmap. Design work never needs more than a
//! couple-thousand pixels on the longest edge, so we decode the source
//! once, fit it inside [`MAX_EDGE`], and re-encode — effectively
//! opaque images as JPEG (small), images with real transparency as
//! PNG (lossless, keeps alpha). A genuinely heavy source whose pixel dimensions are
//! already within budget is still re-encoded (a bloated PNG photo
//! recompresses to a fraction). Anything skia can't decode (SVG,
//! corrupt bytes), already small enough, or that fails to shrink,
//! passes straight through untouched.

use skia_safe::{surfaces, CubicResampler, Data, EncodedImageFormat, Image, Paint, Rect};

/// Longest-edge ceiling (px) for an imported raster image. 2048 keeps
/// crisp detail at typical canvas zoom while collapsing a 6000px photo
/// to ~1/9th the pixels.
const MAX_EDGE: i32 = 2048;
/// Re-encode any source heavier than this even when its pixel
/// dimensions already fit `MAX_EDGE` — a bloated screenshot / PNG photo
/// recompresses to a fraction. The user's reported lag came from a
/// 5.4 MB source; this catches the heavy-but-not-huge case too.
const BYTE_BUDGET: usize = 2_000_000;
/// JPEG quality for re-encoded opaque images — visually lossless for
/// design mock-ups, a fraction of the source size.
const JPEG_QUALITY: u32 = 82;
const THUMB_MAX_EDGE: i32 = 32;
const THUMB_BYTE_BUDGET: usize = 4 * 1024;
const THUMB_JPEG_QUALITIES: [u32; 6] = [60, 50, 40, 30, 20, 10];

/// Results produced from one decode of a Figma import bitmap. The resolver
/// decides the final data-URL MIME after the optional replacement lands.
#[derive(Default)]
pub struct PreparedImportImage {
    pub replacement: Option<Vec<u8>>,
    pub thumbnail: Option<Vec<u8>>,
}

/// Re-encode `bytes` smaller when it decodes to a raster image whose
/// longest edge exceeds [`MAX_EDGE`] or whose payload exceeds
/// [`BYTE_BUDGET`]. Returns `Some((mime, bytes))` with the down-scaled
/// payload (the new MIME, `image/jpeg` or `image/png`), or `None` to
/// keep the original — covering "already small enough", "not a
/// decodable raster" (SVG, corrupt), and "re-encode didn't shrink it".
pub fn maybe_downscale(bytes: &[u8]) -> Option<(&'static str, Vec<u8>)> {
    // Never re-encode an animated source: skia decodes only the first
    // frame, so flattening a GIF / animated WebP to a single JPEG/PNG
    // would silently drop frames from the saved document. Static WebP
    // (no animation chunk) still flows through and gets compressed.
    if is_gif(bytes) || is_animated_webp(bytes) {
        return None;
    }
    let src = Image::from_encoded(Data::new_copy(bytes))?;
    maybe_downscale_decoded(bytes, &src)
}

/// Decode an imported Figma bitmap once, producing both its optional
/// full-size replacement and its blur-up JPEG. The callback runs before the
/// resolver creates the final data URL, so callers bind the thumbnail later.
pub fn prepare_figma_import_image(bytes: &[u8]) -> PreparedImportImage {
    let Some(src) = Image::from_encoded(Data::new_copy(bytes)) else {
        return PreparedImportImage::default();
    };
    let replacement = if is_gif(bytes) || is_animated_webp(bytes) {
        None
    } else {
        maybe_downscale_decoded(bytes, &src).map(|(_mime, replacement)| replacement)
    };
    PreparedImportImage {
        replacement,
        thumbnail: make_blur_thumbnail_from_image(&src),
    }
}

/// Generate a blur-up JPEG from an already-decoded raster. This keeps the
/// fallback path on the decode worker without decoding the payload twice.
pub fn make_blur_thumbnail_from_image(src: &Image) -> Option<Vec<u8>> {
    let (w, h) = (src.width(), src.height());
    if w <= 0 || h <= 0 {
        return None;
    }
    let longest = w.max(h);
    let scale = (THUMB_MAX_EDGE as f32 / longest as f32).min(1.0);
    let nw = ((w as f32 * scale).round() as i32).max(1);
    let nh = ((h as f32 * scale).round() as i32).max(1);
    let mut surface = surfaces::raster_n32_premul((nw, nh))?;
    surface.canvas().clear(skia_safe::Color::WHITE);
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    surface.canvas().draw_image_rect_with_sampling_options(
        src,
        None,
        Rect::from_xywh(0.0, 0.0, nw as f32, nh as f32),
        CubicResampler::mitchell(),
        &paint,
    );
    let thumb = surface.image_snapshot();
    THUMB_JPEG_QUALITIES.iter().find_map(|quality| {
        let encoded = thumb.encode(None, EncodedImageFormat::JPEG, *quality)?;
        (encoded.size() <= THUMB_BYTE_BUDGET).then(|| encoded.as_bytes().to_vec())
    })
}

/// Transcode a web-fetched image into a payload the scene renderer can
/// actually draw. The exact PNG exporter ships only PNG + JPEG codecs, so
/// a WebP (the common Openverse thumbnail container) embedded verbatim
/// renders as an empty placeholder. Animated sources are refused rather
/// than flattened, and an undecodable payload returns `None` so the
/// caller can move on to the next candidate URL instead of committing
/// bytes that will never draw.
pub fn reencode_for_renderer(bytes: &[u8]) -> Option<(&'static str, Vec<u8>)> {
    reencode_for_renderer_inner(bytes, true)
}

/// Pure-Rust decode for containers this skia build ships no codec for
/// (WebP in practice). Emits PNG bytes so the skia path owns sizing and
/// the final JPEG/PNG choice.
fn decode_via_image_crate(bytes: &[u8]) -> Option<Vec<u8>> {
    let decoded = image::load_from_memory(bytes).ok()?;
    let mut png: Vec<u8> = Vec::new();
    decoded
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(png)
}

fn reencode_for_renderer_inner(
    bytes: &[u8],
    allow_fallback_decode: bool,
) -> Option<(&'static str, Vec<u8>)> {
    if is_gif(bytes) || is_animated_webp(bytes) {
        return None;
    }
    let src = match Image::from_encoded(Data::new_copy(bytes)) {
        Some(src) => src,
        None if allow_fallback_decode => {
            let png = decode_via_image_crate(bytes)?;
            return reencode_for_renderer_inner(&png, false);
        }
        None => return None,
    };
    let (w, h) = (src.width(), src.height());
    if w <= 0 || h <= 0 {
        return None;
    }
    let longest = w.max(h);
    let scale = (MAX_EDGE as f32 / longest as f32).min(1.0);
    let nw = ((w as f32 * scale).round() as i32).max(1);
    let nh = ((h as f32 * scale).round() as i32).max(1);
    let mut surface = surfaces::raster_n32_premul((nw, nh))?;
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    surface.canvas().draw_image_rect_with_sampling_options(
        &src,
        None,
        Rect::from_xywh(0.0, 0.0, nw as f32, nh as f32),
        CubicResampler::mitchell(),
        &paint,
    );
    let scaled = surface.image_snapshot();
    let (mime, encoded) = if src.is_opaque() || raster_is_fully_opaque(&scaled) {
        match scaled.encode(None, EncodedImageFormat::JPEG, JPEG_QUALITY) {
            Some(data) => ("image/jpeg", data),
            None => (
                "image/png",
                scaled.encode(None, EncodedImageFormat::PNG, 100)?,
            ),
        }
    } else {
        (
            "image/png",
            scaled.encode(None, EncodedImageFormat::PNG, 100)?,
        )
    };
    Some((mime, encoded.as_bytes().to_vec()))
}

fn maybe_downscale_decoded(bytes: &[u8], src: &Image) -> Option<(&'static str, Vec<u8>)> {
    let oversized_bytes = bytes.len() > BYTE_BUDGET;
    let (w, h) = (src.width(), src.height());
    if w <= 0 || h <= 0 {
        return None;
    }
    let longest = w.max(h);
    let oversized_dims = longest > MAX_EDGE;
    if !oversized_dims && !oversized_bytes {
        return None;
    }

    let scale = if oversized_dims {
        MAX_EDGE as f32 / longest as f32
    } else {
        1.0
    };
    let nw = ((w as f32 * scale).round() as i32).max(1);
    let nh = ((h as f32 * scale).round() as i32).max(1);

    let mut surface = surfaces::raster_n32_premul((nw, nh))?;
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    // Mitchell cubic resampling — the high-quality downscale filter, so
    // a shrunk photo stays smooth instead of aliasing.
    surface.canvas().draw_image_rect_with_sampling_options(
        src,
        None,
        Rect::from_xywh(0.0, 0.0, nw as f32, nh as f32),
        CubicResampler::mitchell(),
        &paint,
    );
    let scaled = surface.image_snapshot();

    // Opaque source → JPEG (small); anything with alpha → PNG so
    // transparency survives (JPEG can't carry an alpha channel).
    // `is_opaque` only reflects the encoded alpha TYPE — Figma stores
    // photos as PNG with a (fully-opaque) alpha channel, so also scan
    // the scaled raster: if no pixel is actually translucent, JPEG is
    // safe and roughly an order of magnitude smaller.
    let (mime, encoded) = if src.is_opaque() || raster_is_fully_opaque(&scaled) {
        match scaled.encode(None, EncodedImageFormat::JPEG, JPEG_QUALITY) {
            Some(data) => ("image/jpeg", data),
            None => (
                "image/png",
                scaled.encode(None, EncodedImageFormat::PNG, 100)?,
            ),
        }
    } else {
        (
            "image/png",
            scaled.encode(None, EncodedImageFormat::PNG, 100)?,
        )
    };

    let out = encoded.as_bytes().to_vec();
    // Only adopt the re-encode when it actually shrank the payload — a
    // source already at MAX_EDGE+1 with tight compression could grow.
    if out.len() < bytes.len() {
        Some((mime, out))
    } else {
        None
    }
}

/// Whether every pixel of a raster image is fully opaque. Peeks the
/// CPU pixels (our down-scale snapshot is always raster) and scans the
/// alpha byte — N32 is 4 bytes/pixel with alpha at index 3 in both the
/// RGBA and BGRA layouts. Returns `false` when pixels can't be peeked
/// (conservative: keeps the lossless PNG path).
fn raster_is_fully_opaque(image: &skia_safe::Image) -> bool {
    let Some(pixmap) = image.peek_pixels() else {
        return false;
    };
    let info = pixmap.info();
    if info.bytes_per_pixel() != 4 {
        return false;
    }
    let Some(bytes) = pixmap.bytes() else {
        return false;
    };
    bytes.chunks_exact(4).all(|px| px[3] == 0xFF)
}

/// GIF magic — `GIF87a` / `GIF89a` both start `GIF`.
fn is_gif(bytes: &[u8]) -> bool {
    bytes.len() >= 6 && &bytes[..3] == b"GIF"
}

/// Animated WebP — a RIFF/`WEBP` container whose extended `VP8X` header
/// sets the animation flag (bit 1 of the flags byte at offset 20). A
/// simple (`VP8 ` / `VP8L`) or non-animated `VP8X` WebP returns false so
/// it still flows through the down-scaler. Per the WebP container spec.
fn is_animated_webp(bytes: &[u8]) -> bool {
    bytes.len() >= 21
        && &bytes[..4] == b"RIFF"
        && &bytes[8..12] == b"WEBP"
        && &bytes[12..16] == b"VP8X"
        && (bytes[20] & 0x02) != 0
}

/// Down-scale an existing `data:<mime>;base64,<payload>` URL in place,
/// returning a new (smaller) data URL or `None` to keep the original.
/// Covers insert paths that already hold a data URL string rather than
/// raw bytes (e.g. an AI provider that returns inline base64). A
/// non-data / non-base64 / undecodable URL is left untouched.
pub fn maybe_downscale_data_url(url: &str) -> Option<String> {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    let after_scheme = url.strip_prefix("data:")?;
    let comma = after_scheme.find(',')?;
    let meta = &after_scheme[..comma];
    if !meta.contains(";base64") {
        return None;
    }
    let bytes = B64.decode(&after_scheme.as_bytes()[comma + 1..]).ok()?;
    let (mime, scaled) = maybe_downscale(&bytes)?;
    Some(format!("data:{mime};base64,{}", B64.encode(&scaled)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Solid-red `w`×`h` source, encoded as `format`. PNG carries an
    /// alpha channel (decodes non-opaque); JPEG is inherently opaque.
    fn solid(w: i32, h: i32, format: EncodedImageFormat) -> Vec<u8> {
        let mut surface = surfaces::raster_n32_premul((w, h)).expect("raster surface");
        surface.canvas().clear(skia_safe::Color::RED);
        surface
            .image_snapshot()
            .encode(None, format, 100)
            .expect("encode source")
            .as_bytes()
            .to_vec()
    }

    #[test]
    fn a_small_image_passes_through_untouched() {
        let png = solid(64, 64, EncodedImageFormat::PNG);
        assert!(
            maybe_downscale(&png).is_none(),
            "a 64px image is within budget — no re-encode"
        );
    }

    #[test]
    fn blur_thumbnail_is_a_bounded_jpeg() {
        let png = solid(320, 160, EncodedImageFormat::PNG);
        let prepared = prepare_figma_import_image(&png);
        assert!(
            prepared.replacement.is_none(),
            "the in-budget full image remains unchanged"
        );
        let thumb = prepared.thumbnail.expect("import pass generates thumbnail");

        assert!(thumb.starts_with(&[0xff, 0xd8]), "thumbnail is JPEG");
        assert!(thumb.len() <= 4 * 1024, "thumbnail stays within 4 KiB");
        let decoded = Image::from_encoded(Data::new_copy(&thumb)).expect("thumbnail decodes");
        assert_eq!(decoded.dimensions(), (32, 16).into());
    }

    #[test]
    fn an_oversized_image_is_downscaled_to_max_edge() {
        // 4000px wide → must shrink to 2048 on the long edge. A PNG
        // whose alpha channel is fully opaque (the Figma-photo case)
        // re-encodes as JPEG — the channel carries no information.
        let png = solid(4000, 1000, EncodedImageFormat::PNG);
        let (mime, out) = maybe_downscale(&png).expect("oversized image downscales");
        let scaled = Image::from_encoded(Data::new_copy(&out)).expect("re-decodes");
        assert_eq!(scaled.width(), MAX_EDGE, "long edge clamps to MAX_EDGE");
        assert_eq!(scaled.height(), 512, "aspect ratio preserved");
        assert!(out.len() < png.len(), "downscale shrinks the payload");
        assert_eq!(mime, "image/jpeg", "fully-opaque PNG re-encodes as JPEG");
    }

    #[test]
    fn a_genuinely_transparent_image_stays_png() {
        // Half-transparent fill → the alpha channel carries real data,
        // so the re-encode must stay PNG to preserve it.
        let mut surface = surfaces::raster_n32_premul((4000, 1000)).expect("raster surface");
        surface
            .canvas()
            .clear(skia_safe::Color::from_argb(128, 255, 0, 0));
        let png = surface
            .image_snapshot()
            .encode(None, EncodedImageFormat::PNG, 100)
            .expect("encode source")
            .as_bytes()
            .to_vec();
        let (mime, _out) = maybe_downscale(&png).expect("oversized image downscales");
        assert_eq!(mime, "image/png", "translucent pixels keep the PNG path");
    }

    #[test]
    fn an_oversized_opaque_source_re_encodes_as_jpeg() {
        // A JPEG source decodes opaque → re-encodes as JPEG. Also proves
        // the JPEG encoder is compiled into this skia binary-cache build.
        let jpeg = solid(4000, 1000, EncodedImageFormat::JPEG);
        let (mime, out) = maybe_downscale(&jpeg).expect("oversized image downscales");
        let scaled = Image::from_encoded(Data::new_copy(&out)).expect("re-decodes");
        assert_eq!(scaled.width(), MAX_EDGE, "long edge clamps to MAX_EDGE");
        assert_eq!(mime, "image/jpeg", "opaque source re-encodes as JPEG");
    }

    #[test]
    fn non_image_bytes_pass_through() {
        assert!(
            maybe_downscale(b"this is not an image").is_none(),
            "undecodable bytes keep the original"
        );
    }

    #[test]
    fn reencode_for_renderer_only_emits_renderer_codecs() {
        // A decodable raster always comes back as PNG or JPEG.
        let png = solid(64, 64, EncodedImageFormat::PNG);
        let (mime, out) = reencode_for_renderer(&png).expect("decodable raster re-encodes");
        assert!(
            mime == "image/png" || mime == "image/jpeg",
            "renderer-safe mime, got {mime}"
        );
        assert!(
            Image::from_encoded(Data::new_copy(&out)).is_some(),
            "re-encoded payload decodes"
        );
        // Animated / undecodable payloads are refused, never embedded.
        let mut gif = b"GIF89a".to_vec();
        gif.resize(128, 0);
        assert!(
            reencode_for_renderer(&gif).is_none(),
            "GIF is refused rather than flattened"
        );
        assert!(
            reencode_for_renderer(b"RIFF\0\0\0\0WEBPVP8 not-really-webp").is_none(),
            "undecodable WebP is refused"
        );
    }

    #[test]
    fn webp_payload_transcodes_via_pure_rust_fallback() {
        // Real (lossless) WebP bytes from the image crate. Whether skia's
        // codec set covers WebP or the pure-Rust fallback kicks in, the
        // result must be a renderer codec the exporter can draw.
        let mut webp = Vec::new();
        let raster = image::RgbaImage::from_pixel(20, 20, image::Rgba([200, 30, 30, 255]));
        image::DynamicImage::ImageRgba8(raster)
            .write_to(
                &mut std::io::Cursor::new(&mut webp),
                image::ImageFormat::WebP,
            )
            .expect("encode webp fixture");
        let (mime, out) = reencode_for_renderer(&webp).expect("webp transcodes");
        assert!(
            mime == "image/jpeg" || mime == "image/png",
            "renderer-safe mime, got {mime}"
        );
        assert!(
            Image::from_encoded(Data::new_copy(&out)).is_some(),
            "skia decodes the transcoded payload"
        );
    }

    #[test]
    fn animated_sources_are_left_whole() {
        // A large GIF must NOT be flattened to a single JPEG/PNG frame —
        // re-encoding would drop its animation from the saved document.
        let mut gif = b"GIF89a".to_vec();
        gif.resize(BYTE_BUDGET + 1, 0);
        assert!(maybe_downscale(&gif).is_none(), "oversized GIF stays whole");
        // Animated WebP (VP8X header with the animation flag) is skipped.
        let mut webp = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0".to_vec();
        webp.push(0x02); // flags byte at offset 20: animation bit set
        webp.resize(BYTE_BUDGET + 1, 0);
        assert!(
            maybe_downscale(&webp).is_none(),
            "oversized animated WebP stays whole"
        );
        assert!(is_animated_webp(&webp), "VP8X animation flag is detected");
        // A non-animated VP8X WebP is NOT treated as animated (it would
        // flow into the down-scaler; here it just fails to decode → None).
        let mut still = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0".to_vec();
        still.push(0x10); // alpha flag only, no animation bit
        still.resize(64, 0);
        assert!(!is_animated_webp(&still), "static VP8X is not animated");
    }

    /// Manual diagnostics: apply `maybe_downscale` to every file in a
    /// directory (`OP_DOWNSCALE_BENCH_DIR`) and print the outcome
    /// distribution — used to tune the budgets against real imports.
    #[test]
    #[ignore = "manual bench — needs OP_DOWNSCALE_BENCH_DIR pointing at an image dir"]
    fn downscale_dir_bench() {
        let Ok(dir) = std::env::var("OP_DOWNSCALE_BENCH_DIR") else {
            eprintln!("OP_DOWNSCALE_BENCH_DIR not set — skipping");
            return;
        };
        let (mut n, mut before, mut after) = (0usize, 0usize, 0usize);
        let (mut skipped, mut skipped_bytes) = (0usize, 0usize);
        let (mut to_jpeg, mut to_png) = (0usize, 0usize);
        let (mut jpeg_bytes, mut png_bytes) = (0usize, 0usize);
        for entry in std::fs::read_dir(&dir).expect("readable dir") {
            let path = entry.expect("dir entry").path();
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            n += 1;
            before += bytes.len();
            match maybe_downscale(&bytes) {
                Some(("image/jpeg", out)) => {
                    to_jpeg += 1;
                    jpeg_bytes += out.len();
                    after += out.len();
                }
                Some((_, out)) => {
                    to_png += 1;
                    png_bytes += out.len();
                    after += out.len();
                }
                None => {
                    skipped += 1;
                    skipped_bytes += bytes.len();
                    after += bytes.len();
                }
            }
        }
        eprintln!(
            "downscale_dir_bench: {n} files {:.0}MB -> {:.0}MB | skipped {skipped} ({:.0}MB) | jpeg {to_jpeg} ({:.0}MB) | png {to_png} ({:.0}MB)",
            before as f64 / 1e6,
            after as f64 / 1e6,
            skipped_bytes as f64 / 1e6,
            jpeg_bytes as f64 / 1e6,
            png_bytes as f64 / 1e6,
        );
    }

    #[test]
    fn data_url_helper_downscales_an_oversized_inline_image() {
        use base64::engine::general_purpose::STANDARD as B64;
        use base64::Engine as _;
        let jpeg = solid(4000, 1000, EncodedImageFormat::JPEG);
        let url = format!("data:image/jpeg;base64,{}", B64.encode(&jpeg));
        let out = maybe_downscale_data_url(&url).expect("oversized inline url downscales");
        assert!(out.starts_with("data:image/jpeg;base64,"));
        assert!(out.len() < url.len(), "re-encoded url is smaller");
        // A small inline image is left untouched.
        let small = format!(
            "data:image/png;base64,{}",
            B64.encode(solid(32, 32, EncodedImageFormat::PNG))
        );
        assert!(maybe_downscale_data_url(&small).is_none());
        // Non-data URLs pass through.
        assert!(maybe_downscale_data_url("https://x/y.png").is_none());
    }
}
