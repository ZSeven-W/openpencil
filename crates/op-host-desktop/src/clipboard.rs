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
#[allow(dead_code)] // Required typed single-image API; paste uses the snapshot API below.
pub fn get_image() -> Option<ClipboardImage> {
    let image = arboard::Clipboard::new().ok()?.get_image().ok()?;
    encode_image(image)
}

/// Snapshot all paste-relevant clipboard flavours through one OS
/// clipboard handle. The keyboard router turns this tuple into its
/// injectable `ClipboardPayload` seam.
pub(crate) fn read_paste_flavours() -> (Option<String>, Option<String>, Option<ClipboardImage>) {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return (None, None, None);
    };
    let text = clipboard.get_text().ok();
    let html = clipboard.get().html().ok();
    let image = clipboard.get_image().ok().and_then(encode_image);
    (text, html, image)
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
#[allow(dead_code)] // Retained single-flavour API; paste uses one shared OS handle.
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
