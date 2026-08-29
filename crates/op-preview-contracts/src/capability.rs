//! Preview host capabilities (R4 `enter_with_capabilities`).
//!
//! The host declares WHAT it can actually do at session start; absence
//! is fail-closed. The legacy `PreviewSession::enter` wrapper supplies
//! an explicit all-false set, so a host that never declares capabilities
//! never gets effects silently allowed (the R3 effect queue reads this
//! same struct when it lands).

use serde::{Deserialize, Serialize};

/// One capability a Preview effect may require. The action→capability
/// mapping is fixed by the Preview action policy (R3); this enum only
/// names the requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewCapability {
    /// `open_url` — open an external link (`http`/`https`/`mailto`/`tel`).
    OpenUrl,
    /// `copy` — write text to the system clipboard.
    Clipboard,
    /// `share` — hand a payload to the platform share sheet.
    Share,
    /// `haptic` — trigger a platform haptic.
    Haptics,
    /// `dismiss_keyboard` — hide the platform soft keyboard.
    DismissKeyboard,
    /// `focus` / `blur` — move keyboard focus programmatically.
    Focus,
    /// `toast` / `alert` / `confirm` — platform presentation surfaces.
    Notifications,
}

/// The capability set a host declares at `enter` time.
///
/// Every field is explicit and the struct deliberately has NO `Default`:
/// a host must spell out what it supports (fail-closed), and a new
/// capability field added later is a compile break at every host
/// instead of a silently-missing declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewHostCapabilities {
    pub open_url: bool,
    pub clipboard: bool,
    pub share: bool,
    pub haptics: bool,
    pub dismiss_keyboard: bool,
    pub notifications: bool,
    pub focus: bool,
}

impl PreviewHostCapabilities {
    /// The all-false set the legacy `enter` wrapper supplies — absence
    /// of a declaration must never read as consent.
    pub const fn none() -> Self {
        Self {
            open_url: false,
            clipboard: false,
            share: false,
            haptics: false,
            dismiss_keyboard: false,
            notifications: false,
            focus: false,
        }
    }

    /// Whether the host declared support for `capability`.
    pub fn supports(&self, capability: PreviewCapability) -> bool {
        match capability {
            PreviewCapability::OpenUrl => self.open_url,
            PreviewCapability::Clipboard => self.clipboard,
            PreviewCapability::Share => self.share,
            PreviewCapability::Haptics => self.haptics,
            PreviewCapability::DismissKeyboard => self.dismiss_keyboard,
            PreviewCapability::Notifications => self.notifications,
            PreviewCapability::Focus => self.focus,
        }
    }
}
