//! Thin wrapper over the OS clipboard.
//!
//! Backs the AI chat input's Cmd+C / Cmd+V / Cmd+X. `arboard` wraps
//! the platform clipboard (NSPasteboard on macOS, the Win32
//! clipboard, X11 / Wayland on Linux); all calls are best-effort —
//! a clipboard that fails to initialise simply yields `None` / a
//! no-op rather than surfacing an error to the user.

/// Read the system clipboard's text. `None` when the clipboard holds
/// no text or could not be opened.
pub fn get_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Read the system clipboard's image flavour (NSPasteboard TIFF/PNG,
/// CF_DIB, X11/Wayland image targets) re-encoded as PNG bytes.
/// `None` when the clipboard holds no image, or the bitmap can't be
/// wrapped / encoded. Backs paste-image-into-chat — the desktop
/// equivalent of the TS chat input's clipboard *files* surface
/// (`ai-chat-input.tsx:85-94`).
pub fn get_image() -> Option<Vec<u8>> {
    let image = arboard::Clipboard::new().ok()?.get_image().ok()?;
    let (width, height) = (image.width, image.height);
    if width == 0 || height == 0 {
        return None;
    }
    // arboard hands back tightly-packed RGBA8; wrap it as a raster
    // skia image and run it through the same PNG encoder the export
    // path uses (no extra image-codec dependency).
    if image.bytes.len() < width * height * 4 {
        return None;
    }
    let info = skia_safe::ImageInfo::new(
        (width as i32, height as i32),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    let data = skia_safe::Data::new_copy(&image.bytes);
    let raster = skia_safe::images::raster_from_data(&info, data, width * 4)?;
    let png = raster.encode(None, skia_safe::EncodedImageFormat::PNG, 100)?;
    Some(png.as_bytes().to_vec())
}

/// Read the system clipboard's HTML flavour (NSPasteboard
/// `public.html` / CF_HTML / text/html). `None` when the clipboard
/// holds no HTML — the Figma-paste path probes this before falling
/// back to the internal node clipboard.
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
