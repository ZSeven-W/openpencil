//! Intrinsic dimensions read directly from encoded image headers.
//!
//! This intentionally avoids a pixel decoder so callers can size images on
//! wasm and native hosts without adding a platform or codec dependency.

/// Largest source edge accepted by the lightweight metadata reader.
///
/// This is deliberately far above practical canvas/texture sizes while still
/// preventing corrupt headers from turning into million-kilometre layout
/// numbers in consumers that write the result directly into a document.
pub const MAX_INTRINSIC_IMAGE_EDGE: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SvgIntrinsicMetadata {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub view_box_ratio: Option<f64>,
}

/// Read intrinsic pixel dimensions from a supported encoded image.
///
/// PNG, JPEG, GIF, WebP, and common UTF-8 SVG roots are supported. Invalid,
/// truncated, zero-sized, or otherwise ambiguous inputs return `None`.
pub fn encoded_image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    png_dimensions(bytes)
        .or_else(|| jpeg_dimensions(bytes))
        .or_else(|| gif_dimensions(bytes))
        .or_else(|| webp_dimensions(bytes))
        .or_else(|| svg_dimensions(bytes))
}

/// Return the preferred aspect ratio carried by an SVG root `viewBox`.
///
/// Unlike [`encoded_image_dimensions`], this deliberately succeeds for a
/// viewBox-only SVG: callers implementing browser replaced-element sizing can
/// combine the ratio with the 300×150 default object box without pretending
/// the viewBox coordinate dimensions are intrinsic CSS pixels.
pub fn encoded_svg_view_box_ratio(bytes: &[u8]) -> Option<f64> {
    encoded_svg_intrinsic_metadata(bytes)?.view_box_ratio
}

/// Read the independent intrinsic axes and preferred ratio of an SVG root.
/// Relative axes such as `100%` remain `None`; a valid viewBox still supplies
/// its ratio so a browser-layout caller can combine one absolute axis with it.
pub fn encoded_svg_intrinsic_metadata(bytes: &[u8]) -> Option<SvgIntrinsicMetadata> {
    let source = std::str::from_utf8(bytes).ok()?;
    let root = svg_root_start_tag(source)?;
    let width = xml_attribute(root, "width").and_then(parse_svg_length);
    let height = xml_attribute(root, "height").and_then(parse_svg_length);
    let view_box_ratio = xml_attribute(root, "viewBox")
        .or_else(|| xml_attribute(root, "viewbox"))
        .and_then(parse_view_box)
        .and_then(|(width, height)| {
            let ratio = width / height;
            (ratio.is_finite() && ratio > 0.0).then_some(ratio)
        });
    Some(SvgIntrinsicMetadata {
        width,
        height,
        view_box_ratio,
    })
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24
        || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
        || u32::from_be_bytes(bytes[8..12].try_into().ok()?) != 13
        || &bytes[12..16] != b"IHDR"
    {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    // PNG reserves the high bit of both IHDR dimensions.
    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        return None;
    }
    nonzero_dimensions(width, height)
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut offset = 2;
    while offset + 3 < bytes.len() {
        if bytes[offset] != 0xff {
            offset += 1;
            continue;
        }
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
        }
        let marker = *bytes.get(offset)?;
        offset += 1;
        if matches!(marker, 0x01 | 0xd0..=0xd9) {
            continue;
        }
        let length = u16::from_be_bytes([*bytes.get(offset)?, *bytes.get(offset + 1)?]) as usize;
        if length < 2 || offset.checked_add(length)? > bytes.len() {
            return None;
        }
        if is_jpeg_start_of_frame(marker) && length >= 7 {
            let height = u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]]) as u32;
            let width = u16::from_be_bytes([bytes[offset + 5], bytes[offset + 6]]) as u32;
            return nonzero_dimensions(width, height);
        }
        offset += length;
    }
    None
}

fn is_jpeg_start_of_frame(marker: u8) -> bool {
    matches!(
        marker,
        0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf
    )
}

fn gif_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 10 || !(bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        return None;
    }
    nonzero_dimensions(
        u16::from_le_bytes(bytes[6..8].try_into().ok()?).into(),
        u16::from_le_bytes(bytes[8..10].try_into().ok()?).into(),
    )
}

fn webp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 20 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return None;
    }
    let riff_size = u32::from_le_bytes(bytes[4..8].try_into().ok()?) as usize;
    let chunk_size = u32::from_le_bytes(bytes[16..20].try_into().ok()?) as usize;
    // RIFF size counts `WEBP`, the first chunk header, and its padded data.
    let padded_chunk_size = chunk_size.checked_add(chunk_size & 1)?;
    if riff_size < 12usize.checked_add(padded_chunk_size)? {
        return None;
    }
    match &bytes[12..16] {
        b"VP8X" if chunk_size == 10 && bytes.len() >= 30 => nonzero_dimensions(
            read_u24_le(&bytes[24..27])?.checked_add(1)?,
            read_u24_le(&bytes[27..30])?.checked_add(1)?,
        ),
        b"VP8 " if chunk_size >= 10 && bytes.len() >= 30 && bytes[23..26] == [0x9d, 0x01, 0x2a] => {
            let width = u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3fff;
            let height = u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3fff;
            nonzero_dimensions(width.into(), height.into())
        }
        b"VP8L" if chunk_size >= 5 && bytes.len() >= 25 && bytes[20] == 0x2f => {
            let bits = u32::from_le_bytes(bytes[21..25].try_into().ok()?);
            nonzero_dimensions((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1)
        }
        _ => None,
    }
}

fn read_u24_le(bytes: &[u8]) -> Option<u32> {
    Some(
        u32::from(*bytes.first()?)
            | (u32::from(*bytes.get(1)?) << 8)
            | (u32::from(*bytes.get(2)?) << 16),
    )
}

fn svg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let source = std::str::from_utf8(bytes).ok()?;
    let root = svg_root_start_tag(source)?;
    let width_attribute = xml_attribute(root, "width");
    let height_attribute = xml_attribute(root, "height");
    let width = width_attribute.and_then(parse_svg_length);
    let height = height_attribute.and_then(parse_svg_length);
    // A present relative/invalid axis is authored rather than absent. Do not
    // replace it with a viewBox-derived hard pixel size.
    if width_attribute.is_some() != width.is_some()
        || height_attribute.is_some() != height.is_some()
    {
        return None;
    }
    let view_box = xml_attribute(root, "viewBox")
        .or_else(|| xml_attribute(root, "viewbox"))
        .and_then(parse_view_box);

    let (width, height) = match (width, height, view_box) {
        (Some(width), Some(height), _) => (width, height),
        (Some(width), None, Some((view_width, view_height))) => {
            (width, width * view_height / view_width)
        }
        (None, Some(height), Some((view_width, view_height))) => {
            (height * view_width / view_height, height)
        }
        _ => return None,
    };
    Some((dimension_to_u32(width)?, dimension_to_u32(height)?))
}

fn svg_root_start_tag(source: &str) -> Option<&str> {
    let mut rest = source.trim_start_matches('\u{feff}').trim_start();
    loop {
        if rest.starts_with("<?xml") {
            rest = rest.get(rest.find("?>")?.checked_add(2)?..)?.trim_start();
        } else if rest.starts_with("<!--") {
            rest = rest.get(rest.find("-->")?.checked_add(3)?..)?.trim_start();
        } else {
            break;
        }
    }
    let after_name = rest.strip_prefix("<svg")?;
    if !after_name
        .as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_whitespace() || matches!(byte, b'/' | b'>'))
    {
        return None;
    }
    let end = find_tag_end(after_name)?;
    after_name.get(..end)
}

fn find_tag_end(tag: &str) -> Option<usize> {
    let mut quote = None;
    for (index, byte) in tag.bytes().enumerate() {
        match (quote, byte) {
            (Some(active), value) if value == active => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'>') => return Some(index),
            _ => {}
        }
    }
    None
}

fn xml_attribute<'a>(tag: &'a str, wanted: &str) -> Option<&'a str> {
    let bytes = tag.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if matches!(bytes.get(index), None | Some(b'/')) {
            break;
        }
        let name_start = index;
        while bytes
            .get(index)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'=' | b'/' | b'>'))
        {
            index += 1;
        }
        let name = tag.get(name_start..index)?;
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            continue;
        }
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        let (value_start, value_end) = match *bytes.get(index)? {
            quote @ (b'\'' | b'"') => {
                index += 1;
                let start = index;
                while bytes.get(index).is_some_and(|byte| *byte != quote) {
                    index += 1;
                }
                (start, index)
            }
            _ => {
                let start = index;
                while bytes
                    .get(index)
                    .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'/' | b'>'))
                {
                    index += 1;
                }
                (start, index)
            }
        };
        if name == wanted {
            return tag.get(value_start..value_end);
        }
        if matches!(bytes.get(index), Some(b'\'' | b'"')) {
            index += 1;
        }
    }
    None
}

fn parse_svg_length(value: &str) -> Option<f64> {
    let value = value.trim();
    let number = value
        .strip_suffix("px")
        .or_else(|| value.strip_suffix("PX"))
        .unwrap_or(value)
        .trim()
        .parse::<f64>()
        .ok()?;
    (number.is_finite() && number > 0.0).then_some(number)
}

fn parse_view_box(value: &str) -> Option<(f64, f64)> {
    let mut values = value
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .filter(|part| !part.is_empty());
    let x = values.next()?.parse::<f64>().ok()?;
    let y = values.next()?.parse::<f64>().ok()?;
    let width = values.next()?.parse::<f64>().ok()?;
    let height = values.next()?.parse::<f64>().ok()?;
    if values.next().is_some() {
        return None;
    }
    (x.is_finite()
        && y.is_finite()
        && width.is_finite()
        && height.is_finite()
        && width > 0.0
        && height > 0.0)
        .then_some((width, height))
}

fn dimension_to_u32(value: f64) -> Option<u32> {
    if !value.is_finite() || value <= 0.0 || value > f64::from(MAX_INTRINSIC_IMAGE_EDGE) {
        return None;
    }
    let rounded = value.round();
    (rounded >= 1.0).then_some(rounded as u32)
}

fn nonzero_dimensions(width: u32, height: u32) -> Option<(u32, u32)> {
    (width > 0
        && height > 0
        && width <= MAX_INTRINSIC_IMAGE_EDGE
        && height <= MAX_INTRINSIC_IMAGE_EDGE)
        .then_some((width, height))
}

#[cfg(test)]
mod tests {
    use super::encoded_image_dimensions;

    #[test]
    fn reads_png_jpeg_gif_and_webp_headers() {
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[8..12].copy_from_slice(&13u32.to_be_bytes());
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&320u32.to_be_bytes());
        png[20..24].copy_from_slice(&180u32.to_be_bytes());
        assert_eq!(encoded_image_dimensions(&png), Some((320, 180)));

        let jpeg = [
            0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 0, 0, 0xff, 0xc0, 0x00, 0x07, 8, 0, 90, 0, 160,
        ];
        assert_eq!(encoded_image_dimensions(&jpeg), Some((160, 90)));

        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&64u16.to_le_bytes());
        gif.extend_from_slice(&32u16.to_le_bytes());
        assert_eq!(encoded_image_dimensions(&gif), Some((64, 32)));

        let mut webp = vec![0; 30];
        webp[..4].copy_from_slice(b"RIFF");
        webp[4..8].copy_from_slice(&22u32.to_le_bytes());
        webp[8..12].copy_from_slice(b"WEBP");
        webp[12..16].copy_from_slice(b"VP8X");
        webp[16..20].copy_from_slice(&10u32.to_le_bytes());
        webp[24..27].copy_from_slice(&[0x3f, 0x01, 0]);
        webp[27..30].copy_from_slice(&[0xb3, 0, 0]);
        assert_eq!(encoded_image_dimensions(&webp), Some((320, 180)));
    }

    #[test]
    fn reads_common_svg_dimensions_and_view_box_ratio() {
        assert_eq!(
            encoded_image_dimensions(br#"<svg width="320px" height='180'></svg>"#),
            Some((320, 180))
        );
        assert_eq!(
            encoded_image_dimensions(
                br#"<?xml version="1.0"?><!-- icon --><svg width="32" viewBox="0 0 16 9"/>"#
            ),
            Some((32, 18))
        );
        // A viewBox describes the internal coordinate system, not the SVG's
        // browser intrinsic size. Without an explicit axis it is ambiguous.
        assert_eq!(
            encoded_image_dimensions(br#"<svg viewBox="-5 -6 64 48"></svg>"#),
            None
        );
        assert_eq!(
            super::encoded_svg_view_box_ratio(br#"<svg viewBox="-5 -6 64 48"></svg>"#),
            Some(4.0 / 3.0)
        );
    }

    #[test]
    fn malformed_and_zero_sized_headers_are_rejected() {
        assert_eq!(encoded_image_dimensions(b"not an image"), None);
        assert_eq!(encoded_image_dimensions(b"\x89PNG\r\n\x1a\n"), None);
        let mut invalid_png = vec![0; 24];
        invalid_png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        invalid_png[8..12].copy_from_slice(&12u32.to_be_bytes());
        invalid_png[12..16].copy_from_slice(b"IHDR");
        invalid_png[16..20].copy_from_slice(&10u32.to_be_bytes());
        invalid_png[20..24].copy_from_slice(&10u32.to_be_bytes());
        assert_eq!(encoded_image_dimensions(&invalid_png), None);
        invalid_png[8..12].copy_from_slice(&13u32.to_be_bytes());
        invalid_png[16..20].copy_from_slice(&(super::MAX_INTRINSIC_IMAGE_EDGE + 1).to_be_bytes());
        assert_eq!(encoded_image_dimensions(&invalid_png), None);
        assert_eq!(encoded_image_dimensions(b"GIF89a\0\0\x01\0"), None);
        assert_eq!(
            encoded_image_dimensions(br#"<svg width="100%" height="20"></svg>"#),
            None
        );
        assert_eq!(
            encoded_image_dimensions(
                br#"<svg width="100%" height="20" viewBox="0 0 100 50"></svg>"#
            ),
            None
        );
        assert_eq!(
            encoded_image_dimensions(br#"<svg viewBox="0 0 0 10"></svg>"#),
            None
        );
        assert_eq!(
            encoded_image_dimensions(
                br#"<svg width="1000001" height="10" viewBox="0 0 10 10"></svg>"#
            ),
            None
        );

        let mut invalid_webp = vec![0; 30];
        invalid_webp[..4].copy_from_slice(b"RIFF");
        invalid_webp[4..8].copy_from_slice(&21u32.to_le_bytes());
        invalid_webp[8..12].copy_from_slice(b"WEBP");
        invalid_webp[12..16].copy_from_slice(b"VP8X");
        invalid_webp[16..20].copy_from_slice(&10u32.to_le_bytes());
        assert_eq!(encoded_image_dimensions(&invalid_webp), None);
    }
}
