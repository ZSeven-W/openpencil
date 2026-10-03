//! Bounded evidence from embedded raster assets. No URL fetches or model calls.
use base64::Engine as _;

/// Sample an opaque raster region in normalized source coordinates.
/// Unknown sources, transparent regions and malformed/oversized images remain
/// unprovable; callers must not turn that into a passing contrast result.
pub fn embedded_rgb_samples(src: &str, bounds: [f64; 4]) -> Option<Vec<[u8; 3]>> {
    sample(&decode(src)?, bounds)
}

fn decode(src: &str) -> Option<image::RgbaImage> {
    if !src.starts_with("data:image/") || src.len() > 16 * 1024 * 1024 {
        return None;
    }
    let (_, encoded) = src.split_once(";base64,")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    Some(reader.decode().ok()?.to_rgba8())
}

pub fn embedded_size(src: &str) -> Option<[u32; 2]> {
    let image = decode(src)?;
    Some([image.width(), image.height()])
}

/// Project a normalized source-image rectangle into the visible image box.
/// A cover-cropped target outside the visible box is rejected, not guessed.
pub fn project_source_box(
    size: [u32; 2],
    image_box: [f64; 4],
    source_box: [f64; 4],
    fit: &str,
) -> Option<[f64; 4]> {
    let [x, y, w, h] = image_box;
    let [sx, sy, sw, sh] = source_box;
    if image_box
        .iter()
        .chain(source_box.iter())
        .any(|v| !v.is_finite())
        || w <= 0.0
        || h <= 0.0
        || size.contains(&0)
        || sx < 0.0
        || sy < 0.0
        || sw <= 0.0
        || sh <= 0.0
        || sx + sw > 1.0
        || sy + sh > 1.0
    {
        return None;
    }
    let (dw, dh) = match fit {
        "cover" | "contain" => {
            let s = if fit == "cover" {
                (w / size[0] as f64).max(h / size[1] as f64)
            } else {
                (w / size[0] as f64).min(h / size[1] as f64)
            };
            (size[0] as f64 * s, size[1] as f64 * s)
        }
        "stretch" => (w, h),
        _ => return None,
    };
    let box_ = [
        x + (w - dw) / 2.0 + sx * dw,
        y + (h - dh) / 2.0 + sy * dh,
        sw * dw,
        sh * dh,
    ];
    (box_[0] >= x - 0.5
        && box_[1] >= y - 0.5
        && box_[0] + box_[2] <= x + w + 0.5
        && box_[1] + box_[3] <= y + h + 0.5)
        .then_some(box_)
}

fn sample(image: &image::RgbaImage, bounds: [f64; 4]) -> Option<Vec<[u8; 3]>> {
    if bounds.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let [left, top, right, bottom] = bounds.map(|v| v.clamp(0.0, 1.0));
    if right <= left || bottom <= top || image.width() == 0 || image.height() == 0 {
        return None;
    }
    let mut samples = Vec::new();
    for y in 0..5 {
        for x in 0..5 {
            let px = ((left + (right - left) * (x as f64 + 0.5) / 5.0) * image.width() as f64)
                .floor()
                .min((image.width() - 1) as f64) as u32;
            let py = ((top + (bottom - top) * (y as f64 + 0.5) / 5.0) * image.height() as f64)
                .floor()
                .min((image.height() - 1) as f64) as u32;
            let p = image.get_pixel(px, py).0;
            if p[3] != 255 {
                return None;
            }
            samples.push([p[0], p[1], p[2]]);
        }
    }
    Some(samples)
}

/// Sample the region as actually painted by centered cover/contain/stretch.
pub fn embedded_rgb_samples_in_box(
    src: &str,
    image_box: [f64; 4],
    region: [f64; 4],
    fit: &str,
) -> Option<Vec<[u8; 3]>> {
    let image = decode(src)?;
    let [x, y, w, h] = image_box;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let (dw, dh) = match fit {
        "cover" | "contain" => {
            let sx = w / image.width() as f64;
            let sy = h / image.height() as f64;
            let scale = if fit == "cover" {
                sx.max(sy)
            } else {
                sx.min(sy)
            };
            (image.width() as f64 * scale, image.height() as f64 * scale)
        }
        "stretch" => (w, h),
        _ => return None,
    };
    let (left, top) = (x + (w - dw) / 2.0, y + (h - dh) / 2.0);
    let [rx, ry, rw, rh] = region;
    if rx < left || ry < top || rx + rw > left + dw || ry + rh > top + dh {
        return None;
    }
    sample(
        &image,
        [
            (rx - left) / dw,
            (ry - top) / dh,
            (rx + rw - left) / dw,
            (ry + rh - top) / dh,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_boxes_follow_the_painted_image_including_letterboxing() {
        assert_eq!(
            project_source_box(
                [100, 200],
                [100.0, 100.0, 600.0, 400.0],
                [0.25, 0.5, 0.5, 0.1],
                "contain"
            ),
            Some([350.0, 300.0, 100.0, 40.0])
        );
        assert!(project_source_box(
            [100, 200],
            [0.0, 0.0, 600.0, 400.0],
            [0.0, 0.0, 0.5, 0.1],
            "cover"
        )
        .is_none());
        assert!(project_source_box(
            [100, 200],
            [0.0, 0.0, 600.0, 400.0],
            [0.9, 0.5, 0.5, 0.1],
            "contain"
        )
        .is_none());
    }
    #[test]
    fn sampling_uses_the_requested_region_and_refuses_unknown_assets() {
        let mut img = image::RgbaImage::from_pixel(10, 10, image::Rgba([20, 30, 40, 255]));
        for y in 0..10 {
            for x in 5..10 {
                img.put_pixel(x, y, image::Rgba([200, 210, 220, 255]));
            }
        }
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let src = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
        );
        assert!(embedded_rgb_samples(&src, [0.5, 0.0, 1.0, 1.0])
            .unwrap()
            .iter()
            .all(|p| *p == [200, 210, 220]));
        assert!(
            embedded_rgb_samples("https://example.invalid/photo.png", [0.0, 0.0, 1.0, 1.0])
                .is_none()
        );
        assert!(embedded_rgb_samples(&src, [0.0, 0.0, 0.0, 1.0]).is_none());
    }
}
