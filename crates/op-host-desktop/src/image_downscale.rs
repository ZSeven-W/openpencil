//! Down-scale oversized raster images at import time so the document
//! never carries a multi-megabyte `src`.
//!
//! A 24-megapixel photo dropped onto the canvas bloats every later
//! operation: each scene rebuild clones + compares the node's base64
//! `src`, every `.op` save serializes it, and the canvas decodes it to
//! a full-resolution GPU bitmap. Design work never needs more than a
//! couple-thousand pixels on the longest edge, so we decode the source
//! once, fit it inside [`MAX_EDGE`], and re-encode — opaque images as
//! JPEG (small), images with alpha as PNG (lossless, keeps
//! transparency). A genuinely heavy source whose pixel dimensions are
//! already within budget is still re-encoded (a bloated PNG photo
//! recompresses to a fraction). Anything skia can't decode (SVG,
//! corrupt bytes), already small enough, or that fails to shrink,
//! passes straight through untouched.

use skia_safe::{
    surfaces, CubicResampler, Data, EncodedImageFormat, Image, Paint, Rect,
};

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
    let oversized_bytes = bytes.len() > BYTE_BUDGET;
    let src = Image::from_encoded(Data::new_copy(bytes))?;
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
        &src,
        None,
        Rect::from_xywh(0.0, 0.0, nw as f32, nh as f32),
        CubicResampler::mitchell(),
        &paint,
    );
    let scaled = surface.image_snapshot();

    // Opaque source → JPEG (small); anything with alpha → PNG so
    // transparency survives (JPEG can't carry an alpha channel).
    let (mime, encoded) = if src.is_opaque() {
        match scaled.encode(None, EncodedImageFormat::JPEG, JPEG_QUALITY) {
            Some(data) => ("image/jpeg", data),
            None => ("image/png", scaled.encode(None, EncodedImageFormat::PNG, 100)?),
        }
    } else {
        ("image/png", scaled.encode(None, EncodedImageFormat::PNG, 100)?)
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
    let bytes = B64.decode(after_scheme[comma + 1..].as_bytes()).ok()?;
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
    fn an_oversized_image_is_downscaled_to_max_edge() {
        // 4000px wide → must shrink to 2048 on the long edge. A PNG with
        // an alpha channel re-encodes as PNG (lossless, keeps alpha).
        let png = solid(4000, 1000, EncodedImageFormat::PNG);
        let (mime, out) = maybe_downscale(&png).expect("oversized image downscales");
        let scaled = Image::from_encoded(Data::new_copy(&out)).expect("re-decodes");
        assert_eq!(scaled.width(), MAX_EDGE, "long edge clamps to MAX_EDGE");
        assert_eq!(scaled.height(), 512, "aspect ratio preserved");
        assert!(out.len() < png.len(), "downscale shrinks the payload");
        assert_eq!(mime, "image/png", "alpha-channel source stays PNG");
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
