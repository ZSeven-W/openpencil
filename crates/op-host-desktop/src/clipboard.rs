//! Thin wrapper over the OS clipboard.
//!
//! Backs the AI chat input's Cmd+C / Cmd+V / Cmd+X. `arboard` wraps
//! the platform clipboard (NSPasteboard on macOS, the Win32
//! clipboard, X11 / Wayland on Linux); all calls are best-effort —
//! a clipboard that fails to initialise simply yields `None` / a
//! no-op rather than surfacing an error to the user.

/// PNG-encoded clipboard bitmap plus its original pixel dimensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Read the system clipboard's text. `None` when the clipboard holds
/// no text or could not be opened.
#[allow(dead_code)] // Retained single-flavour API; paste uses one shared OS handle.
pub fn get_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Read the system clipboard's image flavour (NSPasteboard TIFF/PNG,
/// CF_DIB, X11/Wayland image targets) re-encoded as PNG bytes.
/// `None` when the clipboard holds no image, or the bitmap can't be
/// wrapped / encoded. Backs paste-image-into-chat — the desktop
/// equivalent of the TS chat input's clipboard *files* surface
/// (`ai-chat-input.tsx:85-94`).
#[allow(dead_code)] // Required typed single-image API; paste uses target-aware APIs below.
pub fn get_image() -> Option<ClipboardImage> {
    let image = arboard::Clipboard::new().ok()?.get_image().ok()?;
    encode_image(image)
}

/// Read only the text flavour required by a focused editor input.
///
/// Canvas and chat paste use their own target-aware readers below so a
/// text paste never pays to transfer and PNG-encode a clipboard bitmap.
pub(crate) fn read_text_paste() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Read the flavours relevant to a focused chat input. Images win over
/// text, matching the paste router, so the losing flavour is never
/// transferred from the OS clipboard.
pub(crate) fn read_chat_paste() -> (Option<String>, Option<ClipboardImage>) {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return (None, None);
    };
    if let Some(image) = clipboard.get_image().ok().and_then(encode_image) {
        return (None, Some(image));
    }
    (clipboard.get_text().ok(), None)
}

/// Read the external flavours relevant to a canvas paste. HTML wins over
/// images (Figma and editable HTML ingestion), while plain text is not a
/// canvas payload and deliberately is not requested.
pub(crate) fn read_canvas_paste() -> (Option<String>, Option<ClipboardImage>) {
    #[cfg(target_os = "linux")]
    if let Some(payload) = read_wayland_canvas_paste() {
        return payload;
    }

    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return (None, None);
    };
    let html = clipboard.get().html().ok();
    if html
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return (html, None);
    }
    let image = clipboard.get_image().ok().and_then(encode_image);
    (html, image)
}

#[cfg(target_os = "linux")]
fn read_wayland_canvas_paste() -> Option<(Option<String>, Option<ClipboardImage>)> {
    use wl_clipboard_rs::paste::{get_contents, get_mime_types, ClipboardType, MimeType, Seat};

    std::env::var_os("WAYLAND_DISPLAY")?;
    let mime_types = get_mime_types(ClipboardType::Regular, Seat::Unspecified).ok()?;
    if mime_types.contains("text/html") {
        let bytes = read_wayland_pipe(
            get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                MimeType::Specific("text/html"),
            )
            .ok()?
            .0,
        )?;
        if let Ok(html) = String::from_utf8(bytes) {
            if !html.trim().is_empty() {
                return Some((Some(html), None));
            }
        }
    }
    if mime_types.contains("image/png") {
        let png = read_wayland_pipe(
            get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                MimeType::Specific("image/png"),
            )
            .ok()?
            .0,
        )?;
        return Some((None, Some(clipboard_image_from_png(png)?)));
    }
    Some((None, None))
}

#[cfg(target_os = "linux")]
fn read_wayland_pipe(mut pipe: impl std::io::Read) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

#[cfg(any(target_os = "linux", test))]
fn clipboard_image_from_png(png: Vec<u8>) -> Option<ClipboardImage> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    // `from_encoded` validates the encoded-image container and reads its
    // dimensions without forcing the expensive RGBA raster decode that this
    // Wayland fast path exists to avoid.
    let encoded = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&png))?;
    let width = u32::try_from(encoded.width()).ok()?;
    let height = u32::try_from(encoded.height()).ok()?;
    (width > 0 && height > 0).then_some(ClipboardImage { png, width, height })
}

fn encode_image(image: arboard::ImageData<'_>) -> Option<ClipboardImage> {
    let (width, height) = (image.width, image.height);
    if width == 0 || height == 0 {
        return None;
    }
    // arboard hands back tightly-packed RGBA8; wrap it as a raster
    // skia image and run it through the same PNG encoder the export
    // path uses (no extra image-codec dependency).
    let row_bytes = width.checked_mul(4)?;
    let rgba_len = row_bytes.checked_mul(height)?;
    if image.bytes.len() < rgba_len {
        return None;
    }
    let pixel_width = u32::try_from(width).ok()?;
    let pixel_height = u32::try_from(height).ok()?;
    let info = skia_safe::ImageInfo::new(
        (i32::try_from(width).ok()?, i32::try_from(height).ok()?),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    let data = skia_safe::Data::new_copy(&image.bytes);
    let raster = skia_safe::images::raster_from_data(&info, data, row_bytes)?;
    let png = raster.encode(None, skia_safe::EncodedImageFormat::PNG, 100)?;
    Some(ClipboardImage {
        png: png.as_bytes().to_vec(),
        width: pixel_width,
        height: pixel_height,
    })
}

/// Read the system clipboard's HTML flavour (NSPasteboard
/// `public.html` / CF_HTML / text/html). `None` when the clipboard
/// holds no HTML — the Figma-paste path probes this before falling
/// back to the internal node clipboard.
#[allow(dead_code)] // Retained single-flavour API; paste uses target-aware APIs above.
pub fn get_html() -> Option<String> {
    arboard::Clipboard::new().ok()?.get().html().ok()
}

/// Write `text` to the system clipboard. Best-effort — an init or
/// set failure is swallowed.
pub fn set_text(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn native_clipboard_rgba_is_encoded_as_png() {
        let image = arboard::ImageData {
            width: 2,
            height: 1,
            bytes: Cow::Owned(vec![255, 0, 0, 255, 0, 255, 0, 128]),
        };

        let encoded = encode_image(image).expect("valid clipboard RGBA should encode");

        assert_eq!((encoded.width, encoded.height), (2, 1));
        assert!(encoded.png.starts_with(b"\x89PNG\r\n\x1a\n"));

        let raw_png = encoded.png.clone();
        let preserved = clipboard_image_from_png(raw_png.clone()).expect("valid encoded PNG");
        assert_eq!(preserved.png, raw_png);
        assert_eq!((preserved.width, preserved.height), (2, 1));
    }

    #[test]
    fn native_clipboard_rgba_rejects_truncated_rows() {
        let image = arboard::ImageData {
            width: 2,
            height: 1,
            bytes: Cow::Owned(vec![255, 0, 0, 255]),
        };

        assert!(encode_image(image).is_none());
    }

    #[test]
    fn raw_png_fast_path_rejects_truncated_or_invalid_headers() {
        assert_eq!(
            clipboard_image_from_png(b"\x89PNG\r\n\x1a\n".to_vec()),
            None
        );

        let mut zero_width = vec![0; 24];
        zero_width[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        zero_width[8..12].copy_from_slice(&13_u32.to_be_bytes());
        zero_width[12..16].copy_from_slice(b"IHDR");
        zero_width[20..24].copy_from_slice(&1_u32.to_be_bytes());
        assert_eq!(clipboard_image_from_png(zero_width), None);
    }
}
