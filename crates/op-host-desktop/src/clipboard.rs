//! Thin wrapper over the OS text clipboard.
//!
//! Backs the AI chat input's Cmd+C / Cmd+V / Cmd+X. `arboard` wraps
//! the platform clipboard (NSPasteboard on macOS, the Win32
//! clipboard, X11 / Wayland on Linux); both calls are best-effort —
//! a clipboard that fails to initialise simply yields `None` / a
//! no-op rather than surfacing an error to the user.

/// Read the system clipboard's text. `None` when the clipboard holds
/// no text or could not be opened.
pub fn get_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Write `text` to the system clipboard. Best-effort — an init or
/// set failure is swallowed.
pub fn set_text(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text.to_owned());
    }
}
