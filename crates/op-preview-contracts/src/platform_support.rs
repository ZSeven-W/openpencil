//! Platform support authoring table (R3) — the approved
//! Complete / Adapted / Unsupported matrix the Preview UI (badges) and
//! the host adapters verify against, instead of each layer inventing
//! its own claims. FROZEN: H7 verifies live adapters against this table.

use serde::{Deserialize, Serialize};

/// How completely a platform supports one interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostSupport {
    /// Fully equivalent on every device class.
    Complete,
    /// Available through the approved adaptation (see the table rows).
    Adapted,
    /// Not available; the authoring surface must badge it as such.
    Unsupported,
}

/// The Preview interaction vocabulary the table classifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewInteraction {
    Tap,
    DoubleTap,
    LongPress,
    Press,
    Pan,
    Swipe,
    Scale,
    Rotate,
    Hover,
    ContextMenu,
    Key,
    TextInput,
    Ime,
    Scroll,
    Back,
}

/// The platform a capability question is asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewPlatform {
    /// Native desktop / mobile hosts (op-host-native family).
    Native,
    /// The browser-backed host (op-host-web).
    Web,
}

/// The approved support level for `interaction` on `platform`.
///
/// The two APPROVED ADAPTATIONS (contract §4.3 / design §5.2):
///
/// - `Hover` is ADAPTED on touch platforms: an authored hover handler
///   falls back to Pressed/Focus state styling — touch never produces
///   real hover traffic (the R4 interaction state enforces this).
/// - `ContextMenu` is ADAPTED on touch: an authored `onContextMenu`
///   falls back to the exclusive touch LongPress rule (right-click keeps
///   the direct mapping).
///
/// Everything else is Complete on both platforms until a contract
/// amendment says otherwise — `Unsupported` rows are added with the
/// platform adapter that documents them, never speculatively.
pub fn platform_support(
    _platform: PreviewPlatform,
    interaction: PreviewInteraction,
) -> HostSupport {
    match interaction {
        PreviewInteraction::Hover | PreviewInteraction::ContextMenu => HostSupport::Adapted,
        _ => HostSupport::Complete,
    }
}
