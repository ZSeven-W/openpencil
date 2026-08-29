//! Preview effect DTOs (R3) — the frozen host-facing contract for
//! effect requests and their completion results. `op-preview-core`
//! maps the engine's platform-neutral `EffectSink` requests onto these
//! DTOs; hosts consume and complete them WITHOUT touching the engine.

use crate::activation::UserActivationId;
use crate::capability::PreviewCapability;
use serde::{Deserialize, Serialize};

/// A host-performable effect, with its queue id and factual source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewEffect {
    OpenUrl {
        id: u64,
        url: String,
        source: EffectSource,
    },
    Copy {
        id: u64,
        text: String,
        source: EffectSource,
    },
    Share {
        id: u64,
        payload: SharePayload,
        source: EffectSource,
    },
    Haptic {
        id: u64,
        style: HapticStyle,
        source: EffectSource,
    },
    FocusNode {
        id: u64,
        node_id: String,
        source: EffectSource,
    },
    BlurFocus {
        id: u64,
        source: EffectSource,
    },
    DismissKeyboard {
        id: u64,
        source: EffectSource,
    },
    Toast {
        id: u64,
        message: String,
        source: EffectSource,
    },
    Alert {
        id: u64,
        title: String,
        message: String,
        source: EffectSource,
    },
    Confirm {
        id: u64,
        title: String,
        message: String,
        source: EffectSource,
    },
}

impl PreviewEffect {
    /// The queue id of this effect.
    pub fn id(&self) -> u64 {
        match self {
            PreviewEffect::OpenUrl { id, .. }
            | PreviewEffect::Copy { id, .. }
            | PreviewEffect::Share { id, .. }
            | PreviewEffect::Haptic { id, .. }
            | PreviewEffect::FocusNode { id, .. }
            | PreviewEffect::BlurFocus { id, .. }
            | PreviewEffect::DismissKeyboard { id, .. }
            | PreviewEffect::Toast { id, .. }
            | PreviewEffect::Alert { id, .. }
            | PreviewEffect::Confirm { id, .. } => *id,
        }
    }

    /// The stable effect-class name (for tracing / host dispatch).
    pub fn kind(&self) -> &'static str {
        match self {
            PreviewEffect::OpenUrl { .. } => "open_url",
            PreviewEffect::Copy { .. } => "copy",
            PreviewEffect::Share { .. } => "share",
            PreviewEffect::Haptic { .. } => "haptic",
            PreviewEffect::FocusNode { .. } => "focus",
            PreviewEffect::BlurFocus { .. } => "blur",
            PreviewEffect::DismissKeyboard { .. } => "dismiss_keyboard",
            PreviewEffect::Toast { .. } => "toast",
            PreviewEffect::Alert { .. } => "alert",
            PreviewEffect::Confirm { .. } => "confirm",
        }
    }

    /// The capability this effect requires.
    pub fn required_capability(&self) -> PreviewCapability {
        match self {
            PreviewEffect::OpenUrl { .. } => PreviewCapability::OpenUrl,
            PreviewEffect::Copy { .. } => PreviewCapability::Clipboard,
            PreviewEffect::Share { .. } => PreviewCapability::Share,
            PreviewEffect::Haptic { .. } => PreviewCapability::Haptics,
            PreviewEffect::FocusNode { .. } | PreviewEffect::BlurFocus { .. } => {
                PreviewCapability::Focus
            }
            PreviewEffect::DismissKeyboard { .. } => PreviewCapability::DismissKeyboard,
            PreviewEffect::Toast { .. }
            | PreviewEffect::Alert { .. }
            | PreviewEffect::Confirm { .. } => PreviewCapability::Notifications,
        }
    }

    /// The factual source of this effect.
    pub fn source(&self) -> &EffectSource {
        match self {
            PreviewEffect::OpenUrl { source, .. }
            | PreviewEffect::Copy { source, .. }
            | PreviewEffect::Share { source, .. }
            | PreviewEffect::Haptic { source, .. }
            | PreviewEffect::FocusNode { source, .. }
            | PreviewEffect::BlurFocus { source, .. }
            | PreviewEffect::DismissKeyboard { source, .. }
            | PreviewEffect::Toast { source, .. }
            | PreviewEffect::Alert { source, .. }
            | PreviewEffect::Confirm { source, .. } => source,
        }
    }
}

/// Where an effect came from. Every field is factual: the handler-owner
/// node, the event handler key that spawned the chain, the
/// host-certified activation (absent when the input carried none), and
/// the capability class the host must support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectSource {
    pub node_id: String,
    pub event: String,
    pub activation: Option<UserActivationId>,
    pub required_capability: PreviewCapability,
}

/// Share-sheet payload (R3). Hosts interpret the combination; absent
/// fields stay absent.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharePayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Platform haptic style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HapticStyle {
    Light,
    Medium,
    Heavy,
    Success,
    Warning,
    Error,
}

impl HapticStyle {
    /// Parse an authored style name; unknown names default to Medium
    /// (never guessed into a different semantic).
    pub fn from_authored(name: &str) -> Self {
        match name {
            "light" => HapticStyle::Light,
            "heavy" => HapticStyle::Heavy,
            "success" => HapticStyle::Success,
            "warning" => HapticStyle::Warning,
            "error" => HapticStyle::Error,
            _ => HapticStyle::Medium,
        }
    }
}

/// Why an effect failed on the host side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum PreviewEffectFailureCode {
    InvalidPayload,
    InvalidUrlScheme,
    PermissionDenied,
    ActivationExpired,
    PresentationFailed,
    PlatformFailure,
}

/// A structured effect failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewEffectFailure {
    pub code: PreviewEffectFailureCode,
    pub detail: Option<String>,
}

/// The host's completion result for one effect. `Unsupported` means the
/// host cannot perform this effect CLASS at all (fail-closed when the
/// capability was not declared).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreviewEffectResult {
    Success,
    Cancelled,
    Unsupported,
    Failed(PreviewEffectFailure),
}
