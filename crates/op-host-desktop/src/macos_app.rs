//! macOS app identity for the non-bundled dev binary.
//!
//! Run bare (`./target/release/openpencil-desktop`), the app shows
//! up in the Dock as the raw executable name with a blank icon —
//! there is no `Info.plist` to read `CFBundleName` / the icon from.
//! [`apply`] sets both at runtime: the Dock / menu-bar name becomes
//! "OpenPencil" and the Dock tile gets the brand icon. A packaged
//! `.app` carries them in its bundle, so this is a dev-run nicety.

/// Set the running app's Dock name + icon. macOS only; a no-op
/// elsewhere. Call once at startup, on the main thread.
#[cfg(target_os = "macos")]
pub fn apply() {
    use objc2::ClassType;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::{MainThreadMarker, NSData, NSProcessInfo, NSString};

    // `apply` runs before the event loop, on the main thread.
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    // Dock / menu-bar name — overrides the `argv[0]` basename.
    let name = NSString::from_str("OpenPencil");
    unsafe { NSProcessInfo::processInfo().setProcessName(&name) };

    // Dock icon — decode the embedded PNG into an `NSImage`.
    let data = NSData::with_bytes(include_bytes!("../assets/icon.png"));
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        let app = NSApplication::sharedApplication(mtm);
        unsafe { app.setApplicationIconImage(Some(&image)) };
    }
}

/// No-op on non-macOS platforms — a packaged build carries the
/// name + icon in its own bundle / resources.
#[cfg(not(target_os = "macos"))]
pub fn apply() {}
